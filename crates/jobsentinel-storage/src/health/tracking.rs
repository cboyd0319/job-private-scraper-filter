//! Run tracking - record scraper execution history.
//!
//! Provides functions to track the lifecycle of scraper executions:
//! - Starting runs
//! - Completing successful runs
//! - Recording failures and timeouts
//! - Incrementing retry counters
//!
//! All functions write to the `scraper_runs` table which is used for
//! health metrics calculation and historical analysis.

use crate::Database;
use anyhow::Result;
use chrono::{NaiveDateTime, Utc};
use sqlx::Row;

use super::types::{RunStatus, ScraperRun, SourceRequestOutcome, SourceRequestSummary};

fn count_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn require_terminal_write(rows_affected: u64) -> Result<()> {
    anyhow::ensure!(rows_affected == 1, "Scraper run audit row was unavailable");
    Ok(())
}

/// Record a minimized optional external source request and return its row ID.
///
/// The ledger stores metadata categories only. Callers must not pass raw job
/// titles, raw location values, private notes, resumes, salary floors, or full
/// source links.
pub async fn record_source_request_started(
    db: &Database,
    source: &str,
    endpoint_host: Option<&str>,
    title_count: usize,
    has_location: bool,
    remote_only: bool,
    result_limit: usize,
) -> Result<i64> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query(
        r#"
        INSERT INTO source_request_log (
            source, sent_at, endpoint_host, title_count, has_location,
            remote_only, result_limit, outcome
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, 'started')
        "#,
    )
    .bind(source)
    .bind(now)
    .bind(endpoint_host)
    .bind(count_to_i32(title_count))
    .bind(if has_location { 1 } else { 0 })
    .bind(if remote_only { 1 } else { 0 })
    .bind(count_to_i32(result_limit))
    .execute(db.pool())
    .await?;

    Ok(result.last_insert_rowid())
}

/// Update an optional external source request outcome.
pub async fn finish_source_request(
    db: &Database,
    request_id: i64,
    outcome: SourceRequestOutcome,
) -> Result<()> {
    anyhow::ensure!(
        outcome != SourceRequestOutcome::Started,
        "Source request audit needs a terminal outcome"
    );
    let result = sqlx::query(
        r#"
        UPDATE source_request_log
        SET outcome = ?
        WHERE id = ? AND outcome = 'started'
        "#,
    )
    .bind(outcome.as_str())
    .bind(request_id)
    .execute(db.pool())
    .await?;

    require_terminal_write(result.rows_affected())
}

/// Mark request attempts left without a terminal audit as failed.
pub async fn interrupt_started_source_requests(db: &Database) -> Result<u64> {
    let result = sqlx::query(
        "UPDATE source_request_log
         SET outcome = 'failure'
         WHERE outcome = 'started'",
    )
    .execute(db.pool())
    .await?;
    Ok(result.rows_affected())
}

/// Retrieve the latest minimized source request record.
pub async fn get_latest_source_request(
    db: &Database,
    source: &str,
) -> Result<Option<SourceRequestSummary>> {
    let row = sqlx::query(
        r#"
        SELECT id, source, sent_at, endpoint_host, title_count, has_location,
               remote_only, result_limit, outcome
        FROM source_request_log
        WHERE source = ?
        ORDER BY sent_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(source)
    .fetch_optional(db.pool())
    .await?;

    row.map(|row| {
        let sent_at = row.try_get::<NaiveDateTime, _>("sent_at")?.and_utc();
        let has_location = row.try_get::<i32, _>("has_location")? != 0;
        let remote_only = row.try_get::<i32, _>("remote_only")? != 0;
        let outcome = SourceRequestOutcome::from_str(row.try_get::<String, _>("outcome")?.as_str());

        Ok(SourceRequestSummary {
            id: row.try_get("id")?,
            source: row.try_get("source")?,
            sent_at,
            endpoint_host: row.try_get("endpoint_host")?,
            title_count: row.try_get("title_count")?,
            has_location,
            remote_only,
            result_limit: row.try_get("result_limit")?,
            outcome,
        })
    })
    .transpose()
}

/// Start a new scraper run and return its database ID.
///
/// Creates a new record in `scraper_runs` with status `Running`.
/// This ID should be used in subsequent tracking calls.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `scraper_name` - Scraper identifier (e.g. "greenhouse")
///
/// # Returns
///
/// Database ID of the new run record.
///
/// # Examples
///
/// ```ignore
/// # use jobsentinel_storage::Database;
/// # use jobsentinel_core::health::tracking;
/// # async fn example(db: &Database) -> anyhow::Result<()> {
/// let run_id = tracking::start_run(db, "greenhouse").await?;
/// // ... perform scraping ...
/// tracking::complete_run(db, run_id, 5000, 50, 10).await?;
/// # Ok(())
/// # }
/// ```
pub async fn start_run(db: &Database, scraper_name: &str) -> Result<i64> {
    let now = Utc::now();
    let result = sqlx::query!(
        r#"
        INSERT INTO scraper_runs (scraper_name, started_at, status)
        VALUES (?, ?, 'running')
        RETURNING id
        "#,
        scraper_name,
        now,
    )
    .fetch_one(db.pool())
    .await?;

    Ok(result.id)
}

/// Mark a run as successfully completed.
///
/// Updates the run record with completion time, duration, and job counts.
/// Sets status to `Success`.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `run_id` - Run ID from `start_run`
/// * `duration_ms` - Total execution time in milliseconds
/// * `jobs_found` - Total jobs scraped (including duplicates)
/// * `jobs_new` - New jobs added to database
pub async fn complete_run(
    db: &Database,
    run_id: i64,
    duration_ms: i64,
    jobs_found: usize,
    jobs_new: usize,
) -> Result<()> {
    let now = Utc::now();
    let jobs_found = jobs_found as i32;
    let jobs_new = jobs_new as i32;

    let result = sqlx::query!(
        r#"
        UPDATE scraper_runs
        SET finished_at = ?,
            duration_ms = ?,
            status = 'success',
            jobs_found = ?,
            jobs_new = ?
        WHERE id = ? AND status = 'running'
        "#,
        now,
        duration_ms,
        jobs_found,
        jobs_new,
        run_id,
    )
    .execute(db.pool())
    .await?;

    require_terminal_write(result.rows_affected())
}

/// Record a failed run with error details.
///
/// Updates the run record with failure status, error message, and optional error code.
/// Error codes are typically HTTP status codes (e.g. "429", "503") or error types.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `run_id` - Run ID from `start_run`
/// * `duration_ms` - Execution time before failure
/// * `error_message` - Human-readable error message
/// * `error_code` - Optional error code (HTTP status, error type)
pub async fn fail_run(
    db: &Database,
    run_id: i64,
    duration_ms: i64,
    error_message: &str,
    error_code: Option<&str>,
) -> Result<()> {
    let now = Utc::now();

    let result = sqlx::query!(
        r#"
        UPDATE scraper_runs
        SET finished_at = ?,
            duration_ms = ?,
            status = 'failure',
            error_message = ?,
            error_code = ?
        WHERE id = ? AND status = 'running'
        "#,
        now,
        duration_ms,
        error_message,
        error_code,
        run_id,
    )
    .execute(db.pool())
    .await?;

    require_terminal_write(result.rows_affected())
}

/// Record a timeout failure.
///
/// Updates the run record with timeout status and duration.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `run_id` - Run ID from `start_run`
/// * `duration_ms` - Time elapsed before timeout
pub async fn timeout_run(db: &Database, run_id: i64, duration_ms: i64) -> Result<()> {
    let now = Utc::now();

    let result = sqlx::query!(
        r#"
        UPDATE scraper_runs
        SET finished_at = ?,
            duration_ms = ?,
            status = 'timeout',
            error_message = 'Request timed out'
        WHERE id = ? AND status = 'running'
        "#,
        now,
        duration_ms,
        run_id,
    )
    .execute(db.pool())
    .await?;

    require_terminal_write(result.rows_affected())
}

/// Mark every source run left active by an interrupted process as failed.
pub async fn interrupt_running_runs(db: &Database) -> Result<u64> {
    let now = Utc::now().naive_utc();
    let result = sqlx::query!(
        r#"
        UPDATE scraper_runs
        SET finished_at = ?,
            duration_ms = MAX(0, CAST((julianday(?) - julianday(started_at)) * 86400000 AS INTEGER)),
            status = 'failure',
            error_message = 'Previous source check was interrupted',
            error_code = 'interrupted'
        WHERE status = 'running'
        "#,
        now,
        now,
    )
    .execute(db.pool())
    .await?;

    Ok(result.rows_affected())
}

/// Retrieve recent execution history for a scraper.
///
/// Returns runs ordered by start time (newest first).
///
/// # Arguments
///
/// * `db` - Database connection
/// * `scraper_name` - Scraper identifier
/// * `limit` - Maximum number of runs to return
///
/// # Returns
///
/// Vector of run records with all fields populated.
pub async fn get_scraper_runs(
    db: &Database,
    scraper_name: &str,
    limit: i32,
) -> Result<Vec<ScraperRun>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, scraper_name, started_at, finished_at, duration_ms,
               status, jobs_found, jobs_new, error_message, error_code, retry_attempt
        FROM scraper_runs
        WHERE scraper_name = ?
        ORDER BY started_at DESC
        LIMIT ?
        "#,
        scraper_name,
        limit,
    )
    .fetch_all(db.pool())
    .await?;

    let runs = rows
        .into_iter()
        .map(|row| ScraperRun {
            id: row.id,
            scraper_name: row.scraper_name,
            started_at: row.started_at.and_utc(),
            finished_at: row.finished_at.map(|dt| dt.and_utc()),
            duration_ms: row.duration_ms,
            status: RunStatus::from_str(&row.status),
            jobs_found: row.jobs_found.unwrap_or(0) as i32,
            jobs_new: row.jobs_new.unwrap_or(0) as i32,
            error_message: row.error_message,
            error_code: row.error_code,
            retry_attempt: row.retry_attempt.unwrap_or(0) as i32,
        })
        .collect();

    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn interrupt_started_source_requests_marks_only_unfinished_attempts() {
        let database = crate::test_support::migrated_database().await;
        record_source_request_started(&database, "interrupted", None, 1, false, true, 10)
            .await
            .unwrap();
        let finished =
            record_source_request_started(&database, "finished", None, 1, false, true, 10)
                .await
                .unwrap();
        finish_source_request(&database, finished, SourceRequestOutcome::Success)
            .await
            .unwrap();

        assert_eq!(
            interrupt_started_source_requests(&database).await.unwrap(),
            1
        );
        assert_eq!(
            get_latest_source_request(&database, "interrupted")
                .await
                .unwrap()
                .unwrap()
                .outcome,
            SourceRequestOutcome::Failure
        );
        assert_eq!(
            get_latest_source_request(&database, "finished")
                .await
                .unwrap()
                .unwrap()
                .outcome,
            SourceRequestOutcome::Success
        );
    }

    #[tokio::test]
    async fn interrupt_running_runs_reconciles_all_sources() {
        let database = crate::test_support::migrated_database().await;
        let first = start_run(&database, "disabled-source").await.unwrap();
        let second = start_run(&database, "removed-source").await.unwrap();

        assert_eq!(interrupt_running_runs(&database).await.unwrap(), 2);

        for (source, id) in [("disabled-source", first), ("removed-source", second)] {
            let run = get_scraper_runs(&database, source, 1)
                .await
                .unwrap()
                .pop()
                .unwrap();
            assert_eq!(run.id, id);
            assert_eq!(run.status, RunStatus::Failure);
            assert_eq!(run.error_code.as_deref(), Some("interrupted"));
            assert_eq!(
                run.error_message.as_deref(),
                Some("Previous source check was interrupted")
            );
            assert!(run.finished_at.is_some());
        }
    }

    #[tokio::test]
    async fn terminal_writes_reject_a_missing_audit_row() {
        let database = crate::test_support::migrated_database().await;

        assert!(complete_run(&database, i64::MAX, 1, 1, 1).await.is_err());
        assert!(fail_run(
            &database,
            i64::MAX,
            1,
            "Source check failed",
            Some("network")
        )
        .await
        .is_err());
        assert!(timeout_run(&database, i64::MAX, 1).await.is_err());
        assert!(
            finish_source_request(&database, i64::MAX, SourceRequestOutcome::Failure)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn terminal_writes_reject_an_already_finished_audit_row() {
        let database = crate::test_support::migrated_database().await;
        let run_id = start_run(&database, "finished-source").await.unwrap();
        complete_run(&database, run_id, 1, 1, 1).await.unwrap();

        assert!(
            fail_run(&database, run_id, 2, "Late failure", Some("late_failure"))
                .await
                .is_err()
        );

        let request_id =
            record_source_request_started(&database, "jobswithgpt", None, 1, false, true, 10)
                .await
                .unwrap();
        assert!(
            finish_source_request(&database, request_id, SourceRequestOutcome::Started)
                .await
                .is_err()
        );
        finish_source_request(&database, request_id, SourceRequestOutcome::Success)
            .await
            .unwrap();
        assert!(
            finish_source_request(&database, request_id, SourceRequestOutcome::Failure)
                .await
                .is_err()
        );
    }
}
