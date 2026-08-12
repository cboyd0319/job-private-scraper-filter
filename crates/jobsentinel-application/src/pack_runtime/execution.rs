//! Loads verified static skills and runs reviewed tasks against deterministic local owners.

use std::{path::Path, time::Duration};

use anyhow::{anyhow, Result};
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{
        AgentTask, AgentTaskKind, DataCategory, PackAction, PrivacyLabel, PrivacyReceipt,
    },
    v3_pack_payloads::{ReviewedTaskPlanStep, SelfTestedPackPayload},
    v3_signed_packs::TrustedPublisherKey,
};
use jobsentinel_storage::{
    pack_tasks::{PackTaskContext, PackTaskStatus},
    v3_pack_lifecycle::PackAvailability,
    Database,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::v3_foundation::{prepare_saved_match_debugger, SavedMatchDebugger};

use super::load_active_tested_artifact;

mod packet;
pub(crate) use packet::{execute_draft_packet_task, prepare_draft_packet_task};
pub use packet::{DraftApplicationPacket, DraftPacketTaskResult, DraftPacketTaskReview};

const REVIEW_LIFETIME_MINUTES: i64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackTaskReviewStep {
    pub action: PackAction,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackTaskReview {
    pub run_id: String,
    pub approval_reference: String,
    pub task_kind: AgentTaskKind,
    pub plan: Vec<PackTaskReviewStep>,
    pub failure_message: String,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub data_categories: Vec<DataCategory>,
    pub max_duration_seconds: u32,
    pub max_output_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceReviewTaskResult {
    pub review: SavedMatchDebugger,
    pub receipt_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSkillResource {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSkillHandoff {
    pub task_kind: AgentTaskKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticSkillReview {
    pub skill_name: String,
    pub skill_md: String,
    pub resources: Vec<StaticSkillResource>,
    pub handoff: Option<StaticSkillHandoff>,
}

pub(super) struct ActivePackPayload {
    pub(super) payload: SelfTestedPackPayload,
    release_sequence: u64,
    signed_release_sha256: String,
    stream_generation: u64,
}

pub(super) struct ActiveReviewedTask {
    pub(super) task: AgentTask,
    pub(super) plan: Vec<PackTaskReviewStep>,
    pub(super) failure_message: String,
    pub(super) release_sequence: u64,
    pub(super) signed_release_sha256: String,
    pub(super) stream_generation: u64,
}

pub async fn cancel_reviewed_pack_task(database: &Database, run_id: &str) -> Result<()> {
    database.cancel_pack_task(run_id).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn open_active_static_skill(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<StaticSkillReview> {
    let active = load_active_pack_payload(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        expected_generation,
        trusted_publishers,
        today,
    )
    .await?;
    let SelfTestedPackPayload::StaticSkill {
        skill_name,
        skill_md,
        resources,
        handoff,
        ..
    } = active.payload
    else {
        return Err(anyhow!("pack does not contain a static skill"));
    };
    Ok(StaticSkillReview {
        skill_name,
        skill_md,
        resources: resources
            .into_iter()
            .map(|resource| StaticSkillResource {
                path: resource.path,
                content: resource.content,
            })
            .collect(),
        handoff: handoff.map(|handoff| StaticSkillHandoff {
            task_kind: handoff.task_kind,
            label: handoff.label,
        }),
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn prepare_evidence_review_task(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
    job_hash: &str,
    resume_id: i64,
) -> Result<PackTaskReview> {
    let active = load_active_reviewed_task(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        expected_generation,
        trusted_publishers,
        today,
        AgentTaskKind::EvidenceReview,
    )
    .await?;
    let review = prepare_saved_match_debugger(database, job_hash, resume_id)
        .await
        .map_err(|_| anyhow!("selected evidence is no longer available"))?;
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
        input_sha256: evidence_input_sha256(job_hash, resume_id, &review)?,
        privacy_labels: active.task.privacy_labels.clone(),
        data_categories: active.task.data_categories.clone(),
        created_at: now,
        expires_at: now + ChronoDuration::minutes(REVIEW_LIFETIME_MINUTES),
    };
    database.create_pack_task(&context).await?;
    Ok(PackTaskReview {
        run_id: context.run_id,
        approval_reference: context.approval_reference,
        task_kind: active.task.kind,
        plan: active.plan,
        failure_message: active.failure_message,
        privacy_labels: active.task.privacy_labels,
        data_categories: active.task.data_categories,
        max_duration_seconds: active.task.max_duration_seconds,
        max_output_bytes: active.task.max_output_bytes,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_evidence_review_task(
    database: &Database,
    artifact_root: &Path,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
    run_id: &str,
    approval_reference: &str,
    job_hash: &str,
    resume_id: i64,
) -> Result<EvidenceReviewTaskResult> {
    let run = database
        .get_pack_task(run_id)
        .await?
        .ok_or_else(|| anyhow!("pack task was not found"))?;
    if run.status != PackTaskStatus::Pending
        || run.context.approval_reference != approval_reference
        || run.context.task_kind != AgentTaskKind::EvidenceReview
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
        AgentTaskKind::EvidenceReview,
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

    let execution = tokio::time::timeout(
        Duration::from_secs(u64::from(active.task.max_duration_seconds)),
        prepare_saved_match_debugger(database, job_hash, resume_id),
    )
    .await;
    let review = match execution {
        Ok(Ok(review))
            if evidence_input_sha256(job_hash, resume_id, &review)
                .is_ok_and(|digest| digest == run.context.input_sha256) =>
        {
            review
        }
        _ => return fail(database, run_id, &active.failure_message).await,
    };
    let result = EvidenceReviewTaskResult {
        review,
        receipt_id: format!("pack-task-receipt:{}", Uuid::new_v4()),
    };
    if serde_json::to_vec(&result)?.len() > active.task.max_output_bytes as usize {
        return fail(database, run_id, &active.failure_message).await;
    }
    let receipt = local_task_receipt(&active, result.receipt_id.clone());
    if database
        .complete_reviewed_pack_task(run_id, &receipt, job_hash, None)
        .await
        .is_err()
    {
        return fail(database, run_id, &active.failure_message).await;
    }
    Ok(result)
}

pub(super) async fn load_active_reviewed_task(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
    expected_kind: AgentTaskKind,
) -> Result<ActiveReviewedTask> {
    let active = load_active_pack_payload(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        expected_generation,
        trusted_publishers,
        today,
    )
    .await?;
    let SelfTestedPackPayload::ReviewedWorkflow {
        task,
        plan,
        failure_message,
    } = active.payload
    else {
        return Err(anyhow!("pack does not contain a reviewed task"));
    };
    if task.kind != expected_kind {
        return Err(anyhow!("pack task kind is not supported by this operation"));
    }
    Ok(ActiveReviewedTask {
        task,
        plan: review_steps(plan),
        failure_message,
        release_sequence: active.release_sequence,
        signed_release_sha256: active.signed_release_sha256,
        stream_generation: active.stream_generation,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_active_pack_payload(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<ActivePackPayload> {
    let stream = database.get_pack_stream(publisher_key_id, pack_id).await?;
    if stream.publisher_key_id != publisher_key_id
        || stream.pack_id != pack_id
        || stream.generation != expected_generation
        || stream.availability != PackAvailability::Ready
    {
        return Err(anyhow!("pack is not ready"));
    }
    let release_sequence = stream
        .active_release_sequence
        .ok_or_else(|| anyhow!("pack has no active release"))?;
    let stored = database
        .get_stored_pack_release(publisher_key_id, pack_id, release_sequence)
        .await?;
    let (_, tested, _) = load_active_tested_artifact(
        database,
        artifact_root,
        &stored,
        expected_generation,
        trusted_publishers,
        today,
    )
    .await?;
    Ok(ActivePackPayload {
        payload: tested.into_payload(),
        release_sequence,
        signed_release_sha256: stored.signed_release_sha256,
        stream_generation: stream.generation,
    })
}

fn review_steps(plan: Vec<ReviewedTaskPlanStep>) -> Vec<PackTaskReviewStep> {
    plan.into_iter()
        .map(|step| PackTaskReviewStep {
            action: step.action,
            label: step.label,
        })
        .collect()
}

pub(super) fn local_task_receipt(
    active: &ActiveReviewedTask,
    receipt_id: String,
) -> PrivacyReceipt {
    PrivacyReceipt {
        schema: SchemaId::PrivacyReceiptV1,
        receipt_id,
        task_id: active.task.task_id.clone(),
        pack_id: active.task.pack_id.clone(),
        labels: active.task.privacy_labels.clone(),
        data_categories: active.task.data_categories.clone(),
        stored_locally: true,
        data_left_device: false,
        external_destination: None,
        gateway_policy_id: None,
        approval_reference: None,
        delete_or_revoke_action:
            "Disable or remove the pack to prevent future runs; local audit history remains."
                .to_string(),
    }
}

pub(super) fn matches_context(context: &PackTaskContext, active: &ActiveReviewedTask) -> bool {
    context.release_sequence == active.release_sequence
        && context.signed_release_sha256 == active.signed_release_sha256
        && context.stream_generation == active.stream_generation
        && context.task_kind == active.task.kind
        && context.task_id == active.task.task_id
        && context.privacy_labels == active.task.privacy_labels
        && context.data_categories == active.task.data_categories
}

fn evidence_input_sha256(
    job_hash: &str,
    resume_id: i64,
    review: &SavedMatchDebugger,
) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(b"jobsentinel.pack-task.evidence-review.v1\0");
    let review_json = serde_json::to_vec(review)?;
    for value in [job_hash.as_bytes(), &resume_id.to_le_bytes(), &review_json] {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
    }
    Ok(hex::encode(digest.finalize()))
}

pub(super) async fn fail<T>(database: &Database, run_id: &str, message: &str) -> Result<T> {
    database.fail_pack_task(run_id).await?;
    Err(anyhow!(message.to_string()))
}
