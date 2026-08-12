//! Proves signed pack artifact staging, activation, rollback, disable, and removal.

use super::*;

#[tokio::test]
async fn verified_artifact_is_privately_persisted_for_review_without_activation() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, envelope) = signed_source_pack(1);

    let review = stage_pack_artifact(
        &database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(review.publisher_key_id, PUBLISHER_ID);
    assert_eq!(review.publisher_name, "JobSentinel Test");
    assert_eq!(review.license, "MIT");
    assert_eq!(review.pack_id, PACK_ID);
    assert_eq!(review.release_sequence, 1);
    assert_eq!(review.execution_class, PackExecutionClass::StaticContent);
    assert_eq!(review.minimum_app_version, "3.0.0");
    assert_eq!(review.maximum_app_version, "3.0.0");
    assert_eq!(review.fixture_summary, "Synthetic source fixtures");
    assert!(review.payload_bytes > 0);
    assert_eq!(review.state, PackReviewState::NeedsReview);
    assert_eq!(review.generation, 2);
    assert!(review.allowed_data_categories.is_empty());
    assert!(review.allowed_task_kinds.is_empty());
    assert!(review.allowed_actions.is_empty());
    assert!(review.gateway_policy_id.is_none());
    assert!(review.external_destinations.is_empty());
    assert!(!review.uses_external_ai);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
    assert_eq!(stream.generation, 2);
    let files = walk_files(artifact_root.path());
    assert_eq!(files.len(), 1);
    assert_eq!(std::fs::read(&files[0]).unwrap(), envelope);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let pack_dir = files[0].parent().unwrap();
        let publisher_dir = pack_dir.parent().unwrap();
        assert_eq!(publisher_dir.parent().unwrap(), artifact_root.path());
        for directory in [artifact_root.path(), publisher_dir, pack_dir] {
            assert_eq!(
                std::fs::metadata(directory).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert_eq!(
            std::fs::metadata(&files[0]).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn activation_rereads_and_self_tests_the_owned_artifact_after_explicit_review() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, envelope) = signed_source_pack(1);
    let staged = stage_pack_artifact(
        &database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    let active = activate_pack_artifact(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        1,
        staged.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(active.state, PackReviewState::Ready);
    assert_eq!(active.generation, staged.generation + 1);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Ready);
    assert_eq!(stream.active_release_sequence, Some(1));
}

#[tokio::test]
async fn disabled_pack_requires_fresh_artifact_proof_before_reenable() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;

    let disabled = disable_pack_artifact(&database, PUBLISHER_ID, PACK_ID, active.generation)
        .await
        .unwrap();
    assert_eq!(disabled.state, PackReviewState::Disabled);
    let enabled = enable_pack_artifact(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        disabled.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(enabled.state, PackReviewState::Ready);
    assert_eq!(enabled.generation, disabled.generation + 1);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.active_release_sequence, Some(1));
    assert_eq!(stream.availability, PackAvailability::Ready);
}

#[tokio::test]
async fn rollback_revalidates_the_retained_artifact_before_swapping_releases() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let third_active = stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;

    let rolled_back = rollback_pack_artifact(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        third_active.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(rolled_back.release_sequence, 1);
    assert_eq!(rolled_back.state, PackReviewState::Ready);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.active_release_sequence, Some(1));
    assert_eq!(stream.rollback_release_sequence, Some(3));
    assert_eq!(stream.high_water_sequence, 3);
}

#[tokio::test]
async fn invalid_rollback_artifact_is_unlinked_and_quarantined() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let third_active = stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    let first = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    let first_artifact = walk_files(artifact_root.path())
        .into_iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&first.signed_release_sha256)
        })
        .unwrap();
    std::fs::write(first_artifact, b"altered rollback").unwrap();

    assert!(rollback_pack_artifact(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        third_active.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .is_err());

    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.active_release_sequence, Some(3));
    assert_eq!(stream.rollback_release_sequence, None);
    assert_eq!(stream.high_water_sequence, 3);
    assert_eq!(stream.generation, third_active.generation + 1);
    let first = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(first.lifecycle_state, PackReleaseState::Quarantined);
    assert_eq!(
        first.quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}

#[tokio::test]
async fn uninstall_preserves_the_tombstone_and_deletes_only_owned_artifacts() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    let pack_dir = walk_files(artifact_root.path())[0]
        .parent()
        .unwrap()
        .to_path_buf();
    let unrelated = pack_dir.join("unrelated.txt");
    std::fs::write(&unrelated, b"keep").unwrap();

    let removal = uninstall_pack_artifacts(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        active.generation,
    )
    .await
    .unwrap();

    assert_eq!(removal.generation, active.generation + 1);
    assert!(!removal.cleanup_pending);
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
    assert_eq!(walk_files(artifact_root.path()), vec![unrelated]);
    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Removed);
    assert_eq!(stream.high_water_sequence, 3);
    assert_eq!(stream.active_release_sequence, None);
    assert_eq!(stream.rollback_release_sequence, None);
    assert!(database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap()
        .iter()
        .all(|release| release.lifecycle_state == PackReleaseState::Removed));
}

#[tokio::test]
async fn failed_artifact_cleanup_is_visible_and_retryable_after_removal() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let artifact = walk_files(artifact_root.path()).pop().unwrap();
    std::fs::remove_file(&artifact).unwrap();
    std::fs::create_dir(&artifact).unwrap();

    let pending = uninstall_pack_artifacts(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        active.generation,
    )
    .await
    .unwrap();
    assert!(pending.cleanup_pending);
    std::fs::remove_dir(artifact).unwrap();

    let retried = uninstall_pack_artifacts(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        pending.generation,
    )
    .await
    .unwrap();
    assert_eq!(retried.generation, pending.generation);
    assert!(!retried.cleanup_pending);
}
