//! Proves startup artifact reconciliation, rollback, and fail-closed quarantine behavior.

use super::*;

#[tokio::test]
async fn startup_leaves_a_verified_active_artifact_unchanged() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;

    for _ in 0..2 {
        let reconciled = reconcile_active_pack_artifacts(
            &database,
            artifact_root.path(),
            std::slice::from_ref(&publisher),
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(reconciled.checked, 1);
        assert_eq!(reconciled.rolled_back, 0);
        assert_eq!(reconciled.quarantined, 0);
    }
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Ready);
    assert_eq!(stream.generation, active.generation);
}

#[tokio::test]
async fn startup_treats_an_omitted_publisher_as_revoked_trust() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;

    let reconciled = reconcile_active_pack_artifacts(
        &database,
        artifact_root.path(),
        &[],
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(reconciled.checked, 1);
    assert_eq!(reconciled.rolled_back, 0);
    assert_eq!(reconciled.quarantined, 1);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    let release = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(
        release.quarantine_reason,
        Some(PackQuarantineReason::TrustRevoked)
    );
}

#[tokio::test]
async fn startup_replaces_an_invalid_active_artifact_with_a_verified_rollback() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    let active = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
        .await
        .unwrap();
    let active_artifact = walk_files(artifact_root.path())
        .into_iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&active.signed_release_sha256)
        })
        .unwrap();
    std::fs::write(active_artifact, b"altered active").unwrap();

    let reconciled = reconcile_active_pack_artifacts(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(reconciled.checked, 1);
    assert_eq!(reconciled.rolled_back, 1);
    assert_eq!(reconciled.quarantined, 0);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.active_release_sequence, Some(1));
    assert_eq!(stream.rollback_release_sequence, None);
    assert_eq!(stream.high_water_sequence, 3);
    assert_eq!(
        database
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
            .await
            .unwrap()
            .quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}

#[tokio::test]
async fn startup_records_a_missing_active_artifact_before_verified_rollback() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    let active = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
        .await
        .unwrap();
    let active_artifact = walk_files(artifact_root.path())
        .into_iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&active.signed_release_sha256)
        })
        .unwrap();
    std::fs::remove_file(active_artifact).unwrap();

    reconcile_active_pack_artifacts(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        database
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
            .await
            .unwrap()
            .quarantine_reason,
        Some(PackQuarantineReason::ArtifactMissing)
    );
}

#[tokio::test]
async fn startup_quarantines_invalid_active_and_rollback_artifacts_together() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    for path in walk_files(artifact_root.path()) {
        std::fs::write(path, b"altered").unwrap();
    }

    let reconciled = reconcile_active_pack_artifacts(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(reconciled.checked, 1);
    assert_eq!(reconciled.rolled_back, 0);
    assert_eq!(reconciled.quarantined, 1);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
    assert_eq!(stream.rollback_release_sequence, None);
    for sequence in [1, 3] {
        let stored = database
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, sequence)
            .await
            .unwrap();
        assert_eq!(stored.lifecycle_state, PackReleaseState::Quarantined);
        assert_eq!(
            stored.quarantine_reason,
            Some(PackQuarantineReason::IntegrityFailed)
        );
    }
}
