//! Applies immutable production publisher trust to pack lifecycle and reviewed-use operations.

use anyhow::Result;
use chrono::Utc;
use jobsentinel_storage::Database;

use super::evaluation::{
    evaluate_active_ats_resume_requirement_pack,
    evaluate_active_evidence_reviewer_resume_requirement_pack,
    evaluate_active_static_skill_handoff_pack,
};
use super::{
    activate_pack_artifact, enable_pack_artifact, execute_draft_packet_task,
    execute_evidence_review_task, open_active_static_skill, prepare_draft_packet_task,
    prepare_evidence_review_task, production_trusted_publishers, rollback_pack_artifact,
    stage_pack_artifact, AtsResumeRequirementEvaluation, DraftPacketTaskResult,
    DraftPacketTaskReview, EvidenceReviewTaskResult, EvidenceReviewerResumeRequirementEvaluation,
    PackInstallReview, PackRuntimeEnvironment, PackStateChange, PackTaskReview,
    StaticSkillHandoffEvaluation, StaticSkillReview,
};

pub async fn evaluate_production_active_ats_resume_requirement_pack(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<AtsResumeRequirementEvaluation> {
    evaluate_active_ats_resume_requirement_pack(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn evaluate_production_active_evidence_reviewer_resume_requirement_pack(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<EvidenceReviewerResumeRequirementEvaluation> {
    evaluate_active_evidence_reviewer_resume_requirement_pack(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn evaluate_production_active_static_skill_handoff_pack(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<StaticSkillHandoffEvaluation> {
    evaluate_active_static_skill_handoff_pack(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn stage_production_pack_artifact(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    envelope: &[u8],
) -> Result<PackInstallReview> {
    stage_pack_artifact(
        database,
        runtime.artifact_root(),
        envelope,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn activate_production_pack_artifact(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    release_sequence: u64,
    expected_generation: u64,
) -> Result<PackInstallReview> {
    activate_pack_artifact(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        release_sequence,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn rollback_production_pack_artifact(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<PackInstallReview> {
    rollback_pack_artifact(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn enable_production_pack_artifact(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<PackStateChange> {
    enable_pack_artifact(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

pub async fn open_production_active_static_skill(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<StaticSkillReview> {
    open_active_static_skill(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_production_evidence_review_task(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    job_hash: &str,
    resume_id: i64,
) -> Result<PackTaskReview> {
    prepare_evidence_review_task(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
        job_hash,
        resume_id,
    )
    .await
}

pub async fn execute_production_evidence_review_task(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    run_id: &str,
    approval_reference: &str,
    job_hash: &str,
    resume_id: i64,
) -> Result<EvidenceReviewTaskResult> {
    execute_evidence_review_task(
        database,
        runtime.artifact_root(),
        production_trusted_publishers(),
        Utc::now().date_naive(),
        run_id,
        approval_reference,
        job_hash,
        resume_id,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_production_draft_packet_task(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftPacketTaskReview> {
    prepare_draft_packet_task(
        database,
        runtime.artifact_root(),
        publisher_key_id,
        pack_id,
        expected_generation,
        production_trusted_publishers(),
        Utc::now().date_naive(),
        job_hash,
        resume_id,
    )
    .await
}

pub async fn execute_production_draft_packet_task(
    database: &Database,
    runtime: &PackRuntimeEnvironment,
    run_id: &str,
    approval_reference: &str,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftPacketTaskResult> {
    execute_draft_packet_task(
        database,
        runtime.artifact_root(),
        production_trusted_publishers(),
        Utc::now().date_naive(),
        run_id,
        approval_reference,
        job_hash,
        resume_id,
    )
    .await
}
