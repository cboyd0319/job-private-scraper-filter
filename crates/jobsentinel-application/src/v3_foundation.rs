// Owns application-layer v3 foundation workflows and renderer-safe projections.
use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_foundation::{
        CareerGraphLink, CaseFile, CaseFileEvent, CaseFileEventInput, CaseFileEventKind,
        CompatibilityMetadata, SourceGraphLink, SourcePolicy,
    },
    v3_manifests::PrivacyReceipt,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_consent::{SourceConsentContext, SourceConsentStatus},
    v3_source_manifest::SourceOperation,
};
use jobsentinel_storage::Database;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FoundationError {
    #[error("The v3 foundation input is invalid.")]
    InvalidInput,
    #[error("The reviewed v3 state changed.")]
    Conflict,
    #[error("Local v3 data is unavailable ({0}).")]
    Storage(&'static str),
}

pub async fn create_or_reuse_case_file(
    database: &Database,
    job_hash: &str,
) -> Result<CaseFile, FoundationError> {
    if job_hash.is_empty() || job_hash.len() > 128 {
        return Err(FoundationError::InvalidInput);
    }
    database.ensure_case_file(job_hash).await.map_err(map_error)
}

pub async fn record_case_file_event(
    database: &Database,
    input: &CaseFileEventInput,
) -> Result<CaseFileEvent, FoundationError> {
    input
        .validate()
        .map_err(|_| FoundationError::InvalidInput)?;
    if input.kind == CaseFileEventKind::EvidenceLinked {
        return Err(FoundationError::InvalidInput);
    }
    database
        .append_case_file_event(input)
        .await
        .map_err(map_error)
}

pub async fn add_career_graph_link(
    database: &Database,
    link: &CareerGraphLink,
) -> Result<(), FoundationError> {
    link.validate().map_err(|_| FoundationError::InvalidInput)?;
    database
        .insert_career_graph_link(link)
        .await
        .map_err(map_error)
}

pub async fn add_source_graph_link(
    database: &Database,
    link: &SourceGraphLink,
) -> Result<(), FoundationError> {
    link.validate().map_err(|_| FoundationError::InvalidInput)?;
    database
        .insert_source_graph_link(link)
        .await
        .map_err(map_error)
}

pub async fn record_privacy_receipt(
    database: &Database,
    receipt: &PrivacyReceipt,
) -> Result<(), FoundationError> {
    receipt
        .validate()
        .map_err(|_| FoundationError::InvalidInput)?;
    database
        .store_privacy_receipt(receipt)
        .await
        .map_err(map_error)
}

pub async fn set_source_policy(
    database: &Database,
    policy: &SourcePolicy,
) -> Result<SourcePolicy, FoundationError> {
    policy
        .validate()
        .map_err(|_| FoundationError::InvalidInput)?;
    database
        .upsert_source_policy(policy)
        .await
        .map_err(map_error)
}

pub(crate) async fn review_source_consent(
    database: &Database,
    context: &SourceConsentContext,
) -> Result<SourceConsentStatus, FoundationError> {
    context
        .validate()
        .map_err(|_| FoundationError::InvalidInput)?;
    database
        .source_consent_status(context)
        .await
        .map_err(map_error)
}

pub(crate) async fn revoke_source_consent(
    database: &Database,
    source_id: &str,
) -> Result<bool, FoundationError> {
    database
        .revoke_source_consent(
            source_id,
            jobsentinel_domain::v3_source_consent::SourceConsentOperation::ScheduledCheck,
        )
        .await
        .map_err(map_error)
}

pub async fn read_compatibility_metadata(
    database: &Database,
) -> Result<CompatibilityMetadata, FoundationError> {
    database
        .read_compatibility_metadata()
        .await
        .map_err(map_error)
}

pub async fn authorize_source_action(
    database: &Database,
    source_id: &str,
    operation: SourceOperation,
    today: NaiveDate,
    grant: SourceGrantState,
) -> Result<SourceActionDecision, FoundationError> {
    if source_id.is_empty()
        || source_id.len() > 128
        || !source_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(FoundationError::InvalidInput);
    }
    let policy = database
        .get_source_policy(source_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    let manifest = database
        .get_source_manifest(source_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    manifest
        .authorize(&policy, operation, today, grant)
        .map_err(|_| FoundationError::InvalidInput)
}

pub(crate) fn map_error(error: anyhow::Error) -> FoundationError {
    if let Some(kind) = jobsentinel_storage::v3_source_consent::error_kind(&error) {
        return match kind {
            "invalid" => FoundationError::InvalidInput,
            "conflict" => FoundationError::Conflict,
            kind => FoundationError::Storage(kind),
        };
    }
    match jobsentinel_storage::v3_foundation::error_kind(&error) {
        "invalid" => FoundationError::InvalidInput,
        kind => FoundationError::Storage(kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3_source_governance::{install_usajobs, usajobs_policy};
    use jobsentinel_domain::{
        v3_foundation::{CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin},
        v3_foundation::{SourceAccess, SourcePolicy},
        v3_manifests::{DataCategory, SourceClass},
        v3_source_authorization::{SourceActionDecision, SourceGrantState},
        v3_source_consent::{
            SourceConsentContext, SourceConsentOperation, SourceConsentReviewReason,
            SourceConsentStatus,
        },
        v3_source_manifest::{parse_source_manifest, SourceOperation, SourceStopCondition},
        PrivacyLabel,
    };
    use jobsentinel_storage::Database;

    use crate::test_support::test_job;

    #[tokio::test]
    async fn application_operations_create_and_reuse_a_sanitized_case() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        database
            .insert_job_if_new(&test_job("job-1", "Office Assistant", "Example"))
            .await
            .unwrap();

        let first = create_or_reuse_case_file(&database, "job-1").await.unwrap();
        let second = create_or_reuse_case_file(&database, "job-1").await.unwrap();
        assert_eq!(first.case_file_id, second.case_file_id);

        record_case_file_event(
            &database,
            &CaseFileEventInput {
                case_file_id: first.case_file_id,
                kind: CaseFileEventKind::PrivacyReceiptRecorded,
                origin: EventOrigin::User,
                user_action: true,
                privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
                metadata: EventMetadata::LocalReference {
                    reference_id: "receipt-1".to_string(),
                },
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn application_validation_rejects_mismatched_event_metadata() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let error = record_case_file_event(
            &database,
            &CaseFileEventInput {
                case_file_id: "case-1".to_string(),
                kind: CaseFileEventKind::StatusChanged,
                origin: EventOrigin::User,
                user_action: true,
                privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
                metadata: EventMetadata::Empty,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error, FoundationError::InvalidInput);
    }

    fn consent_context() -> SourceConsentContext {
        SourceConsentContext {
            source_id: "dice".to_string(),
            operation: SourceConsentOperation::ScheduledCheck,
            warning_version: 1,
            behavior_revision: 1,
            policy_ref: "jobsentinel.source-policy.dice".to_string(),
            policy_revision: 1,
            source_class: SourceClass::RestrictedPublicScheduled,
            data_categories: vec![
                DataCategory::PublicJobPosting,
                DataCategory::CareerGoals,
                DataCategory::LocationPreferences,
            ],
            destination_sha256: "a".repeat(64),
            request_sha256: "b".repeat(64),
        }
    }

    #[tokio::test]
    async fn source_consent_operations_fail_closed_and_revoke_locally() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let context = consent_context();
        set_source_policy(
            &database,
            &SourcePolicy {
                source_id: "dice".to_string(),
                source_class: SourceClass::RestrictedPublicScheduled,
                access: SourceAccess::ScheduledPublic,
                request_limit_per_hour: 1,
                user_review_required: true,
                policy_ref: "jobsentinel.source-policy.dice".to_string(),
                revision: 1,
                restriction_reason_code: Some("terms-review".to_string()),
                reviewed_at: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            review_source_consent(&database, &context).await.unwrap(),
            SourceConsentStatus::ReviewRequired {
                reason: SourceConsentReviewReason::Missing,
                latest_event_id: None,
            }
        );
        database.grant_source_consent(&context, None).await.unwrap();
        assert!(matches!(
            review_source_consent(&database, &context).await.unwrap(),
            SourceConsentStatus::Remembered { .. }
        ));
        assert!(revoke_source_consent(&database, "dice").await.unwrap());
        assert_eq!(
            review_source_consent(&database, &context).await.unwrap(),
            SourceConsentStatus::ReviewRequired {
                reason: SourceConsentReviewReason::Revoked,
                latest_event_id: Some(
                    database.source_consent_history("dice", 1).await.unwrap()[0]
                        .event_id
                        .clone()
                ),
            }
        );
    }

    #[tokio::test]
    async fn source_actions_require_a_governed_manifest_and_current_policy() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();

        assert_eq!(
            authorize_source_action(
                &database,
                "synthetic-official-jobs",
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap_err(),
            FoundationError::Conflict
        );
        assert_eq!(
            authorize_source_action(
                &database,
                "private source",
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap_err(),
            FoundationError::InvalidInput
        );

        let mut policy = SourcePolicy {
            source_id: "synthetic-official-jobs".to_string(),
            source_class: SourceClass::OfficialPublicApi,
            access: SourceAccess::ScheduledPublic,
            request_limit_per_hour: 60,
            user_review_required: false,
            policy_ref: "synthetic-official-jobs-v1".to_string(),
            revision: 1,
            restriction_reason_code: None,
            reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-07-19T00:00:00Z")
                .unwrap()
                .to_utc(),
        };
        database.upsert_source_policy(&policy).await.unwrap();
        assert_eq!(
            authorize_source_action(
                &database,
                &policy.source_id,
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap_err(),
            FoundationError::Conflict
        );

        let manifest = parse_source_manifest(
            include_str!("../../jobsentinel-domain/src/fixtures/v3_source_manifest_v1.json"),
            &policy,
        )
        .unwrap();
        database.store_source_manifest(&manifest).await.unwrap();
        assert_eq!(
            authorize_source_action(
                &database,
                &policy.source_id,
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap(),
            SourceActionDecision::Allowed {
                request_limit_per_hour: 60,
                connectivity_required: true,
            }
        );

        policy.revision = 2;
        database.upsert_source_policy(&policy).await.unwrap();
        assert_eq!(
            authorize_source_action(
                &database,
                &policy.source_id,
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap(),
            SourceActionDecision::Blocked(SourceStopCondition::PolicyChanged)
        );
    }

    #[tokio::test]
    async fn usajobs_governance_never_overwrites_same_or_newer_policy_state() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        install_usajobs(&database).await.unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

        let mut newer = usajobs_policy().unwrap();
        newer.revision = 2;
        newer.access = SourceAccess::Disabled;
        newer.request_limit_per_hour = 0;
        database.upsert_source_policy(&newer).await.unwrap();
        install_usajobs(&database).await.unwrap();

        assert_eq!(
            database.get_source_policy("usajobs").await.unwrap(),
            Some(newer)
        );
        assert_eq!(
            authorize_source_action(
                &database,
                "usajobs",
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .await
            .unwrap(),
            SourceActionDecision::Blocked(SourceStopCondition::PolicyChanged)
        );

        let conflicting_database = Database::connect_memory().await.unwrap();
        conflicting_database.migrate().await.unwrap();
        let mut conflicting = usajobs_policy().unwrap();
        conflicting.request_limit_per_hour = 24;
        conflicting_database
            .upsert_source_policy(&conflicting)
            .await
            .unwrap();
        assert_eq!(
            install_usajobs(&conflicting_database).await.unwrap_err(),
            FoundationError::Conflict
        );
    }
}

mod evidence;
pub use evidence::{
    confirm_resume_evidence_for_case, prepare_evidence_bound_packet_claim,
    prepare_evidence_bound_requirement_diagnostic, read_case_file_evidence_links,
    EvidenceBoundPacketClaim, EvidenceBoundRequirementDiagnostic, PacketEvidenceBoundary,
    RequirementWhyNot,
};
mod saved_match_debugger;
pub use saved_match_debugger::{
    confirm_saved_match_debugger_evidence, prepare_saved_match_debugger, SavedMatchDebugger,
    SavedMatchDebuggerEvidence, SavedMatchDebuggerRequirement,
};
mod saved_match_packets;
pub use saved_match_packets::{
    list_saved_match_evidence_packet_claims, save_saved_match_evidence_packet_claim,
    SavedMatchEvidencePacketBoundary, SavedMatchEvidencePacketClaim,
};
mod saved_match_military_transition;
pub use saved_match_military_transition::{
    confirm_pending_saved_match_military_transition_review, confirm_saved_match_military_evidence,
    prepare_pending_saved_match_military_transition_review, PendingMilitaryTransitionReviews,
    SavedMatchMilitaryEvidenceKind, SavedMatchMilitaryTransitionConfirmation,
};
mod military;
pub use jobsentinel_domain::v3_evaluation_inputs::MilitaryBranch;
pub use military::{
    confirm_military_transition_review, prepare_military_transition_review,
    MilitaryOccupationReviewDraft, MilitaryOccupationSuggestion, MilitaryReviewResource,
    MilitarySuggestionBoundary, MilitarySuggestionReviewStatus, MilitaryTransitionWording,
    MilitaryWordingMapping,
};
mod opportunity_case;
pub use opportunity_case::{
    dossier as employer_dossier, open_opportunity_case, OpportunityCaseApplication,
    OpportunityCaseEvidence, OpportunityCaseInterviewSummary, OpportunityCaseJob,
    OpportunityCaseOffer, OpportunityCaseOfferStatus, OpportunityCaseOutcome,
    OpportunityCasePostingRisk, OpportunityCaseSnapshot, OpportunityCaseSource,
    OpportunityCaseTimelineItem, OpportunityCaseTimelineKind,
};
#[cfg(test)]
#[path = "v3_foundation/tests.rs"]
mod v3_foundation_tests;
