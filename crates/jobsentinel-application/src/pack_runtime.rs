//! Owns signed pack staging, lifecycle, recovery, review, and bounded local use.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_manifests::{
        AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackType,
        PrivacyLabel,
    },
    v3_pack_payloads::{parse_and_self_test_pack_payload, SelfTestedPackRelease},
    v3_signed_packs::{TrustedPublisherKey, VerifiedPackRelease},
};
use jobsentinel_storage::{
    v3_pack_lifecycle::{
        PackAvailability, PackReleaseState, PackStageOutcome, PackStream, StoredPackRelease,
    },
    Database,
};
use serde::Serialize;

mod artifact;
mod evaluation;
mod execution;
mod management;
mod production;
mod recovery;
mod trust;

use artifact::{load_tested_artifact, persist_artifact, remove_owned_artifact, ArtifactLoadError};
#[cfg(test)]
pub(crate) use evaluation::{
    evaluate_active_ats_resume_requirement_pack,
    evaluate_active_evidence_reviewer_resume_requirement_pack,
    evaluate_active_static_skill_handoff_pack,
};
pub use evaluation::{
    AtsResumeRequirementEvaluation, EvidenceReviewerResumeRequirementEvaluation,
    StaticSkillHandoffEvaluation,
};
pub use execution::{
    cancel_reviewed_pack_task, DraftApplicationPacket, DraftPacketTaskResult,
    DraftPacketTaskReview, EvidenceReviewTaskResult, PackTaskReview, PackTaskReviewStep,
    StaticSkillHandoff, StaticSkillResource, StaticSkillReview,
};
pub(crate) use execution::{
    execute_draft_packet_task, execute_evidence_review_task, open_active_static_skill,
    prepare_draft_packet_task, prepare_evidence_review_task,
};
pub use management::{
    list_pack_management_reviews, PackManagementReleaseReview, PackManagementReview, PackPurpose,
    PackReleaseReviewState, PackReviewQuarantineReason,
};
pub use production::{
    activate_production_pack_artifact, enable_production_pack_artifact,
    evaluate_production_active_ats_resume_requirement_pack,
    evaluate_production_active_evidence_reviewer_resume_requirement_pack,
    evaluate_production_active_static_skill_handoff_pack, execute_production_draft_packet_task,
    execute_production_evidence_review_task, open_production_active_static_skill,
    prepare_production_draft_packet_task, prepare_production_evidence_review_task,
    rollback_production_pack_artifact, stage_production_pack_artifact,
};
pub(crate) use recovery::reconcile_active_pack_artifacts;
pub use recovery::PackArtifactReconciliation;
pub(crate) use trust::production_trusted_publishers;

pub const MAX_PACK_ARTIFACT_BYTES: usize =
    jobsentinel_domain::v3_signed_packs::MAX_SIGNED_PACK_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRuntimeEnvironment {
    artifact_root: PathBuf,
}

impl PackRuntimeEnvironment {
    #[must_use]
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self {
            artifact_root: data_dir.join("pack-artifacts"),
        }
    }

    #[must_use]
    pub fn artifact_root(&self) -> &Path {
        &self.artifact_root
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackReviewState {
    NeedsReview,
    Ready,
    Disabled,
    Quarantined,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInstallReview {
    pub publisher_key_id: String,
    pub publisher_name: String,
    pub license: String,
    pub pack_id: String,
    pub pack_version: String,
    pub pack_type: PackType,
    pub execution_class: PackExecutionClass,
    pub release_sequence: u64,
    pub minimum_app_version: String,
    pub maximum_app_version: String,
    pub payload_bytes: u64,
    pub fixture_summary: String,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub allowed_data_categories: Vec<DataCategory>,
    pub allowed_task_kinds: Vec<AgentTaskKind>,
    pub allowed_actions: Vec<PackAction>,
    pub approval_gates: Vec<ApprovalGate>,
    pub gateway_policy_id: Option<String>,
    pub external_destinations: Vec<String>,
    pub uses_external_ai: bool,
    pub state: PackReviewState,
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackArtifactRemoval {
    pub generation: u64,
    pub cleanup_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackStateChange {
    pub state: PackReviewState,
    pub generation: u64,
}

pub(crate) async fn stage_pack_artifact(
    database: &Database,
    artifact_root: &Path,
    envelope: &[u8],
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<PackInstallReview> {
    let release = verify_pack(envelope, trusted_publishers)?;
    let publisher = trusted_publishers
        .iter()
        .find(|publisher| publisher.publisher_key_id == release.publisher_key_id())
        .ok_or_else(|| anyhow!("pack publisher is not trusted"))?;
    let staged = match database.stage_verified_pack(&release, publisher).await? {
        PackStageOutcome::Replay(stream) => {
            let state = review_state(stream.availability);
            return Ok(review(&release, stream, state));
        }
        PackStageOutcome::Staged(stream) => stream,
    };
    let tested = match parse_and_self_test_pack_payload(&release, today) {
        Ok(tested) => tested,
        Err(_) => {
            database
                .quarantine_failed_pack_self_test(&release, staged.generation)
                .await?;
            return Err(anyhow!("pack self-test failed"));
        }
    };
    if persist_artifact(artifact_root, &release, envelope).is_err() {
        database
            .quarantine_missing_pack_artifact(&tested, staged.generation)
            .await?;
        return Err(anyhow!("pack artifact could not be stored"));
    }
    let self_tested = database
        .record_pack_self_test(&tested, staged.generation)
        .await?;
    Ok(review(&release, self_tested, PackReviewState::NeedsReview))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn activate_pack_artifact(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    release_sequence: u64,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<PackInstallReview> {
    let stored = database
        .get_stored_pack_release(publisher_key_id, pack_id, release_sequence)
        .await?;
    let (release, tested, publisher) =
        match load_tested_artifact(artifact_root, &stored, trusted_publishers, today) {
            Ok(loaded) => loaded,
            Err(ArtifactLoadError::Missing) => {
                database
                    .quarantine_unavailable_stored_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack artifact is unavailable"));
            }
            Err(ArtifactLoadError::Invalid) => {
                database
                    .quarantine_invalid_stored_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack artifact verification failed"));
            }
            Err(ArtifactLoadError::TrustRevoked) => {
                database
                    .quarantine_trust_revoked_stored_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack publisher trust is revoked"));
            }
        };
    let active = database
        .activate_self_tested_pack(&tested, publisher, expected_generation)
        .await?;
    Ok(review(&release, active, PackReviewState::Ready))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn rollback_pack_artifact(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<PackInstallReview> {
    let stream = database.get_pack_stream(publisher_key_id, pack_id).await?;
    let sequence = stream
        .rollback_release_sequence
        .ok_or_else(|| anyhow!("pack rollback is unavailable"))?;
    let stored = database
        .get_stored_pack_release(publisher_key_id, pack_id, sequence)
        .await?;
    let (release, tested, publisher) =
        match load_tested_artifact(artifact_root, &stored, trusted_publishers, today) {
            Ok(loaded) => loaded,
            Err(ArtifactLoadError::Missing) => {
                database
                    .quarantine_unavailable_rollback_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack rollback artifact is unavailable"));
            }
            Err(ArtifactLoadError::Invalid) => {
                database
                    .quarantine_invalid_rollback_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack rollback artifact is invalid"));
            }
            Err(ArtifactLoadError::TrustRevoked) => {
                database
                    .quarantine_trust_revoked_rollback_pack_artifact(&stored, expected_generation)
                    .await?;
                return Err(anyhow!("pack publisher trust is revoked"));
            }
        };
    let rolled_back = database
        .rollback_pack(&tested, publisher, expected_generation)
        .await?;
    let state = review_state(rolled_back.availability);
    Ok(review(&release, rolled_back, state))
}

pub async fn disable_pack_artifact(
    database: &Database,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<PackStateChange> {
    let disabled = database
        .disable_pack(publisher_key_id, pack_id, expected_generation)
        .await?;
    Ok(state_change(disabled))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn enable_pack_artifact(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<PackStateChange> {
    let stream = database.get_pack_stream(publisher_key_id, pack_id).await?;
    let sequence = stream
        .active_release_sequence
        .ok_or_else(|| anyhow!("pack has no active release"))?;
    let stored = database
        .get_stored_pack_release(publisher_key_id, pack_id, sequence)
        .await?;
    let (_, tested, publisher) = load_active_tested_artifact(
        database,
        artifact_root,
        &stored,
        expected_generation,
        trusted_publishers,
        today,
    )
    .await?;
    let enabled = database
        .enable_pack(&tested, publisher, expected_generation)
        .await?;
    Ok(state_change(enabled))
}

async fn load_active_tested_artifact<'a>(
    database: &Database,
    artifact_root: &Path,
    stored: &StoredPackRelease,
    expected_generation: u64,
    trusted_publishers: &'a [TrustedPublisherKey],
    today: NaiveDate,
) -> Result<(
    VerifiedPackRelease,
    SelfTestedPackRelease,
    &'a TrustedPublisherKey,
)> {
    match load_tested_artifact(artifact_root, stored, trusted_publishers, today) {
        Ok(loaded) => Ok(loaded),
        Err(ArtifactLoadError::Missing) => {
            database
                .quarantine_unavailable_active_pack_artifact(stored, expected_generation)
                .await?;
            Err(anyhow!("pack artifact is unavailable"))
        }
        Err(ArtifactLoadError::Invalid) => {
            database
                .quarantine_invalid_active_pack_artifact(stored, expected_generation)
                .await?;
            Err(anyhow!("pack artifact verification failed"))
        }
        Err(ArtifactLoadError::TrustRevoked) => {
            database
                .quarantine_trust_revoked_active_pack_artifact(stored, expected_generation)
                .await?;
            Err(anyhow!("pack publisher trust is revoked"))
        }
    }
}

pub async fn uninstall_pack_artifacts(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<PackArtifactRemoval> {
    let stream = database.get_pack_stream(publisher_key_id, pack_id).await?;
    let removed = if stream.availability == PackAvailability::Removed
        && stream.generation == expected_generation
    {
        stream
    } else {
        database
            .uninstall_pack(publisher_key_id, pack_id, expected_generation)
            .await?
    };
    let releases = database
        .list_stored_pack_releases(publisher_key_id, pack_id)
        .await?;
    let cleanup_pending = cleanup_removed_pack_artifacts(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        &releases,
    )
    .await?;
    Ok(PackArtifactRemoval {
        generation: removed.generation,
        cleanup_pending,
    })
}

pub async fn retry_pack_artifact_cleanup(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
) -> Result<PackArtifactRemoval> {
    let stream = database.get_pack_stream(publisher_key_id, pack_id).await?;
    if stream.generation != expected_generation {
        return Err(anyhow!("pack changed; refresh before retrying cleanup"));
    }
    let releases = database
        .list_stored_pack_releases(publisher_key_id, pack_id)
        .await?;
    let cleanup_pending = cleanup_removed_pack_artifacts(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        &releases,
    )
    .await?;
    Ok(PackArtifactRemoval {
        generation: stream.generation,
        cleanup_pending,
    })
}

async fn cleanup_removed_pack_artifacts(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    releases: &[StoredPackRelease],
) -> Result<bool> {
    let mut completion_error = None;
    for release in releases.iter().filter(|release| {
        release.lifecycle_state == PackReleaseState::Removed && release.artifact_cleanup_pending
    }) {
        if remove_owned_artifact(artifact_root, release).is_ok() {
            if let Err(error) = database.complete_pack_release_cleanup(release).await {
                completion_error.get_or_insert(error);
            }
        }
    }
    if let Some(error) = completion_error {
        return Err(error);
    }
    Ok(database
        .list_stored_pack_releases(publisher_key_id, pack_id)
        .await?
        .iter()
        .any(|release| release.artifact_cleanup_pending))
}

fn review(
    release: &VerifiedPackRelease,
    stream: PackStream,
    state: PackReviewState,
) -> PackInstallReview {
    let manifest = release.manifest();
    PackInstallReview {
        publisher_key_id: release.publisher_key_id().to_string(),
        publisher_name: release.publisher_name().to_string(),
        license: release.license().to_string(),
        pack_id: manifest.pack_id.clone(),
        pack_version: release.pack_version().to_string(),
        pack_type: manifest.pack_type,
        execution_class: manifest.execution_class,
        release_sequence: release.release_sequence(),
        minimum_app_version: release.minimum_app_version().to_string(),
        maximum_app_version: release.maximum_app_version().to_string(),
        payload_bytes: release.payload_bytes(),
        fixture_summary: release.fixture_summary().to_string(),
        privacy_labels: manifest.privacy_labels.clone(),
        allowed_data_categories: manifest.allowed_data_categories.clone(),
        allowed_task_kinds: manifest.allowed_task_kinds.clone(),
        allowed_actions: manifest.allowed_actions.clone(),
        approval_gates: manifest.approval_gates.clone(),
        gateway_policy_id: manifest.gateway_policy_id.clone(),
        external_destinations: release.external_destinations().to_vec(),
        uses_external_ai: manifest
            .allowed_actions
            .contains(&PackAction::RequestExternalAi),
        state,
        generation: stream.generation,
    }
}

const fn review_state(availability: PackAvailability) -> PackReviewState {
    match availability {
        PackAvailability::Ready => PackReviewState::Ready,
        PackAvailability::Disabled => PackReviewState::Disabled,
        PackAvailability::Quarantined => PackReviewState::Quarantined,
        PackAvailability::Removed => PackReviewState::Removed,
    }
}

fn state_change(stream: PackStream) -> PackStateChange {
    PackStateChange {
        state: review_state(stream.availability),
        generation: stream.generation,
    }
}

fn verify_pack(
    envelope: &[u8],
    trusted_publishers: &[TrustedPublisherKey],
) -> Result<VerifiedPackRelease> {
    jobsentinel_domain::v3_signed_packs::parse_signed_pack_release(envelope, trusted_publishers)
        .map_err(|_| anyhow!("pack verification failed"))
}
