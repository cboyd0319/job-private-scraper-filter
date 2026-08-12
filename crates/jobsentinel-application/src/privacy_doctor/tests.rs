use super::*;
use crate::{config::Config, credentials::CredentialService};
use jobsentinel_domain::ExternalAiProvider;
use jobsentinel_storage::{Database, PortableBackupHistory, StorageHealth};

fn check(report: &PrivacyDoctorReport, id: PrivacyDoctorCheckId) -> &PrivacyDoctorCheck {
    report.checks.iter().find(|check| check.id == id).unwrap()
}

fn safe_signals() -> PrivacyDoctorSignals {
    PrivacyDoctorSignals {
        storage: Some(StorageHealth::Healthy),
        backup: Some(PortableBackupHistory::Succeeded),
        vault: Some(CredentialVaultPrivacyState {
            passphrase_configured: false,
            unlocked: true,
        }),
        external_ai: crate::config::ExternalAiConfig::default(),
        restricted_sources_safe: true,
        browser_import: BrowserImportPrivacyState {
            running: false,
            code_current: true,
        },
    }
}

#[tokio::test]
async fn default_doctor_is_local_read_only_and_keychain_free() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut config = Config::first_run();
    config.dice.query = "registered_nurse Denver".to_string();
    config.jobswithgpt_endpoint = "https://provider.example/private".to_string();
    config.external_ai.model = "private-model-name".to_string();
    config.external_ai.custom_endpoint = "https://provider.example/private".to_string();
    let credentials =
        CredentialService::with_fixed_master_key(database.credentials(), [7_u8; 32], false);

    let report = inspect_privacy_doctor(
        &database,
        &config,
        &credentials,
        BrowserImportPrivacyState {
            running: false,
            code_current: true,
        },
    )
    .await;

    assert_eq!(report.overall, PrivacyDoctorState::OptionalImprovement);
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::Storage).state,
        PrivacyDoctorState::LooksGood
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::BackupHistory).state,
        PrivacyDoctorState::OptionalImprovement
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::Telemetry).state,
        PrivacyDoctorState::LooksGood
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::CredentialVault).state,
        PrivacyDoctorState::OptionalImprovement
    );
    assert!(!report.connectivity_required);
    assert!(report
        .checks
        .iter()
        .all(|check| !check.connectivity_required));
    let serialized = serde_json::to_string(&report).unwrap();
    for private in [
        "registered_nurse",
        "Denver",
        "provider.example",
        "private-model-name",
    ] {
        assert!(!serialized.contains(private));
    }
}

#[test]
fn external_ai_safety_regressions_pause_before_any_send() {
    let mut signals = safe_signals();
    signals.external_ai.enabled = true;
    signals.external_ai.provider = ExternalAiProvider::OpenAi;
    signals.external_ai.enabled_providers = vec![ExternalAiProvider::OpenAi];

    assert_eq!(
        check(
            &build_privacy_doctor(signals.clone()),
            PrivacyDoctorCheckId::ExternalAi
        )
        .state,
        PrivacyDoctorState::LooksGood
    );

    signals.external_ai.enabled_providers = vec![ExternalAiProvider::Anthropic];
    assert_eq!(
        check(
            &build_privacy_doctor(signals.clone()),
            PrivacyDoctorCheckId::ExternalAi
        )
        .state,
        PrivacyDoctorState::PausedForSafety
    );

    signals.external_ai.enabled_providers = vec![ExternalAiProvider::OpenAi];
    signals.external_ai.require_payload_preview = false;

    let report = build_privacy_doctor(signals.clone());
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::ExternalAi).state,
        PrivacyDoctorState::PausedForSafety
    );

    signals.external_ai.require_payload_preview = true;
    signals.external_ai.redaction.enabled = false;
    assert_eq!(
        check(
            &build_privacy_doctor(signals.clone()),
            PrivacyDoctorCheckId::ExternalAi
        )
        .state,
        PrivacyDoctorState::PausedForSafety
    );

    signals.external_ai.redaction.enabled = true;
    signals.external_ai.allow_sensitive_payloads = true;
    assert_eq!(
        check(
            &build_privacy_doctor(signals.clone()),
            PrivacyDoctorCheckId::ExternalAi
        )
        .state,
        PrivacyDoctorState::NeedsAttention
    );

    signals.external_ai.allow_sensitive_payloads = false;
    signals.external_ai.provider = ExternalAiProvider::Custom;
    signals.external_ai.enabled_providers = vec![ExternalAiProvider::Custom];
    signals.external_ai.custom_endpoint = "http://localhost/private".to_string();
    assert_eq!(
        check(
            &build_privacy_doctor(signals),
            PrivacyDoctorCheckId::ExternalAi
        )
        .state,
        PrivacyDoctorState::PausedForSafety
    );
}

#[test]
fn locked_vault_and_damaged_storage_have_plain_repair_states() {
    let mut signals = safe_signals();
    signals.vault = Some(CredentialVaultPrivacyState {
        passphrase_configured: true,
        unlocked: false,
    });
    signals.storage = Some(StorageHealth::RestoreFromBackupRequired);

    let report = build_privacy_doctor(signals);

    assert_eq!(
        check(&report, PrivacyDoctorCheckId::CredentialVault).state,
        PrivacyDoctorState::NeedsAttention
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::CredentialVault).action,
        Some(PrivacyDoctorAction::UnlockCredentialVault)
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::Storage).state,
        PrivacyDoctorState::PausedForSafety
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::Storage).action,
        Some(PrivacyDoctorAction::ReviewRecovery)
    );
    assert_eq!(report.overall, PrivacyDoctorState::PausedForSafety);
}

#[test]
fn backup_and_browser_states_do_not_claim_more_than_local_evidence() {
    for (backup, expected) in [
        (
            PortableBackupHistory::NotRecorded,
            PrivacyDoctorState::OptionalImprovement,
        ),
        (
            PortableBackupHistory::Started,
            PrivacyDoctorState::NeedsAttention,
        ),
        (
            PortableBackupHistory::Failed,
            PrivacyDoctorState::NeedsAttention,
        ),
        (
            PortableBackupHistory::Cancelled,
            PrivacyDoctorState::NeedsAttention,
        ),
        (
            PortableBackupHistory::Succeeded,
            PrivacyDoctorState::OptionalImprovement,
        ),
    ] {
        let mut signals = safe_signals();
        signals.backup = Some(backup);
        assert_eq!(
            check(
                &build_privacy_doctor(signals),
                PrivacyDoctorCheckId::BackupHistory,
            )
            .state,
            expected
        );
    }

    let mut signals = safe_signals();
    signals.browser_import = BrowserImportPrivacyState {
        running: true,
        code_current: false,
    };
    let report = build_privacy_doctor(signals);
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::BrowserImport).action,
        Some(PrivacyDoctorAction::RefreshBrowserImportCode)
    );

    let report = build_privacy_doctor(safe_signals());
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::BackupHistory).action,
        Some(PrivacyDoctorAction::ReviewBackup)
    );
}

#[tokio::test]
async fn retired_source_flags_are_safe_but_payload_drift_pauses() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut signals = safe_signals();
    let mut config = Config::first_run();
    config.dice.enabled = true;
    config.dice.query = "security analyst".to_string();
    config.dice.limit = 25;
    config.restricted_source_acknowledgements.dice = true;
    signals.restricted_sources_safe = restricted_sources_safe(&config);
    assert_eq!(
        check(
            &build_privacy_doctor(signals.clone()),
            PrivacyDoctorCheckId::Sources
        )
        .state,
        PrivacyDoctorState::LooksGood
    );

    let previous = Config {
        restricted_source_acknowledgements: Default::default(),
        ..config.clone()
    };
    crate::restricted_source_consent::reconcile_restricted_source_consents(
        &database,
        &previous,
        &mut config,
    )
    .await
    .unwrap();
    assert!(restricted_sources_safe(&config));

    config.dice.query = "incident responder".to_string();
    assert!(restricted_sources_safe(&config));

    config.jobswithgpt_approval.enabled = true;
    signals.restricted_sources_safe = restricted_sources_safe(&config);
    assert_eq!(
        check(
            &build_privacy_doctor(signals),
            PrivacyDoctorCheckId::Sources
        )
        .state,
        PrivacyDoctorState::PausedForSafety
    );
}

#[test]
fn serialized_doctor_contains_no_private_inputs_or_owner_internals() {
    let mut signals = safe_signals();
    signals.external_ai.model = "registered_nurse Denver".to_string();
    signals.external_ai.custom_endpoint = "https://provider.example/private".to_string();
    signals.external_ai.provider_models.insert(
        ExternalAiProvider::Custom,
        "secret-browser-token".to_string(),
    );
    let report = build_privacy_doctor(signals);
    let serialized = serde_json::to_string(&report).unwrap();
    let value: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["connectivity_required"], false);
    assert_eq!(value["checks"][0]["connectivity_required"], false);
    assert!(value["checks"].as_array().is_some_and(|checks| {
        checks.iter().any(|check| {
            check["id"] == "backup_history"
                && check["state"] == "optional_improvement"
                && check["action"] == "review_backup"
        })
    }));

    for private in [
        "/Users/private/jobs.db",
        "https://provider.example/private",
        "secret-browser-token",
        "registered_nurse Denver",
        "external_ai_openai_api_key",
        "open_ai",
    ] {
        assert!(!serialized.contains(private));
    }
}

#[tokio::test]
async fn unavailable_local_metadata_returns_fixed_attention_without_raw_errors() {
    let database = Database::connect_memory().await.unwrap();
    let credentials =
        CredentialService::with_fixed_master_key(database.credentials(), [7_u8; 32], false);

    let report = inspect_privacy_doctor(
        &database,
        &Config::first_run(),
        &credentials,
        BrowserImportPrivacyState {
            running: false,
            code_current: true,
        },
    )
    .await;
    let serialized = serde_json::to_string(&report).unwrap();

    assert_eq!(
        check(&report, PrivacyDoctorCheckId::BackupHistory).state,
        PrivacyDoctorState::NeedsAttention
    );
    assert_eq!(
        check(&report, PrivacyDoctorCheckId::CredentialVault).state,
        PrivacyDoctorState::NeedsAttention
    );
    for private in ["no such table", "sqlite", "v3_recovery_operations"] {
        assert!(!serialized.to_lowercase().contains(private));
    }
}
