// Creates encrypted portable backups and records their durable recovery outcomes.

use super::{finish_recovery_operation_record, Database, DATABASE_SCHEMA_VERSION, MIGRATOR};
use crate::encryption::{
    connect_encrypted_pool, connect_encrypted_read_only_pool, remove_sqlite_sidecars,
    sqlite_sidecar_path,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::Zeroizing;
const PORTABLE_BACKUP_FORMAT_VERSION: i64 = 1;
const PORTABLE_BACKUP_COMPATIBILITY_LINE: i64 = 3;
const MIN_BACKUP_PASSPHRASE_CHARS: usize = 16;
const MAX_BACKUP_PASSPHRASE_BYTES: usize = 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableBackupInfo {
    pub backup_id: String,
    pub created_at: String,
    pub format_version: i64,
    pub database_schema: i64,
    pub migration_sequence: i64,
}
impl Database {
    pub async fn create_portable_backup(
        &self,
        destination: &Path,
        passphrase: &str,
    ) -> Result<PortableBackupInfo, sqlx::Error> {
        validate_backup_passphrase(passphrase)?;
        let db_path = self
            .db_path
            .as_deref()
            .ok_or_else(|| protocol_error("Portable backup requires an on-disk database"))?;
        if destination.exists() {
            return Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Portable backup destination already exists",
            )));
        }
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| protocol_error("Portable backup destination requires a parent"))?;
        if !parent.is_dir() {
            return Err(sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Portable backup destination directory does not exist",
            )));
        }
        let migration_sequence: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await?;
        let database_schema: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&self.pool)
            .await?;
        let info = PortableBackupInfo {
            backup_id: Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            format_version: PORTABLE_BACKUP_FORMAT_VERSION,
            database_schema,
            migration_sequence,
        };
        self.start_backup_operation(&info).await?;
        let result = self
            .create_portable_backup_inner(db_path, destination, passphrase, &info)
            .await;
        match result {
            Ok(prepared) => {
                if let Err(error) = self.finish_backup_operation(&info.backup_id, None).await {
                    self.fail_backup_operation(&info.backup_id, &error).await;
                    return Err(error);
                }
                if let Err(error) = prepared.publish(destination) {
                    self.fail_backup_operation(&info.backup_id, &error).await;
                    return Err(error);
                }
                match Self::inspect_portable_backup(destination, passphrase).await {
                    Ok(inspected) if inspected == info => Ok(info),
                    Ok(_) => {
                        let error = protocol_error("Portable backup verification failed");
                        self.fail_backup_operation(&info.backup_id, &error).await;
                        Err(error)
                    }
                    Err(error) => {
                        self.fail_backup_operation(&info.backup_id, &error).await;
                        Err(error)
                    }
                }
            }
            Err(error) => {
                self.fail_backup_operation(&info.backup_id, &error).await;
                Err(error)
            }
        }
    }
    async fn create_portable_backup_inner(
        &self,
        db_path: &Path,
        destination: &Path,
        passphrase: &str,
        info: &PortableBackupInfo,
    ) -> Result<TemporaryDatabase, sqlx::Error> {
        let work_dir = db_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| protocol_error("Portable backup requires a database directory"))?
            .join("backups");
        jobsentinel_platform::ensure_private_dir(&work_dir).map_err(sqlx::Error::Io)?;
        let work_path = work_dir.join(format!(".portable_backup_{}.tmp.db", Uuid::new_v4()));
        let work_key = Zeroizing::new(format!(
            "{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        ));
        let work_file = TemporaryDatabase::create(work_path)?;
        export_pool_to_encrypted(&self.pool, work_file.path(), &work_key).await?;
        let work_pool = connect_encrypted_pool(work_file.path(), &work_key, false).await?;
        sqlx::query("DELETE FROM secret_vault")
            .execute(&work_pool)
            .await?;
        sqlx::query("DELETE FROM credential_key_wrapping")
            .execute(&work_pool)
            .await?;
        sqlx::query(
            "CREATE TABLE portable_backup_manifest (
                    backup_id TEXT PRIMARY KEY NOT NULL,
                    created_at TEXT NOT NULL,
                    format_version INTEGER NOT NULL,
                    kind TEXT NOT NULL CHECK(kind = 'backup'),
                    protection TEXT NOT NULL CHECK(protection = 'encrypted'),
                    user_review_required INTEGER NOT NULL
                        CHECK(user_review_required = 1),
                    compatibility_line INTEGER NOT NULL,
                    database_schema INTEGER NOT NULL,
                    migration_sequence INTEGER NOT NULL,
                    contains_secrets INTEGER NOT NULL CHECK(contains_secrets = 0)
                )",
        )
        .execute(&work_pool)
        .await?;
        sqlx::query(
            "INSERT INTO portable_backup_manifest(
                backup_id, created_at, format_version, kind, protection,
                user_review_required, compatibility_line, database_schema,
                migration_sequence, contains_secrets
             ) VALUES (?, ?, ?, 'backup', 'encrypted', 1, ?, ?, ?, 0)",
        )
        .bind(&info.backup_id)
        .bind(&info.created_at)
        .bind(info.format_version)
        .bind(PORTABLE_BACKUP_COMPATIBILITY_LINE)
        .bind(info.database_schema)
        .bind(info.migration_sequence)
        .execute(&work_pool)
        .await?;
        sqlx::query(
            "UPDATE v3_recovery_operations
                 SET outcome = 'succeeded', completed_at = datetime('now')
                 WHERE operation_id = ?",
        )
        .bind(&info.backup_id)
        .execute(&work_pool)
        .await?;
        sqlx::query("VACUUM").execute(&work_pool).await?;
        self.verify_secret_tables_empty(&work_pool).await?;
        work_pool.close().await;
        let destination_parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| protocol_error("Portable backup destination requires a parent"))?;
        let portable_file = TemporaryDatabase::create(destination_parent.join(format!(
            ".jobsentinel_portable_backup_{}.tmp.db",
            Uuid::new_v4()
        )))?;
        export_encrypted_to_encrypted(
            work_file.path(),
            &work_key,
            portable_file.path(),
            passphrase,
        )
        .await?;
        let inspected = Self::inspect_portable_backup(portable_file.path(), passphrase).await?;
        if inspected != *info {
            return Err(protocol_error("Portable backup verification failed"));
        }
        Ok(portable_file)
    }
    pub async fn inspect_portable_backup(
        path: &Path,
        passphrase: &str,
    ) -> Result<PortableBackupInfo, sqlx::Error> {
        validate_backup_passphrase(passphrase)?;
        let pool = connect_encrypted_read_only_pool(path, passphrase)
            .await
            .map_err(|_| protocol_error("Portable backup could not be opened or verified"))?;
        let result = Self::inspect_portable_backup_pool(&pool).await;
        pool.close().await;
        result.map_err(|_| protocol_error("Portable backup could not be opened or verified"))
    }
    async fn inspect_portable_backup_pool(
        pool: &SqlitePool,
    ) -> Result<PortableBackupInfo, sqlx::Error> {
        let row: (String, String, i64, String, String, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT backup_id, created_at, format_version, kind, protection,
                    user_review_required, compatibility_line, database_schema,
                    migration_sequence, contains_secrets
             FROM portable_backup_manifest",
        )
        .fetch_one(pool)
        .await?;
        let info = PortableBackupInfo {
            backup_id: row.0,
            created_at: row.1,
            format_version: row.2,
            database_schema: row.7,
            migration_sequence: row.8,
        };
        let compiled_migration = MIGRATOR
            .iter()
            .filter(|migration| migration.migration_type.is_up_migration())
            .map(|migration| migration.version)
            .max()
            .unwrap_or_default();
        let manifest_rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM portable_backup_manifest")
                .fetch_one(pool)
                .await?;
        let actual_schema: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(pool)
            .await?;
        let applied_migrations: Vec<(i64, Vec<u8>)> = sqlx::query_as(
            "SELECT version, checksum FROM _sqlx_migrations
             WHERE success = 1 ORDER BY version",
        )
        .fetch_all(pool)
        .await?;
        let ledger_matches = applied_migrations
            .iter()
            .map(|(version, checksum)| (*version, checksum.as_slice()))
            .eq(MIGRATOR
                .iter()
                .filter(|migration| {
                    migration.migration_type.is_up_migration()
                        && migration.version <= info.migration_sequence
                })
                .map(|migration| (migration.version, migration.checksum.as_ref())));
        let failed_migrations: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 0")
                .fetch_one(pool)
                .await?;
        let compatibility: (i64, i64, i64) = sqlx::query_as(
            "SELECT compatibility_line, database_schema, migration_version
             FROM v3_compatibility_metadata WHERE singleton = 1",
        )
        .fetch_one(pool)
        .await?;
        if manifest_rows != 1
            || info.format_version != PORTABLE_BACKUP_FORMAT_VERSION
            || row.3 != "backup"
            || row.4 != "encrypted"
            || row.5 != 1
            || row.6 != PORTABLE_BACKUP_COMPATIBILITY_LINE
            || info.database_schema > DATABASE_SCHEMA_VERSION
            || info.migration_sequence > compiled_migration
            || row.9 != 0
            || actual_schema != info.database_schema
            || !ledger_matches
            || failed_migrations != 0
            || compatibility
                != (
                    PORTABLE_BACKUP_COMPATIBILITY_LINE,
                    info.database_schema,
                    info.migration_sequence,
                )
        {
            return Err(protocol_error("Portable backup is not compatible"));
        }
        let quick_check: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(pool)
            .await?;
        if !quick_check.trim().eq_ignore_ascii_case("ok") {
            return Err(protocol_error("Portable backup integrity check failed"));
        }
        let secret_rows: i64 = sqlx::query_scalar(
            "SELECT
                (SELECT COUNT(*) FROM secret_vault)
                + (SELECT COUNT(*) FROM credential_key_wrapping)",
        )
        .fetch_one(pool)
        .await?;
        let operation_outcome: String = sqlx::query_scalar(
            "SELECT outcome FROM v3_recovery_operations
             WHERE operation_id = ? AND operation_kind = 'backup'",
        )
        .bind(&info.backup_id)
        .fetch_one(pool)
        .await?;
        if secret_rows != 0 || operation_outcome != "succeeded" {
            return Err(protocol_error("Portable backup contains prohibited state"));
        }
        Ok(info)
    }
    async fn verify_secret_tables_empty(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let rows: i64 = sqlx::query_scalar(
            "SELECT
                (SELECT COUNT(*) FROM secret_vault)
                + (SELECT COUNT(*) FROM credential_key_wrapping)",
        )
        .fetch_one(pool)
        .await?;
        if rows == 0 {
            Ok(())
        } else {
            Err(protocol_error("Portable backup secret removal failed"))
        }
    }
    async fn start_backup_operation(&self, info: &PortableBackupInfo) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO v3_recovery_operations(
                operation_id, operation_kind, outcome,
                source_migration_sequence, target_migration_sequence
             ) VALUES (?, 'backup', 'started', ?, ?)",
        )
        .bind(&info.backup_id)
        .bind(info.migration_sequence)
        .bind(info.migration_sequence)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn finish_backup_operation(
        &self,
        operation_id: &str,
        error_kind: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        if finish_recovery_operation_record(&self.pool, operation_id, error_kind).await? {
            Ok(())
        } else {
            Err(protocol_error("Portable backup provenance update failed"))
        }
    }

    async fn fail_backup_operation(&self, operation_id: &str, error: &sqlx::Error) {
        let _ = self
            .finish_backup_operation(operation_id, Some(crate::database_error_kind(error)))
            .await;
    }
}

async fn export_pool_to_encrypted(
    source: &SqlitePool,
    destination: &Path,
    destination_key: &str,
) -> Result<(), sqlx::Error> {
    let destination = destination
        .to_str()
        .ok_or_else(|| protocol_error("Portable backup path encoding is invalid"))?;
    let mut connection = source.acquire().await?;
    let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *connection)
        .await?;
    sqlx::query("ATTACH DATABASE ? AS portable_work KEY ?")
        .bind(destination)
        .bind(destination_key)
        .execute(&mut *connection)
        .await?;
    let export = sqlx::query("SELECT sqlcipher_export('portable_work')")
        .execute(&mut *connection)
        .await;
    let set_version = if export.is_ok() {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "PRAGMA portable_work.user_version = {user_version}"
        )))
        .execute(&mut *connection)
        .await
        .map(|_| ())
    } else {
        Ok(())
    };
    let detach = sqlx::query("DETACH DATABASE portable_work")
        .execute(&mut *connection)
        .await;
    export?;
    set_version?;
    detach?;
    Ok(())
}

pub(super) async fn export_encrypted_to_encrypted(
    source: &Path,
    source_key: &str,
    destination: &Path,
    destination_key: &str,
) -> Result<(), sqlx::Error> {
    let source_pool = connect_encrypted_pool(source, source_key, false).await?;
    let result = export_pool_to_encrypted(&source_pool, destination, destination_key).await;
    source_pool.close().await;
    result
}

fn validate_backup_passphrase(passphrase: &str) -> Result<(), sqlx::Error> {
    if passphrase.chars().count() < MIN_BACKUP_PASSPHRASE_CHARS
        || passphrase.len() > MAX_BACKUP_PASSPHRASE_BYTES
        || passphrase.chars().all(char::is_whitespace)
    {
        return Err(protocol_error(
            "Portable backup passphrase must contain at least 16 characters and at most 1024 bytes",
        ));
    }
    Ok(())
}

fn create_exclusive_file(path: &Path) -> Result<(), sqlx::Error> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(sqlx::Error::Io)?;
    if let Err(error) = jobsentinel_platform::ensure_private_file(path) {
        let _ = std::fs::remove_file(path);
        return Err(sqlx::Error::Io(error));
    }
    Ok(())
}

struct TemporaryDatabase {
    path: PathBuf,
}

impl TemporaryDatabase {
    fn create(path: PathBuf) -> Result<Self, sqlx::Error> {
        create_exclusive_file(&path)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn publish(self, destination: &Path) -> Result<(), sqlx::Error> {
        if ["-wal", "-shm", "-journal"]
            .into_iter()
            .any(|suffix| sqlite_sidecar_path(destination, suffix).exists())
        {
            return Err(protocol_error(
                "Portable backup destination has SQLite sidecars",
            ));
        }
        std::fs::hard_link(&self.path, destination).map_err(sqlx::Error::Io)
    }
}

impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        remove_database_files(&self.path);
    }
}

fn remove_database_files(path: &Path) {
    remove_sqlite_sidecars(path);
    let mut journal = path.file_name().unwrap_or_default().to_os_string();
    journal.push("-journal");
    let _ = std::fs::remove_file(path.with_file_name(journal));
    let _ = std::fs::remove_file(path);
}

fn protocol_error(message: &'static str) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}
