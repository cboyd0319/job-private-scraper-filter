//! Exercises desktop startup, recovery, and pre-service pack reconciliation.

use super::*;
use crate::pack_runtime_tests::{signed_source_pack, stage_and_activate, PACK_ID, PUBLISHER_ID};
use chrono::NaiveDate;
use jobsentinel_storage::v3_pack_lifecycle::PackAvailability;

#[tokio::test]
async fn initializes_first_run_desktop_services() {
    let temp_dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let database = Database::connect_memory().await.unwrap();
    let services =
        DesktopServices::initialize_with_database(temp_dir.path().join("config.json"), database)
            .await
            .unwrap();

    assert!(services.is_first_run);
    assert!(!services.config.read().await.auto_refresh.enabled);
    assert_eq!(services.bookmarklet_server.read().await.config().port, 4321);
    assert_eq!(
        services.database.get_statistics().await.unwrap().total_jobs,
        0
    );
}

#[tokio::test]
async fn reconciles_active_pack_artifacts_before_constructing_services() {
    let temp_dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let pack_runtime = PackRuntimeEnvironment::for_data_dir(temp_dir.path());
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, pack_runtime.artifact_root(), &publisher, 1).await;
    let artifact = std::fs::read_dir(pack_runtime.artifact_root())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::remove_dir_all(artifact).unwrap();

    let services = DesktopServices::initialize_ready_with_pack_runtime(
        temp_dir.path().join("config.json"),
        Config::first_run(),
        true,
        database,
        pack_runtime,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await
    .unwrap();

    let stream = services
        .database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(
        services.pack_runtime.artifact_root(),
        temp_dir.path().join("pack-artifacts")
    );
}

#[tokio::test]
async fn pack_artifact_root_failure_prevents_service_construction() {
    let temp_dir = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let pack_runtime = PackRuntimeEnvironment::for_data_dir(temp_dir.path());
    std::fs::write(pack_runtime.artifact_root(), b"not a directory").unwrap();

    let result = DesktopServices::initialize_ready_with_pack_runtime(
        temp_dir.path().join("config.json"),
        Config::first_run(),
        true,
        database,
        pack_runtime,
        production_trusted_publishers(),
        NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
    )
    .await;

    assert!(matches!(result, Err(DesktopStartupError::Database(_))));
}

#[tokio::test]
async fn rejects_invalid_saved_config_before_constructing_services() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    std::fs::write(&config_path, "{").unwrap();

    let result = DesktopServices::initialize_at(config_path, temp_dir.path().join("jobs.db")).await;

    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("invalid configuration unexpectedly initialized"),
    };
    assert!(matches!(error, DesktopStartupError::Configuration(_)));
    assert_eq!(error.kind(), DesktopStartupFailureKind::Configuration);
}

#[tokio::test]
async fn recovery_services_are_ephemeral_offline_and_keychain_free() {
    let services = DesktopServices::initialize_recovery().await.unwrap();

    assert!(!services.is_first_run);
    assert!(!services.config.read().await.auto_refresh.enabled);
    assert!(services.credentials.unlock_status().await.unwrap().unlocked);
    assert_eq!(
        services.database.get_statistics().await.unwrap().total_jobs,
        0
    );
}
