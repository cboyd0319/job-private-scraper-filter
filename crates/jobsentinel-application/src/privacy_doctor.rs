use crate::{
    config::{Config, ExternalAiConfig},
    credentials::CredentialService,
};
use jobsentinel_domain::ExternalAiProvider;
use jobsentinel_security::validate_credential_free_external_https_url;
use jobsentinel_storage::{Database, PortableBackupHistory, StorageHealth};
use serde::Serialize;

/// Stable privacy review state suitable for display or safe support output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyDoctorState {
    LooksGood,
    NeedsAttention,
    PausedForSafety,
    OptionalImprovement,
}

impl PrivacyDoctorState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LooksGood => "looks_good",
            Self::NeedsAttention => "needs_attention",
            Self::PausedForSafety => "paused_for_safety",
            Self::OptionalImprovement => "optional_improvement",
        }
    }
}

/// Fixed privacy review identifiers. No user data is stored in these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyDoctorCheckId {
    Telemetry,
    Storage,
    BackupHistory,
    CredentialVault,
    ExternalAi,
    BrowserImport,
    Sources,
}

impl PrivacyDoctorCheckId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Telemetry => "telemetry",
            Self::Storage => "storage",
            Self::BackupHistory => "backup_history",
            Self::CredentialVault => "credential_vault",
            Self::ExternalAi => "external_ai",
            Self::BrowserImport => "browser_import",
            Self::Sources => "sources",
        }
    }
}

/// Fixed user actions for privacy review findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyDoctorAction {
    CreatePortableBackup,
    ReviewBackup,
    UnlockCredentialVault,
    ReviewRecovery,
    ReviewExternalAi,
    RefreshBrowserImportCode,
    ReviewSourceSafety,
}

impl PrivacyDoctorAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreatePortableBackup => "create_portable_backup",
            Self::ReviewBackup => "review_backup",
            Self::UnlockCredentialVault => "unlock_credential_vault",
            Self::ReviewRecovery => "review_recovery",
            Self::ReviewExternalAi => "review_external_ai",
            Self::RefreshBrowserImportCode => "refresh_browser_import_code",
            Self::ReviewSourceSafety => "review_source_safety",
        }
    }
}

/// One bounded Privacy Doctor result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivacyDoctorCheck {
    pub id: PrivacyDoctorCheckId,
    pub state: PrivacyDoctorState,
    pub message: &'static str,
    pub action: Option<PrivacyDoctorAction>,
    pub connectivity_required: bool,
}

/// Local-only Privacy Doctor report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivacyDoctorReport {
    pub schema_version: u32,
    pub overall: PrivacyDoctorState,
    pub checks: Vec<PrivacyDoctorCheck>,
    pub connectivity_required: bool,
}

/// Caller-owned Browser Import state used without starting the receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserImportPrivacyState {
    pub running: bool,
    pub code_current: bool,
}

#[derive(Debug, Clone)]
struct CredentialVaultPrivacyState {
    passphrase_configured: bool,
    unlocked: bool,
}

#[derive(Debug, Clone)]
struct PrivacyDoctorSignals {
    storage: Option<StorageHealth>,
    backup: Option<PortableBackupHistory>,
    vault: Option<CredentialVaultPrivacyState>,
    external_ai: ExternalAiConfig,
    restricted_sources_safe: bool,
    browser_import: BrowserImportPrivacyState,
}

/// Inspect local privacy controls without reading secrets or using the network.
pub async fn inspect_privacy_doctor(
    database: &Database,
    config: &Config,
    credentials: &CredentialService,
    browser_import: BrowserImportPrivacyState,
) -> PrivacyDoctorReport {
    let storage = database
        .inspect_storage_maintenance()
        .await
        .ok()
        .map(|report| report.health);
    let backup = database.inspect_portable_backup_history().await.ok();
    let vault = credentials
        .unlock_status()
        .await
        .ok()
        .map(|state| CredentialVaultPrivacyState {
            passphrase_configured: state.configured,
            unlocked: state.unlocked,
        });

    build_privacy_doctor(PrivacyDoctorSignals {
        storage,
        backup,
        vault,
        external_ai: config.external_ai.clone(),
        restricted_sources_safe: restricted_sources_safe(config),
        browser_import,
    })
}

fn build_privacy_doctor(signals: PrivacyDoctorSignals) -> PrivacyDoctorReport {
    let mut checks = vec![privacy_check(
        PrivacyDoctorCheckId::Telemetry,
        PrivacyDoctorState::LooksGood,
        "JobSentinel does not send telemetry.",
        None,
    )];

    checks.push(match signals.storage {
        Some(StorageHealth::Healthy) => privacy_check(
            PrivacyDoctorCheckId::Storage,
            PrivacyDoctorState::LooksGood,
            "Local storage passed its integrity checks.",
            None,
        ),
        Some(StorageHealth::RestoreFromBackupRequired) => privacy_check(
            PrivacyDoctorCheckId::Storage,
            PrivacyDoctorState::PausedForSafety,
            "Local storage needs recovery before maintenance continues.",
            Some(PrivacyDoctorAction::ReviewRecovery),
        ),
        None => privacy_check(
            PrivacyDoctorCheckId::Storage,
            PrivacyDoctorState::NeedsAttention,
            "Local storage health could not be checked.",
            Some(PrivacyDoctorAction::ReviewRecovery),
        ),
    });

    checks.push(match signals.backup {
        Some(PortableBackupHistory::NotRecorded) => privacy_check(
            PrivacyDoctorCheckId::BackupHistory,
            PrivacyDoctorState::OptionalImprovement,
            "No portable backup creation is recorded.",
            Some(PrivacyDoctorAction::CreatePortableBackup),
        ),
        Some(PortableBackupHistory::Succeeded) => privacy_check(
            PrivacyDoctorCheckId::BackupHistory,
            PrivacyDoctorState::OptionalImprovement,
            "A portable backup creation is recorded. Confirm the file still exists.",
            Some(PrivacyDoctorAction::ReviewBackup),
        ),
        Some(
            PortableBackupHistory::Started
            | PortableBackupHistory::Failed
            | PortableBackupHistory::Cancelled,
        ) => privacy_check(
            PrivacyDoctorCheckId::BackupHistory,
            PrivacyDoctorState::NeedsAttention,
            "The latest portable backup attempt did not complete successfully.",
            Some(PrivacyDoctorAction::ReviewBackup),
        ),
        None => privacy_check(
            PrivacyDoctorCheckId::BackupHistory,
            PrivacyDoctorState::NeedsAttention,
            "Portable backup history could not be checked.",
            Some(PrivacyDoctorAction::ReviewBackup),
        ),
    });

    checks.push(match signals.vault {
        Some(CredentialVaultPrivacyState {
            passphrase_configured: true,
            unlocked: false,
        }) => privacy_check(
            PrivacyDoctorCheckId::CredentialVault,
            PrivacyDoctorState::NeedsAttention,
            "The passphrase-protected credential vault is locked.",
            Some(PrivacyDoctorAction::UnlockCredentialVault),
        ),
        Some(CredentialVaultPrivacyState {
            passphrase_configured: true,
            unlocked: true,
        }) => privacy_check(
            PrivacyDoctorCheckId::CredentialVault,
            PrivacyDoctorState::LooksGood,
            "The passphrase-protected credential vault is unlocked.",
            None,
        ),
        Some(CredentialVaultPrivacyState {
            passphrase_configured: false,
            ..
        }) => privacy_check(
            PrivacyDoctorCheckId::CredentialVault,
            PrivacyDoctorState::OptionalImprovement,
            "System credential storage is checked only when you use it.",
            None,
        ),
        None => privacy_check(
            PrivacyDoctorCheckId::CredentialVault,
            PrivacyDoctorState::NeedsAttention,
            "Credential vault metadata could not be checked.",
            Some(PrivacyDoctorAction::UnlockCredentialVault),
        ),
    });

    checks.push(external_ai_check(&signals.external_ai));
    checks.push(browser_import_check(signals.browser_import));
    checks.push(if signals.restricted_sources_safe {
        privacy_check(
            PrivacyDoctorCheckId::Sources,
            PrivacyDoctorState::LooksGood,
            "Enabled restricted sources have local acknowledgements.",
            None,
        )
    } else {
        privacy_check(
            PrivacyDoctorCheckId::Sources,
            PrivacyDoctorState::PausedForSafety,
            "A restricted source needs local review before use.",
            Some(PrivacyDoctorAction::ReviewSourceSafety),
        )
    });
    PrivacyDoctorReport {
        schema_version: 1,
        overall: overall_state(&checks),
        checks,
        connectivity_required: false,
    }
}

fn external_ai_check(config: &ExternalAiConfig) -> PrivacyDoctorCheck {
    if !config.enabled {
        return privacy_check(
            PrivacyDoctorCheckId::ExternalAi,
            PrivacyDoctorState::LooksGood,
            "External AI is disabled.",
            None,
        );
    }
    if config.provider == ExternalAiProvider::None
        || !config.enabled_providers.contains(&config.provider)
        || !config.require_payload_preview
        || !config.redaction.enabled
        || (config
            .enabled_providers
            .contains(&ExternalAiProvider::Custom)
            && validate_credential_free_external_https_url(config.custom_endpoint.trim()).is_err())
    {
        return privacy_check(
            PrivacyDoctorCheckId::ExternalAi,
            PrivacyDoctorState::PausedForSafety,
            "External AI must keep provider selection, preview, and redaction safeguards enabled.",
            Some(PrivacyDoctorAction::ReviewExternalAi),
        );
    }
    if config.allow_sensitive_payloads {
        return privacy_check(
            PrivacyDoctorCheckId::ExternalAi,
            PrivacyDoctorState::NeedsAttention,
            "External AI is allowed to receive sensitive payloads after explicit review.",
            Some(PrivacyDoctorAction::ReviewExternalAi),
        );
    }
    privacy_check(
        PrivacyDoctorCheckId::ExternalAi,
        PrivacyDoctorState::LooksGood,
        "External AI keeps preview and redaction safeguards enabled.",
        None,
    )
}

fn browser_import_check(state: BrowserImportPrivacyState) -> PrivacyDoctorCheck {
    if state.running && !state.code_current {
        privacy_check(
            PrivacyDoctorCheckId::BrowserImport,
            PrivacyDoctorState::NeedsAttention,
            "The running Browser Import receiver has no active one-use pairing.",
            Some(PrivacyDoctorAction::RefreshBrowserImportCode),
        )
    } else {
        privacy_check(
            PrivacyDoctorCheckId::BrowserImport,
            PrivacyDoctorState::LooksGood,
            "Browser Import is stopped or has a current one-use pairing.",
            None,
        )
    }
}

fn restricted_sources_safe(config: &Config) -> bool {
    !config.jobswithgpt_approval.enabled || config.jobswithgpt_payload_approved()
}

fn privacy_check(
    id: PrivacyDoctorCheckId,
    state: PrivacyDoctorState,
    message: &'static str,
    action: Option<PrivacyDoctorAction>,
) -> PrivacyDoctorCheck {
    PrivacyDoctorCheck {
        id,
        state,
        message,
        action,
        connectivity_required: false,
    }
}

fn overall_state(checks: &[PrivacyDoctorCheck]) -> PrivacyDoctorState {
    if checks
        .iter()
        .any(|check| check.state == PrivacyDoctorState::PausedForSafety)
    {
        PrivacyDoctorState::PausedForSafety
    } else if checks
        .iter()
        .any(|check| check.state == PrivacyDoctorState::NeedsAttention)
    {
        PrivacyDoctorState::NeedsAttention
    } else if checks
        .iter()
        .any(|check| check.state == PrivacyDoctorState::OptionalImprovement)
    {
        PrivacyDoctorState::OptionalImprovement
    } else {
        PrivacyDoctorState::LooksGood
    }
}

#[cfg(test)]
mod tests;
