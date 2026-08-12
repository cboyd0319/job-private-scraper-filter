//! Verifies staged portable restore, promotion, rollback, and unsafe-path refusal.

use super::*;
use crate::encryption::{
    connect_encrypted_read_only_pool, load_or_create_database_key, sqlite_sidecar_path,
};
use sqlx::{sqlite::SqlitePool, Executor};
use std::path::{Path, PathBuf};

pub(super) const BACKUP_PASSPHRASE: &str = "correct ' restore battery staple";

pub(super) async fn database_with_probe(
    path: &Path,
    value: &str,
    include_secrets: bool,
) -> Database {
    let database = Database::connect(path).await.unwrap();
    database.migrate().await.unwrap();
    database
        .pool()
        .execute("CREATE TABLE restore_probe(value TEXT NOT NULL)")
        .await
        .unwrap();
    sqlx::query("INSERT INTO restore_probe(value) VALUES (?)")
        .bind(value)
        .execute(database.pool())
        .await
        .unwrap();
    if include_secrets {
        insert_private_credential_rows(&database, "restore-secret").await;
    }
    database
}

pub(super) async fn portable_backup(root: &Path) -> PathBuf {
    let source_dir = root.join("source");
    let backup_dir = root.join("input");
    std::fs::create_dir_all(&backup_dir).unwrap();
    let source = database_with_probe(
        &source_dir.join("jobs.db"),
        "restored private history",
        true,
    )
    .await;
    let backup = backup_dir.join("portable.db");
    source
        .create_portable_backup(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    source.pool().close().await;
    backup
}

pub(super) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn restore_id(database_path: &Path) -> String {
    let request: serde_json::Value = serde_json::from_slice(
        &std::fs::read(sibling(database_path, ".restore-request.json")).unwrap(),
    )
    .unwrap();
    request["restore_id"].as_str().unwrap().to_string()
}

fn rewrite_request(database_path: &Path, field: &str, value: &str) {
    let path = sibling(database_path, ".restore-request.json");
    let mut request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    request[field] = value.into();
    std::fs::write(path, serde_json::to_vec(&request).unwrap()).unwrap();
}

fn rollback_path(database_path: &Path, restore_id: &str) -> PathBuf {
    database_path
        .parent()
        .unwrap()
        .join("backups")
        .join(format!("restore_rollback_{restore_id}.db"))
}

pub(super) fn database_bytes(path: &Path) -> Vec<Option<Vec<u8>>> {
    std::iter::once(path.to_path_buf())
        .chain(
            ["-wal", "-shm", "-journal"]
                .into_iter()
                .map(|suffix| sqlite_sidecar_path(path, suffix)),
        )
        .map(|path| std::fs::read(path).ok())
        .collect()
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(path).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[tokio::test]
async fn wrong_passphrase_does_not_touch_primary_or_create_restore_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database.checkpoint_wal().await.unwrap();
    let before = database_bytes(&db_path);

    let error = database
        .stage_portable_restore(&backup, "wrong restore passphrase")
        .await
        .unwrap_err();

    assert!(error.to_string().contains("backup"));
    assert_eq!(database_bytes(&db_path), before);
    assert!(!sibling(&db_path, ".restore-stage").exists());
    assert!(!sibling(&db_path, ".restore-request.json").exists());
}

#[tokio::test]
async fn valid_stage_is_device_encrypted_and_contains_no_secrets_or_plaintext_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;

    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    let stage = sibling(&db_path, ".restore-stage");
    let unkeyed = SqlitePool::connect(&format!("sqlite://{}?mode=ro", stage.display()))
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, String>("SELECT value FROM restore_probe")
            .fetch_one(&unkeyed)
            .await
            .is_err()
    );
    unkeyed.close().await;

    let key = load_or_create_database_key().await.unwrap();
    let staged = connect_encrypted_read_only_pool(&stage, &key)
        .await
        .unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(&staged)
        .await
        .unwrap();
    let secret_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secret_vault")
        .fetch_one(&staged)
        .await
        .unwrap();
    let wrapped_key_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credential_key_wrapping")
        .fetch_one(&staged)
        .await
        .unwrap();
    staged.close().await;
    assert_eq!(value, "restored private history");
    assert_eq!((secret_rows, wrapped_key_rows), (0, 0));

    for file in files_under(db_path.parent().unwrap()) {
        let bytes = std::fs::read(file).unwrap();
        assert!(!bytes.starts_with(b"SQLite format 3\0"));
        assert!(!bytes
            .windows(b"restored private history".len())
            .any(|window| window == b"restored private history"));
    }
}

#[tokio::test]
async fn failed_primary_can_stage_and_cancel_a_portable_restore_offline() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    std::fs::write(&db_path, b"invalid primary retained verbatim").unwrap();
    let before = std::fs::read(&db_path).unwrap();

    Database::stage_portable_restore_at(&db_path, &backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    assert_eq!(
        Database::staged_restore_status(&db_path).unwrap(),
        PortableRestoreStatus::Ready
    );
    assert_eq!(std::fs::read(&db_path).unwrap(), before);
    assert!(Database::cancel_staged_restore_at(&db_path).unwrap());
    assert_eq!(
        Database::staged_restore_status(&db_path).unwrap(),
        PortableRestoreStatus::None
    );
    assert_eq!(std::fs::read(&db_path).unwrap(), before);
}

#[tokio::test]
async fn corrupt_primary_is_quarantined_before_staged_restore_promotion() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let corrupt = b"corrupt private primary retained verbatim";
    std::fs::write(&db_path, corrupt).unwrap();
    Database::stage_portable_restore_at(&db_path, &backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();

    let restored = Database::connect_with_staged_restore(&db_path)
        .await
        .unwrap();

    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(value, "restored private history");
    let quarantine = std::fs::read_dir(db_path.parent().unwrap().join("backups"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("restore_quarantine_"))
        })
        .unwrap();
    assert_eq!(std::fs::read(quarantine).unwrap(), corrupt);
}

#[tokio::test]
async fn interrupted_corrupt_primary_promotion_restores_the_quarantine() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let corrupt = b"interrupted corrupt primary";
    std::fs::write(&db_path, corrupt).unwrap();
    Database::stage_portable_restore_at(&db_path, &backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let restore_id = restore_id(&db_path);
    let quarantine = db_path
        .parent()
        .unwrap()
        .join("backups")
        .join(format!("restore_quarantine_{restore_id}.db"));
    std::fs::create_dir_all(quarantine.parent().unwrap()).unwrap();
    std::fs::hard_link(&db_path, &quarantine).unwrap();
    rewrite_request(&db_path, "phase", "promoting");

    assert!(!Database::promote_staged_restore(&db_path).await.unwrap());

    assert_eq!(std::fs::read(&db_path).unwrap(), corrupt);
    assert_eq!(std::fs::read(&quarantine).unwrap(), corrupt);
    std::fs::write(&db_path, b"primary changed after rollback").unwrap();
    assert_eq!(std::fs::read(&quarantine).unwrap(), corrupt);
    assert!(!sibling(&db_path, ".restore-request.json").exists());
    assert!(!sibling(&db_path, ".restore-stage").exists());
}

#[tokio::test]
async fn static_restore_mutations_refuse_a_live_primary_owner() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;

    let stage_error = Database::stage_portable_restore_at(&db_path, &backup, BACKUP_PASSPHRASE)
        .await
        .unwrap_err();
    assert!(stage_error.to_string().contains("in use"));

    let stage = sibling(&db_path, ".restore-stage");
    std::fs::write(&stage, b"incomplete stage").unwrap();
    let cancel_error = Database::cancel_staged_restore_at(&db_path).unwrap_err();
    assert!(cancel_error.to_string().contains("in use"));
    assert!(stage.exists());
    drop(database);
}

#[cfg(unix)]
#[test]
fn incomplete_dangling_restore_link_is_visible_and_removable() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("jobs.db");
    let stage = sibling(&db_path, ".restore-stage");
    symlink(temp_dir.path().join("missing-target"), &stage).unwrap();

    assert_eq!(
        Database::staged_restore_status(&db_path).unwrap(),
        PortableRestoreStatus::Incomplete
    );
    assert!(Database::cancel_staged_restore_at(&db_path).unwrap());
    assert!(std::fs::symlink_metadata(stage).is_err());
}

#[tokio::test]
async fn startup_ignores_stage_without_marker_and_preserves_primary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let stage = sibling(&db_path, ".restore-stage");
    std::fs::remove_file(sibling(&db_path, ".restore-request.json")).unwrap();
    database.checkpoint_wal().await.unwrap();
    database.pool().close().await;
    drop(database);
    let before = database_bytes(&db_path);

    assert!(!Database::promote_staged_restore(&db_path).await.unwrap());

    assert_eq!(database_bytes(&db_path), before);
    assert!(stage.exists());
}

#[tokio::test]
async fn cancel_revokes_marker_before_attempting_stage_cleanup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let stage = sibling(&db_path, ".restore-stage");
    let marker = sibling(&db_path, ".restore-request.json");
    std::fs::remove_file(&stage).unwrap();
    std::fs::create_dir(&stage).unwrap();

    assert!(database.cancel_staged_restore().await.is_err());

    assert!(!marker.exists(), "cancel must revoke authorization first");
    assert!(stage.is_dir(), "the injected cleanup failure must remain");
}

#[tokio::test]
async fn marked_stage_promotes_and_retains_device_encrypted_rollback() {
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
    drop(database);

    assert!(Database::promote_staged_restore(&db_path).await.unwrap());

    let restored = Database::connect(&db_path).await.unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(value, "restored private history");

    let rollback_files: Vec<_> = std::fs::read_dir(db_path.parent().unwrap().join("backups"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("restore_rollback_"))
        })
        .collect();
    assert_eq!(rollback_files.len(), 1);
    let key = load_or_create_database_key().await.unwrap();
    let rollback = connect_encrypted_read_only_pool(&rollback_files[0], &key)
        .await
        .unwrap();
    let old_value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(&rollback)
        .await
        .unwrap();
    assert_eq!(old_value, "current local history");
}

#[tokio::test]
async fn promotion_refuses_while_the_primary_database_owner_is_live() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    database.checkpoint_wal().await.unwrap();
    let before = database_bytes(&db_path);

    let error = Database::promote_staged_restore(&db_path)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("in use"));
    assert_eq!(database_bytes(&db_path), before);
    assert!(sibling(&db_path, ".restore-stage").exists());
    assert!(sibling(&db_path, ".restore-request.json").exists());
}

#[path = "portable_restore_recovery_tests.rs"]
mod recovery;

#[tokio::test]
async fn promotion_rejects_noncanonical_restore_id_without_changing_primary() {
    let temp_dir = tempfile::tempdir().unwrap();
    let backup = portable_backup(temp_dir.path()).await;
    let db_path = temp_dir.path().join("primary/jobs.db");
    let database = database_with_probe(&db_path, "current local history", false).await;
    database
        .stage_portable_restore(&backup, BACKUP_PASSPHRASE)
        .await
        .unwrap();
    let noncanonical = format!("{{{}}}", restore_id(&db_path));
    rewrite_request(&db_path, "restore_id", &noncanonical);
    database.checkpoint_wal().await.unwrap();
    database.pool().close().await;
    drop(database);
    let before = database_bytes(&db_path);

    let error = Database::promote_staged_restore(&db_path)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("request is invalid"));
    assert_eq!(database_bytes(&db_path), before);
}
#[tokio::test]
async fn startup_connection_promotes_and_finishes_the_staged_restore() {
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
    let restored = Database::connect_with_staged_restore(&db_path)
        .await
        .unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM restore_probe")
        .fetch_one(restored.pool())
        .await
        .unwrap();
    assert_eq!(value, "restored private history");
    assert!(!sibling(&db_path, ".restore-request.json").exists());
}
