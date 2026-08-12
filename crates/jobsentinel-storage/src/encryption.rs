use sqlx::{
    sqlite::{
        SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
    },
    ConnectOptions,
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

#[cfg(test)]
const DATABASE_KEY_LEN: usize = 32;
const DATABASE_ENCRYPTION_ERROR: &str =
    "JobSentinel could not unlock local database encryption. Check system permission prompts, then try again.";

pub(super) async fn load_or_create_database_key() -> Result<Zeroizing<String>, sqlx::Error> {
    #[cfg(test)]
    {
        return Ok(Zeroizing::new(hex::encode([0xDB; DATABASE_KEY_LEN])));
    }

    #[cfg(not(test))]
    {
        jobsentinel_platform::load_or_create_database_key()
            .await
            .map_err(|_| database_encryption_error())
    }
}

pub(super) async fn connect_encrypted_pool(
    path: &Path,
    key: &str,
    create_if_missing: bool,
) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .pragma("key", sqlcipher_key_pragma_value(key))
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .pragma("defer_foreign_keys", "OFF")
        .pragma("cell_size_check", "ON")
        .pragma("checksum_verification", "ON")
        .pragma("trusted_schema", "OFF")
        .pragma("secure_delete", "ON")
        .pragma("auto_vacuum", "INCREMENTAL")
        .pragma("wal_autocheckpoint", "1000")
        .pragma("cache_size", "-128000")
        .pragma("temp_store", "MEMORY")
        .pragma("mmap_size", "268435456")
        .pragma("locking_mode", "NORMAL")
        .create_if_missing(create_if_missing);
    #[cfg(debug_assertions)]
    let options = options.pragma("reverse_unordered_selects", "ON");
    let smoke_logging = package_smoke_logging_enabled();
    let pool = match SqlitePoolOptions::new().connect_with(options).await {
        Ok(pool) => {
            if smoke_logging {
                tracing::info!("Opened macOS package-smoke encrypted database connection");
            }
            pool
        }
        Err(error) => {
            if smoke_logging {
                tracing::warn!(
                    error = ?error,
                    "Failed to open macOS package-smoke encrypted database connection"
                );
            }
            return Err(error);
        }
    };
    if let Err(error) = verify_sqlcipher_connection(&pool).await {
        if smoke_logging {
            tracing::warn!(
                error = ?error,
                "Failed to verify macOS package-smoke SQLCipher connection"
            );
        }
        return Err(error);
    }
    Ok(pool)
}

pub(super) async fn connect_encrypted_read_only_pool(
    path: &Path,
    key: &str,
) -> Result<SqlitePool, sqlx::Error> {
    if ["-wal", "-shm", "-journal"]
        .into_iter()
        .any(|suffix| sqlite_sidecar_path(path, suffix).exists())
    {
        return Err(sqlx::Error::Protocol(
            "Portable backup sidecars are not supported".into(),
        ));
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .pragma("key", sqlcipher_key_pragma_value(key))
        .immutable(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    verify_sqlcipher_connection(&pool).await?;
    Ok(pool)
}

pub(super) async fn probe_encrypted_user_version(
    path: &Path,
    key: &str,
) -> Result<i64, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .pragma("key", sqlcipher_key_pragma_value(key));
    let options = if sqlite_sidecar_path(path, "-wal").exists() {
        options
    } else {
        options.immutable(true)
    };
    let mut connection = options.connect().await?;
    let cipher_version: Option<String> = sqlx::query_scalar("PRAGMA cipher_version")
        .fetch_optional(&mut connection)
        .await?;
    if cipher_version
        .as_deref()
        .is_none_or(|version| version.trim().is_empty())
    {
        return Err(database_encryption_error());
    }
    sqlx::query("SELECT COUNT(*) FROM sqlite_master")
        .fetch_one(&mut connection)
        .await?;
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut connection)
        .await
}

pub(super) async fn probe_plaintext_user_version(path: &Path) -> Result<i64, sqlx::Error> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let options = if sqlite_sidecar_path(path, "-wal").exists() {
        options
    } else {
        options.immutable(true)
    };
    let mut connection = options.connect().await?;
    sqlx::query("SELECT COUNT(*) FROM sqlite_master")
        .fetch_one(&mut connection)
        .await?;
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut connection)
        .await
}

pub(super) async fn encrypt_plaintext_database(
    db_path: &Path,
    key: &str,
    backup_dir: &Path,
) -> Result<(), sqlx::Error> {
    let temp_encrypted_path = db_path.with_extension("db.sqlcipher-new");
    let backup_path = backup_dir.join(format!(
        "plaintext_before_encryption_{}.db",
        chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f")
    ));

    if temp_encrypted_path.exists() {
        std::fs::remove_file(&temp_encrypted_path).map_err(sqlx::Error::Io)?;
    }

    jobsentinel_platform::ensure_private_dir(backup_dir).map_err(sqlx::Error::Io)?;
    std::fs::copy(db_path, &backup_path).map_err(sqlx::Error::Io)?;
    jobsentinel_platform::ensure_private_file(&backup_path).map_err(sqlx::Error::Io)?;

    let temp_pool = connect_encrypted_pool(&temp_encrypted_path, key, true).await?;
    temp_pool.close().await;

    if let Err(error) =
        export_plaintext_database_to_encrypted(db_path, &temp_encrypted_path, key).await
    {
        let _ = std::fs::remove_file(&temp_encrypted_path);
        let _ = std::fs::remove_file(&backup_path);
        return Err(error);
    }

    let verified_temp_pool = connect_encrypted_pool(&temp_encrypted_path, key, false).await?;
    verified_temp_pool.close().await;

    remove_sqlite_sidecars(db_path);
    if let Err(error) = replace_database_file(db_path, &temp_encrypted_path, &backup_path) {
        return Err(sqlx::Error::Io(error));
    }

    let _ = std::fs::remove_file(&backup_path);
    jobsentinel_platform::ensure_private_sqlite_files(db_path).map_err(sqlx::Error::Io)?;
    Ok(())
}

async fn verify_sqlcipher_connection(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let cipher_version: Option<String> = sqlx::query_scalar("PRAGMA cipher_version")
        .fetch_optional(pool)
        .await?;
    if cipher_version
        .as_deref()
        .is_none_or(|version| version.trim().is_empty())
    {
        return Err(database_encryption_error());
    }

    sqlx::query("SELECT COUNT(*) FROM sqlite_master")
        .fetch_one(pool)
        .await?;
    Ok(())
}

async fn export_plaintext_database_to_encrypted(
    plaintext_path: &Path,
    encrypted_path: &Path,
    key: &str,
) -> Result<(), sqlx::Error> {
    let mut conn = SqliteConnectOptions::new()
        .filename(plaintext_path)
        .create_if_missing(false)
        .connect()
        .await?;

    let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&mut conn)
        .await;

    let encrypted_path = encrypted_path
        .to_str()
        .ok_or_else(database_encryption_error)?;
    sqlx::query("ATTACH DATABASE ? AS encrypted KEY ?")
        .bind(encrypted_path)
        .bind(key)
        .execute(&mut conn)
        .await?;
    sqlx::query("SELECT sqlcipher_export('encrypted')")
        .execute(&mut conn)
        .await?;
    sqlx::query("DETACH DATABASE encrypted")
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub(super) fn remove_sqlite_sidecars(db_path: &Path) {
    for suffix in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(sqlite_sidecar_path(db_path, suffix));
    }
}

pub(super) fn sqlite_sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar_name = db_path.file_name().unwrap_or_default().to_os_string();
    sidecar_name.push(suffix);
    db_path.with_file_name(sidecar_name)
}

fn replace_database_file(
    db_path: &Path,
    encrypted_path: &Path,
    backup_path: &Path,
) -> std::io::Result<()> {
    std::fs::remove_file(db_path)?;
    if let Err(error) = std::fs::rename(encrypted_path, db_path) {
        let _ = std::fs::copy(backup_path, db_path);
        return Err(error);
    }
    Ok(())
}

fn database_encryption_error() -> sqlx::Error {
    sqlx::Error::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        DATABASE_ENCRYPTION_ERROR,
    ))
}

fn sqlcipher_key_pragma_value(key: &str) -> String {
    format!("'{}'", key.replace('\'', "''"))
}

fn package_smoke_logging_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        jobsentinel_platform::package_smoke_root().is_some()
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::connection::Database;
    use sqlx::sqlite::SqlitePool;

    #[tokio::test]
    async fn file_database_is_not_readable_without_encryption_key() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");

        {
            let database = Database::connect(&db_path).await.unwrap();
            database.migrate().await.unwrap();
            sqlx::query("CREATE TABLE encryption_probe(value TEXT NOT NULL)")
                .execute(database.pool())
                .await
                .unwrap();
            sqlx::query("INSERT INTO encryption_probe(value) VALUES ('private job note')")
                .execute(database.pool())
                .await
                .unwrap();
            database.checkpoint_wal().await.unwrap();
        }

        let url = format!("sqlite://{}?mode=ro", db_path.display());
        let unkeyed_pool = SqlitePool::connect(&url).await.unwrap();
        let unkeyed_read = sqlx::query_scalar::<_, String>("SELECT value FROM encryption_probe")
            .fetch_one(&unkeyed_pool)
            .await;

        assert!(
            unkeyed_read.is_err(),
            "unkeyed SQLite connection should not read a JobSentinel database"
        );
    }

    #[tokio::test]
    async fn encrypted_database_handles_paths_with_spaces() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir
            .path()
            .join("Library")
            .join("Application Support")
            .join("JobSentinel")
            .join("jobs.db");

        let database = Database::connect(&db_path).await.unwrap();
        database.migrate().await.unwrap();
        sqlx::query("CREATE TABLE path_probe(value TEXT NOT NULL)")
            .execute(database.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO path_probe(value) VALUES ('macOS app data path')")
            .execute(database.pool())
            .await
            .unwrap();

        let value: String = sqlx::query_scalar("SELECT value FROM path_probe")
            .fetch_one(database.pool())
            .await
            .unwrap();
        assert_eq!(value, "macOS app data path");
        database.checkpoint_wal().await.unwrap();
        database.pool().close().await;
    }

    #[tokio::test]
    async fn encrypted_database_handles_hex_keys_starting_with_digits() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let key = "58ffd25e23c63a6fcab6baffe23e9a667c4a1504ae07454573607f036017a9c4";

        let pool = super::connect_encrypted_pool(&db_path, key, true)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE numeric_key_probe(value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO numeric_key_probe(value) VALUES ('opened')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let reopened_pool = super::connect_encrypted_pool(&db_path, key, false)
            .await
            .unwrap();
        let value: String = sqlx::query_scalar("SELECT value FROM numeric_key_probe")
            .fetch_one(&reopened_pool)
            .await
            .unwrap();
        assert_eq!(value, "opened");
        reopened_pool.close().await;
    }

    #[tokio::test]
    async fn encrypted_connection_sets_privacy_pragmas_before_migration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let key = "f13ce74a61cc859993f02767e3bb751546e4a588985f517fb8c3f8b9ad12c901";

        let pool = super::connect_encrypted_pool(&db_path, key, true)
            .await
            .unwrap();

        let secure_delete: i64 = sqlx::query_scalar("PRAGMA secure_delete")
            .fetch_one(&pool)
            .await
            .unwrap();
        let temp_store: i64 = sqlx::query_scalar("PRAGMA temp_store")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(secure_delete, 1);
        assert_eq!(temp_store, 2);
        pool.close().await;
    }

    #[tokio::test]
    async fn plaintext_database_is_migrated_to_encrypted_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let url = format!("sqlite://{}?mode=rwc", db_path.display());

        {
            let plaintext_pool = SqlitePool::connect(&url).await.unwrap();
            sqlx::query("CREATE TABLE encryption_probe(value TEXT NOT NULL)")
                .execute(&plaintext_pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO encryption_probe(value) VALUES ('preserved local record')")
                .execute(&plaintext_pool)
                .await
                .unwrap();
            plaintext_pool.close().await;
        }

        {
            let database = Database::connect(&db_path).await.unwrap();
            let value: String = sqlx::query_scalar("SELECT value FROM encryption_probe")
                .fetch_one(database.pool())
                .await
                .unwrap();
            assert_eq!(value, "preserved local record");
            database.checkpoint_wal().await.unwrap();
        }

        let unkeyed_pool = SqlitePool::connect(&url).await.unwrap();
        let unkeyed_read = sqlx::query_scalar::<_, String>("SELECT value FROM encryption_probe")
            .fetch_one(&unkeyed_pool)
            .await;
        assert!(
            unkeyed_read.is_err(),
            "plaintext migration should leave only encrypted storage behind"
        );
    }

    #[tokio::test]
    async fn plaintext_upgrade_deletes_temporary_plaintext_backup_after_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let backup_dir = temp_dir.path().join("backups");
        let url = format!("sqlite://{}?mode=rwc", db_path.display());

        {
            let plaintext_pool = SqlitePool::connect(&url).await.unwrap();
            sqlx::query("CREATE TABLE encryption_probe(value TEXT NOT NULL)")
                .execute(&plaintext_pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO encryption_probe(value) VALUES ('temporary backup probe')")
                .execute(&plaintext_pool)
                .await
                .unwrap();
            plaintext_pool.close().await;
        }
        let key = super::load_or_create_database_key().await.unwrap();
        super::encrypt_plaintext_database(&db_path, &key, &backup_dir)
            .await
            .unwrap();

        let leftovers: Vec<_> = std::fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("plaintext_before_encryption_")
            })
            .collect();

        assert!(
            leftovers.is_empty(),
            "successful plaintext upgrade must not leave plaintext backup files"
        );
    }
}
