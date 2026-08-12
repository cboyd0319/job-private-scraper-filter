//! Builds one deterministic, renderer-safe application packet under an exact reviewed approval.

use std::{path::Path, time::Duration};

use anyhow::{anyhow, Result};
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use jobsentinel_domain::{v3_manifests::AgentTaskKind, v3_signed_packs::TrustedPublisherKey};
use jobsentinel_storage::{
    pack_tasks::{DraftPacketInputGuard, PackTaskStatus},
    Database,
};
use serde::Serialize;
use uuid::Uuid;

use crate::v3_foundation::{
    list_saved_match_evidence_packet_claims, prepare_saved_match_debugger, SavedMatchDebugger,
    SavedMatchEvidencePacketClaim,
};

use super::{
    fail, load_active_reviewed_task, local_task_receipt, matches_context, PackTaskContext,
    PackTaskReview, REVIEW_LIFETIME_MINUTES,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftApplicationPacket {
    pub job_title: String,
    pub company: String,
    pub resume_name: String,
    pub evidence_review: SavedMatchDebugger,
    pub reviewed_claims: Vec<SavedMatchEvidencePacketClaim>,
    pub attachment_checklist: Vec<String>,
    pub questions_to_review: Vec<String>,
    pub screening_answers_require_manual_review: bool,
    pub final_submission_by_user: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPacketTaskReview {
    pub task: PackTaskReview,
    pub packet: DraftApplicationPacket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftPacketTaskResult {
    pub packet: DraftApplicationPacket,
    pub receipt_id: String,
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_draft_packet_task(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftPacketTaskReview> {
    let active = load_active_reviewed_task(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        expected_generation,
        trusted_publishers,
        today,
        AgentTaskKind::DraftPacket,
    )
    .await?;
    let (packet, input_guard) = build_current_packet(database, job_hash, resume_id).await?;
    if draft_packet_output_size(&packet)? > active.task.max_output_bytes as usize {
        return Err(anyhow!("draft packet exceeds the signed output limit"));
    }
    let now = Utc::now();
    let context = PackTaskContext {
        run_id: format!("pack-task:{}", Uuid::new_v4()),
        approval_reference: format!("pack-task-approval:{}", Uuid::new_v4()),
        publisher_key_id: publisher_key_id.to_string(),
        pack_id: pack_id.to_string(),
        release_sequence: active.release_sequence,
        signed_release_sha256: active.signed_release_sha256,
        stream_generation: active.stream_generation,
        task_kind: active.task.kind,
        task_id: active.task.task_id.clone(),
        input_sha256: input_guard.digest().to_string(),
        privacy_labels: active.task.privacy_labels.clone(),
        data_categories: active.task.data_categories.clone(),
        created_at: now,
        expires_at: now + ChronoDuration::minutes(REVIEW_LIFETIME_MINUTES),
    };
    database.create_pack_task(&context).await?;
    Ok(DraftPacketTaskReview {
        task: PackTaskReview {
            run_id: context.run_id,
            approval_reference: context.approval_reference,
            task_kind: active.task.kind,
            plan: active.plan,
            failure_message: active.failure_message,
            privacy_labels: active.task.privacy_labels,
            data_categories: active.task.data_categories,
            max_duration_seconds: active.task.max_duration_seconds,
            max_output_bytes: active.task.max_output_bytes,
        },
        packet,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_draft_packet_task(
    database: &Database,
    artifact_root: &Path,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
    run_id: &str,
    approval_reference: &str,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftPacketTaskResult> {
    let run = database
        .get_pack_task(run_id)
        .await?
        .ok_or_else(|| anyhow!("pack task was not found"))?;
    if run.status != PackTaskStatus::Pending
        || run.context.approval_reference != approval_reference
        || run.context.task_kind != AgentTaskKind::DraftPacket
    {
        return Err(anyhow!(
            "pack task approval is invalid or no longer pending"
        ));
    }
    let active = match load_active_reviewed_task(
        database,
        artifact_root,
        &run.context.publisher_key_id,
        &run.context.pack_id,
        run.context.stream_generation,
        trusted_publishers,
        today,
        AgentTaskKind::DraftPacket,
    )
    .await
    {
        Ok(active) => active,
        Err(error) => {
            database.cancel_pack_task(run_id).await?;
            return Err(error);
        }
    };
    if !matches_context(&run.context, &active) {
        database.cancel_pack_task(run_id).await?;
        return Err(anyhow!("pack task definition changed before execution"));
    }
    if let Err(error) = database.start_pack_task(&run.context).await {
        database.cancel_pack_task(run_id).await?;
        return Err(error);
    }

    let (packet, input_guard) = match tokio::time::timeout(
        Duration::from_secs(u64::from(active.task.max_duration_seconds)),
        build_current_packet(database, job_hash, resume_id),
    )
    .await
    {
        Ok(Ok((packet, input_guard))) if input_guard.digest() == run.context.input_sha256 => {
            (packet, input_guard)
        }
        _ => return fail(database, run_id, &active.failure_message).await,
    };
    let result = DraftPacketTaskResult {
        packet,
        receipt_id: format!("pack-task-receipt:{}", Uuid::new_v4()),
    };
    if draft_packet_output_size(&result.packet)? > active.task.max_output_bytes as usize {
        return fail(database, run_id, &active.failure_message).await;
    }
    let receipt = local_task_receipt(&active, result.receipt_id.clone());
    if database
        .complete_reviewed_pack_task(run_id, &receipt, job_hash, Some(&input_guard))
        .await
        .is_err()
    {
        return fail(database, run_id, &active.failure_message).await;
    }
    Ok(result)
}

async fn build_current_packet(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
) -> Result<(DraftApplicationPacket, DraftPacketInputGuard)> {
    let before = database
        .read_draft_packet_input_guard(job_hash, resume_id)
        .await?;
    let packet = build_packet(database, job_hash, resume_id).await?;
    let after = database
        .read_draft_packet_input_guard(job_hash, resume_id)
        .await?;
    if before != after {
        return Err(anyhow!("draft packet inputs changed while preparing"));
    }
    Ok((packet, after))
}

async fn build_packet(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftApplicationPacket> {
    let case = database
        .read_opportunity_case(job_hash)
        .await
        .map_err(|_| anyhow!("selected case is no longer available"))?;
    if case.outcome_status.is_some() || case.offer_status.as_deref() == Some("accepted") {
        return Err(anyhow!(
            "closed opportunities cannot prepare another packet"
        ));
    }
    let resume = database
        .resume_matcher()
        .get_resume(resume_id)
        .await
        .map_err(|_| anyhow!("selected resume is no longer available"))?;
    let evidence_review = prepare_saved_match_debugger(database, job_hash, resume_id)
        .await
        .map_err(|_| anyhow!("selected evidence is no longer available"))?;
    let reviewed_claims = list_saved_match_evidence_packet_claims(database, job_hash, resume_id)
        .await
        .map_err(|_| anyhow!("reviewed packet claims are no longer current"))?;
    let questions_to_review = evidence_review
        .requirements()
        .iter()
        .filter(|requirement| requirement.why_not().is_some() || requirement.requires_review())
        .map(|requirement| requirement.requirement().to_string())
        .collect();
    Ok(DraftApplicationPacket {
        job_title: case.title,
        company: case.company,
        resume_name: resume.name,
        evidence_review,
        reviewed_claims,
        attachment_checklist: vec![
            "Verify the selected resume before attaching it.".to_string(),
            "Review or write the cover note; none is generated automatically.".to_string(),
            "Select work samples only when the employer requests them.".to_string(),
            "Confirm references before sharing their contact details.".to_string(),
            "Verify every employer-requested attachment before submission.".to_string(),
        ],
        questions_to_review,
        screening_answers_require_manual_review: true,
        final_submission_by_user: true,
    })
}

fn draft_packet_output_size(packet: &DraftApplicationPacket) -> Result<usize> {
    Ok(serde_json::to_vec(&DraftPacketTaskResult {
        packet: packet.clone(),
        receipt_id: format!("pack-task-receipt:{}", Uuid::nil()),
    })?
    .len())
}
