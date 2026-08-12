use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use jobsentinel_domain::v3_manifests::{DataCategory, PrivacyLabel, EXTERNAL_AI_GATEWAY_POLICY};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{sqlite_time::parse_sqlite_datetime, v3_foundation::enum_text, Database};

const JOB_DESCRIPTION_SUMMARY: &str = "job-description-summary";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideAiContext {
    pub feature_id: String,
    pub provider_id: String,
    pub destination: String,
    pub request_sha256: String,
    pub labels: Vec<PrivacyLabel>,
    pub data_categories: Vec<DataCategory>,
    pub gateway_policy_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutsideAiStatus {
    Pending,
    Started,
    Succeeded,
    Failed,
    Ambiguous,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutsideAiCompletion {
    Succeeded,
    Failed,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideAiOperation {
    pub operation_id: String,
    pub approval_reference: String,
    pub context: OutsideAiContext,
    pub status: OutsideAiStatus,
    pub receipt_recorded: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideAiActivity {
    pub provider_id: String,
    pub destination: String,
    pub status: OutsideAiStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutsideAiReconciliation {
    pub ambiguous: u64,
    pub cancelled: u64,
}

#[derive(FromRow)]
struct OutsideAiRow {
    operation_id: String,
    approval_reference: String,
    feature_id: String,
    provider_id: String,
    destination: String,
    request_sha256: String,
    labels_json: String,
    data_categories_json: String,
    gateway_policy_revision: String,
    status: String,
    receipt_recorded: bool,
    created_at: String,
    expires_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

#[derive(FromRow)]
struct OutsideAiActivityRow {
    provider_id: String,
    destination: String,
    status: String,
    created_at: String,
    completed_at: Option<String>,
}

impl Database {
    pub async fn create_outside_ai_review(
        &self,
        context: &OutsideAiContext,
        expires_at: DateTime<Utc>,
    ) -> Result<OutsideAiOperation> {
        let (labels_json, data_categories_json) = validate_context(context)?;
        let created_at = Utc::now();
        if expires_at <= created_at {
            return Err(anyhow!("outside AI review expiry must be in the future"));
        }
        let operation_id = format!("outside-ai:{}", Uuid::new_v4());
        let approval_reference = format!("outside-ai-approval:{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO v3_outside_ai_operations (
                operation_id, approval_reference, feature_id, provider_id,
                destination, request_sha256, labels_json, data_categories_json,
                gateway_policy_revision, status, created_at, expires_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?)",
        )
        .bind(&operation_id)
        .bind(&approval_reference)
        .bind(&context.feature_id)
        .bind(&context.provider_id)
        .bind(&context.destination)
        .bind(&context.request_sha256)
        .bind(labels_json)
        .bind(data_categories_json)
        .bind(&context.gateway_policy_revision)
        .bind(created_at.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .execute(self.pool())
        .await?;
        self.get_outside_ai_operation(&operation_id)
            .await?
            .ok_or_else(|| anyhow!("outside AI review was not recorded"))
    }

    pub async fn start_reviewed_outside_ai_request(
        &self,
        operation_id: &str,
        approval_reference: &str,
        context: &OutsideAiContext,
    ) -> Result<OutsideAiOperation> {
        validate_identifier("outside AI operation", operation_id)?;
        validate_identifier("outside AI approval", approval_reference)?;
        let (labels_json, data_categories_json) = validate_context(context)?;
        let started_at = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let updated = sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = 'started', started_at = ?
             WHERE operation_id = ?
               AND approval_reference = ?
               AND feature_id = ?
               AND provider_id = ?
               AND destination = ?
               AND request_sha256 = ?
               AND labels_json = ?
               AND data_categories_json = ?
               AND gateway_policy_revision = ?
               AND status = 'pending'
               AND julianday(expires_at) > julianday(?)",
        )
        .bind(&started_at)
        .bind(operation_id)
        .bind(approval_reference)
        .bind(&context.feature_id)
        .bind(&context.provider_id)
        .bind(&context.destination)
        .bind(&context.request_sha256)
        .bind(labels_json)
        .bind(data_categories_json)
        .bind(&context.gateway_policy_revision)
        .bind(&started_at)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(anyhow!(
                "outside AI review does not match, is expired, or was already used"
            ));
        }
        let row = fetch_operation(&mut transaction, operation_id).await?;
        transaction.commit().await?;
        operation_from_row(row)
    }

    pub async fn cancel_outside_ai_review(&self, operation_id: &str) -> Result<()> {
        validate_identifier("outside AI operation", operation_id)?;
        let completed_at = Utc::now().to_rfc3339();
        let updated = sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = 'cancelled', completed_at = ?
             WHERE operation_id = ? AND status = 'pending'",
        )
        .bind(completed_at)
        .bind(operation_id)
        .execute(self.pool())
        .await?;
        require_one_transition(updated.rows_affected())
    }

    pub async fn cancel_outside_ai_operation(&self, operation_id: &str) -> Result<OutsideAiStatus> {
        validate_identifier("outside AI operation", operation_id)?;
        sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = CASE status
                    WHEN 'pending' THEN 'cancelled'
                    ELSE 'ambiguous'
                 END,
                 completed_at = ?
             WHERE operation_id = ? AND status IN ('pending', 'started')",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(operation_id)
        .execute(self.pool())
        .await?;
        self.get_outside_ai_operation(operation_id)
            .await?
            .map(|operation| operation.status)
            .ok_or_else(|| anyhow!("outside AI operation was not found"))
    }

    pub async fn finish_outside_ai_request(
        &self,
        operation_id: &str,
        completion: OutsideAiCompletion,
    ) -> Result<()> {
        validate_identifier("outside AI operation", operation_id)?;
        let status = match completion {
            OutsideAiCompletion::Succeeded => "succeeded",
            OutsideAiCompletion::Failed => "failed",
            OutsideAiCompletion::Ambiguous => "ambiguous",
        };
        let updated = sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = ?, completed_at = ?
             WHERE operation_id = ?
               AND status = 'started'
               AND (? <> 'succeeded' OR receipt_recorded = 1)",
        )
        .bind(status)
        .bind(Utc::now().to_rfc3339())
        .bind(operation_id)
        .bind(status)
        .execute(self.pool())
        .await?;
        require_one_transition(updated.rows_affected())
    }

    pub async fn get_outside_ai_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<OutsideAiOperation>> {
        validate_identifier("outside AI operation", operation_id)?;
        sqlx::query_as::<_, OutsideAiRow>(
            "SELECT operation_id, approval_reference, feature_id, provider_id,
                    destination, request_sha256, labels_json, data_categories_json,
                    gateway_policy_revision, status, receipt_recorded,
                    created_at, expires_at, started_at, completed_at
             FROM v3_outside_ai_operations
             WHERE operation_id = ?",
        )
        .bind(operation_id)
        .fetch_optional(self.pool())
        .await?
        .map(operation_from_row)
        .transpose()
    }

    pub async fn get_outside_ai_operation_by_approval(
        &self,
        approval_reference: &str,
    ) -> Result<Option<OutsideAiOperation>> {
        validate_identifier("outside AI approval", approval_reference)?;
        sqlx::query_as::<_, OutsideAiRow>(
            "SELECT operation_id, approval_reference, feature_id, provider_id,
                    destination, request_sha256, labels_json, data_categories_json,
                    gateway_policy_revision, status, receipt_recorded,
                    created_at, expires_at, started_at, completed_at
             FROM v3_outside_ai_operations
             WHERE approval_reference = ?",
        )
        .bind(approval_reference)
        .fetch_optional(self.pool())
        .await?
        .map(operation_from_row)
        .transpose()
    }

    pub async fn list_recent_outside_ai_activity(
        &self,
        limit: u32,
    ) -> Result<Vec<OutsideAiActivity>> {
        if !(1..=50).contains(&limit) {
            return Err(anyhow!(
                "outside AI activity limit must be between 1 and 50"
            ));
        }
        sqlx::query_as::<_, OutsideAiActivityRow>(
            "SELECT provider_id, destination, status, created_at, completed_at
             FROM v3_outside_ai_operations
             ORDER BY created_at DESC, operation_id DESC
             LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(activity_from_row)
        .collect()
    }

    pub async fn reconcile_outside_ai_operations(&self) -> Result<OutsideAiReconciliation> {
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let ambiguous = sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = 'ambiguous', completed_at = ?
             WHERE status = 'started'",
        )
        .bind(&now)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        let cancelled = sqlx::query(
            "UPDATE v3_outside_ai_operations
             SET status = 'cancelled', completed_at = ?
             WHERE status = 'pending'
               AND julianday(expires_at) <= julianday(?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        transaction.commit().await?;
        Ok(OutsideAiReconciliation {
            ambiguous,
            cancelled,
        })
    }
}

async fn fetch_operation(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    operation_id: &str,
) -> Result<OutsideAiRow> {
    sqlx::query_as(
        "SELECT operation_id, approval_reference, feature_id, provider_id,
                destination, request_sha256, labels_json, data_categories_json,
                gateway_policy_revision, status, receipt_recorded,
                created_at, expires_at, started_at, completed_at
         FROM v3_outside_ai_operations
         WHERE operation_id = ?",
    )
    .bind(operation_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(Into::into)
}

fn operation_from_row(row: OutsideAiRow) -> Result<OutsideAiOperation> {
    let status = outside_ai_status(&row.status)?;
    let context = OutsideAiContext {
        feature_id: row.feature_id,
        provider_id: row.provider_id,
        destination: row.destination,
        request_sha256: row.request_sha256,
        labels: serde_json::from_str(&row.labels_json)
            .map_err(|_| anyhow!("invalid stored outside AI labels"))?,
        data_categories: serde_json::from_str(&row.data_categories_json)
            .map_err(|_| anyhow!("invalid stored outside AI data categories"))?,
        gateway_policy_revision: row.gateway_policy_revision,
    };
    validate_context(&context)?;
    Ok(OutsideAiOperation {
        operation_id: row.operation_id,
        approval_reference: row.approval_reference,
        context,
        status,
        receipt_recorded: row.receipt_recorded,
        created_at: parse_sqlite_datetime(&row.created_at)?,
        expires_at: parse_sqlite_datetime(&row.expires_at)?,
        started_at: row
            .started_at
            .as_deref()
            .map(parse_sqlite_datetime)
            .transpose()?,
        completed_at: row
            .completed_at
            .as_deref()
            .map(parse_sqlite_datetime)
            .transpose()?,
    })
}

fn outside_ai_status(value: &str) -> Result<OutsideAiStatus> {
    Ok(match value {
        "pending" => OutsideAiStatus::Pending,
        "started" => OutsideAiStatus::Started,
        "succeeded" => OutsideAiStatus::Succeeded,
        "failed" => OutsideAiStatus::Failed,
        "ambiguous" => OutsideAiStatus::Ambiguous,
        "cancelled" => OutsideAiStatus::Cancelled,
        _ => return Err(anyhow!("invalid stored outside AI status")),
    })
}

fn activity_from_row(row: OutsideAiActivityRow) -> Result<OutsideAiActivity> {
    Ok(OutsideAiActivity {
        provider_id: row.provider_id,
        destination: row.destination,
        status: outside_ai_status(&row.status)?,
        created_at: parse_sqlite_datetime(&row.created_at)?,
        completed_at: row
            .completed_at
            .as_deref()
            .map(parse_sqlite_datetime)
            .transpose()?,
    })
}

fn validate_context(context: &OutsideAiContext) -> Result<(String, String)> {
    validate_identifier("outside AI feature", &context.feature_id)?;
    validate_identifier("outside AI provider", &context.provider_id)?;
    if context.feature_id != JOB_DESCRIPTION_SUMMARY {
        return Err(anyhow!("outside AI feature is unsupported"));
    }
    if context.destination.len() > 2048 {
        return Err(anyhow!("outside AI destination exceeds the local limit"));
    }
    jobsentinel_security::validate_credential_free_external_https_url(&context.destination)
        .map_err(|_| anyhow!("outside AI destination is not credential-free public HTTPS"))?;
    validate_sha256(&context.request_sha256)?;
    if context.gateway_policy_revision != EXTERNAL_AI_GATEWAY_POLICY {
        return Err(anyhow!("outside AI gateway policy revision is unsupported"));
    }
    let labels_json = canonical_enum_json(&context.labels)?;
    if labels_json != r#"["external_ai_optional","public_data_only"]"# {
        return Err(anyhow!("outside AI privacy labels are invalid"));
    }
    let data_categories_json = canonical_enum_json(&context.data_categories)?;
    if data_categories_json != r#"["public_job_posting"]"# {
        return Err(anyhow!(
            "outside AI data categories are not public job posting metadata"
        ));
    }
    Ok((labels_json, data_categories_json))
}

fn canonical_enum_json<T: Copy + Serialize>(values: &[T]) -> Result<String> {
    if values.is_empty() {
        return Err(anyhow!("outside AI typed metadata is required"));
    }
    let mut values = values
        .iter()
        .copied()
        .map(enum_text)
        .collect::<Result<Vec<_>>>()?;
    values.sort_unstable();
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(anyhow!(
            "outside AI typed metadata cannot contain duplicates"
        ));
    }
    serde_json::to_string(&values).map_err(|_| anyhow!("invalid outside AI typed metadata"))
}

fn validate_identifier(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(anyhow!("{label} is not a bounded opaque identifier"));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(anyhow!("outside AI request digest is invalid"));
    }
    Ok(())
}

fn require_one_transition(rows_affected: u64) -> Result<()> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(anyhow!("outside AI lifecycle transition was rejected"))
    }
}
