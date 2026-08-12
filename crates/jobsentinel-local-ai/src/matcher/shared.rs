use super::{
    SemanticMatchResult, SemanticRuntimeProfile, SemanticUnmatchedReason, SkillMatch,
    UnmatchedRequirementDiagnostic,
};
use crate::manifest::load_model_manifest;
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::HashSet;

/// Similarity threshold for considering skills as matching.
pub(super) const SIMILARITY_THRESHOLD: f32 = 0.7;
pub(super) const QWEN3_RERANK_TOP_K: usize = 25;
pub(super) const QWEN3_REQUIREMENT_INSTRUCTION: &str =
    "Retrieve resume skills or passages that provide concrete evidence for this job requirement.";

pub(super) fn dense_candidates(
    query_embedding: &[f32],
    candidate_embeddings: &[Vec<f32>],
    threshold: f32,
    top_k: usize,
) -> Result<Vec<(usize, f32)>> {
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        anyhow::bail!("invalid local matching threshold");
    }
    let mut candidates = Vec::new();
    for (candidate_idx, candidate_embedding) in candidate_embeddings.iter().enumerate() {
        let similarity = checked_cosine_similarity(query_embedding, candidate_embedding)?;
        if similarity >= threshold {
            candidates.push((candidate_idx, similarity));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    candidates.truncate(top_k);
    Ok(candidates)
}

pub(super) fn checked_cosine_similarity(left: &[f32], right: &[f32]) -> Result<f32> {
    if left.is_empty()
        || left.len() != right.len()
        || left.iter().chain(right).any(|value| !value.is_finite())
    {
        anyhow::bail!("invalid local embedding output");
    }
    let (dot, left_norm, right_norm) =
        left.iter()
            .zip(right)
            .fold((0.0_f64, 0.0_f64, 0.0_f64), |totals, (left, right)| {
                let left = f64::from(*left);
                let right = f64::from(*right);
                (
                    totals.0 + left * right,
                    totals.1 + left * left,
                    totals.2 + right * right,
                )
            });
    if !left_norm.is_finite() || !right_norm.is_finite() || left_norm <= 0.0 || right_norm <= 0.0 {
        anyhow::bail!("invalid local embedding output");
    }
    let similarity = dot / (left_norm.sqrt() * right_norm.sqrt());
    if !similarity.is_finite() {
        anyhow::bail!("invalid local matching score");
    }
    Ok(similarity.clamp(-1.0, 1.0) as f32)
}

pub(super) fn require_embedding_count(expected: usize, actual: usize) -> Result<()> {
    if expected != actual {
        anyhow::bail!("invalid local embedding output");
    }
    Ok(())
}

pub(super) fn validate_resume_embeddings(
    expected_count: usize,
    expected_dimension: usize,
    embeddings: &[Vec<f32>],
) -> Result<()> {
    require_embedding_count(expected_count, embeddings.len())?;
    if expected_dimension == 0
        || embeddings.iter().any(|embedding| {
            if embedding.len() != expected_dimension
                || embedding.iter().any(|value| !value.is_finite())
            {
                return true;
            }
            let norm = embedding
                .iter()
                .map(|value| f64::from(*value).powi(2))
                .sum::<f64>()
                .sqrt();
            !norm.is_finite() || (norm - 1.0).abs() > 0.001
        })
    {
        anyhow::bail!("invalid local embedding output");
    }
    Ok(())
}

pub(super) fn build_match_result(
    runtime_profile: SemanticRuntimeProfile,
    user_skills: &[String],
    job_requirements: &[String],
    matched_skills: Vec<SkillMatch>,
    unmatched_reasons: Vec<Option<SemanticUnmatchedReason>>,
    matched_job_indices: HashSet<usize>,
    matched_user_indices: HashSet<usize>,
) -> Result<SemanticMatchResult> {
    if unmatched_reasons.len() != job_requirements.len() {
        anyhow::bail!("invalid local matching diagnostics");
    }
    let coverage = matched_job_indices.len() as f64 / job_requirements.len() as f64;
    let avg_similarity = if matched_skills.is_empty() {
        0.0
    } else {
        matched_skills
            .iter()
            .map(|m| m.similarity as f64)
            .sum::<f64>()
            / matched_skills.len() as f64
    };
    let overall_score = coverage * 0.7 + avg_similarity * 0.3;

    let unmatched_diagnostics = job_requirements
        .iter()
        .enumerate()
        .filter_map(|(idx, requirement)| {
            let matched = matched_job_indices.contains(&idx);
            match (matched, unmatched_reasons[idx]) {
                (true, None) => None,
                (false, Some(reason)) => Some(Ok(UnmatchedRequirementDiagnostic {
                    requirement: requirement.clone(),
                    reason,
                })),
                _ => Some(Err(anyhow::anyhow!("invalid local matching diagnostics"))),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let unmatched_requirements = unmatched_diagnostics
        .iter()
        .map(|diagnostic| diagnostic.requirement.clone())
        .collect();

    let unused_skills: Vec<String> = user_skills
        .iter()
        .enumerate()
        .filter(|(idx, _)| !matched_user_indices.contains(idx))
        .map(|(_, skill)| skill.clone())
        .collect();

    Ok(SemanticMatchResult {
        runtime_profile,
        overall_score,
        matched_skills,
        unmatched_requirements,
        unmatched_diagnostics,
        unused_skills,
    })
}

pub(super) fn empty_match_result(
    runtime_profile: SemanticRuntimeProfile,
    user_skills: &[String],
    job_requirements: &[String],
    reason: SemanticUnmatchedReason,
) -> SemanticMatchResult {
    SemanticMatchResult {
        runtime_profile,
        overall_score: 0.0,
        matched_skills: Vec::new(),
        unmatched_requirements: job_requirements.to_vec(),
        unmatched_diagnostics: job_requirements
            .iter()
            .map(|requirement| UnmatchedRequirementDiagnostic {
                requirement: requirement.clone(),
                reason,
            })
            .collect(),
        unused_skills: user_skills.to_vec(),
    }
}

pub(super) fn qwen3_match_threshold() -> f32 {
    load_model_manifest()
        .ok()
        .and_then(|manifest| {
            manifest
                .thresholds
                .get("resume_requirement")
                .map(|thresholds| thresholds.retrieval)
        })
        .unwrap_or(SIMILARITY_THRESHOLD)
}

pub(super) fn qwen3_reranker_acceptance_threshold() -> f32 {
    load_model_manifest()
        .ok()
        .and_then(|manifest| {
            manifest
                .reranker_acceptance
                .get("resume_requirement")
                .copied()
        })
        .unwrap_or(f32::INFINITY)
}
