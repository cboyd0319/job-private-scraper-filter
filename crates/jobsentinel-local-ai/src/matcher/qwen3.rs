use super::shared::{
    build_match_result, checked_cosine_similarity, dense_candidates, empty_match_result,
    qwen3_match_threshold, qwen3_reranker_acceptance_threshold, require_embedding_count,
    validate_resume_embeddings, QWEN3_REQUIREMENT_INSTRUCTION, QWEN3_RERANK_TOP_K,
};
use super::{SemanticMatchResult, SemanticRuntimeProfile, SemanticUnmatchedReason, SkillMatch};
use crate::runtime::{
    EmbeddingBackend, EmbeddingInput, EmbeddingInputKind, RerankCandidate, RerankQuery,
    RerankQueryKind, RerankScore, RerankerBackend,
};
use crate::{Qwen3EmbeddingBackend, Qwen3RerankerBackend};
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::HashSet;

#[cfg(test)]
mod tests;

pub(super) struct Qwen3SemanticRuntime {
    embedding: Qwen3EmbeddingBackend,
    reranker: Qwen3RerankerBackend,
    threshold: f32,
    reranker_acceptance: f32,
}

#[derive(Debug, PartialEq)]
struct RerankedMatch {
    user_index: usize,
    dense_score: f32,
    reranker_score: f32,
    reranker_rank: usize,
}

fn select_reranked_match(
    dense_candidates: &[(usize, f32)],
    scores: &[RerankScore],
) -> Result<Option<RerankedMatch>> {
    if dense_candidates.len() != scores.len() {
        anyhow::bail!("invalid reranker output");
    }

    let mut dense_ids = HashSet::new();
    for (candidate_id, dense_score) in dense_candidates {
        if !dense_score.is_finite()
            || !(-1.0..=1.0).contains(dense_score)
            || !dense_ids.insert(*candidate_id)
        {
            anyhow::bail!("invalid reranker output");
        }
    }

    let mut score_ids = HashSet::new();
    let mut previous: Option<&RerankScore> = None;
    let mut selected = None;
    for (position, score) in scores.iter().enumerate() {
        if !score.score.is_finite()
            || score.rank != position + 1
            || !score_ids.insert(score.candidate_id.as_str())
        {
            anyhow::bail!("invalid reranker output");
        }
        if previous.is_some_and(|prior| match score.score.total_cmp(&prior.score) {
            Ordering::Greater => true,
            Ordering::Equal => score.candidate_id < prior.candidate_id,
            Ordering::Less => false,
        }) {
            anyhow::bail!("invalid reranker output");
        }
        let Some((user_index, dense_score)) = dense_candidates
            .iter()
            .find(|(candidate_id, _)| candidate_id.to_string() == score.candidate_id)
        else {
            anyhow::bail!("invalid reranker output");
        };
        if position == 0 {
            selected = Some(RerankedMatch {
                user_index: *user_index,
                dense_score: *dense_score,
                reranker_score: score.score,
                reranker_rank: score.rank,
            });
        }
        previous = Some(score);
    }

    Ok(selected)
}

fn apply_reranker_acceptance(
    selected: Option<RerankedMatch>,
    minimum_score: f32,
) -> Result<Option<RerankedMatch>> {
    if !minimum_score.is_finite() {
        anyhow::bail!("invalid reranker acceptance threshold");
    }
    Ok(selected.filter(|candidate| candidate.reranker_score >= minimum_score))
}

fn qwen_unmatched_reason(
    dense_candidates: &[(usize, f32)],
    selected: Option<&RerankedMatch>,
) -> Option<SemanticUnmatchedReason> {
    if selected.is_some() {
        None
    } else if dense_candidates.is_empty() {
        Some(SemanticUnmatchedReason::BelowRetrievalThreshold)
    } else {
        Some(SemanticUnmatchedReason::BelowRerankerAcceptance)
    }
}

impl Qwen3SemanticRuntime {
    pub(super) fn new(embedding: Qwen3EmbeddingBackend, reranker: Qwen3RerankerBackend) -> Self {
        Self {
            embedding,
            reranker,
            threshold: qwen3_match_threshold(),
            reranker_acceptance: qwen3_reranker_acceptance_threshold(),
        }
    }

    pub(super) fn embed_resume_chunks(&self, chunks: &[String]) -> Result<Vec<Vec<f32>>> {
        let inputs = chunks
            .iter()
            .map(|text| EmbeddingInput {
                text: text.clone(),
                instruction: None,
                input_kind: EmbeddingInputKind::ResumeChunk,
            })
            .collect::<Vec<_>>();
        let embeddings = self.embedding.embed_documents(&inputs)?;
        validate_resume_embeddings(chunks.len(), self.embedding.dimension(), &embeddings)?;
        Ok(embeddings)
    }

    pub(super) fn match_skills_with_resume_vectors(
        &self,
        user_skills: &[String],
        job_requirements: &[String],
        resume_vectors: Option<&[Vec<f32>]>,
    ) -> Result<SemanticMatchResult> {
        if user_skills.is_empty() || job_requirements.is_empty() {
            return Ok(empty_match_result(
                SemanticRuntimeProfile::Qwen3Reranked,
                user_skills,
                job_requirements,
                SemanticUnmatchedReason::BelowRetrievalThreshold,
            ));
        }

        let generated;
        let user_embeddings = if let Some(resume_vectors) = resume_vectors {
            validate_resume_embeddings(
                user_skills.len(),
                self.embedding.dimension(),
                resume_vectors,
            )?;
            resume_vectors
        } else {
            generated = self.embed_resume_chunks(user_skills)?;
            &generated
        };

        let job_inputs: Vec<EmbeddingInput> = job_requirements
            .iter()
            .map(|requirement| EmbeddingInput {
                text: requirement.clone(),
                instruction: Some(QWEN3_REQUIREMENT_INSTRUCTION.to_string()),
                input_kind: EmbeddingInputKind::Requirement,
            })
            .collect();
        let job_embeddings = job_inputs
            .iter()
            .map(|input| self.embedding.embed_query(input))
            .collect::<Result<Vec<_>>>()?;

        let mut matched_skills = Vec::new();
        let mut matched_job_indices = HashSet::new();
        let mut matched_user_indices = HashSet::new();
        let mut unmatched_reasons = vec![None; job_requirements.len()];

        for (job_idx, job_emb) in job_embeddings.iter().enumerate() {
            let dense_candidates =
                dense_candidates(job_emb, user_embeddings, self.threshold, QWEN3_RERANK_TOP_K)?;

            let selected = self.reranked_best_match(
                job_idx,
                job_requirements,
                user_skills,
                &dense_candidates,
            )?;
            unmatched_reasons[job_idx] =
                qwen_unmatched_reason(&dense_candidates, selected.as_ref());
            if let Some(selected) = selected {
                matched_skills.push(SkillMatch {
                    job_skill: job_requirements[job_idx].clone(),
                    user_skill: user_skills[selected.user_index].clone(),
                    similarity: selected.dense_score,
                    reranker_score: Some(selected.reranker_score),
                    reranker_rank: Some(selected.reranker_rank),
                });
                matched_job_indices.insert(job_idx);
                matched_user_indices.insert(selected.user_index);
            }
        }

        build_match_result(
            SemanticRuntimeProfile::Qwen3Reranked,
            user_skills,
            job_requirements,
            matched_skills,
            unmatched_reasons,
            matched_job_indices,
            matched_user_indices,
        )
    }

    fn reranked_best_match(
        &self,
        job_idx: usize,
        job_requirements: &[String],
        user_skills: &[String],
        dense_candidates: &[(usize, f32)],
    ) -> Result<Option<RerankedMatch>> {
        if dense_candidates.is_empty() {
            return Ok(None);
        }

        let candidates: Vec<RerankCandidate> = dense_candidates
            .iter()
            .map(|(user_idx, similarity)| RerankCandidate {
                id: user_idx.to_string(),
                text: user_skills[*user_idx].clone(),
                metadata: serde_json::json!({ "dense_similarity": similarity }),
            })
            .collect();
        let scores = self.reranker.rerank(
            &RerankQuery {
                text: job_requirements[job_idx].clone(),
                instruction: Some(QWEN3_REQUIREMENT_INSTRUCTION.to_string()),
                query_kind: RerankQueryKind::ResumeRequirement,
            },
            &candidates,
        )?;

        apply_reranker_acceptance(
            select_reranked_match(dense_candidates, &scores)?,
            self.reranker_acceptance,
        )
    }

    pub(super) fn find_similar_skills(
        &self,
        query_skill: &str,
        candidate_skills: &[String],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        if candidate_skills.is_empty() {
            return Ok(Vec::new());
        }

        let query_embedding = self.embedding.embed_query(&EmbeddingInput {
            text: query_skill.to_string(),
            instruction: Some(QWEN3_REQUIREMENT_INSTRUCTION.to_string()),
            input_kind: EmbeddingInputKind::Requirement,
        })?;
        let candidate_inputs: Vec<EmbeddingInput> = candidate_skills
            .iter()
            .map(|skill| EmbeddingInput {
                text: skill.clone(),
                instruction: None,
                input_kind: EmbeddingInputKind::ResumeChunk,
            })
            .collect();
        let candidate_embeddings = self.embedding.embed_documents(&candidate_inputs)?;
        require_embedding_count(candidate_skills.len(), candidate_embeddings.len())?;

        let mut similarities = candidate_embeddings
            .iter()
            .enumerate()
            .map(|(idx, emb)| {
                Ok((
                    candidate_skills[idx].clone(),
                    checked_cosine_similarity(&query_embedding, emb)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        similarities.truncate(top_k);

        Ok(similarities)
    }
}
