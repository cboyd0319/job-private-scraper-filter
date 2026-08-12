use super::*;

mod connection_tests {
    use super::*;
    use std::path::Path;

    fn sqlite_sidecar_path(path: &Path, suffix: &str) -> std::path::PathBuf {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        sidecar.into()
    }

    fn remove_sqlite_artifacts(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(sqlite_sidecar_path(path, "-shm"));
        let _ = std::fs::remove_file(sqlite_sidecar_path(path, "-wal"));
        let _ = std::fs::remove_file(sqlite_sidecar_path(path, ".owner.lock"));
    }

    #[tokio::test]
    async fn test_connect_memory_and_migrate() {
        let db = Database::connect_memory().await.unwrap();
        let result = db.migrate().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connect_creates_parent_directory() {
        let temp_dir = std::env::temp_dir();
        let test_db_dir = temp_dir.join("jobsentinel_test_db_parent");
        let db_path = test_db_dir.join("test_create_parent.db");

        // Ensure directory doesn't exist
        let _ = std::fs::remove_dir_all(&test_db_dir);

        let db = Database::connect(&db_path).await.unwrap();
        db.migrate().await.unwrap();

        // Verify directory was created
        assert!(test_db_dir.exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_db_dir);
    }

    #[tokio::test]
    async fn test_connect_with_existing_file() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_existing.db");

        // Create database
        let db1 = Database::connect(&db_path).await.unwrap();
        db1.migrate().await.unwrap();

        // Insert a job
        let job = create_test_job("existing_test", "Test Job", 0.9);
        let id = db1.upsert_job(&job).await.unwrap();
        db1.pool().close().await;
        drop(db1);

        // Reconnect to same database
        let db2 = Database::connect(&db_path).await.unwrap();

        // Should be able to read the job
        let fetched = db2.get_job_by_id(id).await.unwrap();
        assert!(fetched.is_some());

        // Cleanup
        let _ = std::fs::remove_file(&db_path);
    }

    #[tokio::test]
    async fn test_connect_without_parent_directory() {
        // Test connecting to a path in root directory (should succeed)
        let db_path = Path::new("test_no_parent.db");
        remove_sqlite_artifacts(db_path);

        let db = Database::connect(db_path).await.unwrap();
        db.migrate().await.unwrap();

        // Cleanup
        drop(db);
        remove_sqlite_artifacts(db_path);
    }

    #[tokio::test]
    async fn test_pool_accessor() {
        let db = crate::test_support::migrated_database().await;

        let pool = db.pool();

        // Verify pool is usable
        let result = sqlx::query("SELECT COUNT(*) FROM jobs")
            .fetch_one(pool)
            .await;
        assert!(result.is_ok());
    }
}
