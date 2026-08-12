//! Stages one bounded dropped signed pack through the production trust boundary.

use super::{has_pack_extension, open_regular_source, NativeFileDropState};
use crate::bootstrap::AppState;
use jobsentinel_application::pack_runtime::{
    stage_production_pack_artifact, PackInstallReview, MAX_PACK_ARTIFACT_BYTES,
};
use std::{io::Read, path::Path};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

const DROPPED_PACK_ERROR: &str = "Drop a signed .jspack file within the pack size limit.";

#[tauri::command]
pub(crate) async fn choose_and_stage_pack(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("JobSentinel signed pack", &["jspack"])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let path = file
        .into_path()
        .map_err(|_| "The selected signed pack is unavailable.".to_string())?;
    if !has_pack_extension(&path) {
        return Err(DROPPED_PACK_ERROR.to_string());
    }
    let envelope = read_bounded_pack_artifact(&path)?;
    stage_production_pack_artifact(state.database.as_ref(), &state.pack_runtime, &envelope)
        .await
        .map_err(|_| "This signed pack could not be verified and staged.".to_string())?;
    Ok(true)
}

#[tauri::command]
pub(crate) async fn stage_dropped_pack(
    drop_id: String,
    state: State<'_, AppState>,
    native_file_drop: State<'_, NativeFileDropState>,
) -> Result<PackInstallReview, String> {
    let staged = native_file_drop.current(&drop_id)?;
    if !has_pack_extension(Path::new(&staged.name)) {
        return Err(DROPPED_PACK_ERROR.to_string());
    }
    let envelope = read_bounded_pack_artifact(staged.path())?;
    let result =
        stage_production_pack_artifact(state.database.as_ref(), &state.pack_runtime, &envelope)
            .await
            .map_err(|_| "This signed pack could not be verified and staged.".to_string())?;
    native_file_drop.discard_after_success(&drop_id);
    Ok(result)
}

pub(super) fn read_bounded_pack_artifact(path: &Path) -> Result<Vec<u8>, String> {
    let file = open_regular_source(path).map_err(|_| DROPPED_PACK_ERROR.to_string())?;
    match file.metadata() {
        Ok(metadata) if metadata.len() <= MAX_PACK_ARTIFACT_BYTES as u64 => {}
        _ => return Err(DROPPED_PACK_ERROR.to_string()),
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACK_ARTIFACT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DROPPED_PACK_ERROR.to_string())?;
    if bytes.len() > MAX_PACK_ARTIFACT_BYTES {
        return Err(DROPPED_PACK_ERROR.to_string());
    }
    Ok(bytes)
}
