use crate::{
    config::Config,
    credentials::CredentialService,
    pending::PendingUrlImports,
    privacy_doctor::{inspect_privacy_doctor, BrowserImportPrivacyState, PrivacyDoctorReport},
};
use chrono::Utc;
pub use jobsentinel_platform::{
    PlatformHealthReport, PlatformPermissionRepair, PlatformStorageArea,
};
use jobsentinel_storage::{Database, StorageHealth, StorageMaintenanceReport};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalStorageRecoveryState {
    Ready,
    RestoreFromBackupRequired,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LocalStorageRecoveryReport {
    pub state: LocalStorageRecoveryState,
    pub reclaimable_bytes: u64,
    pub wal_bytes: Option<u64>,
    pub incremental_vacuum_supported: bool,
    pub cleanup_available: bool,
    pub connectivity_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct QueuedLocalWorkReport {
    pub pending_url_imports: usize,
    pub capacity: usize,
    pub available_offline: bool,
    pub connectivity_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalRecoveryReport {
    pub schema_version: u32,
    pub connectivity_required: bool,
    pub queued_local_work: QueuedLocalWorkReport,
    pub storage: LocalStorageRecoveryReport,
    pub privacy_doctor: PrivacyDoctorReport,
    pub platform_health: PlatformHealthReport,
}

pub async fn inspect_local_recovery(
    database: &Database,
    config: &Config,
    credentials: &CredentialService,
    pending_url_imports: &PendingUrlImports,
    browser_import: BrowserImportPrivacyState,
) -> LocalRecoveryReport {
    inspect_local_recovery_with_platform(
        database,
        config,
        credentials,
        pending_url_imports,
        browser_import,
        jobsentinel_platform::inspect_platform_health(),
    )
    .await
}

async fn inspect_local_recovery_with_platform(
    database: &Database,
    config: &Config,
    credentials: &CredentialService,
    pending_url_imports: &PendingUrlImports,
    browser_import: BrowserImportPrivacyState,
    platform_health: PlatformHealthReport,
) -> LocalRecoveryReport {
    let storage = database
        .inspect_storage_maintenance()
        .await
        .map(storage_report)
        .unwrap_or_else(|_| unavailable_storage_report());
    let privacy_doctor =
        inspect_privacy_doctor(database, config, credentials, browser_import).await;

    LocalRecoveryReport {
        schema_version: 2,
        connectivity_required: false,
        queued_local_work: QueuedLocalWorkReport {
            pending_url_imports: pending_url_imports.current_count(Utc::now()),
            capacity: PendingUrlImports::capacity(),
            available_offline: true,
            connectivity_required: false,
        },
        storage,
        privacy_doctor,
        platform_health,
    }
}

#[must_use]
pub fn repair_local_permissions(area: PlatformStorageArea) -> PlatformPermissionRepair {
    jobsentinel_platform::repair_platform_permissions(area)
}

pub async fn run_local_storage_cleanup(
    database: &Database,
) -> anyhow::Result<LocalStorageRecoveryReport> {
    Ok(storage_report(database.run_storage_cleanup().await?))
}

fn storage_report(report: StorageMaintenanceReport) -> LocalStorageRecoveryReport {
    let state = match report.health {
        StorageHealth::Healthy => LocalStorageRecoveryState::Ready,
        StorageHealth::RestoreFromBackupRequired => {
            LocalStorageRecoveryState::RestoreFromBackupRequired
        }
    };
    LocalStorageRecoveryReport {
        state,
        reclaimable_bytes: report.reclaimable_bytes,
        wal_bytes: report.wal_bytes,
        incremental_vacuum_supported: report.incremental_vacuum_supported,
        cleanup_available: state == LocalStorageRecoveryState::Ready,
        connectivity_required: false,
    }
}

fn unavailable_storage_report() -> LocalStorageRecoveryReport {
    LocalStorageRecoveryReport {
        state: LocalStorageRecoveryState::Unavailable,
        reclaimable_bytes: 0,
        wal_bytes: None,
        incremental_vacuum_supported: false,
        cleanup_available: false,
        connectivity_required: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config, credentials::CredentialService, privacy_doctor::BrowserImportPrivacyState,
    };
    use jobsentinel_platform::{
        PackageRepairAction, PackageRepairActionId, PackageRepairGuidance, PackageRepairMode,
        PlatformHealthReport, PlatformPermissionAction, PlatformPermissionCheck,
        PlatformPermissionState, PlatformStorageArea, PLATFORM_HEALTH_SCHEMA_VERSION,
    };
    use jobsentinel_storage::Database;

    fn platform_health() -> PlatformHealthReport {
        PlatformHealthReport {
            schema_version: PLATFORM_HEALTH_SCHEMA_VERSION,
            permissions: [
                PlatformPermissionCheck {
                    area: PlatformStorageArea::ApplicationData,
                    state: PlatformPermissionState::Private,
                    action: None,
                    connectivity_required: false,
                },
                PlatformPermissionCheck {
                    area: PlatformStorageArea::Configuration,
                    state: PlatformPermissionState::NeedsRepair,
                    action: Some(PlatformPermissionAction::RepairLocally),
                    connectivity_required: false,
                },
                PlatformPermissionCheck {
                    area: PlatformStorageArea::Cache,
                    state: PlatformPermissionState::Missing,
                    action: Some(PlatformPermissionAction::RepairLocally),
                    connectivity_required: false,
                },
            ],
            package_repair: PackageRepairGuidance {
                mode: PackageRepairMode::GuidanceOnly,
                actions: [
                    PackageRepairAction {
                        action: PackageRepairActionId::UseDownloadedVerifiedInstaller,
                        connectivity_required: false,
                    },
                    PackageRepairAction {
                        action: PackageRepairActionId::ObtainVerifiedInstaller,
                        connectivity_required: true,
                    },
                ],
            },
        }
    }

    #[tokio::test]
    async fn local_recovery_report_is_bounded_offline_and_keychain_free() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let config = Config::first_run();
        let credentials =
            CredentialService::with_fixed_master_key(database.credentials(), [3_u8; 32], false);
        let pending_url_imports = PendingUrlImports::default();

        let report = inspect_local_recovery_with_platform(
            &database,
            &config,
            &credentials,
            &pending_url_imports,
            BrowserImportPrivacyState {
                running: false,
                code_current: true,
            },
            platform_health(),
        )
        .await;

        assert_eq!(report.schema_version, 2);
        assert!(!report.connectivity_required);
        assert_eq!(report.queued_local_work.pending_url_imports, 0);
        assert_eq!(report.queued_local_work.capacity, 20);
        assert!(report.queued_local_work.available_offline);
        assert!(!report.queued_local_work.connectivity_required);
        assert_eq!(report.storage.state, LocalStorageRecoveryState::Ready);
        assert!(report.storage.cleanup_available);
        assert!(!report.storage.connectivity_required);
        assert!(report
            .privacy_doctor
            .checks
            .iter()
            .all(|check| !check.connectivity_required));
        assert!(report
            .platform_health
            .package_repair
            .actions
            .iter()
            .any(|action| action.connectivity_required));

        let serialized = serde_json::to_string(&report).unwrap();
        for private in [
            "/Users/private/jobs.db",
            "C:\\Users\\private\\jobs.db",
            "secret-token",
            "registered_nurse Denver",
        ] {
            assert!(!serialized.contains(private));
        }
    }

    #[tokio::test]
    async fn cleanup_returns_a_fresh_local_report_without_network() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();

        let report = run_local_storage_cleanup(&database).await.unwrap();

        assert_eq!(report.state, LocalStorageRecoveryState::Ready);
        assert!(report.cleanup_available);
        assert!(!report.connectivity_required);
    }
}
