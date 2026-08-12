//! Semantic Skill Matching
//!
//! Uses sentence embeddings to match job requirements to user skills semantically.

use super::embeddings::EmbeddingGenerator;
use super::model::ModelManager;
use super::{Qwen3EmbeddingBackend, Qwen3RerankerBackend};
use anyhow::Result;
use jobsentinel_security::{
    contains_instruction_override_phrase, contains_review_required_invisible_control,
};
use std::path::PathBuf;

mod deterministic;
mod legacy;
mod qwen3;
mod shared;

use qwen3::Qwen3SemanticRuntime;

const LOCAL_MATCH_INPUT_REVIEW_REQUIRED: &str = "local matching input requires review";

pub fn validate_resume_embeddings(
    expected_count: usize,
    expected_dimension: usize,
    embeddings: &[Vec<f32>],
) -> Result<()> {
    shared::validate_resume_embeddings(expected_count, expected_dimension, embeddings)
}

pub fn validate_local_matching_inputs(
    user_skills: &[String],
    job_requirements: &[String],
) -> Result<()> {
    validate_local_matching_input(
        user_skills
            .iter()
            .chain(job_requirements)
            .map(String::as_str),
    )
}

/// Local runtime that produced a match result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRuntimeProfile {
    Qwen3Reranked,
    #[serde(rename = "minilm")]
    MiniLm,
    DeterministicExact,
}

/// Bounded explanation for why one requirement remains unmatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticUnmatchedReason {
    NoExactEvidence,
    BelowRetrievalThreshold,
    BelowRerankerAcceptance,
}

/// Safe requirement-level diagnostic without candidate or model content.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnmatchedRequirementDiagnostic {
    pub requirement: String,
    pub reason: SemanticUnmatchedReason,
}

/// Result of semantic skill matching
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticMatchResult {
    pub runtime_profile: SemanticRuntimeProfile,

    /// Overall semantic match score (0.0-1.0)
    pub overall_score: f64,

    /// Matched skills with similarity scores
    pub matched_skills: Vec<SkillMatch>,

    /// Job requirements that didn't match any user skills
    pub unmatched_requirements: Vec<String>,

    /// Stable reason for each unmatched requirement, in requirement order.
    #[serde(default)]
    pub unmatched_diagnostics: Vec<UnmatchedRequirementDiagnostic>,

    /// User skills that didn't match any job requirements
    pub unused_skills: Vec<String>,
}

/// Individual skill match with similarity score
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMatch {
    pub job_skill: String,
    pub user_skill: String,
    /// Dense cosine similarity or deterministic exact score.
    pub similarity: f32,
    /// Raw Qwen3 reranker score, meaningful only within the same query kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranker_score: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reranker_rank: Option<usize>,
}

/// Semantic matcher for skill comparison
pub struct SemanticMatcher {
    runtime: SemanticMatcherRuntime,
}

enum SemanticMatcherRuntime {
    Qwen3(Box<Qwen3SemanticRuntime>),
    Legacy(Box<EmbeddingGenerator>),
    Deterministic,
    #[cfg(test)]
    RejectOnly,
}

impl SemanticMatcher {
    /// Create new semantic matcher
    pub fn new(app_data_dir: PathBuf) -> Result<Self> {
        let manager = ModelManager::new(app_data_dir.clone());
        if manager.is_default_semantic_runtime_downloaded() {
            let embedding = Qwen3EmbeddingBackend::new(app_data_dir.clone())?;
            let reranker = Qwen3RerankerBackend::new(app_data_dir)?;
            return Ok(Self {
                runtime: SemanticMatcherRuntime::Qwen3(Box::new(Qwen3SemanticRuntime::new(
                    embedding, reranker,
                ))),
            });
        }

        if manager.is_model_downloaded() {
            let generator = EmbeddingGenerator::new(app_data_dir)?;
            return Ok(Self {
                runtime: SemanticMatcherRuntime::Legacy(Box::new(generator)),
            });
        }
        Ok(Self {
            runtime: SemanticMatcherRuntime::Deterministic,
        })
    }

    /// Match user skills against job requirements semantically
    pub fn match_skills(
        &self,
        user_skills: &[String],
        job_requirements: &[String],
    ) -> Result<SemanticMatchResult> {
        self.match_skills_inner(user_skills, job_requirements, None)
    }

    pub fn match_skills_with_resume_vectors(
        &self,
        user_skills: &[String],
        job_requirements: &[String],
        resume_vectors: &[Vec<f32>],
    ) -> Result<SemanticMatchResult> {
        self.match_skills_inner(user_skills, job_requirements, Some(resume_vectors))
    }

    pub fn uses_persistent_resume_vectors(&self) -> bool {
        matches!(self.runtime, SemanticMatcherRuntime::Qwen3(_))
    }

    pub fn embed_resume_chunks(&self, chunks: &[String]) -> Result<Vec<Vec<f32>>> {
        validate_local_matching_input(chunks.iter().map(String::as_str))?;
        match &self.runtime {
            SemanticMatcherRuntime::Qwen3(runtime) => runtime.embed_resume_chunks(chunks),
            _ => anyhow::bail!("persistent resume vectors require the governed Qwen3 runtime"),
        }
    }

    fn match_skills_inner(
        &self,
        user_skills: &[String],
        job_requirements: &[String],
        resume_vectors: Option<&[Vec<f32>]>,
    ) -> Result<SemanticMatchResult> {
        validate_local_matching_inputs(user_skills, job_requirements)?;
        match &self.runtime {
            SemanticMatcherRuntime::Qwen3(runtime) => runtime.match_skills_with_resume_vectors(
                user_skills,
                job_requirements,
                resume_vectors,
            ),
            SemanticMatcherRuntime::Legacy(generator) => {
                if resume_vectors.is_some() {
                    anyhow::bail!("persistent resume vectors require the governed Qwen3 runtime");
                }
                legacy::match_skills(generator.as_ref(), user_skills, job_requirements)
            }
            SemanticMatcherRuntime::Deterministic => {
                if resume_vectors.is_some() {
                    anyhow::bail!("persistent resume vectors require the governed Qwen3 runtime");
                }
                deterministic::match_skills(user_skills, job_requirements)
            }
            #[cfg(test)]
            SemanticMatcherRuntime::RejectOnly => {
                unreachable!("unsafe matching input reached runtime dispatch")
            }
        }
    }

    /// Find similar skills to a query skill
    pub fn find_similar_skills(
        &self,
        query_skill: &str,
        candidate_skills: &[String],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>> {
        validate_local_matching_input(
            std::iter::once(query_skill).chain(candidate_skills.iter().map(String::as_str)),
        )?;
        match &self.runtime {
            SemanticMatcherRuntime::Qwen3(runtime) => {
                runtime.find_similar_skills(query_skill, candidate_skills, top_k)
            }
            SemanticMatcherRuntime::Legacy(generator) => legacy::find_similar_skills(
                generator.as_ref(),
                query_skill,
                candidate_skills,
                top_k,
            ),
            SemanticMatcherRuntime::Deterministic => {
                deterministic::find_similar_skills(query_skill, candidate_skills, top_k)
            }
            #[cfg(test)]
            SemanticMatcherRuntime::RejectOnly => {
                unreachable!("unsafe matching input reached runtime dispatch")
            }
        }
    }
}

fn validate_local_matching_input<'a>(values: impl IntoIterator<Item = &'a str>) -> Result<()> {
    if values.into_iter().any(|value| {
        contains_review_required_invisible_control(value)
            || contains_instruction_override_phrase(value)
    }) {
        anyhow::bail!(LOCAL_MATCH_INPUT_REVIEW_REQUIRED);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
