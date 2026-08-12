//! Startup integrity verification for SQLite data.

mod checks;
mod types;

#[cfg(test)]
mod tests;

use sqlx::SqlitePool;

pub(crate) use types::IntegrityStatus;

struct DatabaseIntegrity {
    db: SqlitePool,
}

impl DatabaseIntegrity {
    fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub(super) async fn startup_check(&self) -> Result<IntegrityStatus, sqlx::Error> {
        tracing::info!("Running database integrity check");
        let start_time = std::time::Instant::now();

        let quick_result = self.quick_check().await?;
        if !quick_result.is_ok {
            tracing::error!("Database quick integrity check failed");
            self.log_check(
                "quick",
                "failed",
                Some("Database quick integrity check failed"),
                start_time.elapsed(),
            )
            .await?;
            return Ok(IntegrityStatus::Corrupted);
        }

        let fk_violation_count = self.foreign_key_violation_count().await?;
        if fk_violation_count > 0 {
            tracing::warn!(
                "Foreign key violations detected: {} issues",
                fk_violation_count
            );
            self.log_check(
                "foreign_key",
                "warning",
                Some(&format!("{fk_violation_count} violations")),
                start_time.elapsed(),
            )
            .await?;
            return Ok(IntegrityStatus::ForeignKeyViolations);
        }

        if self.should_run_full_check().await? {
            tracing::info!("Running scheduled full database integrity check");
            let full_result = self.full_integrity_check().await?;
            if !full_result.is_ok {
                tracing::error!("Full database integrity check failed");
                self.log_check(
                    "full",
                    "failed",
                    Some("Full database integrity check failed"),
                    start_time.elapsed(),
                )
                .await?;
                return Ok(IntegrityStatus::Corrupted);
            }

            self.update_last_full_check().await?;
        }

        self.log_check("quick", "passed", None, start_time.elapsed())
            .await?;
        tracing::info!(
            "Database integrity check passed ({:?})",
            start_time.elapsed()
        );
        Ok(IntegrityStatus::Healthy)
    }
}

pub(super) async fn verify_startup(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    match DatabaseIntegrity::new(pool.clone()).startup_check().await? {
        IntegrityStatus::Healthy => Ok(()),
        IntegrityStatus::Corrupted => Err(sqlx::Error::Protocol(
            "Database integrity check failed".into(),
        )),
        IntegrityStatus::ForeignKeyViolations => Err(sqlx::Error::Protocol(
            "Database relational integrity check failed".into(),
        )),
    }
}

pub(crate) async fn inspect_status(pool: &SqlitePool) -> Result<IntegrityStatus, sqlx::Error> {
    let integrity = DatabaseIntegrity::new(pool.clone());
    if !integrity.quick_check().await?.is_ok {
        return Ok(IntegrityStatus::Corrupted);
    }
    if integrity.foreign_key_violation_count().await? > 0 {
        return Ok(IntegrityStatus::ForeignKeyViolations);
    }
    Ok(IntegrityStatus::Healthy)
}
