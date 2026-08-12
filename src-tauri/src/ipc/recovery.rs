use crate::{
    application::{
        desktop::{Database, PortableRestoreStatus, StartupConfigRepair},
        privacy_doctor::BrowserImportPrivacyState,
        recovery::{
            self, LocalRecoveryReport, LocalStorageRecoveryReport, PlatformPermissionRepair,
            PlatformStorageArea,
        },
        user_data::{ReviewedExportPlan, ReviewedExportSelection},
    },
    bootstrap::{AppState, StartupRecoveryState},
    ipc::errors::user_friendly_error,
};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::oneshot;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct StartupRecoveryStatus {
    pub required: bool,
    pub platform: bool,
    pub configuration: bool,
    pub database: bool,
    pub connectivity_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryFileActionOutcome {
    Succeeded,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct RecoveryFileActionResult {
    pub outcome: RecoveryFileActionOutcome,
    pub connectivity_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PortableRestoreActionOutcome {
    Staged,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct PortableRestoreActionResult {
    pub outcome: PortableRestoreActionOutcome,
    pub connectivity_required: bool,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CancelRestoreOutcome {
    Cancelled,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct CancelRestoreResult {
    pub outcome: CancelRestoreOutcome,
    pub connectivity_required: bool,
    pub restart_required: bool,
}

fn startup_recovery_status(state: &StartupRecoveryState) -> StartupRecoveryStatus {
    StartupRecoveryStatus {
        required: state.required(),
        platform: state.platform(),
        configuration: state.configuration(),
        database: state.database(),
        connectivity_required: false,
    }
}

#[tauri::command]
pub(crate) fn get_startup_recovery_status(
    state: State<'_, StartupRecoveryState>,
) -> StartupRecoveryStatus {
    startup_recovery_status(&state)
}

#[tauri::command]
pub(crate) async fn get_local_recovery_report(
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
) -> Result<LocalRecoveryReport, String> {
    require_ready_storage(&recovery)?;
    let config = state.config.read().await.clone();
    let browser_import = {
        let server = state.bookmarklet_server.read().await;
        BrowserImportPrivacyState {
            running: server.is_running(),
            code_current: server.pairing_is_current(),
        }
    };

    Ok(recovery::inspect_local_recovery(
        state.database.as_ref(),
        &config,
        state.credentials.as_ref(),
        &state.pending_url_imports,
        browser_import,
    )
    .await)
}

#[tauri::command]
pub(crate) async fn run_local_storage_cleanup(
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
) -> Result<LocalStorageRecoveryReport, String> {
    require_ready_storage(&recovery)?;
    recovery::run_local_storage_cleanup(state.database.as_ref())
        .await
        .map_err(|error| user_friendly_error("Local storage cleanup failed", error))
}

#[tauri::command]
pub(crate) fn repair_local_permissions(area: PlatformStorageArea) -> PlatformPermissionRepair {
    recovery::repair_local_permissions(area)
}

#[tauri::command]
pub(crate) fn repair_invalid_startup_config(
    state: State<'_, StartupRecoveryState>,
) -> Result<StartupConfigRepair, String> {
    if !state.configuration() {
        return Err("Saved settings are not eligible for startup recovery.".to_string());
    }
    crate::application::desktop::repair_invalid_startup_config()
        .map_err(|error| user_friendly_error(error.context(), error))
}

#[tauri::command]
pub(crate) async fn create_portable_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    passphrase: String,
) -> Result<RecoveryFileActionResult, String> {
    if recovery.required() {
        return Err("Portable backup is unavailable until startup recovery finishes.".to_string());
    }
    let passphrase = Zeroizing::new(passphrase);
    validate_portable_passphrase(&passphrase)?;
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("JobSentinel encrypted backup", &["db"])
        .set_file_name("JobSentinel-portable-backup.db")
        .blocking_save_file()
    else {
        return Ok(cancelled_file_action());
    };
    let destination = file
        .into_path()
        .map_err(|_| "The selected backup destination is unavailable.".to_string())?;
    state
        .database
        .create_portable_backup(&destination, &passphrase)
        .await
        .map_err(|error| user_friendly_error("Encrypted portable backup failed", error))?;
    Ok(succeeded_file_action())
}

#[tauri::command]
pub(crate) async fn stage_portable_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    passphrase: String,
) -> Result<PortableRestoreActionResult, String> {
    require_restore_available(&recovery)?;
    let passphrase = Zeroizing::new(passphrase);
    validate_portable_passphrase(&passphrase)?;
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("JobSentinel encrypted backup", &["db"])
        .blocking_pick_file()
    else {
        return Ok(PortableRestoreActionResult {
            outcome: PortableRestoreActionOutcome::Cancelled,
            connectivity_required: false,
            restart_required: false,
        });
    };
    let backup = file
        .into_path()
        .map_err(|_| "The selected encrypted backup is unavailable.".to_string())?;
    stage_portable_restore_at_validated_path(&state, &recovery, &backup, &passphrase).await
}

pub(crate) async fn stage_portable_restore_from_path(
    state: &AppState,
    recovery: &StartupRecoveryState,
    backup: &std::path::Path,
    passphrase: &str,
) -> Result<PortableRestoreActionResult, String> {
    require_restore_available(recovery)?;
    validate_portable_passphrase(passphrase)?;
    stage_portable_restore_at_validated_path(state, recovery, backup, passphrase).await
}

async fn stage_portable_restore_at_validated_path(
    state: &AppState,
    recovery: &StartupRecoveryState,
    backup: &std::path::Path,
    passphrase: &str,
) -> Result<PortableRestoreActionResult, String> {
    if recovery.database() {
        Database::stage_portable_restore_at(&Database::default_path(), backup, passphrase)
            .await
            .map_err(|error| user_friendly_error("Encrypted restore could not be staged", error))?;
    } else {
        state
            .database
            .stage_portable_restore(backup, passphrase)
            .await
            .map_err(|error| user_friendly_error("Encrypted restore could not be staged", error))?;
    }
    Ok(PortableRestoreActionResult {
        outcome: PortableRestoreActionOutcome::Staged,
        connectivity_required: false,
        restart_required: true,
    })
}

#[tauri::command]
pub(crate) fn get_staged_restore_status() -> Result<PortableRestoreStatus, String> {
    Database::staged_restore_status(&Database::default_path())
        .map_err(|error| user_friendly_error("Restore status is unavailable", error))
}

#[tauri::command]
pub(crate) async fn cancel_staged_restore(
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
) -> Result<CancelRestoreResult, String> {
    if recovery.required() && !recovery.database() {
        return Err("Portable restore is available only for local database recovery.".to_string());
    }
    let cancelled = if recovery.database() {
        Database::cancel_staged_restore_at(&Database::default_path())
            .map_err(|error| user_friendly_error("Staged restore could not be cancelled", error))?
    } else {
        state
            .database
            .cancel_staged_restore()
            .await
            .map_err(|error| user_friendly_error("Staged restore could not be cancelled", error))?
    };
    Ok(CancelRestoreResult {
        outcome: if cancelled {
            CancelRestoreOutcome::Cancelled
        } else {
            CancelRestoreOutcome::NotFound
        },
        connectivity_required: false,
        restart_required: false,
    })
}

#[tauri::command]
pub(crate) async fn create_reviewed_export(
    app: AppHandle,
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    include_protected_records: bool,
) -> Result<RecoveryFileActionResult, String> {
    if recovery.required() {
        return Err("Reviewed export is unavailable until startup recovery finishes.".to_string());
    }
    let selection = if include_protected_records {
        ReviewedExportSelection::including_protected_records()
    } else {
        ReviewedExportSelection::default()
    };
    let plan = state
        .database
        .review_plaintext_export(selection)
        .await
        .map_err(|error| user_friendly_error("Reviewed export could not be prepared", error))?;
    if !confirm_reviewed_export(&app, &plan).await? {
        return Ok(cancelled_file_action());
    }
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("JobSentinel reviewed export", &["jsonl"])
        .set_file_name("JobSentinel-reviewed-export.jsonl")
        .blocking_save_file()
    else {
        return Ok(cancelled_file_action());
    };
    let destination = file
        .into_path()
        .map_err(|_| "The selected export destination is unavailable.".to_string())?;
    state
        .database
        .create_reviewed_export(&destination, plan)
        .await
        .map_err(|error| user_friendly_error("Reviewed export failed", error))?;
    Ok(succeeded_file_action())
}

async fn confirm_reviewed_export(
    app: &AppHandle,
    plan: &ReviewedExportPlan,
) -> Result<bool, String> {
    let message = reviewed_export_confirmation(
        plan.total_record_count(),
        plan.section_counts(),
        plan.protected_application_answer_count(),
        plan.protected_resume_draft_count(),
        plan.protected_records_included(),
    );
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(message)
        .title("Confirm Reviewed Plaintext Export")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Create Plaintext Export".to_string(),
            "Keep Local".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "Reviewed export confirmation could not be completed.".to_string())
}

fn reviewed_export_confirmation(
    total_records: u64,
    section_counts: &std::collections::BTreeMap<String, u64>,
    protected_answers: u64,
    protected_drafts: u64,
    protected_included: bool,
) -> String {
    let sections = section_counts
        .iter()
        .map(|(section, count)| format!("{section}: {count}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Create a plaintext JSON Lines export with {total_records} reviewed record(s)?\n\n\
         Selected categories:\n{sections}\n\n\
         Protected application answers: {protected_answers}. \
         Structured resume drafts with protected fields: {protected_drafts}. \
         Protected records included: {}.\n\n\
         JobSentinel-managed credentials and private app paths are excluded. \
         User-authored text is copied as written and may contain private details you pasted. \
         Review the saved file before sharing it.",
        if protected_included { "yes" } else { "no" }
    )
}

const fn succeeded_file_action() -> RecoveryFileActionResult {
    RecoveryFileActionResult {
        outcome: RecoveryFileActionOutcome::Succeeded,
        connectivity_required: false,
    }
}

const fn cancelled_file_action() -> RecoveryFileActionResult {
    RecoveryFileActionResult {
        outcome: RecoveryFileActionOutcome::Cancelled,
        connectivity_required: false,
    }
}

fn require_ready_storage(state: &StartupRecoveryState) -> Result<(), String> {
    if state.required() {
        return Err(
            "Local storage actions are unavailable until startup recovery finishes.".to_string(),
        );
    }
    Ok(())
}

fn require_restore_available(state: &StartupRecoveryState) -> Result<(), String> {
    if state.platform() {
        return Err("Repair local permissions before staging a restore.".to_string());
    }
    if state.required() && !state.database() {
        return Err("Portable restore is available only for local database recovery.".to_string());
    }
    Ok(())
}

fn validate_portable_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.chars().count() < 16 {
        return Err("Use a backup passphrase with at least 16 characters.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::DesktopStartupFailureKind;

    #[test]
    fn startup_status_is_bounded_and_offline() {
        let state = StartupRecoveryState::new(true, Some(DesktopStartupFailureKind::Database));

        assert_eq!(
            startup_recovery_status(&state),
            StartupRecoveryStatus {
                required: true,
                platform: true,
                configuration: false,
                database: true,
                connectivity_required: false,
            }
        );
    }

    #[test]
    fn local_storage_actions_fail_closed_during_startup_recovery() {
        let state = StartupRecoveryState::new(false, Some(DesktopStartupFailureKind::Database));

        assert_eq!(
            require_ready_storage(&state),
            Err(
                "Local storage actions are unavailable until startup recovery finishes."
                    .to_string()
            )
        );
    }

    #[test]
    fn portable_recovery_requires_a_substantial_passphrase() {
        assert!(validate_portable_passphrase("too-short").is_err());
        assert!(validate_portable_passphrase("sixteen-letters!!").is_ok());
    }

    #[test]
    fn restore_staging_waits_for_platform_repair() {
        let state = StartupRecoveryState::new(true, Some(DesktopStartupFailureKind::Database));

        assert_eq!(
            require_restore_available(&state),
            Err("Repair local permissions before staging a restore.".to_string())
        );
    }

    #[test]
    fn reviewed_export_confirmation_is_exact_and_warns_about_plaintext() {
        let sections = std::collections::BTreeMap::from([
            ("applications".to_string(), 12),
            ("jobs".to_string(), 30),
        ]);
        let message = reviewed_export_confirmation(42, &sections, 3, 2, false);

        assert!(message.contains("42 reviewed record(s)"));
        assert!(message.contains("applications: 12"));
        assert!(message.contains("jobs: 30"));
        assert!(message.contains("Protected application answers: 3"));
        assert!(message.contains("Structured resume drafts with protected fields: 2"));
        assert!(message.contains("Protected records included: no"));
        assert!(message.contains("User-authored text is copied as written"));
        assert!(!message.contains("/Users/"));
        assert!(!message.contains("secret-token"));
    }
}
