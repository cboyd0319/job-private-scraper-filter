//! Verifies encrypted portable backups, metadata, safety checks, and recovery boundaries.

use super::*;
use crate::encryption::connect_encrypted_read_only_pool;
use sqlx::{sqlite::SqlitePool, Executor};

const BACKUP_PASSPHRASE: &str = "correct ' 🔒 horse battery staple";

async fn database_with_private_rows(path: &std::path::Path) -> Database {
    let database = Database::connect(path).await.unwrap();
    database.migrate().await.unwrap();
    database
        .pool()
        .execute("CREATE TABLE backup_probe(value TEXT NOT NULL)")
        .await
        .unwrap();
    database
        .pool()
        .execute("INSERT INTO backup_probe(value) VALUES ('private local history')")
        .await
        .unwrap();
    insert_private_credential_rows(&database, "jobsentinel_usajobs_api_key").await;
    database
}

#[tokio::test]
async fn portable_backup_is_encrypted_verified_and_excludes_secrets() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("JobSentinel portable backup.db");

    let created = database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    assert_eq!(created.format_version, 1);
    assert_eq!(created.database_schema, 2);
    assert_eq!(created.migration_sequence, 26);
    assert!(!created.backup_id.is_empty());
    assert_eq!(
        Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
            .await
            .unwrap(),
        created
    );

    let unkeyed = SqlitePool::connect(&format!("sqlite://{}?mode=ro", backup_path.display()))
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, String>("SELECT value FROM backup_probe")
            .fetch_one(&unkeyed)
            .await
            .is_err(),
        "portable backup must not be readable without its passphrase"
    );
    unkeyed.close().await;

    let backup = connect_encrypted_read_only_pool(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let probe: String = sqlx::query_scalar("SELECT value FROM backup_probe")
        .fetch_one(&backup)
        .await
        .unwrap();
    let secret_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_vault")
        .fetch_one(&backup)
        .await
        .unwrap();
    let wrapped_key_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credential_key_wrapping")
        .fetch_one(&backup)
        .await
        .unwrap();
    let actual_schema: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&backup)
        .await
        .unwrap();
    let envelope: (String, String, i64, i64) = sqlx::query_as(
        "SELECT kind, protection, user_review_required, contains_secrets
         FROM portable_backup_manifest",
    )
    .fetch_one(&backup)
    .await
    .unwrap();
    assert_eq!(probe, "private local history");
    assert_eq!(secret_rows, 0);
    assert_eq!(wrapped_key_rows, 0);
    assert_eq!(actual_schema, 2);
    assert_eq!(
        envelope,
        ("backup".to_string(), "encrypted".to_string(), 1, 0)
    );
    backup.close().await;

    let source_secret_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_vault")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let source_wrapped_key_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM credential_key_wrapping")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(source_secret_rows, 1);
    assert_eq!(source_wrapped_key_rows, 1);

    let provenance: (String, String) = sqlx::query_as(
        "SELECT operation_kind, outcome
         FROM v3_recovery_operations
         WHERE operation_id = ?",
    )
    .bind(created.backup_id)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(provenance, ("backup".to_string(), "succeeded".to_string()));
}

#[tokio::test]
async fn portable_backup_rejects_wrong_or_weak_passphrases_without_publishing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("portable.db");

    let error = database
        .create_portable_backup(&backup_path, "too short")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("passphrase"));
    assert!(!backup_path.exists());

    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let before = std::fs::read(&backup_path).unwrap();
    Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let error = Database::inspect_portable_backup(&backup_path, "wrong passphrase value")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("backup"));
    assert!(!error.to_string().contains("wrong passphrase value"));
    assert_eq!(std::fs::read(&backup_path).unwrap(), before);
    assert!(!backup_path.with_extension("db-wal").exists());
    assert!(!backup_path.with_extension("db-shm").exists());
}

#[tokio::test]
async fn portable_backup_never_overwrites_an_existing_destination() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("existing.db");
    std::fs::write(&backup_path, b"keep me").unwrap();

    let error = database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap_err();

    assert_eq!(std::fs::read(&backup_path).unwrap(), b"keep me");
    assert!(error.to_string().contains("already exists"));
}

#[tokio::test]
async fn portable_backup_preserves_unowned_destination_sidecars() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("portable.db");
    let sidecar_path = backup_path.with_file_name("portable.db-wal");
    std::fs::write(&sidecar_path, b"unowned state").unwrap();

    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap_err();

    assert!(!backup_path.exists());
    assert_eq!(std::fs::read(sidecar_path).unwrap(), b"unowned state");
}

#[tokio::test]
async fn cancelling_backup_does_not_publish_and_is_reconciled_on_startup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        std::sync::Arc::new(database_with_private_rows(&temp_dir.path().join("jobs.db")).await);
    sqlx::query("INSERT INTO backup_probe(value) VALUES (hex(zeroblob(16777216)))")
        .execute(database.pool())
        .await
        .unwrap();
    let backup_path = temp_dir.path().join("cancelled.db");
    let task_database = std::sync::Arc::clone(&database);
    let task_path = backup_path.clone();
    let task = tokio::spawn(async move {
        task_database
            .create_portable_backup(&task_path, BACKUP_PASSPHRASE)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let started: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM v3_recovery_operations WHERE outcome = 'started'",
            )
            .fetch_one(database.pool())
            .await
            .unwrap();
            if started == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut lock = database.pool().acquire().await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *lock)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !std::fs::read_dir(temp_dir.path())
            .unwrap()
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".jobsentinel_portable_backup_")
            })
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!task.is_finished());
    assert!(!backup_path.exists());

    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    sqlx::query("ROLLBACK").execute(&mut *lock).await.unwrap();
    assert!(!backup_path.exists());
    database.migrate().await.unwrap();
    let outcomes: (i64, i64) = sqlx::query_as(
        "SELECT
            SUM(CASE WHEN outcome = 'started' THEN 1 ELSE 0 END),
            SUM(CASE WHEN outcome = 'cancelled' THEN 1 ELSE 0 END)
         FROM v3_recovery_operations",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(outcomes, (0, 1));
}

#[tokio::test]
async fn portable_backup_refuses_newer_metadata_before_restore() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("future.db");
    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    let backup = connect_encrypted_pool(&backup_path, BACKUP_PASSPHRASE, false)
        .await
        .unwrap();
    sqlx::query("PRAGMA user_version = 3")
        .execute(&backup)
        .await
        .unwrap();
    backup.close().await;

    let error = Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("backup"));
    assert!(!error
        .to_string()
        .contains(&backup_path.to_string_lossy().to_string()));
}

#[tokio::test]
async fn portable_backup_refuses_external_sidecars_and_corruption() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("portable.db");
    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let original = std::fs::read(&backup_path).unwrap();

    let wal_path = backup_path.with_extension("db-wal");
    std::fs::write(&wal_path, b"external state").unwrap();
    assert!(
        Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&backup_path).unwrap(), original);
    std::fs::remove_file(wal_path).unwrap();

    let mut corrupted = original;
    let midpoint = corrupted.len() / 2;
    corrupted[midpoint] ^= 0xFF;
    std::fs::write(&backup_path, corrupted).unwrap();
    assert!(
        Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn portable_backup_refuses_a_newer_migration_ledger() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("future-ledger.db");
    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    let backup = connect_encrypted_pool(&backup_path, BACKUP_PASSPHRASE, false)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations(
            version, description, success, checksum, execution_time
         )
         SELECT COALESCE(MAX(version), 0) + 1, 'future migration', 1, zeroblob(48), 0
         FROM _sqlx_migrations",
    )
    .execute(&backup)
    .await
    .unwrap();
    backup.close().await;

    assert!(
        Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn portable_backup_refuses_an_incomplete_migration_ledger() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = database_with_private_rows(&temp_dir.path().join("jobs.db")).await;
    let backup_path = temp_dir.path().join("missing-migration.db");
    database
        .create_portable_backup(&backup_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    let backup = connect_encrypted_pool(&backup_path, BACKUP_PASSPHRASE, false)
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 1")
        .execute(&backup)
        .await
        .unwrap();
    backup.close().await;

    assert!(
        Database::inspect_portable_backup(&backup_path, BACKUP_PASSPHRASE)
            .await
            .is_err()
    );

    let tampered_path = temp_dir.path().join("tampered-migration.db");
    database
        .create_portable_backup(&tampered_path, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let backup = connect_encrypted_pool(&tampered_path, BACKUP_PASSPHRASE, false)
        .await
        .unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = zeroblob(48) WHERE version = 1")
        .execute(&backup)
        .await
        .unwrap();
    backup.close().await;
    assert!(
        Database::inspect_portable_backup(&tampered_path, BACKUP_PASSPHRASE)
            .await
            .is_err()
    );
}
