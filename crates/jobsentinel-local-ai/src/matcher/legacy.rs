use super::shared::{
    build_match_result, checked_cosine_similarity, dense_candidates, empty_match_result,
    require_embedding_count, SIMILARITY_THRESHOLD,
};
use super::{SemanticMatchResult, SemanticRuntimeProfile, SemanticUnmatchedReason, SkillMatch};
use crate::embeddings::EmbeddingGenerator;
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::HashSet;

pub(super) fn match_skills(
    generator: &EmbeddingGenerator,
    user_skills: &[String],
    job_requirements: &[String],
) -> Result<SemanticMatchResult> {
    if user_skills.is_empty() || job_requirements.is_empty() {
        return Ok(empty_match_result(
            SemanticRuntimeProfile::MiniLm,
            user_skills,
            job_requirements,
            SemanticUnmatchedReason::BelowRetrievalThreshold,
        ));
    }

    let user_texts: Vec<&str> = user_skills.iter().map(|s| s.as_str()).collect();
    let job_texts: Vec<&str> = job_requirements.iter().map(|s| s.as_str()).collect();

    let user_embeddings = generator.embed_batch(&user_texts)?;
    let job_embeddings = generator.embed_batch(&job_texts)?;
    require_embedding_count(user_skills.len(), user_embeddings.len())?;
    require_embedding_count(job_requirements.len(), job_embeddings.len())?;

    let user_embeddings: Vec<Vec<f32>> = user_embeddings
        .iter()
        .map(|e| EmbeddingGenerator::normalize_embedding(e))
        .collect();

    let job_embeddings: Vec<Vec<f32>> = job_embeddings
        .iter()
        .map(|e| EmbeddingGenerator::normalize_embedding(e))
        .collect();

    let mut matched_skills = Vec::new();
    let mut matched_job_indices = HashSet::new();
    let mut matched_user_indices = HashSet::new();
    let mut unmatched_reasons =
        vec![Some(SemanticUnmatchedReason::BelowRetrievalThreshold); job_requirements.len()];

    for (job_idx, job_emb) in job_embeddings.iter().enumerate() {
        let best_match = dense_candidates(job_emb, &user_embeddings, SIMILARITY_THRESHOLD, 1)?
            .into_iter()
            .next();

        if let Some((user_idx, similarity)) = best_match {
            matched_skills.push(SkillMatch {
                job_skill: job_requirements[job_idx].clone(),
                user_skill: user_skills[user_idx].clone(),
                similarity,
                reranker_score: None,
                reranker_rank: None,
            });
            matched_job_indices.insert(job_idx);
            matched_user_indices.insert(user_idx);
            unmatched_reasons[job_idx] = None;
        }
    }

    build_match_result(
        SemanticRuntimeProfile::MiniLm,
        user_skills,
        job_requirements,
        matched_skills,
        unmatched_reasons,
        matched_job_indices,
        matched_user_indices,
    )
}

pub(super) fn find_similar_skills(
    generator: &EmbeddingGenerator,
    query_skill: &str,
    candidate_skills: &[String],
    top_k: usize,
) -> Result<Vec<(String, f32)>> {
    if candidate_skills.is_empty() {
        return Ok(Vec::new());
    }

    let query_embedding = generator.embed_text(query_skill)?;
    let query_embedding = EmbeddingGenerator::normalize_embedding(&query_embedding);

    let candidate_texts: Vec<&str> = candidate_skills.iter().map(|s| s.as_str()).collect();
    let candidate_embeddings = generator.embed_batch(&candidate_texts)?;
    require_embedding_count(candidate_skills.len(), candidate_embeddings.len())?;

    let mut similarities = candidate_embeddings
        .iter()
        .enumerate()
        .map(|(idx, emb)| {
            let normalized = EmbeddingGenerator::normalize_embedding(emb);
            Ok((
                candidate_skills[idx].clone(),
                checked_cosine_similarity(&query_embedding, &normalized)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    similarities.truncate(top_k);

    Ok(similarities)
}
