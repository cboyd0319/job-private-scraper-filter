//! Owns encrypted SQLite connections, configuration, migrations, and startup recovery.

mod backups;
mod portable_backup;
mod portable_restore;
mod portable_restore_bundle;
mod portable_restore_promotion;
mod reviewed_export;
mod reviewed_export_inspect;
mod reviewed_export_sanitize;
mod reviewed_export_schema;

pub use maintenance::{PortableBackupHistory, StorageHealth, StorageMaintenanceReport};
pub use portable_backup::PortableBackupInfo;
pub use portable_restore::PortableRestoreStatus;
pub use reviewed_export::{ReviewedExportInfo, ReviewedExportPlan, ReviewedExportSelection};

use sqlx::{
    sqlite::{SqlitePool, SqlitePoolOptions},
    Row,
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use super::encryption::{
    connect_encrypted_pool, encrypt_plaintext_database, load_or_create_database_key,
    probe_encrypted_user_version, probe_plaintext_user_version,
};

const DATABASE_SCHEMA_VERSION: i64 = 2;
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Database handle
#[derive(Debug)]
pub struct Database {
    pool: SqlitePool,
    /// Path to the on-disk database file. `None` for in-memory databases.
    db_path: Option<PathBuf>,
    _owner_lock: Option<File>,
}

impl Database {
    /// Create the bounded encrypted-credential repository for this database.
    #[must_use]
    pub fn credentials(&self) -> crate::CredentialRepository {
        crate::CredentialRepository::new(self.pool.clone())
    }

    /// Connect to SQLite database with optimized settings
    pub async fn connect(path: &Path) -> Result<Self, sqlx::Error> {
        // Ensure parent directory exists
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            let parent_existed = parent.exists();
            let parent_result = if parent_existed {
                std::fs::create_dir_all(parent)
            } else {
                jobsentinel_platform::ensure_private_dir(parent)
            };

            parent_result.map_err(|e| {
                tracing::warn!(
                    error_kind = ?e.kind(),
                    "Failed to create database directory"
                );
                sqlx::Error::Io(e)
            })?;
        }
        let owner_lock = Self::acquire_owner_lock(path)?;

        let plaintext_version = if path.exists() {
            probe_plaintext_user_version(path).await.ok()
        } else {
            None
        };
        if let Some(version) = plaintext_version {
            Self::refuse_unsupported_database(version)?;
        }

        let key = load_or_create_database_key().await?;
        let pool = if plaintext_version.is_some() {
            let backup_dir = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| parent.join("backups"))
                .unwrap_or_else(Self::default_backup_dir);
            encrypt_plaintext_database(path, &key, &backup_dir).await?;
            connect_encrypted_pool(path, &key, false).await?
        } else if path.exists() {
            let version = probe_encrypted_user_version(path, &key).await?;
            Self::refuse_unsupported_database(version)?;
            connect_encrypted_pool(path, &key, false).await?
        } else {
            connect_encrypted_pool(path, &key, true).await?
        };

        jobsentinel_platform::ensure_private_sqlite_files(path).map_err(sqlx::Error::Io)?;

        Ok(Database {
            pool,
            db_path: Some(path.to_path_buf()),
            _owner_lock: Some(owner_lock),
        })
    }

    fn refuse_unsupported_database(stored_version: i64) -> Result<(), sqlx::Error> {
        if stored_version > DATABASE_SCHEMA_VERSION {
            return Err(sqlx::Error::Protocol(
                format!(
                    "Unsupported newer database schema version {stored_version}; this app supports through version {DATABASE_SCHEMA_VERSION}"
                ),
            ));
        }
        Ok(())
    }

    /// Configure database-persistent settings and log connection diagnostics.
    async fn configure_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        tracing::info!("Configuring SQLite database");

        let stored_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(pool)
            .await?;
        Self::refuse_unsupported_database(stored_version)?;

        // Set page size to 4096 bytes (optimal for most systems)
        // MUST be set before any tables are created (only affects new databases)
        // Common page sizes: 1024, 2048, 4096, 8192, 16384, 32768
        sqlx::query("PRAGMA page_size = 4096")
            .execute(pool)
            .await
            .ok(); // Ignore errors (can't change after DB created)
        tracing::debug!("Page size = 4096 bytes (if new DB)");

        // Set application ID (unique identifier for JobSentinel)
        // Helps identify database files in forensic analysis
        // Using ASCII "JSDB" = 0x4A534442
        sqlx::query("PRAGMA application_id = 1246970946")
            .execute(pool)
            .await?;
        tracing::debug!("Application ID set (JSDB)");

        // Run query optimizer analysis to update statistics
        // Helps SQLite choose better query plans
        sqlx::query("PRAGMA optimize").execute(pool).await?;
        tracing::debug!("Query optimizer statistics updated");

        // Log SQLite compile options (useful for debugging)
        if let Ok(rows) = sqlx::query("PRAGMA compile_options").fetch_all(pool).await {
            let options: Vec<String> = rows
                .iter()
                .filter_map(|row| row.try_get::<String, _>(0).ok())
                .collect();
            tracing::debug!("SQLite compile options: {} features", options.len());

            // Check for important features
            let has_fts5 = options.iter().any(|opt| opt.contains("FTS5"));
            let has_json = options.iter().any(|opt| opt.contains("JSON"));
            let has_rtree = options.iter().any(|opt| opt.contains("RTREE"));

            if has_fts5 {
                tracing::debug!("FTS5 full-text search available");
            }
            if has_json {
                tracing::debug!("JSON1 extension available");
            }
            if has_rtree {
                tracing::debug!("R*Tree spatial index available");
            }
        }

        // Log SQLite version
        if let Ok(row) = sqlx::query("SELECT sqlite_version()").fetch_one(pool).await {
            if let Ok(version) = row.try_get::<String, _>(0) {
                tracing::info!("SQLite version: {}", version);
            }
        }

        // Verify cache size is AT LEAST 64MB
        if let Ok(row) = sqlx::query("PRAGMA cache_size").fetch_one(pool).await {
            if let Ok(cache_size) = row.try_get::<i64, _>(0) {
                let actual_mb = if cache_size < 0 {
                    // Negative = kilobytes
                    cache_size.abs() / 1024
                } else {
                    // Positive = pages (4KB each typically)
                    cache_size * 4 / 1024
                };

                if actual_mb >= 64 {
                    tracing::debug!("Cache size verified: {}MB (>= 64MB)", actual_mb);
                } else {
                    tracing::warn!("Cache size only {}MB (< 64MB minimum)", actual_mb);
                }
            }
        }

        // Verify WAL mode is actually enabled
        if let Ok(row) = sqlx::query("PRAGMA journal_mode").fetch_one(pool).await {
            if let Ok(mode) = row.try_get::<String, _>(0) {
                if mode.eq_ignore_ascii_case("wal") {
                    tracing::debug!("WAL mode verified");
                } else {
                    tracing::error!("WAL mode not enabled (got: {})", mode);
                }
            }
        }

        // Verify foreign keys are enforced
        if let Ok(row) = sqlx::query("PRAGMA foreign_keys").fetch_one(pool).await {
            if let Ok(enabled) = row.try_get::<i64, _>(0) {
                if enabled == 1 {
                    tracing::debug!("Foreign keys verified");
                } else {
                    tracing::error!("Foreign keys not enabled");
                }
            }
        }

        tracing::info!("Database configuration verified");
        Ok(())
    }

    /// Run pending database migrations.
    ///
    /// An existing database receives one verified, same-device SQLCipher
    /// recovery snapshot per source-to-target transition. Retries reuse that
    /// snapshot. This internal recovery copy is not a portable user export.
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        self.migrate_with(&MIGRATOR).await
    }

    async fn migrate_with(&self, migrator: &sqlx::migrate::Migrator) -> Result<(), sqlx::Error> {
        let pending = self.pending_migration_range(migrator).await?;
        let snapshot =
            if let (Some(db_path), Some((from_version, to_version))) = (&self.db_path, pending) {
                Self::ensure_migration_snapshot(&self.pool, db_path, from_version, to_version)
                    .await
                    .map_err(|_| {
                        tracing::error!("Required migration recovery snapshot failed");
                        sqlx::Error::Protocol("Required migration recovery snapshot failed".into())
                    })?
            } else {
                None
            };

        migrator.run(&self.pool).await?;
        self.reconcile_interrupted_recovery_operations().await?;
        self.reconcile_interrupted_pack_lifecycle()
            .await
            .map_err(|_| {
                tracing::error!("Pack lifecycle startup reconciliation failed");
                sqlx::Error::Protocol("Pack lifecycle startup reconciliation failed".into())
            })?;
        self.reconcile_pack_tasks().await.map_err(|_| {
            tracing::error!("Reviewed pack task startup reconciliation failed");
            sqlx::Error::Protocol("Reviewed pack task startup reconciliation failed".into())
        })?;
        Self::configure_database(&self.pool).await?;
        sqlx::query("PRAGMA user_version = 2")
            .execute(&self.pool)
            .await?;
        self.verify_integrity().await?;
        if let Some(db_path) = &self.db_path {
            jobsentinel_platform::ensure_private_sqlite_files(db_path).map_err(sqlx::Error::Io)?;
            if let Some(snapshot) = snapshot {
                if let Some(backup_dir) = snapshot.parent() {
                    Self::prune_superseded_migration_snapshots(backup_dir, &snapshot);
                }
            }
        }
        Ok(())
    }

    async fn pending_migration_range(
        &self,
        migrator: &sqlx::migrate::Migrator,
    ) -> Result<Option<(i64, i64)>, sqlx::Error> {
        let Some(target_version) = migrator
            .iter()
            .filter(|migration| migration.migration_type.is_up_migration())
            .map(|migration| migration.version)
            .max()
        else {
            return Ok(None);
        };
        let has_migration_ledger: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_one(&self.pool)
        .await?;
        if has_migration_ledger == 0 {
            return Ok(None);
        }
        let applied_version: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await?;
        Ok((applied_version < target_version).then_some((applied_version, target_version)))
    }

    async fn verify_integrity(&self) -> Result<(), sqlx::Error> {
        super::integrity::verify_startup(&self.pool).await
    }

    /// Connect to in-memory SQLite database (for testing)
    /// Available in test builds and for integration tests
    pub async fn connect_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        // Apply pragmas that are compatible with in-memory SQLite.
        // WAL journal mode, mmap_size, auto_vacuum, and WAL checkpointing
        // do not work with in-memory databases and are intentionally skipped.
        Self::configure_memory_pragmas(&pool).await?;
        Ok(Database {
            pool,
            db_path: None,
            _owner_lock: None,
        })
    }

    pub(super) fn acquire_owner_lock(path: &Path) -> Result<File, sqlx::Error> {
        let lock_path = Self::sibling_path(path, ".owner.lock");
        let lock = loop {
            match std::fs::symlink_metadata(&lock_path) {
                Ok(metadata) if metadata.file_type().is_file() => {
                    break OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&lock_path)
                        .map_err(sqlx::Error::Io)?;
                }
                Ok(_) => {
                    return Err(sqlx::Error::Protocol(
                        "JobSentinel owner lock is invalid".into(),
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    match OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create_new(true)
                        .open(&lock_path)
                    {
                        Ok(lock) => break lock,
                        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                        Err(error) => return Err(sqlx::Error::Io(error)),
                    }
                }
                Err(error) => return Err(sqlx::Error::Io(error)),
            }
        };
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                sqlx::Error::Protocol("JobSentinel database is in use".into())
            }
            std::fs::TryLockError::Error(error) => sqlx::Error::Io(error),
        })?;
        if !owner_lock_matches(&lock, &lock_path).map_err(sqlx::Error::Io)? {
            return Err(sqlx::Error::Protocol(
                "JobSentinel owner lock is invalid".into(),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let original_permissions = lock.metadata().map_err(sqlx::Error::Io)?.permissions();
            lock.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(sqlx::Error::Io)?;
            if !owner_lock_matches(&lock, &lock_path).map_err(sqlx::Error::Io)? {
                let _ = lock.set_permissions(original_permissions);
                return Err(sqlx::Error::Protocol(
                    "JobSentinel owner lock is invalid".into(),
                ));
            }
        }
        if !owner_lock_matches(&lock, &lock_path).map_err(sqlx::Error::Io)? {
            return Err(sqlx::Error::Protocol(
                "JobSentinel owner lock is invalid".into(),
            ));
        }
        Ok(lock)
    }

    fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(suffix);
        path.with_file_name(name)
    }

    /// Configure SQLite PRAGMA settings compatible with in-memory databases (tests).
    ///
    /// Applies the same integrity and performance settings as [`configure_pragmas`]
    /// except for features that require a real file (WAL, mmap, auto_vacuum).
    /// Most importantly, this enables `foreign_keys = ON` so that FK violations
    /// are caught in tests the same way they would be in production.
    async fn configure_memory_pragmas(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        // INTEGRITY: Enable foreign key constraints — the main reason this exists.
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(pool)
            .await?;

        // INTEGRITY: Enforce immediate FK checking (no deferring).
        sqlx::query("PRAGMA defer_foreign_keys = OFF")
            .execute(pool)
            .await?;

        // INTEGRITY: Cell size verification for corruption detection.
        sqlx::query("PRAGMA cell_size_check = ON")
            .execute(pool)
            .await?;

        // JOURNAL: Use DELETE mode — WAL requires a real file and does not
        // work with sqlite::memory: connections.
        sqlx::query("PRAGMA journal_mode = DELETE")
            .execute(pool)
            .await?;

        // PERFORMANCE: Synchronous = NORMAL (balanced; no fsync cost for in-memory anyway).
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(pool)
            .await?;

        // PERFORMANCE: Keep temp tables in memory.
        sqlx::query("PRAGMA temp_store = MEMORY")
            .execute(pool)
            .await?;

        // PERFORMANCE: 32MB cache for test databases (enough without wasting memory).
        sqlx::query("PRAGMA cache_size = -32000")
            .execute(pool)
            .await?;

        // LOCKING: Wait up to 5 s before failing on a locked database.
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(pool)
            .await?;

        // DEBUG: Randomise result order to surface implicit ORDER BY dependencies.
        #[cfg(debug_assertions)]
        {
            sqlx::query("PRAGMA reverse_unordered_selects = ON")
                .execute(pool)
                .await
                .ok();
        }

        tracing::debug!("In-memory SQLite configured (FK enforcement ON)");
        Ok(())
    }

    /// Internal SQL owner access for repositories, integrity checks, and backups.
    #[must_use]
    pub(crate) const fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn owner_lock_matches(file: &File, path: &Path) -> std::io::Result<bool> {
    let path_metadata = std::fs::symlink_metadata(path)?;
    if !path_metadata.file_type().is_file() {
        return Ok(false);
    }
    let opened_metadata = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok((opened_metadata.dev(), opened_metadata.ino())
            == (path_metadata.dev(), path_metadata.ino())
            && opened_metadata.nlink() == 1)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        Ok(
            opened_metadata.volume_serial_number() == path_metadata.volume_serial_number()
                && opened_metadata.file_index() == path_metadata.file_index()
                && opened_metadata.file_index().is_some()
                && opened_metadata.number_of_links() == Some(1),
        )
    }
    #[cfg(not(any(unix, windows)))]
    Ok(true)
}

mod maintenance;

#[cfg(test)]
mod tests;
