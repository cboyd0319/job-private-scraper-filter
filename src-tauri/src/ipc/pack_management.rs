//! Exposes renderer-safe pack review and generation-bound removal controls.

use crate::application::pack_runtime::{
    activate_production_pack_artifact, disable_pack_artifact, enable_production_pack_artifact,
    list_pack_management_reviews, open_production_active_static_skill, retry_pack_artifact_cleanup,
    rollback_production_pack_artifact, uninstall_pack_artifacts, PackArtifactRemoval,
    PackInstallReview, PackManagementReview, PackStateChange, StaticSkillReview,
};
use crate::bootstrap::AppState;
use crate::desktop::Database;
use crate::ipc::errors::user_friendly_error;
use tauri::State;

#[tauri::command]
pub(crate) async fn list_pack_management(
    state: State<'_, AppState>,
) -> Result<Vec<PackManagementReview>, String> {
    list_pack_management_for_database(state.database.as_ref()).await
}

#[tauri::command]
pub(crate) async fn disable_pack(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackStateChange, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    disable_pack_artifact(
        state.database.as_ref(),
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack could not be disabled", error))
}

#[tauri::command]
pub(crate) async fn activate_pack(
    publisher_key_id: String,
    pack_id: String,
    release_sequence: u64,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackInstallReview, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    if release_sequence == 0 {
        return Err("Refresh Packs before activating this release.".to_string());
    }
    activate_production_pack_artifact(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        release_sequence,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack could not be activated", error))
}

#[tauri::command]
pub(crate) async fn enable_pack(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackStateChange, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    enable_production_pack_artifact(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack could not be enabled", error))
}

#[tauri::command]
pub(crate) async fn rollback_pack(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackInstallReview, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    rollback_production_pack_artifact(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack could not be rolled back", error))
}

#[tauri::command]
pub(crate) async fn open_static_skill(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<StaticSkillReview, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    open_production_active_static_skill(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Static skill could not be opened", error))
}

#[tauri::command]
pub(crate) async fn uninstall_pack(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackArtifactRemoval, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    uninstall_pack_artifacts(
        state.database.as_ref(),
        state.pack_runtime.artifact_root(),
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack could not be removed", error))
}

#[tauri::command]
pub(crate) async fn retry_pack_cleanup(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    state: State<'_, AppState>,
) -> Result<PackArtifactRemoval, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    retry_pack_artifact_cleanup(
        state.database.as_ref(),
        state.pack_runtime.artifact_root(),
        &publisher_key_id,
        &pack_id,
        expected_generation,
    )
    .await
    .map_err(|error| user_friendly_error("Pack cleanup could not be retried", error))
}

async fn list_pack_management_for_database(
    database: &Database,
) -> Result<Vec<PackManagementReview>, String> {
    list_pack_management_reviews(database)
        .await
        .map_err(|error| user_friendly_error("Pack information is unavailable", error))
}

pub(super) fn validate_pack_identity(publisher_key_id: &str, pack_id: &str) -> Result<(), String> {
    if [publisher_key_id, pack_id].into_iter().all(|value| {
        !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':')
            })
    }) {
        Ok(())
    } else {
        Err("Refresh Packs before changing this pack.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{list_pack_management_for_database, validate_pack_identity};
    use crate::desktop::Database;

    #[tokio::test]
    async fn list_command_returns_the_renderer_safe_management_projection() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();

        assert!(list_pack_management_for_database(&database)
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn lifecycle_commands_accept_only_bounded_pack_identities() {
        assert_eq!(
            validate_pack_identity("jobsentinel-release-v1", "jobsentinel.sources.us"),
            Ok(())
        );
        for invalid in ["", "bad pack", "bad\0pack", &"a".repeat(129)] {
            assert!(validate_pack_identity(invalid, "jobsentinel.sources.us").is_err());
            assert!(validate_pack_identity("jobsentinel-release-v1", invalid).is_err());
        }
    }
}
