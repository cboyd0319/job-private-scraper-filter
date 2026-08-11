//! Proves renderer-safe pack management across review, update, and cleanup states.

use super::*;

#[tokio::test]
async fn management_distinguishes_needs_review_from_ready_update() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, first_envelope) = signed_source_pack(1);
    let staged = stage_pack_artifact(
        &database,
        artifact_root.path(),
        &first_envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    let needs_review = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(needs_review.len(), 1);
    assert_eq!(needs_review[0].state, PackReviewState::NeedsReview);
    assert_eq!(
        needs_review[0].publisher_public_key_sha256,
        hex::encode(Sha256::digest(publisher.public_key))
    );
    assert!(!needs_review[0].update_available);
    assert_eq!(needs_review[0].current_release.release_sequence, 1);
    assert_eq!(
        needs_review[0].current_release.publisher_name,
        "JobSentinel Test"
    );
    assert_eq!(needs_review[0].current_release.minimum_app_version, "3.0.0");
    assert_eq!(needs_review[0].current_release.maximum_app_version, "3.0.0");
    assert!(!needs_review[0].current_release.uses_external_ai);

    activate_pack_artifact(
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
    let (_, update_envelope) = signed_source_pack(3);
    stage_pack_artifact(
        &database,
        artifact_root.path(),
        &update_envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    let update = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(update[0].state, PackReviewState::Ready);
    assert!(update[0].update_available);
    assert_eq!(update[0].current_release.release_sequence, 1);
    assert_eq!(update[0].releases.len(), 2);
    assert!(update[0].releases[0].is_active);
    assert!(update[0].releases[1].is_high_water);

    disable_pack_artifact(&database, PUBLISHER_ID, PACK_ID, update[0].generation)
        .await
        .unwrap();
    let disabled = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(disabled[0].state, PackReviewState::Disabled);
    assert!(disabled[0].update_available);
}

#[tokio::test]
async fn management_retains_quarantined_and_removed_review_without_artifact() {
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
    std::fs::remove_file(walk_files(artifact_root.path()).pop().unwrap()).unwrap();

    assert!(activate_pack_artifact(
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
    .is_err());
    let quarantined = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(quarantined[0].state, PackReviewState::Quarantined);
    assert_eq!(
        quarantined[0].current_release.quarantine_reason,
        Some(crate::pack_runtime::PackReviewQuarantineReason::ArtifactMissing)
    );

    let removed = uninstall_pack_artifacts(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        quarantined[0].generation,
    )
    .await
    .unwrap();
    assert!(!removed.cleanup_pending);
    let management = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(management[0].state, PackReviewState::Removed);
    assert_eq!(
        management[0].current_release.publisher_name,
        "JobSentinel Test"
    );
    let serialized = serde_json::to_value(&management[0]).unwrap();
    assert_eq!(
        serialized["publisherPublicKeySha256"],
        hex::encode(Sha256::digest(publisher.public_key))
    );
    assert!(serialized.get("publicKey").is_none());
    assert!(serialized.get("signedReleaseSha256").is_none());
    assert!(serialized.get("artifactPath").is_none());
    assert!(serialized.get("envelope").is_none());
    assert!(serialized.get("signature").is_none());
}

#[tokio::test]
async fn cleanup_attempts_every_removed_release_and_survives_restage() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 3).await;
    let releases = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    let first = releases
        .iter()
        .find(|release| release.release_sequence == 1)
        .unwrap();
    let third = releases
        .iter()
        .find(|release| release.release_sequence == 3)
        .unwrap();
    let files = walk_files(artifact_root.path());
    let first_path = files
        .iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&first.signed_release_sha256)
        })
        .unwrap()
        .clone();
    let third_path = files
        .iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&third.signed_release_sha256)
        })
        .unwrap()
        .clone();
    std::fs::remove_file(&first_path).unwrap();
    std::fs::create_dir(&first_path).unwrap();

    let removal = uninstall_pack_artifacts(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        active.generation,
    )
    .await
    .unwrap();
    assert!(removal.cleanup_pending);
    assert!(!third_path.exists());
    let partly_cleaned = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(
        partly_cleaned
            .iter()
            .map(|release| release.artifact_cleanup_pending)
            .collect::<Vec<_>>(),
        vec![true, false]
    );

    let (_, envelope) = signed_source_pack(4);
    stage_pack_artifact(
        &database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();
    let new_release = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 4)
        .await
        .unwrap();
    let new_path = walk_files(artifact_root.path())
        .into_iter()
        .find(|path| {
            path.to_string_lossy()
                .contains(&new_release.signed_release_sha256)
        })
        .unwrap();
    std::fs::remove_dir(&first_path).unwrap();

    let retried = crate::pack_runtime::retry_pack_artifact_cleanup(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
    )
    .await
    .unwrap();
    assert!(!retried.cleanup_pending);
    assert!(new_path.exists());
    let after_retry = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert!(after_retry
        .iter()
        .all(|release| !release.artifact_cleanup_pending));
}
