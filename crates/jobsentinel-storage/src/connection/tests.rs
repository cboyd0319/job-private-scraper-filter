// Verifies encrypted database connection, migration, recovery, backup, and export behavior.

use super::*;

async fn insert_private_credential_rows(database: &Database, key: &str) {
    sqlx::query(
        "INSERT INTO secret_vault(
            key, algorithm, key_version, nonce, ciphertext
         ) VALUES (?, 'xchacha20poly1305', 1, ?, ?)",
    )
    .bind(key)
    .bind(vec![1_u8; 24])
    .bind(vec![2_u8; 32])
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO credential_key_wrapping(
            id, mode, kdf, memory_kib, iterations, parallelism,
            salt, algorithm, nonce, ciphertext
         ) VALUES (1, 'passphrase', 'argon2id', 19456, 2, 1, ?,
                   'xchacha20poly1305', ?, ?)",
    )
    .bind(vec![3_u8; 16])
    .bind(vec![4_u8; 24])
    .bind(vec![5_u8; 32])
    .execute(database.pool())
    .await
    .unwrap();
}

mod maintenance_tests;
mod migration_tests;
mod portable_backup_tests;
mod portable_restore_marker_tests;
mod portable_restore_tests;
mod reviewed_export_packet_tests;
mod reviewed_export_tests;
mod v3_pack_migration_tests;
mod v3_policy_migration_tests;

#[cfg(test)]
mod memory_tests {
    use super::*;
    use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};

    fn sqlite_bytes(path: &std::path::Path) -> Vec<Option<Vec<u8>>> {
        std::iter::once(path.to_path_buf())
            .chain(["-wal", "-shm"].map(|suffix| {
                let mut name = path.file_name().unwrap().to_os_string();
                name.push(suffix);
                path.with_file_name(name)
            }))
            .map(|path| std::fs::read(path).ok())
            .collect()
    }

    #[tokio::test]
    async fn in_memory_database_uses_one_connection() {
        let database = Database::connect_memory().await.unwrap();

        assert_eq!(database.pool().options().get_max_connections(), 1);
    }

    #[tokio::test]
    async fn every_pooled_connection_receives_connection_settings() {
        let temp_dir = tempfile::tempdir().unwrap();
        let database = Database::connect(&temp_dir.path().join("jobs.db"))
            .await
            .unwrap();
        let _first_connection = database.pool().acquire().await.unwrap();
        let mut second_connection = database.pool().acquire().await.unwrap();

        let trusted_schema: i64 = sqlx::query_scalar("PRAGMA trusted_schema")
            .fetch_one(&mut *second_connection)
            .await
            .unwrap();
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(&mut *second_connection)
            .await
            .unwrap();
        let cache_size: i64 = sqlx::query_scalar("PRAGMA cache_size")
            .fetch_one(&mut *second_connection)
            .await
            .unwrap();

        assert_eq!(trusted_schema, 0);
        assert_eq!(synchronous, 1);
        assert_eq!(cache_size, -128_000);
    }

    #[tokio::test]
    async fn connect_refuses_and_preserves_unsupported_newer_database_version() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let database = Database::connect(&db_path).await.unwrap();
        sqlx::query("PRAGMA user_version = 3")
            .execute(database.pool())
            .await
            .unwrap();
        database.pool().close().await;
        drop(database);
        let before = sqlite_bytes(&db_path);

        let error = Database::connect(&db_path).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("newer database schema version 3"));
        let after = sqlite_bytes(&db_path);
        assert_eq!(after[0], before[0], "refusal modified the database");
        assert_eq!(after[1], before[1], "refusal modified the WAL");

        let key = load_or_create_database_key().await.unwrap();
        let preserved_version = probe_encrypted_user_version(&db_path, &key).await.unwrap();
        assert_eq!(preserved_version, 3);
    }

    #[tokio::test]
    async fn connect_refuses_newer_version_in_wal_without_mutating_database_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("connection::tests::memory_tests::wal_crash_writer_helper")
            .env("JOBSENTINEL_WAL_CRASH_DB", &db_path)
            .status()
            .unwrap();
        assert!(status.success());
        let before = sqlite_bytes(&db_path);
        assert!(before[1].is_some(), "test requires a crash-retained WAL");

        let error = Database::connect(&db_path).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("newer database schema version 3"));
        let after = sqlite_bytes(&db_path);
        assert_eq!(after[0], before[0], "refusal modified the database");
        assert_eq!(after[1], before[1], "refusal modified the WAL");
    }

    #[tokio::test]
    async fn wal_crash_writer_helper() {
        let Ok(db_path) = std::env::var("JOBSENTINEL_WAL_CRASH_DB") else {
            return;
        };
        let database = Database::connect(std::path::Path::new(&db_path))
            .await
            .unwrap();
        sqlx::query("PRAGMA wal_autocheckpoint = 0")
            .execute(database.pool())
            .await
            .unwrap();
        sqlx::query("CREATE TABLE newer_data (value TEXT NOT NULL)")
            .execute(database.pool())
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 3")
            .execute(database.pool())
            .await
            .unwrap();
        std::process::exit(0);
    }

    #[tokio::test]
    async fn connect_preserves_supported_schema_generation_until_migration_succeeds() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let database = Database::connect(&db_path).await.unwrap();
        sqlx::query("PRAGMA user_version = 1")
            .execute(database.pool())
            .await
            .unwrap();
        database.pool().close().await;
        drop(database);

        let reopened = Database::connect(&db_path).await.unwrap();
        let stored_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(reopened.pool())
            .await
            .unwrap();

        assert_eq!(stored_version, 1);
    }

    #[tokio::test]
    async fn plaintext_newer_database_is_refused_without_encryption_or_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let mut connection = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .connect()
            .await
            .unwrap();
        sqlx::query("CREATE TABLE local_data (value TEXT NOT NULL)")
            .execute(&mut connection)
            .await
            .unwrap();
        sqlx::query("INSERT INTO local_data VALUES ('preserve me')")
            .execute(&mut connection)
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 3")
            .execute(&mut connection)
            .await
            .unwrap();
        drop(connection);
        let before = sqlite_bytes(&db_path);

        let error = Database::connect(&db_path).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("newer database schema version 3"));
        assert!(
            sqlite_bytes(&db_path) == before,
            "plaintext refusal modified SQLite files"
        );
        assert!(!temp_dir.path().join("backups").exists());
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};
    use std::str::FromStr;

    #[tokio::test]
    async fn pre_migration_backup_captures_committed_wal_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let backup_dir = temp_dir.path().join("backups");
        let database = Database::connect(&db_path).await.unwrap();

        sqlx::query("CREATE TABLE qa_wal_capture (id INTEGER PRIMARY KEY, note TEXT NOT NULL)")
            .execute(database.pool())
            .await
            .unwrap();
        sqlx::query("INSERT INTO qa_wal_capture (note) VALUES ('saved in wal')")
            .execute(database.pool())
            .await
            .unwrap();

        let backup_path =
            Database::backup_pre_migration_to_dir(database.pool(), &db_path, &backup_dir)
                .await
                .unwrap()
                .expect("existing database should create pre-migration backup");

        let backup_url = format!("sqlite://{}?mode=ro", backup_path.display());
        let backup_key = hex::encode([0xDB; 32]);
        let mut backup_connection = SqliteConnectOptions::from_str(&backup_url)
            .unwrap()
            .pragma("key", backup_key)
            .connect()
            .await
            .unwrap();
        let note: String = sqlx::query_scalar("SELECT note FROM qa_wal_capture WHERE id = 1")
            .fetch_one(&mut backup_connection)
            .await
            .unwrap();

        assert_eq!(note, "saved in wal");
    }

    #[tokio::test]
    async fn migration_without_pending_work_does_not_require_a_backup_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("jobs.db");
        let database = Database::connect(&db_path).await.unwrap();
        database.migrate().await.unwrap();

        std::fs::write(temp_dir.path().join("backups"), b"not a directory").unwrap();

        database.migrate().await.unwrap();
    }

    #[tokio::test]
    async fn migration_finishes_with_a_recorded_integrity_check() {
        let database = Database::connect_memory().await.unwrap();

        database.migrate().await.unwrap();

        let passed_checks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM integrity_check_log WHERE status = 'passed'")
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert!(passed_checks > 0);
    }
}

#[cfg(all(test, unix))]
mod permission_tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &std::path::Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[tokio::test]
    async fn connect_creates_private_database_directory_and_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().join("JobSentinel");
        let db_path = data_dir.join("jobs.db");

        let database = Database::connect(&db_path).await.unwrap();

        assert_eq!(mode(&data_dir), 0o700);
        assert_eq!(mode(&db_path), 0o600);
        if data_dir.join("jobs.db-wal").exists() {
            assert_eq!(mode(&data_dir.join("jobs.db-wal")), 0o600);
        }
        if data_dir.join("jobs.db-shm").exists() {
            assert_eq!(mode(&data_dir.join("jobs.db-shm")), 0o600);
        }

        drop(database);
    }

    #[tokio::test]
    async fn connect_does_not_chmod_existing_arbitrary_parent_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let data_dir = temp_dir.path().join("external-parent");
        fs::create_dir_all(&data_dir).unwrap();
        fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o755)).unwrap();

        let db_path = data_dir.join("jobs.db");
        fs::write(&db_path, []).unwrap();
        fs::set_permissions(&db_path, fs::Permissions::from_mode(0o644)).unwrap();

        let database = Database::connect(&db_path).await.unwrap();

        assert_eq!(mode(&data_dir), 0o755);
        assert_eq!(mode(&db_path), 0o600);

        drop(database);
    }
}
