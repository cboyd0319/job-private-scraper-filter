use super::*;

#[tokio::test]
async fn altered_owned_artifact_is_quarantined_instead_of_activated() {
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
    let artifact = walk_files(artifact_root.path()).pop().unwrap();
    std::fs::write(artifact, b"altered").unwrap();

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

    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
    assert_eq!(stream.generation, staged.generation + 1);
    let release = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(release.lifecycle_state, PackReleaseState::Quarantined);
    assert_eq!(
        release.quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}

#[tokio::test]
async fn altered_disabled_artifact_cannot_be_reenabled() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let disabled = disable_pack_artifact(&database, PUBLISHER_ID, PACK_ID, active.generation)
        .await
        .unwrap();
    std::fs::write(walk_files(artifact_root.path()).pop().unwrap(), b"altered").unwrap();

    assert!(enable_pack_artifact(
        &database,
        artifact_root.path(),
        PUBLISHER_ID,
        PACK_ID,
        disabled.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .is_err());

    let stream = database
        .get_pack_stream(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
    assert_eq!(stream.generation, disabled.generation + 1);
}

#[cfg(unix)]
#[tokio::test]
async fn symlinked_artifact_root_is_rejected_without_writing_outside_app_storage() {
    use std::os::unix::fs::symlink;

    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let owner = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let artifact_root = owner.path().join("pack-artifacts");
    symlink(outside.path(), &artifact_root).unwrap();
    let (publisher, envelope) = signed_source_pack(1);

    assert!(stage_pack_artifact(
        &database,
        &artifact_root,
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .is_err());

    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    let release = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(release.lifecycle_state, PackReleaseState::Quarantined);
    assert_eq!(
        release.quarantine_reason,
        Some(PackQuarantineReason::ArtifactMissing)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn symlinked_artifact_parent_cannot_redirect_uninstall_cleanup() {
    use std::os::unix::fs::symlink;

    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    let active = stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let artifact = walk_files(artifact_root.path()).pop().unwrap();
    let file_name = artifact.file_name().unwrap().to_owned();
    let pack_dir = artifact.parent().unwrap().to_path_buf();
    std::fs::remove_file(artifact).unwrap();
    std::fs::remove_dir(&pack_dir).unwrap();
    let outside_artifact = outside.path().join(file_name);
    std::fs::write(&outside_artifact, b"outside").unwrap();
    symlink(outside.path(), &pack_dir).unwrap();

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
    assert_eq!(std::fs::read(outside_artifact).unwrap(), b"outside");
}

#[cfg(unix)]
#[tokio::test]
async fn symlinked_active_artifact_parent_is_an_integrity_failure() {
    use std::os::unix::fs::symlink;

    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let artifact_root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (publisher, _) = signed_source_pack(1);
    stage_and_activate(&database, artifact_root.path(), &publisher, 1).await;
    let artifact = walk_files(artifact_root.path()).pop().unwrap();
    let pack_dir = artifact.parent().unwrap().to_path_buf();
    std::fs::remove_file(artifact).unwrap();
    std::fs::remove_dir(&pack_dir).unwrap();
    symlink(outside.path(), pack_dir).unwrap();

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
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
            .await
            .unwrap()
            .quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}
