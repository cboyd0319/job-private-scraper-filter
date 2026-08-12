use super::{
    portable_restore_tests::{
        database_bytes, database_with_probe, portable_backup, sibling, BACKUP_PASSPHRASE,
    },
    Database, PortableRestoreStatus,
};

#[cfg(unix)]
#[tokio::test]
async fn promotion_refuses_a_symlinked_backup_directory_without_external_writes() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    database.checkpoint_wal().await.unwrap();
    database.pool().close().await;
    let before = database_bytes(&db_path);
    drop(database);
    let external = temp_dir.path().join("external");
    std::fs::create_dir(&external).unwrap();
    symlink(&external, db_path.parent().unwrap().join("backups")).unwrap();

    let error = Database::promote_staged_restore(&db_path)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("directory"));
    assert_eq!(database_bytes(&db_path), before);
    assert!(std::fs::read_dir(external).unwrap().next().is_none());
}

#[test]
fn malformed_restore_marker_can_be_preserved_and_cleared_offline() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let marker = sibling(&db_path, ".restore-request.json");
    let stage = sibling(&db_path, ".restore-stage");
    std::fs::write(&db_path, b"primary remains unchanged").unwrap();
    std::fs::write(&marker, b"{invalid restore request").unwrap();
    std::fs::write(&stage, b"private staged bytes").unwrap();
    let before = std::fs::read(&db_path).unwrap();

    assert_eq!(
        Database::staged_restore_status(&db_path).unwrap(),
        PortableRestoreStatus::Invalid
    );
    assert!(Database::cancel_staged_restore_at(&db_path).unwrap());

    assert_eq!(std::fs::read(&db_path).unwrap(), before);
    assert!(!marker.exists());
    assert!(!stage.exists());
    let preserved: Vec<_> = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().contains(".restore-invalid-"))
        })
        .collect();
    assert_eq!(preserved.len(), 2);
    assert!(preserved
        .iter()
        .any(|path| std::fs::read(path).unwrap() == b"{invalid restore request"));
    assert!(preserved
        .iter()
        .any(|path| std::fs::read(path).unwrap() == b"private staged bytes"));
}

#[tokio::test]
async fn promotion_and_rollback_lock_before_reading_the_restore_marker() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    std::fs::write(
        sibling(&db_path, ".restore-request.json"),
        b"{stale restore request",
    )
    .unwrap();

    let promotion = Database::promote_staged_restore(&db_path)
        .await
        .unwrap_err();
    let rollback = Database::rollback_staged_restore(&db_path)
        .await
        .unwrap_err();

    assert!(promotion.to_string().contains("in use"));
    assert!(rollback.to_string().contains("in use"));
    drop(database);
}

#[cfg(unix)]
#[test]
fn linked_restore_marker_is_never_followed_or_removed() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let marker = sibling(&db_path, ".restore-request.json");
    let external = temp_dir.path().join("external-marker");
    std::fs::write(&external, b"external private bytes").unwrap();
    symlink(&external, &marker).unwrap();

    assert!(Database::staged_restore_status(&db_path).is_err());
    assert!(Database::cancel_staged_restore_at(&db_path).is_err());
    assert!(std::fs::symlink_metadata(marker)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(std::fs::read(external).unwrap(), b"external private bytes");
}

#[cfg(unix)]
#[tokio::test]
async fn dangling_owner_lock_link_cannot_create_an_external_file() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let external = temp_dir.path().join("external-lock");
    let lock = sibling(&db_path, ".owner.lock");
    symlink(&external, &lock).unwrap();

    assert!(Database::stage_portable_restore_at(
        &db_path,
        &temp_dir.path().join("missing-backup"),
        BACKUP_PASSPHRASE,
    )
    .await
    .is_err());
    assert!(!external.exists());
    assert!(std::fs::symlink_metadata(lock)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn owner_lock_link_cannot_change_external_file_permissions() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let external = temp_dir.path().join("external-lock");
    std::fs::write(&external, b"external").unwrap();
    std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o644)).unwrap();
    let lock = sibling(&db_path, ".owner.lock");
    symlink(&external, &lock).unwrap();

    assert!(Database::cancel_staged_restore_at(&db_path).is_err());
    assert_eq!(
        std::fs::metadata(external).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[cfg(unix)]
#[test]
fn hard_linked_owner_lock_cannot_change_external_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let external = temp_dir.path().join("external-lock");
    std::fs::write(&external, b"external").unwrap();
    std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::hard_link(&external, sibling(&db_path, ".owner.lock")).unwrap();

    assert!(Database::cancel_staged_restore_at(&db_path).is_err());
    assert_eq!(
        std::fs::metadata(external).unwrap().permissions().mode() & 0o777,
        0o644
    );
}
