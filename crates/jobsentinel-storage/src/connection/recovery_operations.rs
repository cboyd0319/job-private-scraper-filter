// Finalizes durable recovery audit records for backup and reviewed export operations.

use sqlx::SqlitePool;

pub(super) async fn finish_recovery_operation_record(
    pool: &SqlitePool,
    operation_id: &str,
    error_kind: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let updated = sqlx::query(
        "UPDATE v3_recovery_operations
         SET outcome = CASE WHEN ? IS NULL THEN 'succeeded' ELSE 'failed' END,
             error_kind = ?,
             completed_at = datetime('now')
         WHERE operation_id = ?
           AND (outcome = 'started' OR (? IS NOT NULL AND outcome = 'succeeded'))",
    )
    .bind(error_kind)
    .bind(error_kind)
    .bind(operation_id)
    .bind(error_kind)
    .execute(pool)
    .await?;
    Ok(updated.rows_affected() == 1)
}
