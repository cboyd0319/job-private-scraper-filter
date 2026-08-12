//! Local semantic matching diagnostics.

#[cfg(feature = "embedded-ml")]
use crate::desktop;
#[cfg(feature = "embedded-ml")]
use crate::desktop::{
    load_model_manifest, model_lock_hash, ModelCacheHealth, ModelKind, ModelManager, ModelSpec,
};
#[cfg(feature = "embedded-ml")]
use crate::ipc::errors::user_friendly_error;
#[cfg(feature = "embedded-ml")]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
#[cfg(feature = "embedded-ml")]
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticMatchingRuntimeStatus {
    #[cfg(feature = "embedded-ml")]
    Ready,
    #[cfg(feature = "embedded-ml")]
    NeedsModelDownload,
    #[cfg(feature = "embedded-ml")]
    Misconfigured,
    #[cfg(not(feature = "embedded-ml"))]
    DisabledInThisBuild,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SemanticMatchingDiagnostics {
    pub build_enabled: bool,
    pub runtime_status: SemanticMatchingRuntimeStatus,
    pub active_profile: String,
    pub privacy_mode: String,
    pub manifest_hash: Option<String>,
    pub models: Vec<SemanticMatchingModelDiagnostic>,
    pub scoring_signals: Vec<SemanticMatchingSignal>,
    pub eval_contract: Vec<String>,
    pub user_action: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SemanticMatchingModelDiagnostic {
    pub id: String,
    pub role: String,
    pub repo: String,
    pub revision: String,
    pub backend: String,
    pub license: String,
    pub dimension: Option<usize>,
    pub max_tokens: usize,
    pub required_files: usize,
    pub required_files_present: usize,
    pub locked_size_bytes: Option<u64>,
    pub downloaded: bool,
    pub cache_present: bool,
    #[cfg(feature = "embedded-ml")]
    pub health: ModelCacheHealth,
    pub required_for_qwen3_runtime: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SemanticMatchingSignal {
    pub id: String,
    pub label: String,
    pub state: String,
    pub explanation: String,
}

#[tauri::command]
pub(crate) async fn get_semantic_matching_diagnostics(
) -> Result<SemanticMatchingDiagnostics, String> {
    semantic_matching_diagnostics()
}

#[cfg(not(feature = "embedded-ml"))]
#[tauri::command]
pub(crate) async fn repair_semantic_matching_model_cache(
    _app: tauri::AppHandle,
    _model_id: String,
) -> Result<bool, String> {
    Err("Local model repair is unavailable in this build.".to_string())
}

#[cfg(feature = "embedded-ml")]
#[tauri::command]
pub(crate) async fn repair_semantic_matching_model_cache(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<bool, String> {
    let _reservation = crate::ipc::ml::reserve_model_lifecycle(None)?;
    let manifest = load_model_manifest()
        .map_err(|_| "Local model repair could not be prepared.".to_string())?;
    let model = [manifest.default_embedding(), manifest.default_reranker()]
        .into_iter()
        .flatten()
        .find(|model| model.id == model_id)
        .ok_or_else(|| "Local model repair is not available.".to_string())?;
    let manager = ModelManager::new(desktop::get_data_dir());
    if manager.cache_health_for(model) != ModelCacheHealth::IntegrityMismatch {
        return Err("Local model repair is not available.".to_string());
    }
    if !confirm_model_cache_removal(&app, model_role(&manifest, model)).await? {
        return Ok(false);
    }
    manager
        .repair_invalid_default_cache(&model_id)
        .map_err(|_| "Local model repair could not be completed.".to_string())?;
    Ok(true)
}

#[cfg(not(feature = "embedded-ml"))]
fn semantic_matching_diagnostics() -> Result<SemanticMatchingDiagnostics, String> {
    Ok(SemanticMatchingDiagnostics {
        build_enabled: false,
        runtime_status: SemanticMatchingRuntimeStatus::DisabledInThisBuild,
        active_profile: "Built-in local matching rules".to_string(),
        privacy_mode: "Local only. No resume or job text leaves this device.".to_string(),
        manifest_hash: None,
        models: Vec::new(),
        scoring_signals: base_scoring_signals(),
        eval_contract: base_eval_contract(),
        user_action: Some(
            "This app build uses the built-in local matching rules. Advanced local model diagnostics appear in embedded-ML builds."
                .to_string(),
        ),
    })
}

#[cfg(feature = "embedded-ml")]
fn semantic_matching_diagnostics() -> Result<SemanticMatchingDiagnostics, String> {
    let manifest = load_model_manifest()
        .map_err(|error| user_friendly_error("Failed to read local model lock", error))?;
    let manager = ModelManager::new(desktop::get_data_dir());
    Ok(semantic_matching_diagnostics_for(&manifest, &manager))
}

#[cfg(feature = "embedded-ml")]
fn semantic_matching_diagnostics_for(
    manifest: &crate::desktop::ModelManifest,
    manager: &ModelManager,
) -> SemanticMatchingDiagnostics {
    let model_health = manifest
        .models
        .iter()
        .map(|model| (model, manager.cache_health_for(model)))
        .collect::<Vec<_>>();
    let models = model_health
        .iter()
        .map(|(model, health)| model_diagnostic(manager, manifest, model, *health))
        .collect();
    let health_for = |id: &str| {
        model_health
            .iter()
            .find(|(model, _)| model.id == id)
            .map_or(ModelCacheHealth::IntegrityMismatch, |(_, health)| *health)
    };
    let legacy_health = health_for(&manifest.defaults.legacy_runtime_embedding);
    let runtime_status = semantic_runtime_status(
        health_for(&manifest.defaults.embedding),
        health_for(&manifest.defaults.reranker),
        legacy_health,
    );
    let (active_profile, user_action) = semantic_runtime_copy(runtime_status, legacy_health);

    SemanticMatchingDiagnostics {
        build_enabled: true,
        runtime_status,
        active_profile: active_profile.to_string(),
        privacy_mode: "Local only. Model downloads fetch model files only and never send resume or job-search data."
            .to_string(),
        manifest_hash: Some(model_lock_hash()),
        models,
        scoring_signals: base_scoring_signals(),
        eval_contract: base_eval_contract(),
        user_action: user_action.map(str::to_string),
    }
}

#[cfg(feature = "embedded-ml")]
fn semantic_runtime_status(
    embedding: ModelCacheHealth,
    reranker: ModelCacheHealth,
    legacy: ModelCacheHealth,
) -> SemanticMatchingRuntimeStatus {
    match (embedding, reranker, legacy) {
        (ModelCacheHealth::Ready, ModelCacheHealth::Ready, _) => {
            SemanticMatchingRuntimeStatus::Ready
        }
        (_, _, ModelCacheHealth::Incomplete | ModelCacheHealth::IntegrityMismatch) => {
            SemanticMatchingRuntimeStatus::Misconfigured
        }
        (ModelCacheHealth::Missing, ModelCacheHealth::Missing, _) => {
            SemanticMatchingRuntimeStatus::NeedsModelDownload
        }
        _ => SemanticMatchingRuntimeStatus::Misconfigured,
    }
}

#[cfg(feature = "embedded-ml")]
fn semantic_runtime_copy(
    status: SemanticMatchingRuntimeStatus,
    legacy: ModelCacheHealth,
) -> (&'static str, Option<&'static str>) {
    match (status, legacy == ModelCacheHealth::Ready) {
        (SemanticMatchingRuntimeStatus::Ready, _) => {
            ("Qwen3 embedding plus Qwen3 reranker", None)
        }
        (SemanticMatchingRuntimeStatus::NeedsModelDownload, true) => (
            "Verified MiniLM local matching; Qwen3 models are not installed",
            Some(
                "Download the pinned Qwen3 models to enable advanced matching. Verified MiniLM matching remains active.",
            ),
        ),
        (SemanticMatchingRuntimeStatus::NeedsModelDownload, false) => (
            "Exact-only deterministic matching; Qwen3 models are not installed",
            Some(
                "Download the pinned Qwen3 models to enable advanced matching. Exact-only local matching remains active.",
            ),
        ),
        (SemanticMatchingRuntimeStatus::Misconfigured, true) => (
            "Verified MiniLM local matching; Qwen3 model cache needs attention",
            Some(
                "Local model files are incomplete or failed integrity checks. Download the pinned models again before using Qwen3 matching. Verified MiniLM matching remains active.",
            ),
        ),
        (SemanticMatchingRuntimeStatus::Misconfigured, false) => (
            "Exact-only deterministic matching; local model cache needs attention",
            Some(
                "Local model files are incomplete or failed integrity checks. Download the pinned models again before using advanced matching. Exact-only local matching remains active.",
            ),
        ),
    }
}

#[cfg(feature = "embedded-ml")]
fn model_diagnostic(
    manager: &ModelManager,
    manifest: &crate::desktop::ModelManifest,
    model: &ModelSpec,
    health: ModelCacheHealth,
) -> SemanticMatchingModelDiagnostic {
    let required_files = model.required_files().count();

    SemanticMatchingModelDiagnostic {
        id: model.id.clone(),
        role: model_role(manifest, model).to_string(),
        repo: model.repo.clone(),
        revision: model.revision.clone(),
        backend: model.backend.clone(),
        license: model.license.clone(),
        dimension: model.dimension,
        max_tokens: model.max_tokens,
        required_files,
        required_files_present: manager.required_files_present(model),
        locked_size_bytes: locked_size_bytes(model),
        downloaded: health == ModelCacheHealth::Ready,
        cache_present: manager.cache_exists_for(model),
        health,
        required_for_qwen3_runtime: model.id == manifest.defaults.embedding
            || model.id == manifest.defaults.reranker,
    }
}

#[cfg(feature = "embedded-ml")]
async fn confirm_model_cache_removal(app: &tauri::AppHandle, role: &str) -> Result<bool, String> {
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(format!(
            "Remove the integrity-invalid files for {role}? No resume or job data is involved. \
             JobSentinel will not download replacement files automatically."
        ))
        .title("Review Local Model Repair")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Remove Damaged Files".to_string(),
            "Keep Files".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "Local model repair confirmation could not be completed.".to_string())
}

#[cfg(feature = "embedded-ml")]
fn model_role(manifest: &crate::desktop::ModelManifest, model: &ModelSpec) -> &'static str {
    if model.id == manifest.defaults.embedding {
        "Default embedding"
    } else if model.id == manifest.defaults.reranker {
        "Default reranker"
    } else if model.id == manifest.defaults.legacy_runtime_embedding {
        "Legacy direct matcher"
    } else {
        match model.kind {
            ModelKind::Embedding => "Embedding",
            ModelKind::Reranker => "Reranker",
        }
    }
}

#[cfg(feature = "embedded-ml")]
fn locked_size_bytes(model: &ModelSpec) -> Option<u64> {
    let mut total = 0_u64;
    for file in &model.files {
        total = total.checked_add(file.size_bytes?)?;
    }
    Some(total)
}

fn base_scoring_signals() -> Vec<SemanticMatchingSignal> {
    vec![
        signal(
            "exact_skills",
            "Exact skills",
            "Always on",
            "Matches visible skills and aliases before any model estimate is used.",
        ),
        signal(
            "required_coverage",
            "Must-have coverage",
            "Always on",
            "Caps a match when required skills, credentials, location, or pay do not line up.",
        ),
        signal(
            "seniority",
            "Seniority fit",
            "Always on",
            "Checks whether the resume evidence fits the level of the posting.",
        ),
        signal(
            "qwen3_embedding",
            "Qwen3 semantic retrieval",
            "Embedded-ML builds",
            "Finds related evidence after the pinned local model files are downloaded and verified.",
        ),
        signal(
            "qwen3_reranker",
            "Qwen3 reranker",
            "Embedded-ML builds",
            "Reranks only a bounded top set of candidate evidence so broad searches stay controlled.",
        ),
    ]
}

fn signal(id: &str, label: &str, state: &str, explanation: &str) -> SemanticMatchingSignal {
    SemanticMatchingSignal {
        id: id.to_string(),
        label: label.to_string(),
        state: state.to_string(),
        explanation: explanation.to_string(),
    }
}

fn base_eval_contract() -> Vec<String> {
    vec![
        "Direct evidence must outrank keyword-only near misses.".to_string(),
        "Hard blockers must cap otherwise strong-looking matches.".to_string(),
        "Role-family and seniority checks must cover technical and non-technical work.".to_string(),
        "Fairness checks must avoid rewarding a candidate for matching only a single narrow profile.".to_string(),
        "Generated advice must stay separate from real job evidence.".to_string(),
    ]
}

#[cfg(test)]
mod tests;
