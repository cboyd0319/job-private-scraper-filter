//! Aggregates count-only local history for one exact saved employer name.

use anyhow::{anyhow, Result};
use sqlx::{FromRow, Sqlite, Transaction};

use super::EmployerHistoryRead;

#[derive(FromRow)]
struct EmployerHistoryRow {
    saved_job_count: i64,
    application_count: i64,
    interview_count: i64,
    offer_count: i64,
    terminal_outcome_count: i64,
}

pub(super) async fn read_employer_history(
    transaction: &mut Transaction<'_, Sqlite>,
    company: &str,
) -> Result<EmployerHistoryRead> {
    let row = sqlx::query_as::<_, EmployerHistoryRow>(
        "SELECT COUNT(DISTINCT j.hash) AS saved_job_count,
                COUNT(DISTINCT a.id) AS application_count,
                COUNT(DISTINCT i.id) AS interview_count,
                COUNT(DISTINCT CASE
                    WHEN o.id IS NOT NULL OR a.status IN
                        ('offer_received', 'offer_accepted', 'offer_rejected')
                    THEN a.id END) AS offer_count,
                COUNT(DISTINCT CASE
                    WHEN a.status IN
                        ('offer_accepted', 'offer_rejected', 'rejected', 'ghosted', 'withdrawn')
                    THEN a.id END) AS terminal_outcome_count
         FROM jobs AS j
         LEFT JOIN applications AS a ON a.job_hash = j.hash
         LEFT JOIN interviews AS i ON i.application_id = a.id
         LEFT JOIN offers AS o ON o.application_id = a.id
         WHERE j.company = ?",
    )
    .bind(company)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(EmployerHistoryRead {
        saved_job_count: checked_count(row.saved_job_count)?,
        application_count: checked_count(row.application_count)?,
        interview_count: checked_count(row.interview_count)?,
        offer_count: checked_count(row.offer_count)?,
        terminal_outcome_count: checked_count(row.terminal_outcome_count)?,
    })
}

fn checked_count(value: i64) -> Result<u32> {
    u32::try_from(value).map_err(|_| anyhow!("invalid employer-history count"))
}
