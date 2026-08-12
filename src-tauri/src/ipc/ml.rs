//! Machine Learning Tauri Commands
//!
//! Commands for model download, status checking, and semantic matching.

#![cfg(feature = "embedded-ml")]

use crate::bootstrap::AppState;
use crate::desktop;
use crate::desktop::path_label_for_logging;
use crate::desktop::{load_model_manifest, ModelManager, ModelStatus, SemanticMatcher};
use crate::ipc::errors::user_friendly_error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::{oneshot, watch};

enum ActiveModelLifecycle {
    Download(watch::Sender<bool>),
    Exclusive,
}

static MODEL_LIFECYCLE: LazyLock<Mutex<Option<ActiveModelLifecycle>>> =
    LazyLock::new(|| Mutex::new(None));

pub(crate) struct ModelLifecycleReservation;

impl Drop for ModelLifecycleReservation {
    fn drop(&mut self) {
        *model_lifecycle_slot() = None;
    }
}

fn model_lifecycle_slot() -> MutexGuard<'static, Option<ActiveModelLifecycle>> {
    MODEL_LIFECYCLE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn reserve_model_lifecycle(
    cancellation: Option<watch::Sender<bool>>,
) -> Result<ModelLifecycleReservation, String> {
    let mut active = model_lifecycle_slot();
    if active.is_some() {
        return Err("Another local model action is already running.".to_string());
    }
    *active = Some(cancellation.map_or(
        ActiveModelLifecycle::Exclusive,
        ActiveModelLifecycle::Download,
    ));
    Ok(ModelLifecycleReservation)
}

#[derive(Clone, serde::Serialize)]
struct ModelDownloadProgress {
    completed_bytes: u64,
    total_bytes: u64,
}

const PROGRESS_EMIT_STEP_BYTES: u64 = 8 * 1024 * 1024;
const PROGRESS_NEVER_EMITTED: u64 = u64::MAX;

fn should_emit_progress(last_emitted: u64, completed_bytes: u64, total_bytes: u64) -> bool {
    last_emitted == PROGRESS_NEVER_EMITTED
        || completed_bytes >= total_bytes
        || completed_bytes.saturating_sub(last_emitted) >= PROGRESS_EMIT_STEP_BYTES
}

/// Download the default local semantic models from HuggingFace Hub.
#[tauri::command]
pub(crate) async fn download_ml_model(app: tauri::AppHandle) -> Result<bool, String> {
    tracing::info!("Command: download_ml_model");
    let (cancel_tx, mut cancel_rx) = watch::channel(false);
    let _reservation = reserve_model_lifecycle(Some(cancel_tx))?;

    let app_data_dir = desktop::get_data_dir();
    let manager = ModelManager::new(app_data_dir);

    if manager.is_default_semantic_runtime_downloaded() {
        tracing::info!("Default semantic models already downloaded");
        return Ok(true);
    }
    if !confirm_model_download(&app).await? {
        return Ok(false);
    }

    tracing::info!("Starting default semantic model download from HuggingFace Hub");
    let progress_app = app.clone();
    let last_emitted = AtomicU64::new(PROGRESS_NEVER_EMITTED);
    let progress = Arc::new(move |completed_bytes, total_bytes| {
        if !should_emit_progress(
            last_emitted.load(Ordering::Relaxed),
            completed_bytes,
            total_bytes,
        ) {
            return;
        }
        last_emitted.store(completed_bytes, Ordering::Relaxed);
        let _ = progress_app.emit(
            "local-model-download-progress",
            ModelDownloadProgress {
                completed_bytes,
                total_bytes,
            },
        );
    });
    let result = tokio::select! {
        result = manager.download_default_semantic_models_with_progress(progress) => {
            result.map(Some)
        }
        changed = cancel_rx.changed() => {
            changed
                .map(|()| None)
                .map_err(|_| anyhow::anyhow!("model download cancellation channel closed"))
        }
    };
    let Some(model_paths) = result
        .map_err(|error| user_friendly_error("Failed to download local matching models", error))?
    else {
        tracing::info!("Local matching model download canceled");
        return Ok(false);
    };

    tracing::info!(
        model_count = model_paths.len(),
        model_paths = ?model_paths.iter().map(path_label_for_logging).collect::<Vec<_>>(),
        "Local matching models downloaded successfully"
    );
    Ok(true)
}

#[tauri::command]
pub(crate) async fn cancel_ml_model_download() -> Result<bool, String> {
    tracing::info!("Command: cancel_ml_model_download");
    Ok(matches!(
        model_lifecycle_slot().as_ref(),
        Some(ActiveModelLifecycle::Download(cancellation))
            if cancellation.send(true).is_ok()
    ))
}

#[tauri::command]
pub(crate) async fn remove_ml_models(app: tauri::AppHandle) -> Result<bool, String> {
    tracing::info!("Command: remove_ml_models");
    let _reservation = reserve_model_lifecycle(None)?;
    if !confirm_model_removal(&app).await? {
        return Ok(false);
    }

    ModelManager::new(desktop::get_data_dir())
        .remove_default_semantic_model_caches()
        .map_err(|error| user_friendly_error("Failed to remove local matching models", error))
}

/// Get ML model status
#[tauri::command]
pub(crate) async fn get_ml_status() -> Result<ModelStatus, String> {
    tracing::info!("Command: get_ml_status");

    let app_data_dir = desktop::get_data_dir();
    let manager = ModelManager::new(app_data_dir);
    Ok(manager.get_status())
}

/// Perform semantic skill matching
#[tauri::command]
pub(crate) async fn semantic_match_skills(
    user_skills: Vec<String>,
    job_requirements: Vec<String>,
) -> Result<serde_json::Value, String> {
    tracing::info!(
        "Command: semantic_match_skills ({} user skills, {} job requirements)",
        user_skills.len(),
        job_requirements.len()
    );

    let app_data_dir = desktop::get_data_dir();
    let matcher = SemanticMatcher::new(app_data_dir)
        .map_err(|e| user_friendly_error("Failed to create matcher", e))?;

    let result = matcher
        .match_skills(&user_skills, &job_requirements)
        .map_err(|e| user_friendly_error("Failed to match skills", e))?;

    serde_json::to_value(&result).map_err(|e| user_friendly_error("Failed to serialize result", e))
}

/// Enhanced resume matching with semantic understanding
#[tauri::command]
pub(crate) async fn match_resume_semantic(
    resume_id: i64,
    job_hash: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        "Command: match_resume_semantic"
    );

    let app_data_dir = desktop::get_data_dir();

    let result = jobsentinel_application::resume::match_resume_semantic(
        state.database.as_ref(),
        app_data_dir,
        resume_id,
        &job_hash,
    )
    .await
    .map_err(|e| user_friendly_error("Failed to match skills", e))?;

    serde_json::to_value(&result).map_err(|e| user_friendly_error("Failed to serialize result", e))
}

async fn confirm_model_download(app: &tauri::AppHandle) -> Result<bool, String> {
    let manifest = load_model_manifest()
        .map_err(|_| "Local model setup could not be prepared.".to_string())?;
    let models = [
        manifest
            .default_embedding()
            .ok_or_else(|| "Local model setup could not be prepared.".to_string())?,
        manifest
            .default_reranker()
            .ok_or_else(|| "Local model setup could not be prepared.".to_string())?,
    ];
    let bytes = models
        .iter()
        .flat_map(|model| model.required_files())
        .filter_map(|file| file.size_bytes)
        .sum();
    let license = models
        .first()
        .map_or("the model license", |model| model.license.as_str());
    confirm_model_action(
        app,
        "Set Up Stronger Local Matching",
        format!(
            "Download approximately {} of pinned {license} model files from Hugging Face and its file delivery network?\n\n\
             Setup can need additional temporary disk space. \
             Resume text, job data, salary preferences, notes, and application history are not sent. \
             Matching stays on this device. Built-in matching remains available during the download, \
             after cancellation, and if setup fails.",
            format_bytes(bytes)
        ),
        "Download models",
        "Keep built-in matching",
    )
    .await
}

async fn confirm_model_removal(app: &tauri::AppHandle) -> Result<bool, String> {
    confirm_model_action(
        app,
        "Remove Local Models",
        "Remove JobSentinel's downloaded Qwen3 model files and any incomplete download data?\n\n\
         Built-in local matching remains available. Stronger local matching can be downloaded again later.",
        "Remove model files",
        "Keep model files",
    )
    .await
}

async fn confirm_model_action(
    app: &tauri::AppHandle,
    title: &str,
    message: impl Into<String>,
    confirm_label: &str,
    cancel_label: &str,
) -> Result<bool, String> {
    let parent = app
        .get_webview_window("main")
        .ok_or_else(|| "Local model confirmation is unavailable.".to_string())?;
    let (sender, receiver) = oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .parent(&parent)
        .buttons(MessageDialogButtons::OkCancelCustom(
            confirm_label.to_string(),
            cancel_label.to_string(),
        ))
        .show(move |confirmed| {
            let _ = sender.send(confirmed);
        });
    receiver
        .await
        .map_err(|_| "Local model confirmation could not be completed.".to_string())
}

fn format_bytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_emission_throttles_between_bounded_steps() {
        assert!(should_emit_progress(PROGRESS_NEVER_EMITTED, 0, 100));
        assert!(!should_emit_progress(
            0,
            PROGRESS_EMIT_STEP_BYTES - 1,
            u64::MAX
        ));
        assert!(should_emit_progress(0, PROGRESS_EMIT_STEP_BYTES, u64::MAX));
        assert!(should_emit_progress(0, 50, 50));
    }

    #[tokio::test]
    async fn model_lifecycle_reservation_blocks_overlap_and_targets_download_cancel() {
        let exclusive = reserve_model_lifecycle(None).unwrap();
        assert!(reserve_model_lifecycle(None).is_err());
        assert!(!cancel_ml_model_download().await.unwrap());
        drop(exclusive);

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let download = reserve_model_lifecycle(Some(cancel_tx)).unwrap();
        assert!(cancel_ml_model_download().await.unwrap());
        cancel_rx.changed().await.unwrap();
        drop(download);

        assert!(reserve_model_lifecycle(None).is_ok());
    }
}
