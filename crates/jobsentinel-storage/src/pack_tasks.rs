//! Persists reviewed pack-task approvals, transitions, reconciliation, and atomic audit completion.

use anyhow::Result;
use chrono::Utc;
use jobsentinel_domain::{
    v3_foundation::{CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin},
    v3_manifests::{AgentTaskKind, PrivacyLabel, PrivacyReceipt},
};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

use crate::{v3_foundation::enum_text, Database};

mod input_guard;
mod types;

use input_guard::guard_matches_current;
pub use input_guard::DraftPacketInputGuard;
use types::{
    canonical_json, invalid, task_kind_text, time_text, validate_context, validate_identifier,
    PackTaskRow,
};
pub use types::{PackTaskContext, PackTaskReconciliation, PackTaskRun, PackTaskStatus};

impl Database {
    pub async fn create_pack_task(&self, context: &PackTaskContext) -> Result<PackTaskRun> {
        validate_context(context)?;
        let mut transaction = self.pool().begin().await?;
        require_exact_ready_pack(&mut transaction, context, true).await?;
        sqlx::query(
            "INSERT INTO pack_task_runs (
                run_id, approval_reference, publisher_key_id, pack_id,
                release_sequence, signed_release_sha256, stream_generation,
                task_kind, task_id, input_sha256, privacy_labels_json,
                data_categories_json, status, receipt_id, created_at, expires_at,
                started_at, completed_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', NULL, ?, ?, NULL, NULL)",
        )
        .bind(&context.run_id)
        .bind(&context.approval_reference)
        .bind(&context.publisher_key_id)
        .bind(&context.pack_id)
        .bind(i64::try_from(context.release_sequence).map_err(|_| invalid())?)
        .bind(&context.signed_release_sha256)
        .bind(i64::try_from(context.stream_generation).map_err(|_| invalid())?)
        .bind(task_kind_text(context.task_kind)?)
        .bind(&context.task_id)
        .bind(&context.input_sha256)
        .bind(canonical_json(&context.privacy_labels)?)
        .bind(canonical_json(&context.data_categories)?)
        .bind(time_text(context.created_at))
        .bind(time_text(context.expires_at))
        .execute(&mut *transaction)
        .await?;
        let run = get_in_transaction(&mut transaction, &context.run_id).await?;
        transaction.commit().await?;
        Ok(run)
    }

    pub async fn start_pack_task(&self, context: &PackTaskContext) -> Result<PackTaskRun> {
        validate_context(context)?;
        let mut transaction = self.pool().begin().await?;
        let stored = get_in_transaction(&mut transaction, &context.run_id).await?;
        if stored.context != *context || stored.status != PackTaskStatus::Pending {
            return Err(invalid());
        }
        require_exact_ready_pack(&mut transaction, context, true).await?;
        let now = time_text(Utc::now());
        let updated = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'started', started_at = ?
             WHERE run_id = ? AND status = 'pending'",
        )
        .bind(&now)
        .bind(&context.run_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(invalid());
        }
        let run = get_in_transaction(&mut transaction, &context.run_id).await?;
        transaction.commit().await?;
        Ok(run)
    }

    /// Cancelling a pending task is terminal. Repeating cancellation returns the
    /// same cancelled record; every other terminal state is rejected.
    pub async fn cancel_pack_task(&self, run_id: &str) -> Result<PackTaskRun> {
        validate_identifier(run_id)?;
        let mut transaction = self.pool().begin().await?;
        let stored = get_in_transaction(&mut transaction, run_id).await?;
        if stored.status == PackTaskStatus::Cancelled {
            transaction.commit().await?;
            return Ok(stored);
        }
        if stored.status != PackTaskStatus::Pending {
            return Err(invalid());
        }
        let updated = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'cancelled', completed_at = ?
             WHERE run_id = ? AND status = 'pending'",
        )
        .bind(time_text(Utc::now()))
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(invalid());
        }
        let run = get_in_transaction(&mut transaction, run_id).await?;
        transaction.commit().await?;
        Ok(run)
    }

    pub async fn fail_pack_task(&self, run_id: &str) -> Result<PackTaskRun> {
        validate_identifier(run_id)?;
        let mut transaction = self.pool().begin().await?;
        let updated = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'failed', completed_at = ?
             WHERE run_id = ? AND status = 'started'",
        )
        .bind(time_text(Utc::now()))
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(invalid());
        }
        let run = get_in_transaction(&mut transaction, run_id).await?;
        transaction.commit().await?;
        Ok(run)
    }

    pub async fn complete_reviewed_pack_task(
        &self,
        run_id: &str,
        receipt: &PrivacyReceipt,
        job_hash: &str,
        draft_packet_guard: Option<&DraftPacketInputGuard>,
    ) -> Result<PackTaskRun> {
        validate_identifier(run_id)?;
        validate_identifier(job_hash)?;
        receipt.validate().map_err(|_| invalid())?;
        if !receipt.stored_locally
            || receipt.data_left_device
            || receipt.external_destination.is_some()
            || receipt.gateway_policy_id.is_some()
            || receipt.approval_reference.is_some()
        {
            return Err(invalid());
        }
        let receipt_json = serde_json::to_string(receipt).map_err(|_| invalid())?;
        if receipt_json.len() > 8 * 1024 {
            return Err(invalid());
        }
        let mut transaction = if draft_packet_guard.is_some() {
            self.pool().begin_with("BEGIN IMMEDIATE").await?
        } else {
            self.pool().begin().await?
        };
        let stored = get_in_transaction(&mut transaction, run_id).await?;
        if stored.status != PackTaskStatus::Started
            || receipt.task_id != stored.context.task_id
            || receipt.pack_id.as_deref() != Some(&stored.context.pack_id)
            || receipt.labels != stored.context.privacy_labels
            || receipt.data_categories != stored.context.data_categories
        {
            return Err(invalid());
        }
        match (stored.context.task_kind, draft_packet_guard) {
            (AgentTaskKind::EvidenceReview, None) => {}
            (AgentTaskKind::DraftPacket, Some(guard))
                if guard.digest() == stored.context.input_sha256
                    && guard_matches_current(&mut transaction, guard, job_hash).await? => {}
            _ => return Err(invalid()),
        }
        require_exact_ready_pack(&mut transaction, &stored.context, true).await?;

        let case_file_id = create_case_file_in_transaction(&mut transaction, job_hash).await?;
        sqlx::query(
            "INSERT INTO v3_privacy_receipts (
                receipt_id, schema, receipt_json, stored_locally,
                data_left_device, created_at
             ) VALUES (?, ?, ?, 1, 0, ?)",
        )
        .bind(&receipt.receipt_id)
        .bind("jobsentinel.v3.privacy-receipt.v1")
        .bind(receipt_json)
        .bind(time_text(Utc::now()))
        .execute(&mut *transaction)
        .await?;
        let event = CaseFileEventInput {
            case_file_id: case_file_id.clone(),
            kind: CaseFileEventKind::PrivacyReceiptRecorded,
            origin: EventOrigin::User,
            user_action: true,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::LocalReference {
                reference_id: receipt.receipt_id.clone(),
            },
        };
        event.validate().map_err(|_| invalid())?;
        sqlx::query(
            "INSERT INTO v3_job_events (
                event_id, case_file_id, event_kind, origin, user_action,
                local_only, sensitive, metadata_json, created_at
             ) VALUES (?, ?, ?, ?, 1, 1, 1, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&case_file_id)
        .bind(enum_text(event.kind)?)
        .bind(enum_text(event.origin)?)
        .bind(event.metadata.to_json().map_err(|_| invalid())?)
        .bind(time_text(Utc::now()))
        .execute(&mut *transaction)
        .await?;
        if let Some(guard) = draft_packet_guard {
            if !guard_matches_current(&mut transaction, guard, job_hash).await? {
                return Err(invalid());
            }
        }
        let updated = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'succeeded', receipt_id = ?, completed_at = ?
             WHERE run_id = ? AND status = 'started'",
        )
        .bind(&receipt.receipt_id)
        .bind(time_text(Utc::now()))
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(invalid());
        }
        let run = get_in_transaction(&mut transaction, run_id).await?;
        transaction.commit().await?;
        Ok(run)
    }

    pub async fn reconcile_pack_tasks(&self) -> Result<PackTaskReconciliation> {
        let mut transaction = self.pool().begin().await?;
        let now = time_text(Utc::now());
        let expired_pending = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'cancelled', completed_at = ?
             WHERE status = 'pending' AND datetime(expires_at) <= datetime(?)",
        )
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        let interrupted_started = sqlx::query(
            "UPDATE pack_task_runs
             SET status = 'failed', completed_at = ?
             WHERE status = 'started'",
        )
        .bind(&now)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        transaction.commit().await?;
        Ok(PackTaskReconciliation {
            expired_pending,
            interrupted_started,
        })
    }

    pub async fn get_pack_task(&self, run_id: &str) -> Result<Option<PackTaskRun>> {
        validate_identifier(run_id)?;
        sqlx::query_as::<_, PackTaskRow>(
            "SELECT run_id, approval_reference, publisher_key_id, pack_id,
                    release_sequence, signed_release_sha256, stream_generation,
                    task_kind, task_id, input_sha256, privacy_labels_json,
                    data_categories_json, status, receipt_id, created_at, expires_at,
                    started_at, completed_at
             FROM pack_task_runs WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_optional(self.pool())
        .await?
        .map(TryInto::try_into)
        .transpose()
    }
}

async fn get_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    run_id: &str,
) -> Result<PackTaskRun> {
    sqlx::query_as::<_, PackTaskRow>(
        "SELECT run_id, approval_reference, publisher_key_id, pack_id,
                release_sequence, signed_release_sha256, stream_generation,
                task_kind, task_id, input_sha256, privacy_labels_json,
                data_categories_json, status, receipt_id, created_at, expires_at,
                started_at, completed_at
         FROM pack_task_runs WHERE run_id = ?",
    )
    .bind(run_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(invalid)?
    .try_into()
}

async fn require_exact_ready_pack(
    transaction: &mut Transaction<'_, Sqlite>,
    context: &PackTaskContext,
    require_unexpired: bool,
) -> Result<()> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1
            FROM v3_pack_streams AS stream
            JOIN v3_pack_releases AS release
              ON release.publisher_key_id = stream.publisher_key_id
             AND release.pack_id = stream.pack_id
             AND release.release_sequence = stream.active_release_sequence
            JOIN v3_pack_publishers AS publisher
              ON publisher.publisher_key_id = stream.publisher_key_id
            JOIN pack_release_reviews AS review
              ON review.publisher_key_id = release.publisher_key_id
             AND review.pack_id = release.pack_id
             AND review.release_sequence = release.release_sequence
            WHERE stream.publisher_key_id = ? AND stream.pack_id = ?
              AND stream.generation = ? AND stream.availability = 'ready'
              AND stream.active_release_sequence = ?
              AND release.signed_release_sha256 = ?
              AND release.lifecycle_state = 'ready'
              AND release.self_tested_at IS NOT NULL
              AND publisher.trust_state = 'trusted'
              AND (? = 0 OR datetime('now') < datetime(?))
        )",
    )
    .bind(&context.publisher_key_id)
    .bind(&context.pack_id)
    .bind(i64::try_from(context.stream_generation).map_err(|_| invalid())?)
    .bind(i64::try_from(context.release_sequence).map_err(|_| invalid())?)
    .bind(&context.signed_release_sha256)
    .bind(i64::from(require_unexpired))
    .bind(time_text(context.expires_at))
    .fetch_one(&mut **transaction)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(invalid())
    }
}

async fn create_case_file_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    job_hash: &str,
) -> Result<String> {
    sqlx::query(
        "INSERT INTO opportunity_case_files (case_file_id, job_hash, created_at)
         SELECT ?, ?, ? WHERE EXISTS (SELECT 1 FROM jobs WHERE hash = ?)
         ON CONFLICT(job_hash) DO NOTHING",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(job_hash)
    .bind(time_text(Utc::now()))
    .bind(job_hash)
    .execute(&mut **transaction)
    .await?;
    sqlx::query_scalar("SELECT case_file_id FROM opportunity_case_files WHERE job_hash = ?")
        .bind(job_hash)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or_else(invalid)
}
