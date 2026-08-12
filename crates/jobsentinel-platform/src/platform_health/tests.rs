// Verifies bounded platform-health inspection and repair outcomes for owned storage roots.

use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
fn resolved_temp_root(temp_dir: &tempfile::TempDir) -> PathBuf {
    temp_dir.path().canonicalize().unwrap()
}

#[cfg(unix)]
#[test]
fn unix_inspection_and_repair_are_local_and_bounded() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_root = resolved_temp_root(&temp_dir);
    let data = temp_root.join("data");
    let config = temp_root.join("config");
    let cache = temp_root.join("cache");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o755)).unwrap();

    let paths = PlatformPaths::new(&data, &config, &cache);
    let report = inspect_platform_health_at(&paths, true);

    assert_eq!(report.schema_version, PLATFORM_HEALTH_SCHEMA_VERSION);
    assert_eq!(report.permissions.len(), 3);
    assert_eq!(
        report.permissions[0],
        PlatformPermissionCheck {
            area: PlatformStorageArea::ApplicationData,
            state: PlatformPermissionState::NeedsRepair,
            action: Some(PlatformPermissionAction::RepairLocally),
            connectivity_required: false,
        }
    );
    assert_eq!(
        repair_platform_permissions_at(PlatformStorageArea::ApplicationData, &paths, true,),
        PlatformPermissionRepair {
            schema_version: PLATFORM_HEALTH_SCHEMA_VERSION,
            area: PlatformStorageArea::ApplicationData,
            outcome: PlatformPermissionRepairOutcome::Repaired,
            connectivity_required: false,
        }
    );
    assert_eq!(
        std::fs::metadata(&data).unwrap().permissions().mode() & 0o777,
        0o700
    );
}

#[cfg(unix)]
#[test]
fn unix_repair_refuses_a_linked_storage_root() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let temp_root = resolved_temp_root(&temp_dir);
    let external = temp_root.join("external");
    let data = temp_root.join("data");
    std::fs::create_dir(&external).unwrap();
    std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&external, &data).unwrap();
    let paths = PlatformPaths::new(&data, temp_root.join("config"), temp_root.join("cache"));

    assert_eq!(
        repair_platform_permissions_at(PlatformStorageArea::ApplicationData, &paths, true).outcome,
        PlatformPermissionRepairOutcome::ManualGuidanceRequired
    );
    assert_eq!(
        std::fs::metadata(&external).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[cfg(unix)]
#[test]
fn unix_repair_refuses_hard_linked_children_without_changing_external_modes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_root = resolved_temp_root(&temp_dir);
    let data = temp_root.join("data");
    let external = temp_root.join("external-tool");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(&external, b"external").unwrap();
    std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::hard_link(&external, data.join("linked-tool")).unwrap();
    let paths = PlatformPaths::new(&data, temp_root.join("config"), temp_root.join("cache"));

    assert_eq!(
        inspect_platform_health_at(&paths, true).permissions[0].state,
        PlatformPermissionState::ManualReview
    );
    assert_eq!(
        repair_platform_permissions_at(PlatformStorageArea::ApplicationData, &paths, true).outcome,
        PlatformPermissionRepairOutcome::ManualGuidanceRequired
    );
    assert_eq!(
        std::fs::metadata(external).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[cfg(unix)]
#[test]
fn unix_health_requires_manual_review_for_symlinked_children() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let temp_dir = tempfile::tempdir().unwrap();
    let temp_root = resolved_temp_root(&temp_dir);
    let data = temp_root.join("data");
    let external = temp_root.join("external");
    std::fs::create_dir(&data).unwrap();
    std::fs::write(&external, b"external").unwrap();
    std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o755)).unwrap();
    symlink(&external, data.join("linked-tool")).unwrap();
    let paths = PlatformPaths::new(&data, temp_root.join("config"), temp_root.join("cache"));

    assert_eq!(
        inspect_platform_health_at(&paths, true).permissions[0].state,
        PlatformPermissionState::ManualReview
    );
    assert_eq!(
        repair_platform_permissions_at(PlatformStorageArea::ApplicationData, &paths, true).outcome,
        PlatformPermissionRepairOutcome::ManualGuidanceRequired
    );
    assert_eq!(
        std::fs::metadata(external).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn windows_contract_never_claims_permissions_are_healthy() {
    let temp_dir = tempfile::tempdir().unwrap();
    let paths = PlatformPaths::new(
        temp_dir.path().join("data"),
        temp_dir.path().join("config"),
        temp_dir.path().join("cache"),
    );

    let report = inspect_platform_health_at(&paths, false);

    assert!(report.permissions.iter().all(|check| {
        check.state == PlatformPermissionState::Unchecked
            && check.action == Some(PlatformPermissionAction::FollowManualGuidance)
            && !check.connectivity_required
    }));
    assert_eq!(
        repair_platform_permissions_at(PlatformStorageArea::ApplicationData, &paths, false).outcome,
        PlatformPermissionRepairOutcome::ManualGuidanceRequired
    );
}

#[test]
fn package_repair_is_guidance_only_with_explicit_connectivity() {
    let temp_dir = tempfile::tempdir().unwrap();
    let report = inspect_platform_health_at(
        &PlatformPaths::new(
            temp_dir.path().join("data"),
            temp_dir.path().join("config"),
            temp_dir.path().join("cache"),
        ),
        false,
    );

    assert_eq!(report.package_repair.mode, PackageRepairMode::GuidanceOnly);
    assert_eq!(
        report.package_repair.actions,
        [
            PackageRepairAction {
                action: PackageRepairActionId::UseDownloadedVerifiedInstaller,
                connectivity_required: false,
            },
            PackageRepairAction {
                action: PackageRepairActionId::ObtainVerifiedInstaller,
                connectivity_required: true,
            },
        ]
    );
}

#[test]
fn storage_area_deserialization_rejects_unknown_values() {
    assert_eq!(
        serde_json::from_str::<PlatformStorageArea>(r#""application_data""#).unwrap(),
        PlatformStorageArea::ApplicationData
    );
    assert!(serde_json::from_str::<PlatformStorageArea>(r#""other""#).is_err());
}

#[test]
fn serialized_report_contains_only_bounded_fields() {
    let temp_dir = tempfile::tempdir().unwrap();
    let private_context = temp_dir.path().to_string_lossy().into_owned();
    let report = inspect_platform_health_at(
        &PlatformPaths::new(
            temp_dir.path().join("username-data"),
            temp_dir.path().join("raw-os-error-config"),
            temp_dir.path().join("secret-cache"),
        ),
        false,
    );

    let serialized = serde_json::to_string(&report).unwrap();

    assert!(!serialized.contains(&private_context));
    assert!(!serialized.contains("username-data"));
    assert!(!serialized.contains("raw-os-error-config"));
    assert!(!serialized.contains("secret-cache"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serialized).unwrap()["schema_version"],
        PLATFORM_HEALTH_SCHEMA_VERSION
    );
}
