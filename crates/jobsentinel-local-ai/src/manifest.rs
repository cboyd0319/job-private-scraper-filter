//! Pinned local model governance.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MODEL_LOCK_TOML: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/models.lock.toml"));

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelManifest {
    pub manifest_version: u32,
    pub defaults: ModelDefaults,
    #[serde(default)]
    pub instruction_profiles: BTreeMap<String, InstructionProfile>,
    #[serde(default)]
    pub thresholds: BTreeMap<String, ScoreThresholds>,
    #[serde(default)]
    pub reranker_acceptance: BTreeMap<String, f32>,
    #[serde(default)]
    pub models: Vec<ModelSpec>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelDefaults {
    pub embedding: String,
    pub reranker: String,
    pub legacy_runtime_embedding: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InstructionProfile {
    pub query: String,
    pub document: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScoreThresholds {
    pub strong: f32,
    pub medium: f32,
    pub weak: f32,
    pub retrieval: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelKind {
    Embedding,
    Reranker,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelSpec {
    pub id: String,
    pub kind: ModelKind,
    pub repo: String,
    pub revision: String,
    pub source_url: String,
    pub license: String,
    pub backend: String,
    #[serde(default)]
    pub backend_compatibility: Vec<String>,
    pub dimension: Option<usize>,
    pub native_dimension: Option<usize>,
    pub max_tokens: usize,
    pub tokenizer_family: String,
    pub pooling: String,
    pub normalization: String,
    pub supports_instruction: bool,
    pub notes: Option<String>,
    #[serde(default)]
    pub files: Vec<ModelFileSpec>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelFileSpec {
    pub path: String,
    pub sha256: String,
    pub size_bytes: Option<u64>,
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

#[must_use]
pub fn model_lock_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(MODEL_LOCK_TOML.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn load_model_manifest() -> Result<ModelManifest> {
    ModelManifest::from_lock_str(MODEL_LOCK_TOML)
}

pub fn validate_v3_vector_contract(
    provenance: &jobsentinel_domain::v3_manifests::ModelProvenance,
    freshness: &jobsentinel_domain::v3_manifests::VectorFreshness,
) -> Result<()> {
    provenance
        .validate()
        .map_err(anyhow::Error::msg)
        .context("invalid v3 model provenance")?;
    freshness
        .validate(provenance)
        .map_err(anyhow::Error::msg)
        .context("invalid v3 vector freshness")?;

    let manifest = load_model_manifest()?;
    let model = manifest
        .model(&provenance.model_id)
        .context("v3 model is absent from the checked-in model lock")?;
    let tokenizer = model
        .file("tokenizer.json")
        .context("v3 model lock is missing tokenizer.json")?;
    if model.kind != ModelKind::Embedding
        || model.revision != provenance.revision
        || model.backend != provenance.backend
        || model.dimension != Some(provenance.dimension as usize)
        || tokenizer.sha256 != provenance.tokenizer_sha256
        || model.pooling != provenance.pooling
        || model.normalization != provenance.normalization
        || provenance.manifest_sha256 != model_lock_hash()
        || !manifest
            .instruction_profiles
            .contains_key(&provenance.instruction_profile)
    {
        anyhow::bail!("v3 vector contract does not match the checked-in model lock");
    }
    Ok(())
}

impl ModelManifest {
    pub fn from_lock_str(value: &str) -> Result<Self> {
        let manifest: Self =
            basic_toml::from_str(value).context("failed to parse checked-in model lockfile")?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn model(&self, id: &str) -> Option<&ModelSpec> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn default_embedding(&self) -> Option<&ModelSpec> {
        self.model(&self.defaults.embedding)
    }

    pub fn default_reranker(&self) -> Option<&ModelSpec> {
        self.model(&self.defaults.reranker)
    }

    pub fn legacy_runtime_embedding(&self) -> Option<&ModelSpec> {
        self.model(&self.defaults.legacy_runtime_embedding)
    }

    fn validate(&self) -> Result<()> {
        if self.manifest_version != 1 {
            anyhow::bail!(
                "unsupported manifest_version {}; this runtime supports version 1",
                self.manifest_version
            );
        }

        validate_default(self, &self.defaults.embedding, ModelKind::Embedding)?;
        validate_default(self, &self.defaults.reranker, ModelKind::Reranker)?;
        validate_default(
            self,
            &self.defaults.legacy_runtime_embedding,
            ModelKind::Embedding,
        )?;

        for model in &self.models {
            model.validate()?;
        }
        for (name, thresholds) in &self.thresholds {
            thresholds.validate(name)?;
        }
        if self
            .reranker_acceptance
            .values()
            .any(|threshold| !threshold.is_finite())
        {
            anyhow::bail!("model lockfile reranker acceptance must be finite");
        }

        Ok(())
    }
}

impl ScoreThresholds {
    fn validate(&self, name: &str) -> Result<()> {
        if ![self.retrieval, self.weak, self.medium, self.strong]
            .into_iter()
            .all(|threshold| threshold.is_finite() && (0.0..=1.0).contains(&threshold))
            || self.retrieval > self.weak
            || self.weak > self.medium
            || self.medium > self.strong
        {
            anyhow::bail!("model lockfile threshold order is invalid for {name}");
        }
        Ok(())
    }
}

impl ModelSpec {
    pub fn required_files(&self) -> impl Iterator<Item = &ModelFileSpec> {
        self.files.iter().filter(|file| file.required)
    }

    pub fn file(&self, path: &str) -> Option<&ModelFileSpec> {
        self.files.iter().find(|file| file.path == path)
    }

    pub fn supports_backend(&self, backend: &str) -> bool {
        self.backend == backend
            || self
                .backend_compatibility
                .iter()
                .any(|item| item == backend)
    }

    fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || matches!(self.id.as_str(), "." | "..")
            || !self
                .id
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.'))
        {
            anyhow::bail!("model id must be one portable path component");
        }

        if self.revision.len() != 40
            || !self.revision.chars().all(|value| value.is_ascii_hexdigit())
        {
            anyhow::bail!("model {} revision must be a full commit SHA", self.id);
        }

        if self.required_files().next().is_none() {
            anyhow::bail!("model {} must have at least one required file", self.id);
        }

        for file in &self.files {
            file.validate(&self.id)?;
        }

        Ok(())
    }
}

impl ModelFileSpec {
    fn validate(&self, model_id: &str) -> Result<()> {
        if self.path.trim().is_empty()
            || self.path.starts_with('/')
            || self.path.contains("..")
            || self.path.contains('\\')
        {
            anyhow::bail!("model {} has unsafe file path in lockfile", model_id);
        }

        if self.sha256.len() != 64 || !self.sha256.chars().all(|value| value.is_ascii_hexdigit()) {
            anyhow::bail!("model {} file {} has invalid sha256", model_id, self.path);
        }

        Ok(())
    }
}

fn validate_default(manifest: &ModelManifest, id: &str, kind: ModelKind) -> Result<()> {
    let model = manifest
        .model(id)
        .with_context(|| format!("model lockfile default references missing model {}", id))?;

    if model.kind != kind {
        anyhow::bail!("model lockfile default {} has wrong model kind", id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_model_lock_parses() {
        let manifest = load_model_manifest().expect("model lockfile should parse");

        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(
            manifest.default_embedding().map(|model| model.id.as_str()),
            Some("qwen3-embedding-0.6b")
        );
        assert_eq!(
            manifest.default_reranker().map(|model| model.id.as_str()),
            Some("qwen3-reranker-0.6b")
        );
        assert_eq!(
            manifest
                .legacy_runtime_embedding()
                .map(|model| model.id.as_str()),
            Some("all-minilm-l6-v2-baseline")
        );
        assert_eq!(
            manifest
                .thresholds
                .get("resume_requirement")
                .map(|thresholds| thresholds.retrieval),
            Some(0.30)
        );
        assert_eq!(
            manifest
                .reranker_acceptance
                .get("resume_requirement")
                .copied(),
            Some(3.0)
        );
    }

    #[test]
    fn retrieval_threshold_must_not_exceed_weak_score_band() {
        let invalid = MODEL_LOCK_TOML.replacen("retrieval = 0.30", "retrieval = 0.49", 1);

        let error = ModelManifest::from_lock_str(&invalid)
            .expect_err("retrieval above the weak band must fail closed");

        assert!(error.to_string().contains("threshold order"));
    }

    #[test]
    fn reranker_acceptance_threshold_must_be_finite() {
        let invalid =
            MODEL_LOCK_TOML.replacen("resume_requirement = 3.0", "resume_requirement = nan", 1);

        let error = ModelManifest::from_lock_str(&invalid)
            .expect_err("non-finite reranker acceptance must fail closed");

        assert!(error.to_string().contains("reranker acceptance"));
    }

    #[test]
    fn model_lock_hash_is_stable_sha256() {
        let hash = model_lock_hash();

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|value| value.is_ascii_hexdigit()));
    }

    #[test]
    fn unsupported_newer_model_manifest_version_is_rejected() {
        let newer = MODEL_LOCK_TOML.replacen("manifest_version = 1", "manifest_version = 2", 1);

        let error =
            ModelManifest::from_lock_str(&newer).expect_err("newer manifest must fail closed");

        assert!(error.to_string().contains("unsupported manifest_version 2"));
    }

    #[test]
    fn qwen3_lock_approves_only_wired_candle_backends() {
        let manifest = load_model_manifest().expect("model lockfile should parse");
        let model = manifest
            .default_embedding()
            .expect("default embedding model should exist");
        let reranker = manifest
            .default_reranker()
            .expect("default reranker model should exist");

        assert_eq!(model.repo, "Qwen/Qwen3-Embedding-0.6B");
        assert_eq!(model.dimension, Some(768));
        assert_eq!(model.native_dimension, Some(1024));
        assert!(model.backend_compatibility.is_empty());
        assert!(reranker.backend_compatibility.is_empty());
        assert!(model.file("config.json").is_some());
        assert!(model.file("tokenizer.json").is_some());
        assert!(model.file("model.safetensors").is_some());
    }

    #[test]
    fn v3_vector_contract_must_match_the_canonical_model_lock() {
        use jobsentinel_domain::v3_manifests::{ModelProvenance, VectorFreshness};

        let manifest = load_model_manifest().unwrap();
        let model = manifest.default_embedding().unwrap();
        let provenance = ModelProvenance {
            schema: jobsentinel_domain::v3_contracts::SchemaId::ModelProvenanceV1,
            model_id: model.id.clone(),
            revision: model.revision.clone(),
            backend: model.backend.clone(),
            dimension: model.dimension.unwrap() as u32,
            tokenizer_sha256: model.file("tokenizer.json").unwrap().sha256.clone(),
            manifest_sha256: model_lock_hash(),
            instruction_profile: "resume_requirement_v1".to_string(),
            pooling: model.pooling.clone(),
            normalization: model.normalization.clone(),
        };
        let freshness = VectorFreshness {
            schema: jobsentinel_domain::v3_contracts::SchemaId::VectorFreshnessV1,
            model_id: provenance.model_id.clone(),
            model_revision: provenance.revision.clone(),
            backend: provenance.backend.clone(),
            dimension: provenance.dimension,
            instruction_profile: provenance.instruction_profile.clone(),
            chunker_version: "chunker_v1".to_string(),
            normalizer_version: "normalizer_v1".to_string(),
            pooling: provenance.pooling.clone(),
            normalization: provenance.normalization.clone(),
            model_manifest_sha256: provenance.manifest_sha256.clone(),
            content_sha256: "a".repeat(64),
        };

        validate_v3_vector_contract(&provenance, &freshness).unwrap();

        let mut wrong = provenance.clone();
        wrong.backend = "qwen3_candle_v1".to_string();
        let mut matching_wrong_freshness = freshness.clone();
        matching_wrong_freshness.backend = wrong.backend.clone();
        assert!(
            validate_v3_vector_contract(&wrong, &matching_wrong_freshness)
                .unwrap_err()
                .to_string()
                .contains("checked-in model lock")
        );

        wrong = provenance.clone();
        wrong.instruction_profile = "invented_profile".to_string();
        matching_wrong_freshness = freshness.clone();
        matching_wrong_freshness.instruction_profile = wrong.instruction_profile.clone();
        assert!(
            validate_v3_vector_contract(&wrong, &matching_wrong_freshness)
                .unwrap_err()
                .to_string()
                .contains("checked-in model lock")
        );
    }

    #[test]
    fn unsafe_model_paths_are_rejected() {
        let lock = r#"
manifest_version = 1

[defaults]
embedding = "bad"
reranker = "rerank"
legacy_runtime_embedding = "bad"

[[models]]
id = "bad"
kind = "embedding"
repo = "org/model"
revision = "1111111111111111111111111111111111111111"
source_url = "https://example.test/model"
license = "Apache-2.0"
backend = "test"
max_tokens = 1
tokenizer_family = "test"
pooling = "mean"
normalization = "l2"
supports_instruction = false

[[models.files]]
path = "../model.safetensors"
sha256 = "1111111111111111111111111111111111111111111111111111111111111111"

[[models]]
id = "rerank"
kind = "reranker"
repo = "org/model"
revision = "2222222222222222222222222222222222222222"
source_url = "https://example.test/rerank"
license = "Apache-2.0"
backend = "test"
max_tokens = 1
tokenizer_family = "test"
pooling = "score"
normalization = "raw"
supports_instruction = false
[[models.files]]
path = "model.safetensors"
sha256 = "2222222222222222222222222222222222222222222222222222222222222222"
"#;

        let error = ModelManifest::from_lock_str(lock).expect_err("unsafe path should fail");
        assert!(error.to_string().contains("unsafe file path"));
    }
    #[test]
    fn model_ids_must_be_single_portable_path_components() {
        let manifest = load_model_manifest().unwrap();
        for id in ["../qwen3", "folder/model", r"folder\model", "C:model", "."] {
            let mut model = manifest.models[0].clone();
            model.id = id.to_string();
            assert!(model.validate().is_err());
        }
    }
}
