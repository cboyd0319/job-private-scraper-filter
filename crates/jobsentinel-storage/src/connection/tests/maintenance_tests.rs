use super::*;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::{path::Path, str::FromStr, time::Duration};

async fn migrated_database(path: &Path) -> Database {
    let database = Database::connect(path).await.unwrap();
    database.migrate().await.unwrap();
    database
}

async fn plaintext_database_without_auto_vacuum(path: &Path) -> Database {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .unwrap()
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(25));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    Database {
        pool,
        db_path: Some(path.to_path_buf()),
        _owner_lock: None,
    }
}

async fn create_reclaimable_pages(database: &Database) {
    sqlx::query("CREATE TABLE cleanup_payloads(id INTEGER PRIMARY KEY, content BLOB NOT NULL)")
        .execute(database.pool())
        .await
        .unwrap();
    for _ in 0..160 {
        sqlx::query("INSERT INTO cleanup_payloads(content) VALUES (zeroblob(8192))")
            .execute(database.pool())
            .await
            .unwrap();
    }
    database.checkpoint_wal().await.unwrap();
    sqlx::query("DELETE FROM cleanup_payloads WHERE id > 1")
        .execute(database.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn storage_maintenance_reports_only_local_aggregate_health() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("private-user-data.db");
    let database = migrated_database(&db_path).await;
    create_reclaimable_pages(&database).await;
    let page_size: i64 =
        sqlx::query_scalar("SELECT CAST(page_size AS INTEGER) FROM pragma_page_size")
            .fetch_one(database.pool())
            .await
            .unwrap();
    let free_pages: i64 =
        sqlx::query_scalar("SELECT CAST(freelist_count AS INTEGER) FROM pragma_freelist_count")
            .fetch_one(database.pool())
            .await
            .unwrap();
    let auto_vacuum: i64 =
        sqlx::query_scalar("SELECT CAST(auto_vacuum AS INTEGER) FROM pragma_auto_vacuum")
            .fetch_one(database.pool())
            .await
            .unwrap();

    let report = database.inspect_storage_maintenance().await.unwrap();

    assert_eq!(report.health, StorageHealth::Healthy);
    assert_eq!(
        report.reclaimable_bytes,
        u64::try_from(page_size * free_pages).unwrap()
    );
    assert!(report.wal_bytes.is_some());
    assert_eq!(auto_vacuum, 2);
    assert!(report.incremental_vacuum_supported);
    assert!(!report.connectivity_required);
    let visible = format!("{report:?}");
    assert!(!visible.contains("private-user-data"));
    assert!(!visible.contains(temp_dir.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn storage_cleanup_preserves_user_rows_and_records_success() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = migrated_database(&temp_dir.path().join("jobs.db")).await;
    create_reclaimable_pages(&database).await;
    sqlx::query("CREATE TABLE cleanup_sentinel(value TEXT NOT NULL)")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO cleanup_sentinel VALUES ('keep this user record')")
        .execute(database.pool())
        .await
        .unwrap();
    let before = database.inspect_storage_maintenance().await.unwrap();
    let before_file_bytes = std::fs::metadata(temp_dir.path().join("jobs.db"))
        .unwrap()
        .len();

    let report = database.run_storage_cleanup().await.unwrap();

    let sentinel: String = sqlx::query_scalar("SELECT value FROM cleanup_sentinel")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let quick: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let foreign_key_violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(database.pool())
        .await
        .unwrap();
    let successful_cleanup: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v3_recovery_operations
         WHERE operation_kind = 'cleanup' AND outcome = 'succeeded'
           AND error_kind IS NULL AND completed_at IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();

    assert_eq!(sentinel, "keep this user record");
    assert_eq!(quick, "ok");
    assert!(foreign_key_violations.is_empty());
    assert_eq!(successful_cleanup, 1);
    assert_eq!(report.health, StorageHealth::Healthy);
    assert!(report.reclaimable_bytes < before.reclaimable_bytes);
    assert_eq!(report.wal_bytes, Some(0));
    assert!(
        std::fs::metadata(temp_dir.path().join("jobs.db"))
            .unwrap()
            .len()
            < before_file_bytes
    );
    assert!(!report.connectivity_required);
}

#[tokio::test]
async fn busy_wal_checkpoint_fails_honestly_without_losing_rows() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = migrated_database(&temp_dir.path().join("jobs.db")).await;
    sqlx::query("CREATE TABLE cleanup_sentinel(value TEXT NOT NULL)")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query("INSERT INTO cleanup_sentinel VALUES ('still here')")
        .execute(database.pool())
        .await
        .unwrap();
    let mut reader = database.pool().begin().await.unwrap();
    let _: String = sqlx::query_scalar("SELECT value FROM cleanup_sentinel")
        .fetch_one(&mut *reader)
        .await
        .unwrap();

    let error = database.run_storage_cleanup().await.unwrap_err();
    reader.rollback().await.unwrap();

    assert!(error.to_string().contains("busy"));
    let sentinel: String = sqlx::query_scalar("SELECT value FROM cleanup_sentinel")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let failed_cleanup: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v3_recovery_operations
         WHERE operation_kind = 'cleanup' AND outcome = 'failed'
           AND error_kind = 'protocol' AND completed_at IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(sentinel, "still here");
    assert_eq!(failed_cleanup, 1);
}

#[tokio::test]
async fn relational_damage_requires_restore_before_cleanup_mutates_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = migrated_database(&temp_dir.path().join("jobs.db")).await;
    let mut connection = database.pool().acquire().await.unwrap();
    sqlx::query("CREATE TABLE cleanup_parent(id INTEGER PRIMARY KEY)")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE cleanup_child(parent_id INTEGER REFERENCES cleanup_parent(id))")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query("INSERT INTO cleanup_child VALUES (42)")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *connection)
        .await
        .unwrap();
    drop(connection);

    let report = database.inspect_storage_maintenance().await.unwrap();
    let error = database.run_storage_cleanup().await.unwrap_err();
    let cleanup_operations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v3_recovery_operations WHERE operation_kind = 'cleanup'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();

    assert_eq!(report.health, StorageHealth::RestoreFromBackupRequired);
    assert!(error.to_string().contains("restore from backup"));
    assert_eq!(cleanup_operations, 0);
}

#[tokio::test]
async fn in_memory_storage_inspection_needs_no_filesystem_or_connectivity() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run(database.pool()).await.unwrap();

    let report = database.inspect_storage_maintenance().await.unwrap();

    assert_eq!(report.health, StorageHealth::Healthy);
    assert_eq!(report.wal_bytes, None);
    assert!(!report.incremental_vacuum_supported);
    assert!(!report.connectivity_required);
}

#[tokio::test]
async fn storage_inspection_rejects_non_file_wal_without_exposing_its_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("private-user-data.db");
    let mut database = Database::connect_memory().await.unwrap();
    MIGRATOR.run(database.pool()).await.unwrap();
    database.db_path = Some(db_path.clone());
    std::fs::create_dir(Database::sibling_path(&db_path, "-wal")).unwrap();

    let error = database.inspect_storage_maintenance().await.unwrap_err();

    assert!(error.to_string().contains("not a regular file"));
    assert!(!error.to_string().contains("private-user-data"));
    assert!(!error
        .to_string()
        .contains(temp_dir.path().to_string_lossy().as_ref()));
}

#[tokio::test]
async fn cleanup_does_not_claim_incremental_reclamation_when_sqlite_disables_it() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = plaintext_database_without_auto_vacuum(&temp_dir.path().join("jobs.db")).await;
    create_reclaimable_pages(&database).await;
    let mode: i64 =
        sqlx::query_scalar("SELECT CAST(auto_vacuum AS INTEGER) FROM pragma_auto_vacuum")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(mode, 0);

    let report = database.run_storage_cleanup().await.unwrap();

    assert!(!report.incremental_vacuum_supported);
    assert_eq!(report.health, StorageHealth::Healthy);
}

#[tokio::test]
async fn portable_backup_history_reports_only_the_latest_structural_outcome() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run(database.pool()).await.unwrap();
    assert_eq!(
        database.inspect_portable_backup_history().await.unwrap(),
        PortableBackupHistory::NotRecorded
    );
    sqlx::query(
        "INSERT INTO v3_recovery_operations(
            operation_id, operation_kind, outcome,
            source_migration_sequence, target_migration_sequence, created_at
         ) VALUES ('backup-z', 'backup', 'started', 12, 12, '2026-07-19 01:00:00')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert_eq!(
        database.inspect_portable_backup_history().await.unwrap(),
        PortableBackupHistory::Started
    );
    sqlx::query(
        "UPDATE v3_recovery_operations
         SET outcome = 'failed', error_kind = 'io', completed_at = datetime('now')
         WHERE operation_id = 'backup-z'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert_eq!(
        database.inspect_portable_backup_history().await.unwrap(),
        PortableBackupHistory::Failed
    );
    sqlx::query(
        "INSERT INTO v3_recovery_operations(
            operation_id, operation_kind, outcome,
            source_migration_sequence, target_migration_sequence, created_at, completed_at
         ) VALUES (
             'backup-a', 'backup', 'succeeded', 12, 12,
             '2026-07-19 01:00:00', '2026-07-19 01:00:01'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert_eq!(
        database.inspect_portable_backup_history().await.unwrap(),
        PortableBackupHistory::Succeeded
    );
    sqlx::query(
        "INSERT INTO v3_recovery_operations(
            operation_id, operation_kind, outcome,
            source_migration_sequence, target_migration_sequence,
            error_kind, created_at, completed_at
         ) VALUES (
             'backup-old', 'backup', 'cancelled', 12, 12, 'unknown',
             '2026-07-18 01:00:00', '2026-07-18 01:00:01'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_recovery_operations(
            operation_id, operation_kind, outcome,
            source_migration_sequence, target_migration_sequence,
            created_at, completed_at
         ) VALUES (
             'cleanup-new', 'cleanup', 'succeeded', 12, 12,
             '2026-07-20 01:00:00', '2026-07-20 01:00:01'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert_eq!(
        database.inspect_portable_backup_history().await.unwrap(),
        PortableBackupHistory::Succeeded
    );
}
