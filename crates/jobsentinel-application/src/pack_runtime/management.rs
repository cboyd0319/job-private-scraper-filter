//! Maps durable pack lifecycle history into bounded renderer-safe management views.

use anyhow::Result;
use chrono::{DateTime, Utc};
use jobsentinel_domain::v3_manifests::{
    AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackType,
    PrivacyLabel,
};
use jobsentinel_storage::{
    v3_pack_lifecycle::{
        PackAvailability, PackManagementRelease, PackManagementStream, PackQuarantineReason,
        PackReleaseState,
    },
    Database,
};
use serde::Serialize;

use super::PackReviewState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackReleaseReviewState {
    Staged,
    SelfTested,
    Ready,
    Quarantined,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackReviewQuarantineReason {
    SelfTestFailed,
    TrustRevoked,
    Interrupted,
    ArtifactMissing,
    IntegrityFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackPurpose {
    StaticGuidance,
    ResumeEvidenceReview,
    DraftApplicationPacket,
    ReviewedAgent,
    ReviewedWorkflow,
    RoleGuidance,
    RegionalGuidance,
    SourceSupport,
    ReviewRubric,
    SyntheticProductEvaluation,
    JobSearchTemplate,
    OperatingSystemHelper,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManagementReleaseReview {
    pub release_sequence: u64,
    pub pack_version: String,
    pub pack_type: PackType,
    pub execution_class: PackExecutionClass,
    pub state: PackReleaseReviewState,
    pub quarantine_reason: Option<PackReviewQuarantineReason>,
    pub artifact_cleanup_pending: bool,
    pub is_active: bool,
    pub is_rollback: bool,
    pub is_high_water: bool,
    pub publisher_name: String,
    pub license: String,
    pub minimum_app_version: String,
    pub maximum_app_version: String,
    pub payload_bytes: u64,
    pub fixture_summary: String,
    pub purpose: PackPurpose,
    pub model_download_required: bool,
    pub last_self_tested_at: Option<DateTime<Utc>>,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub allowed_data_categories: Vec<DataCategory>,
    pub allowed_task_kinds: Vec<AgentTaskKind>,
    pub allowed_actions: Vec<PackAction>,
    pub approval_gates: Vec<ApprovalGate>,
    pub gateway_policy_id: Option<String>,
    pub external_destinations: Vec<String>,
    pub uses_external_ai: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManagementReview {
    pub publisher_key_id: String,
    pub publisher_public_key_sha256: String,
    pub pack_id: String,
    pub state: PackReviewState,
    pub update_available: bool,
    pub cleanup_pending: bool,
    pub generation: u64,
    pub current_release: PackManagementReleaseReview,
    pub releases: Vec<PackManagementReleaseReview>,
}

pub async fn list_pack_management_reviews(
    database: &Database,
) -> Result<Vec<PackManagementReview>> {
    database
        .list_pack_management()
        .await?
        .into_iter()
        .map(management_review)
        .collect()
}

fn management_review(stream: PackManagementStream) -> Result<PackManagementReview> {
    let high_water = stream
        .releases
        .iter()
        .find(|release| release.release_sequence == stream.high_water_sequence)
        .ok_or_else(|| anyhow::anyhow!("pack management history is invalid"))?;
    let state = match stream.availability {
        PackAvailability::Ready => PackReviewState::Ready,
        PackAvailability::Disabled => PackReviewState::Disabled,
        PackAvailability::Removed => PackReviewState::Removed,
        PackAvailability::Quarantined
            if stream.active_release_sequence.is_none()
                && high_water.lifecycle_state == PackReleaseState::SelfTested
                && high_water.quarantine_reason.is_none() =>
        {
            PackReviewState::NeedsReview
        }
        PackAvailability::Quarantined => PackReviewState::Quarantined,
    };
    let update_available = matches!(
        stream.availability,
        PackAvailability::Ready | PackAvailability::Disabled
    ) && stream.active_release_sequence != Some(stream.high_water_sequence)
        && high_water.lifecycle_state == PackReleaseState::SelfTested
        && high_water.quarantine_reason.is_none();
    let selected_sequence = match stream.availability {
        PackAvailability::Ready | PackAvailability::Disabled => stream
            .active_release_sequence
            .ok_or_else(|| anyhow::anyhow!("pack management state is invalid"))?,
        PackAvailability::Quarantined | PackAvailability::Removed => stream.high_water_sequence,
    };
    let active_sequence = stream.active_release_sequence;
    let rollback_sequence = stream.rollback_release_sequence;
    let high_water_sequence = stream.high_water_sequence;
    let releases = stream
        .releases
        .into_iter()
        .map(|release| {
            release_review(
                release,
                active_sequence,
                rollback_sequence,
                high_water_sequence,
            )
        })
        .collect::<Vec<_>>();
    let current_release = releases
        .iter()
        .find(|release| release.release_sequence == selected_sequence)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("pack management state is invalid"))?;
    Ok(PackManagementReview {
        publisher_key_id: stream.publisher_key_id,
        publisher_public_key_sha256: stream.publisher_public_key_sha256,
        pack_id: stream.pack_id,
        state,
        update_available,
        cleanup_pending: stream.cleanup_pending,
        generation: stream.generation,
        current_release,
        releases,
    })
}

fn release_review(
    release: PackManagementRelease,
    active_sequence: Option<u64>,
    rollback_sequence: Option<u64>,
    high_water_sequence: u64,
) -> PackManagementReleaseReview {
    let purpose = pack_purpose(release.pack_type, &release.allowed_task_kinds);
    PackManagementReleaseReview {
        release_sequence: release.release_sequence,
        pack_version: release.pack_version,
        pack_type: release.pack_type,
        execution_class: release.execution_class,
        state: release_state(release.lifecycle_state),
        quarantine_reason: release.quarantine_reason.map(quarantine_reason),
        artifact_cleanup_pending: release.artifact_cleanup_pending,
        is_active: active_sequence == Some(release.release_sequence),
        is_rollback: rollback_sequence == Some(release.release_sequence),
        is_high_water: high_water_sequence == release.release_sequence,
        publisher_name: release.publisher_name,
        license: release.license,
        minimum_app_version: release.minimum_app_version,
        maximum_app_version: release.maximum_app_version,
        payload_bytes: release.payload_bytes,
        fixture_summary: release.fixture_summary,
        purpose,
        model_download_required: false,
        last_self_tested_at: release.last_self_tested_at,
        privacy_labels: release.privacy_labels,
        allowed_data_categories: release.allowed_data_categories,
        allowed_task_kinds: release.allowed_task_kinds,
        uses_external_ai: release
            .allowed_actions
            .contains(&PackAction::RequestExternalAi),
        allowed_actions: release.allowed_actions,
        approval_gates: release.approval_gates,
        gateway_policy_id: release.gateway_policy_id,
        external_destinations: release.external_destinations,
    }
}

fn pack_purpose(pack_type: PackType, task_kinds: &[AgentTaskKind]) -> PackPurpose {
    match (pack_type, task_kinds) {
        (PackType::Skill, _) => PackPurpose::StaticGuidance,
        (PackType::Agent, [AgentTaskKind::EvidenceReview]) => PackPurpose::ResumeEvidenceReview,
        (PackType::Agent | PackType::Workflow, [AgentTaskKind::DraftPacket]) => {
            PackPurpose::DraftApplicationPacket
        }
        (PackType::Agent, _) => PackPurpose::ReviewedAgent,
        (PackType::Workflow, _) => PackPurpose::ReviewedWorkflow,
        (PackType::Role, _) => PackPurpose::RoleGuidance,
        (PackType::Region, _) => PackPurpose::RegionalGuidance,
        (PackType::Source, _) => PackPurpose::SourceSupport,
        (PackType::Rubric, _) => PackPurpose::ReviewRubric,
        (PackType::Evaluation, _) => PackPurpose::SyntheticProductEvaluation,
        (PackType::Template, _) => PackPurpose::JobSearchTemplate,
        (PackType::OsHelper, _) => PackPurpose::OperatingSystemHelper,
    }
}

const fn release_state(state: PackReleaseState) -> PackReleaseReviewState {
    match state {
        PackReleaseState::Staged => PackReleaseReviewState::Staged,
        PackReleaseState::SelfTested => PackReleaseReviewState::SelfTested,
        PackReleaseState::Ready => PackReleaseReviewState::Ready,
        PackReleaseState::Quarantined => PackReleaseReviewState::Quarantined,
        PackReleaseState::Removed => PackReleaseReviewState::Removed,
    }
}

const fn quarantine_reason(reason: PackQuarantineReason) -> PackReviewQuarantineReason {
    match reason {
        PackQuarantineReason::SelfTestFailed => PackReviewQuarantineReason::SelfTestFailed,
        PackQuarantineReason::TrustRevoked => PackReviewQuarantineReason::TrustRevoked,
        PackQuarantineReason::Interrupted => PackReviewQuarantineReason::Interrupted,
        PackQuarantineReason::ArtifactMissing => PackReviewQuarantineReason::ArtifactMissing,
        PackQuarantineReason::IntegrityFailed => PackReviewQuarantineReason::IntegrityFailed,
    }
}
