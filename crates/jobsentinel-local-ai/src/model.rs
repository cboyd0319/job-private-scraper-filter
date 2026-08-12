//! Model Management
//!
//! Handles downloading, caching, and loading governed local model artifacts.

mod cache;
mod device;
mod download;
mod integrity;
mod legacy;
mod loading;
pub(super) mod status;

use super::manifest::{load_model_manifest, model_lock_hash, ModelFileSpec, ModelSpec};
use super::MlError;
use anyhow::Result;
use std::path::PathBuf;

pub(crate) use legacy::SentenceTransformer;
pub(crate) use status::ModelCacheMetadata;

const RUNTIME_MODEL_LOCK_MISSING: &str = "runtime model lock entry missing";
const DEFAULT_EMBEDDING_MODEL_LOCK_MISSING: &str = "default embedding model lock entry missing";
const DEFAULT_RERANKER_MODEL_LOCK_MISSING: &str = "default reranker model lock entry missing";

/// Manages model download and caching.
pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    /// Create new model manager with app data directory.
    pub fn new(app_data_dir: PathBuf) -> Self {
        let cache_dir = app_data_dir.join("ml_models");
        Self { cache_dir }
    }

    pub(crate) fn runtime_model_spec() -> Result<ModelSpec> {
        let manifest = load_model_manifest()?;
        manifest
            .legacy_runtime_embedding()
            .cloned()
            .ok_or_else(|| MlError::ModelLoadFailed(RUNTIME_MODEL_LOCK_MISSING.to_string()).into())
    }

    pub(crate) fn default_embedding_model_spec() -> Result<ModelSpec> {
        let manifest = load_model_manifest()?;
        manifest.default_embedding().cloned().ok_or_else(|| {
            MlError::ModelLoadFailed(DEFAULT_EMBEDDING_MODEL_LOCK_MISSING.to_string()).into()
        })
    }

    pub(crate) fn default_reranker_model_spec() -> Result<ModelSpec> {
        let manifest = load_model_manifest()?;
        manifest.default_reranker().cloned().ok_or_else(|| {
            MlError::ModelLoadFailed(DEFAULT_RERANKER_MODEL_LOCK_MISSING.to_string()).into()
        })
    }

    pub(crate) fn model_cache_dir(&self, spec: &ModelSpec) -> PathBuf {
        self.cache_dir
            .join(&spec.id)
            .join(&spec.revision)
            .join(model_lock_hash())
    }

    pub(crate) fn model_file_path(&self, spec: &ModelSpec, file: &ModelFileSpec) -> PathBuf {
        self.model_cache_dir(spec).join(&file.path)
    }
}

fn fallback_runtime_model_spec() -> ModelSpec {
    ModelSpec {
        id: "model-lock-error".to_string(),
        kind: super::manifest::ModelKind::Embedding,
        repo: "unknown".to_string(),
        revision: "0000000000000000000000000000000000000000".to_string(),
        source_url: "unknown".to_string(),
        license: "unknown".to_string(),
        backend: "unavailable".to_string(),
        backend_compatibility: Vec::new(),
        dimension: None,
        native_dimension: None,
        max_tokens: 0,
        tokenizer_family: "unknown".to_string(),
        pooling: "unknown".to_string(),
        normalization: "unknown".to_string(),
        supports_instruction: false,
        notes: None,
        files: Vec::new(),
    }
}

/// Mirror the anonymous app-scoped download policy the desktop shell sets at
/// startup so opt-in downloading tests exercise the production checks.
/// Downloading tests must run single-threaded because this state is global.
#[cfg(test)]
pub(crate) fn set_download_policy_env(app_data_dir: &std::path::Path) {
    std::env::set_var("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1");
    std::env::set_var("HF_XET_CACHE", app_data_dir.join("ml_models").join(".xet"));
}
