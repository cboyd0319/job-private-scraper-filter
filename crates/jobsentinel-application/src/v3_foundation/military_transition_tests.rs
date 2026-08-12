// Verifies evidence-bound military-to-civilian wording review and confirmation.

use super::*;
use crate::test_support::test_job;
use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_evaluation_inputs::MilitaryBranch,
    v3_foundation::{CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin},
    v3_manifests::PrivacyLabel,
    v3_veteran_public_service::{VeteranResourceAccess, VeteranResourceUse},
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};
use jobsentinel_storage::Database;

const RESUME_TEXT: &str = "Army 25B\nConfigured tactical networks\nResolved service incidents\nCompTIA Security+\nCurrent Secret clearance";

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 19).unwrap()
}

fn user_reference(case_file_id: &str, reference_id: &str) -> CaseFileEventInput {
    CaseFileEventInput {
        case_file_id: case_file_id.to_string(),
        kind: CaseFileEventKind::EvidenceLinked,
        origin: EventOrigin::User,
        user_action: true,
        privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        metadata: EventMetadata::LocalReference {
            reference_id: reference_id.to_string(),
        },
    }
}

fn review_receipt(case_file_id: &str, review_id: &str) -> CaseFileEventInput {
    let mut receipt = user_reference(case_file_id, review_id);
    receipt.kind = CaseFileEventKind::PrivacyReceiptRecorded;
    receipt
}

fn mapping(military_evidence: &str, civilian_wording: &str) -> MilitaryWordingMapping {
    MilitaryWordingMapping {
        military_evidence: military_evidence.to_string(),
        civilian_wording: civilian_wording.to_string(),
    }
}

fn transition_wording() -> MilitaryTransitionWording {
    MilitaryTransitionWording {
        occupation_code: "25B".to_string(),
        civilian_role: "Technical support specialist".to_string(),
        responsibility_mappings: vec![
            mapping(
                "Configured tactical networks",
                "Maintained secure network services",
            ),
            mapping(
                "Resolved service incidents",
                "Resolved user support incidents",
            ),
        ],
        credential_mappings: vec![mapping("CompTIA Security+", "CompTIA Security+")],
        current_clearance: Some("Secret".to_string()),
    }
}

async fn confirmed_context(
    resume_text: &str,
) -> (
    Database,
    String,
    i64,
    ResumeEvidenceSnapshot,
    ResumeEvidenceCitation,
) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-transition", "Support Specialist", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-transition")
        .await
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("military.txt");
    std::fs::write(&path, resume_text).unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Military Resume", path.to_str().unwrap())
        .await
        .unwrap();
    let snapshot = database
        .resume_matcher()
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap();
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "military_info").unwrap();
    confirm_resume_evidence_for_case(
        &database,
        &snapshot,
        &citation,
        &user_reference(&case_file.case_file_id, &citation.evidence_id),
    )
    .await
    .unwrap();
    (
        database,
        case_file.case_file_id,
        resume_id,
        snapshot,
        citation,
    )
}

async fn confirm_clearance(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
) -> ResumeEvidenceCitation {
    let citation = ResumeEvidenceCitation::for_field(snapshot, "clearance").unwrap();
    confirm_resume_evidence_for_case(
        database,
        snapshot,
        &citation,
        &user_reference(case_file_id, &citation.evidence_id),
    )
    .await
    .unwrap();
    citation
}

async fn prepared_transition() -> (Database, String, i64, MilitaryOccupationReviewDraft) {
    let (database, case_file_id, resume_id, snapshot, military_citation) =
        confirmed_context(RESUME_TEXT).await;
    let clearance_citation = confirm_clearance(&database, &case_file_id, &snapshot).await;
    let review = prepare_military_transition_review(
        &database,
        &case_file_id,
        MilitaryBranch::Army,
        transition_wording(),
        &snapshot,
        &military_citation,
        Some(&clearance_citation),
        today(),
    )
    .await
    .unwrap();
    (database, case_file_id, resume_id, review)
}

async fn assert_transition_preparation_conflict(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
    military_citation: &ResumeEvidenceCitation,
    clearance_citation: &ResumeEvidenceCitation,
) {
    assert_eq!(
        prepare_military_transition_review(
            database,
            case_file_id,
            MilitaryBranch::Army,
            transition_wording(),
            snapshot,
            military_citation,
            Some(clearance_citation),
            today(),
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn reviewed_wording_preserves_exact_backed_transition_claims_without_writes() {
    let (database, case_file_id, _, snapshot, military_citation) =
        confirmed_context(RESUME_TEXT).await;
    let clearance_citation = confirm_clearance(&database, &case_file_id, &snapshot).await;
    let before_links = read_case_file_evidence_links(&database, &case_file_id)
        .await
        .unwrap()
        .len();
    let before_events = database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .len();
    let review = prepare_military_transition_review(
        &database,
        &case_file_id,
        MilitaryBranch::Army,
        transition_wording(),
        &snapshot,
        &military_citation,
        Some(&clearance_citation),
        today(),
    )
    .await
    .unwrap();

    assert_eq!(
        review.responsibility_mappings()[0].military_evidence,
        "Configured tactical networks"
    );
    assert_eq!(
        review.responsibility_mappings()[0].civilian_wording,
        "Maintained secure network services"
    );
    assert_eq!(
        review.credential_mappings()[0].military_evidence,
        "CompTIA Security+"
    );
    assert_eq!(review.user_confirmed_current_clearance(), Some("Secret"));
    assert_eq!(
        (
            review.occupation_review_resource().resource_id(),
            review
                .credential_review_resource()
                .expect("credential review resource")
                .resource_id(),
        ),
        ("onet-military-crosswalk", "dod-cool-military-occupations")
    );
    assert_eq!(
        (
            review.credential_review_resource().unwrap().intended_use(),
            review
                .credential_review_resource()
                .unwrap()
                .runtime_access(),
        ),
        (
            VeteranResourceUse::CredentialResearch,
            VeteranResourceAccess::ManualReviewOnly
        )
    );

    let receipt = review_receipt(&case_file_id, review.review_id());
    let suggestion = confirm_military_transition_review(&database, review, &receipt)
        .await
        .unwrap();
    assert_eq!(
        suggestion.civilian_responsibilities(),
        [
            "Maintained secure network services",
            "Resolved user support incidents"
        ]
    );
    assert_eq!(suggestion.credential_wording(), ["CompTIA Security+"]);
    assert_eq!(
        suggestion.user_confirmed_current_clearance(),
        Some("Secret")
    );
    assert_eq!(
        suggestion.clearance_evidence_id(),
        Some(clearance_citation.evidence_id.as_str())
    );
    assert_eq!(
        (
            read_case_file_evidence_links(&database, &case_file_id)
                .await
                .unwrap()
                .len(),
            database
                .list_case_file_events(&case_file_id)
                .await
                .unwrap()
                .len(),
        ),
        (before_links, before_events)
    );
}

#[tokio::test]
async fn preparation_rejects_unbacked_or_unconfirmed_transition_claims() {
    let (database, case_file_id, _, snapshot, military_citation) =
        confirmed_context(
            "Army 25B\nConfigured tactical networks\nResolved service incidents\nCompTIA SecurityPlus\nSecretary",
        )
        .await;
    let clearance_citation = confirm_clearance(&database, &case_file_id, &snapshot).await;
    assert_transition_preparation_conflict(
        &database,
        &case_file_id,
        &snapshot,
        &military_citation,
        &clearance_citation,
    )
    .await;

    let (database, case_file_id, _, snapshot, military_citation) =
        confirmed_context(RESUME_TEXT).await;
    let clearance_citation = ResumeEvidenceCitation::for_field(&snapshot, "clearance").unwrap();
    assert_transition_preparation_conflict(
        &database,
        &case_file_id,
        &snapshot,
        &military_citation,
        &clearance_citation,
    )
    .await;
}

#[tokio::test]
async fn confirmation_rejects_changed_or_deleted_saved_evidence() {
    let (database, case_file_id, resume_id, review) = prepared_transition().await;
    database
        .resume_matcher()
        .extract_skills(resume_id)
        .await
        .unwrap();
    let receipt = review_receipt(&case_file_id, review.review_id());
    assert_eq!(
        confirm_military_transition_review(&database, review, &receipt)
            .await
            .err()
            .unwrap(),
        FoundationError::Conflict
    );

    let (database, case_file_id, resume_id, review) = prepared_transition().await;
    database
        .resume_matcher()
        .delete_resume(resume_id)
        .await
        .unwrap();
    let receipt = review_receipt(&case_file_id, review.review_id());
    assert_eq!(
        confirm_military_transition_review(&database, review, &receipt)
            .await
            .err()
            .unwrap(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn transition_wording_is_bounded_and_clearance_cannot_be_substituted() {
    let (database, case_file_id, _, snapshot, military_citation) =
        confirmed_context(RESUME_TEXT).await;
    let mut oversized = transition_wording();
    oversized.responsibility_mappings = (0..17)
        .map(|_| mapping("Configured tactical networks", "Reviewed duty"))
        .collect();
    for (wording, clearance) in [
        (oversized, Some(&military_citation)),
        (
            MilitaryTransitionWording {
                credential_mappings: vec![mapping("CompTIA Security+", "Unsafe\ncredential")],
                ..transition_wording()
            },
            Some(&military_citation),
        ),
        (
            MilitaryTransitionWording {
                current_clearance: None,
                ..transition_wording()
            },
            Some(&military_citation),
        ),
    ] {
        assert_eq!(
            prepare_military_transition_review(
                &database,
                &case_file_id,
                MilitaryBranch::Army,
                wording,
                &snapshot,
                &military_citation,
                clearance,
                today(),
            )
            .await
            .err()
            .unwrap(),
            FoundationError::InvalidInput
        );
    }
}
