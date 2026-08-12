use super::*;
use crate::integrity::IntegrityStatus;
use uuid::Uuid;

/// Local SQLite health relevant to safe storage maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageHealth {
    Healthy,
    RestoreFromBackupRequired,
}

/// Non-sensitive aggregate state used to review local storage maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageMaintenanceReport {
    pub health: StorageHealth,
    pub reclaimable_bytes: u64,
    pub wal_bytes: Option<u64>,
    pub incremental_vacuum_supported: bool,
    pub connectivity_required: bool,
}

/// Latest recorded portable-backup outcome, without paths or artifact claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableBackupHistory {
    NotRecorded,
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

impl Database {
    /// Get default database path
    #[must_use]
    pub fn default_path() -> PathBuf {
        jobsentinel_platform::get_data_dir().join("jobs.db")
    }

    /// Get default backup directory path
    #[must_use]
    pub fn default_backup_dir() -> PathBuf {
        jobsentinel_platform::get_data_dir().join("backups")
    }

    /// Create Database from an existing pool (for testing/advanced use cases)
    #[doc(hidden)]
    #[must_use]
    pub fn from_pool(pool: SqlitePool) -> Self {
        Database {
            pool,
            db_path: None,
            _owner_lock: None,
        }
    }

    /// Inspect aggregate local storage health without changing user data.
    pub async fn inspect_storage_maintenance(
        &self,
    ) -> Result<StorageMaintenanceReport, sqlx::Error> {
        let health = match crate::integrity::inspect_status(&self.pool).await? {
            IntegrityStatus::Healthy => StorageHealth::Healthy,
            IntegrityStatus::Corrupted | IntegrityStatus::ForeignKeyViolations => {
                StorageHealth::RestoreFromBackupRequired
            }
        };
        let page_size: i64 =
            sqlx::query_scalar("SELECT CAST(page_size AS INTEGER) FROM pragma_page_size")
                .fetch_one(&self.pool)
                .await?;
        let free_pages: i64 =
            sqlx::query_scalar("SELECT CAST(freelist_count AS INTEGER) FROM pragma_freelist_count")
                .fetch_one(&self.pool)
                .await?;
        let reclaimable_bytes = u64::try_from(page_size)
            .ok()
            .and_then(|size| {
                u64::try_from(free_pages)
                    .ok()
                    .and_then(|pages| size.checked_mul(pages))
            })
            .ok_or_else(|| maintenance_error("SQLite storage metrics are invalid"))?;
        let auto_vacuum: i64 =
            sqlx::query_scalar("SELECT CAST(auto_vacuum AS INTEGER) FROM pragma_auto_vacuum")
                .fetch_one(&self.pool)
                .await?;
        Ok(StorageMaintenanceReport {
            health,
            reclaimable_bytes,
            wal_bytes: self.wal_bytes()?,
            incremental_vacuum_supported: auto_vacuum == 2,
            connectivity_required: false,
        })
    }

    /// Inspect only the latest recorded portable-backup outcome.
    pub async fn inspect_portable_backup_history(
        &self,
    ) -> Result<PortableBackupHistory, sqlx::Error> {
        let outcome: Option<String> = sqlx::query_scalar(
            "SELECT outcome
             FROM v3_recovery_operations
             WHERE operation_kind = 'backup'
             ORDER BY created_at DESC, rowid DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        match outcome.as_deref() {
            None => Ok(PortableBackupHistory::NotRecorded),
            Some("started") => Ok(PortableBackupHistory::Started),
            Some("succeeded") => Ok(PortableBackupHistory::Succeeded),
            Some("failed") => Ok(PortableBackupHistory::Failed),
            Some("cancelled") => Ok(PortableBackupHistory::Cancelled),
            Some(_) => Err(maintenance_error(
                "Portable backup history contains an unsupported outcome",
            )),
        }
    }

    /// Checkpoint local SQLite state and reclaim up to 100 already-free pages.
    ///
    /// This refuses damaged databases and never deletes application records.
    pub async fn run_storage_cleanup(&self) -> Result<StorageMaintenanceReport, sqlx::Error> {
        if self.inspect_storage_maintenance().await?.health != StorageHealth::Healthy {
            return Err(maintenance_error(
                "Storage cleanup requires restore from backup",
            ));
        }
        let operation_id = Uuid::new_v4().to_string();
        self.start_cleanup_operation(&operation_id).await?;
        if let Err(error) = self.run_storage_cleanup_inner().await {
            let _ = self
                .finish_cleanup_operation(&operation_id, Some(crate::database_error_kind(&error)))
                .await;
            return Err(error);
        }
        self.finish_cleanup_operation(&operation_id, None).await?;
        let result = self.finish_storage_cleanup().await;
        if let Err(error) = &result {
            let _ = self
                .finish_cleanup_operation(&operation_id, Some(crate::database_error_kind(error)))
                .await;
        }
        result
    }

    #[cfg(test)]
    pub(crate) async fn checkpoint_wal(&self) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn run_storage_cleanup_inner(&self) -> Result<(), sqlx::Error> {
        self.checkpoint_wal_checked().await?;
        let auto_vacuum: i64 =
            sqlx::query_scalar("SELECT CAST(auto_vacuum AS INTEGER) FROM pragma_auto_vacuum")
                .fetch_one(&self.pool)
                .await?;
        if auto_vacuum == 2 {
            sqlx::query("PRAGMA incremental_vacuum(100)")
                .execute(&self.pool)
                .await?;
        }
        let report = self.inspect_storage_maintenance().await?;
        if report.health != StorageHealth::Healthy {
            return Err(maintenance_error(
                "Storage cleanup integrity verification failed",
            ));
        }
        Ok(())
    }

    async fn finish_storage_cleanup(&self) -> Result<StorageMaintenanceReport, sqlx::Error> {
        self.checkpoint_wal_checked().await?;
        self.inspect_storage_maintenance().await
    }

    async fn checkpoint_wal_checked(&self) -> Result<(), sqlx::Error> {
        if self.db_path.is_none() {
            return Ok(());
        }
        let (busy, _, _): (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(&self.pool)
            .await?;
        if busy != 0 {
            return Err(maintenance_error("SQLite WAL checkpoint is busy"));
        }
        Ok(())
    }

    async fn start_cleanup_operation(&self, operation_id: &str) -> Result<(), sqlx::Error> {
        let migration: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await?;
        sqlx::query(
            "INSERT INTO v3_recovery_operations(
                operation_id, operation_kind, outcome,
                source_migration_sequence, target_migration_sequence
             ) VALUES (?, 'cleanup', 'started', ?, ?)",
        )
        .bind(operation_id)
        .bind(migration)
        .bind(migration)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn finish_cleanup_operation(
        &self,
        operation_id: &str,
        error_kind: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let updated = sqlx::query(
            "UPDATE v3_recovery_operations
             SET outcome = CASE WHEN ? IS NULL THEN 'succeeded' ELSE 'failed' END,
                 error_kind = ?, completed_at = datetime('now')
             WHERE operation_id = ? AND operation_kind = 'cleanup'
               AND (outcome = 'started' OR (? IS NOT NULL AND outcome = 'succeeded'))",
        )
        .bind(error_kind)
        .bind(error_kind)
        .bind(operation_id)
        .bind(error_kind)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 1 {
            Ok(())
        } else {
            Err(maintenance_error(
                "Storage cleanup provenance update failed",
            ))
        }
    }

    fn wal_bytes(&self) -> Result<Option<u64>, sqlx::Error> {
        let Some(path) = self.db_path.as_deref() else {
            return Ok(None);
        };
        let wal = Self::sibling_path(path, "-wal");
        match std::fs::symlink_metadata(wal) {
            Ok(metadata) if metadata.file_type().is_file() => Ok(Some(metadata.len())),
            Ok(_) => Err(maintenance_error("Database WAL is not a regular file")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Some(0)),
            Err(error) => Err(sqlx::Error::Io(error)),
        }
    }
}

fn maintenance_error(message: &'static str) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}
