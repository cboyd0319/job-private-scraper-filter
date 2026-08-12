use super::*;

#[tokio::test]
async fn finishing_promoted_restore_cleans_raw_artifacts_and_allows_another_backup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let restore_id = restore_id(&db_path);
    database.checkpoint_wal().await.unwrap();
    database.pool().close().await;
    drop(database);
    Database::promote_staged_restore(&db_path).await.unwrap();
    let previous = sibling(&db_path, &format!(".restore-{restore_id}.previous"));
    let rollback = rollback_path(&db_path, &restore_id);
    assert!(previous.exists());
    assert!(rollback.exists());

    let stage = sibling(&db_path, ".restore-stage");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("connection::tests::portable_restore_tests::recovery::finish_crash_writer_helper")
        .env("JOBSENTINEL_RESTORE_FINISH_CRASH_DB", &db_path)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(sqlite_sidecar_path(&db_path, "-wal").exists());
    std::fs::remove_dir(&stage).unwrap();
    let restored = Database::connect_with_staged_restore(&db_path)
        .await
        .unwrap();

    assert!(!sibling(&db_path, ".restore-request.json").exists());
    assert!(!previous.exists());
    assert!(!sqlite_sidecar_path(&previous, "-wal").exists());
    assert!(!sqlite_sidecar_path(&previous, "-shm").exists());
    assert!(rollback.exists());
    let manifest_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name = 'portable_backup_manifest'",
    )
    .fetch_one(restored.pool())
    .await
    .unwrap();
    let outcome: (String, Option<String>) = sqlx::query_as(
        "SELECT outcome, completed_at
         FROM v3_recovery_operations
         WHERE operation_id = ? AND operation_kind = 'restore'",
    )
    .bind(&restore_id)
    .fetch_one(restored.pool())
    .await
    .unwrap();
    assert_eq!(manifest_count, 0);
    assert_eq!(outcome.0, "succeeded");
    assert!(outcome.1.is_some());

    let next_backup = temp_dir.path().join("second-portable.db");
    restored
        .create_portable_backup(&next_backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    Database::inspect_portable_backup(&next_backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
}

#[tokio::test]
async fn finish_crash_writer_helper() {
    let Ok(db_path) = std::env::var("JOBSENTINEL_RESTORE_FINISH_CRASH_DB") else {
        return;
    };
    let db_path = std::path::Path::new(&db_path);
    let restored = Database::connect(db_path).await.unwrap();
    restored.migrate().await.unwrap();
    sqlx::query("PRAGMA wal_autocheckpoint = 0")
        .execute(restored.pool())
        .await
        .unwrap();
    let stage = sibling(db_path, ".restore-stage");
    std::fs::remove_file(&stage).unwrap();
    std::fs::create_dir(&stage).unwrap();
    assert!(restored.finish_staged_restore().await.is_err());
    assert!(sqlite_sidecar_path(db_path, "-wal").exists());
    std::process::exit(0);
}

#[tokio::test]
async fn promoted_restart_accepts_a_compatible_older_manifest_after_migration() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    database.pool().close().await;
    drop(database);
    Database::promote_staged_restore(&db_path).await.unwrap();
    let restored = Database::connect(&db_path).await.unwrap();
    restored.migrate().await.unwrap();
    sqlx::query(
        "UPDATE portable_backup_manifest
         SET migration_sequence = migration_sequence - 1",
    )
    .execute(restored.pool())
    .await
    .unwrap();
    restored.pool().close().await;
    drop(restored);

    let restored = Database::connect_with_staged_restore(&db_path)
        .await
        .unwrap();

    assert!(!sibling(&db_path, ".restore-request.json").exists());
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(value, "restored private history");
}

#[cfg(unix)]
#[tokio::test]
async fn promotion_rejects_a_symlinked_retained_rollback() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let restore_id = restore_id(&db_path);
    database.pool().close().await;
    drop(database);
    let rollback = rollback_path(&db_path, &restore_id);
    std::fs::create_dir_all(rollback.parent().unwrap()).unwrap();
    symlink(&db_path, &rollback).unwrap();
    let before = database_bytes(&db_path);

    let error = Database::promote_staged_restore(&db_path)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("regular file"));
    assert!(database_bytes(&db_path) == before);
}

#[tokio::test]
async fn ready_restore_rebuilds_a_partial_marker_owned_rollback() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let restore_id = restore_id(&db_path);
    database.pool().close().await;
    drop(database);
    let rollback = rollback_path(&db_path, &restore_id);
    std::fs::create_dir_all(rollback.parent().unwrap()).unwrap();
    std::fs::write(&rollback, b"partial").unwrap();

    assert!(Database::promote_staged_restore(&db_path).await.unwrap());

    let key = load_or_create_database_key().await.unwrap();
    let rollback = connect_encrypted_read_only_pool(&rollback, &key)
        .await
        .unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(&rollback)
        .await
        .unwrap();
    assert_eq!(value, "current local history");
}

#[tokio::test]
async fn rollback_restores_original_and_records_failed_restore() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let restore_id = restore_id(&db_path);
    database.checkpoint_wal().await.unwrap();
    database.pool().close().await;
    drop(database);
    Database::promote_staged_restore(&db_path).await.unwrap();
    let restored = Database::connect(&db_path).await.unwrap();
    restored.migrate().await.unwrap();
    restored.pool().close().await;
    drop(restored);
    let rollback_path = rollback_path(&db_path, &restore_id);
    let rollback_before = std::fs::read(&rollback_path).unwrap();

    assert!(Database::rollback_staged_restore(&db_path).await.unwrap());

    assert!(!sibling(&db_path, ".restore-request.json").exists());
    let original = Database::connect(&db_path).await.unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(original.pool())
        .await
        .unwrap();
    let outcome: (String, Option<String>) = sqlx::query_as(
        "SELECT outcome, completed_at
         FROM v3_recovery_operations
         WHERE operation_id = ? AND operation_kind = 'restore'",
    )
    .bind(&restore_id)
    .fetch_one(original.pool())
    .await
    .unwrap();
    assert_eq!(value, "current local history");
    assert_eq!(outcome.0, "failed");
    assert!(outcome.1.is_some());
    sqlx::query("INSERT INTO restore_probe(value) VALUES ('new active write')")
        .execute(original.pool())
        .await
        .unwrap();
    original.checkpoint_wal().await.unwrap();
    assert!(
        std::fs::read(&rollback_path).unwrap() == rollback_before,
        "active writes changed the retained rollback"
    );

    assert!(rollback_path.exists());
    let key = load_or_create_database_key().await.unwrap();
    let rollback = connect_encrypted_read_only_pool(&rollback_path, &key)
        .await
        .unwrap();
    let rollback_value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(&rollback)
        .await
        .unwrap();
    assert_eq!(rollback_value, "current local history");
}

#[tokio::test]
async fn interrupted_promotion_recovers_original_with_or_without_published_primary() {
    for (orphan_sidecar, orphan_publish) in [
        (None, false),
        (Some("-wal"), false),
        (Some("-journal"), false),
        (None, true),
    ] {
        let temp_dir = tempfile::tempdir().unwrap();
        let backup = portable_backup(temp_dir.path()).await;
        let db_path = temp_dir.path().join("primary/jobs.db");
        let database = database_with_probe(&db_path, "current local history", false).await;
        database
            .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
            .await
            .unwrap();
        let restore_id = restore_id(&db_path);
        database.checkpoint_wal().await.unwrap();
        database.pool().close().await;
        drop(database);
        Database::promote_staged_restore(&db_path).await.unwrap();
        rewrite_request(&db_path, "phase", "promoting");
        if orphan_sidecar.is_some() || orphan_publish {
            std::fs::remove_file(&db_path).unwrap();
        }
        if let Some(suffix) = orphan_sidecar {
            std::fs::write(sqlite_sidecar_path(&db_path, suffix), b"stale").unwrap();
        }
        if orphan_publish {
            std::fs::write(
                sibling(&db_path, &format!(".restore-{restore_id}.rollback-publish")),
                b"partial",
            )
            .unwrap();
        }

        assert!(!Database::promote_staged_restore(&db_path).await.unwrap());
        assert!(!sibling(&db_path, ".restore-request.json").exists());
        assert!(rollback_path(&db_path, &restore_id).exists());
        let original = Database::connect(&db_path).await.unwrap();
        let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
            .fetch_one(original.pool())
            .await
            .unwrap();
        assert_eq!(value, "current local history");
    }
}
