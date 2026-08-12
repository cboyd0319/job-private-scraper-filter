//! Local ML runtime boundaries and provenance contracts.

use super::manifest::ModelSpec;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingInputKind {
    Query,
    Document,
    Summary,
    Requirement,
    ResumeChunk,
    JobChunk,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingInput {
    pub text: String,
    pub instruction: Option<String>,
    pub input_kind: EmbeddingInputKind,
}

pub trait EmbeddingBackend: Send + Sync {
    fn model_id(&self) -> &str;
    fn dimension(&self) -> usize;
    fn compatibility(&self) -> &RuntimeCompatibility;
    fn embed_documents(&self, docs: &[EmbeddingInput]) -> Result<Vec<Vec<f32>>>;
    fn embed_query(&self, query: &EmbeddingInput) -> Result<Vec<f32>>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RerankQueryKind {
    ResumeRequirement,
    JobSearch,
    SkillEvidence,
    TitleSeniority,
    GapAnalysis,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RerankQuery {
    pub text: String,
    pub instruction: Option<String>,
    pub query_kind: RerankQueryKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RerankCandidate {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RerankScore {
    pub candidate_id: String,
    pub score: f32,
    pub rank: usize,
}

pub trait RerankerBackend: Send + Sync {
    fn model_id(&self) -> &str;
    fn compatibility(&self) -> &RuntimeCompatibility;
    fn rerank(
        &self,
        query: &RerankQuery,
        candidates: &[RerankCandidate],
    ) -> Result<Vec<RerankScore>>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeCompatibility {
    pub model_id: String,
    pub backend: String,
    pub expected_dim: Option<usize>,
    pub actual_dim: Option<usize>,
    pub max_tokens: usize,
    pub tokenizer_family: String,
    pub tokenizer_sha256: String,
    pub pooling: String,
    pub normalization: String,
    pub supports_instruction: bool,
}

impl RuntimeCompatibility {
    pub fn from_spec(
        spec: &ModelSpec,
        backend: impl Into<String>,
        actual_dim: Option<usize>,
    ) -> Self {
        let tokenizer_sha256 = spec
            .file("tokenizer.json")
            .map(|file| file.sha256.clone())
            .unwrap_or_default();

        Self {
            model_id: spec.id.clone(),
            backend: backend.into(),
            expected_dim: spec.dimension,
            actual_dim,
            max_tokens: spec.max_tokens,
            tokenizer_family: spec.tokenizer_family.clone(),
            tokenizer_sha256,
            pooling: spec.pooling.clone(),
            normalization: spec.normalization.clone(),
            supports_instruction: spec.supports_instruction,
        }
    }

    pub fn validate_for_model(&self, spec: &ModelSpec) -> Result<()> {
        if self.model_id != spec.id {
            anyhow::bail!("runtime model id does not match the model lock");
        }

        if !spec.supports_backend(&self.backend) {
            anyhow::bail!("runtime backend is not compatible with the model lock");
        }

        if self.expected_dim != spec.dimension {
            anyhow::bail!("runtime expected dimension does not match the model lock");
        }

        if let (Some(expected), Some(actual)) = (spec.dimension, self.actual_dim) {
            if expected != actual {
                anyhow::bail!("runtime embedding dimension does not match the model lock");
            }
        }

        if self.max_tokens != spec.max_tokens {
            anyhow::bail!("runtime max token limit does not match the model lock");
        }

        if self.tokenizer_family != spec.tokenizer_family {
            anyhow::bail!("runtime tokenizer family does not match the model lock");
        }

        if self.pooling != spec.pooling || self.normalization != spec.normalization {
            anyhow::bail!("runtime scoring semantics do not match the model lock");
        }

        if self.supports_instruction != spec.supports_instruction {
            anyhow::bail!("runtime instruction support does not match the model lock");
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorProvenance {
    pub model_id: String,
    pub repo: String,
    pub revision: String,
    pub backend: String,
    pub dimension: usize,
    pub instruction_profile: String,
    pub chunker_version: String,
    pub normalizer_version: String,
    pub pooling: String,
    pub normalization: String,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VectorFreshnessKey {
    pub text_hash: String,
    pub model_id: String,
    pub model_revision: String,
    pub backend: String,
    pub dimension: usize,
    pub instruction_profile: String,
    pub chunker_version: String,
    pub normalizer_version: String,
    pub pooling: String,
    pub normalization: String,
    pub model_manifest_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorFreshness {
    Valid,
    StaleRebuildNeeded,
    InvalidDimension,
    MissingModel,
}

impl VectorFreshnessKey {
    pub fn compare(&self, current: &Self, model_available: bool) -> VectorFreshness {
        if !model_available {
            return VectorFreshness::MissingModel;
        }

        if self.dimension != current.dimension {
            return VectorFreshness::InvalidDimension;
        }

        if self.text_hash == current.text_hash
            && self.model_id == current.model_id
            && self.model_revision == current.model_revision
            && self.backend == current.backend
            && self.instruction_profile == current.instruction_profile
            && self.chunker_version == current.chunker_version
            && self.normalizer_version == current.normalizer_version
            && self.pooling == current.pooling
            && self.normalization == current.normalization
            && self.model_manifest_hash == current.model_manifest_hash
        {
            VectorFreshness::Valid
        } else {
            VectorFreshness::StaleRebuildNeeded
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_model_manifest;

    fn freshness_key() -> VectorFreshnessKey {
        VectorFreshnessKey {
            text_hash: "a".to_string(),
            model_id: "qwen3-embedding-0.6b".to_string(),
            model_revision: "r".to_string(),
            backend: "qwen3-candle".to_string(),
            dimension: 768,
            instruction_profile: "resume_requirement_v1".to_string(),
            chunker_version: "chunker_v1".to_string(),
            normalizer_version: "normalizer_v1".to_string(),
            pooling: "last-token".to_string(),
            normalization: "l2".to_string(),
            model_manifest_hash: "manifest-a".to_string(),
        }
    }

    #[test]
    fn compatibility_accepts_matching_qwen3_embedding_spec() {
        let manifest = load_model_manifest().expect("model lockfile should parse");
        let model = manifest
            .default_embedding()
            .expect("default embedding should exist");
        let compatibility = RuntimeCompatibility::from_spec(model, "qwen3-candle", Some(768));

        compatibility
            .validate_for_model(model)
            .expect("matching compatibility should validate");
    }

    #[test]
    fn compatibility_accepts_matching_qwen3_reranker_spec() {
        let manifest = load_model_manifest().expect("model lockfile should parse");
        let model = manifest
            .default_reranker()
            .expect("default reranker should exist");
        let compatibility = RuntimeCompatibility::from_spec(model, "qwen3-reranker-candle", None);

        compatibility
            .validate_for_model(model)
            .expect("matching reranker compatibility should validate");
    }

    #[test]
    fn compatibility_rejects_dimension_mismatch() {
        let manifest = load_model_manifest().expect("model lockfile should parse");
        let model = manifest
            .default_embedding()
            .expect("default embedding should exist");
        let compatibility = RuntimeCompatibility::from_spec(model, "qwen3-candle", Some(1024));

        let error = compatibility
            .validate_for_model(model)
            .expect_err("dimension mismatch should fail");
        assert!(error.to_string().contains("dimension"));
    }

    #[test]
    fn freshness_key_detects_stale_instruction_profile() {
        let stored = freshness_key();
        let mut current = stored.clone();
        current.instruction_profile = "resume_requirement_v2".to_string();

        assert_eq!(
            stored.compare(&current, true),
            VectorFreshness::StaleRebuildNeeded
        );
    }

    #[test]
    fn freshness_key_detects_changed_model_backend_and_manifest() {
        let stored = freshness_key();
        let mut changed_model = stored.clone();
        changed_model.model_id = "other-model".to_string();
        let mut changed_backend = stored.clone();
        changed_backend.backend = "other-backend".to_string();
        let mut changed_manifest = stored.clone();
        changed_manifest.model_manifest_hash = "manifest-b".to_string();

        for current in [changed_model, changed_backend, changed_manifest] {
            assert_eq!(
                stored.compare(&current, true),
                VectorFreshness::StaleRebuildNeeded
            );
        }
    }
}
