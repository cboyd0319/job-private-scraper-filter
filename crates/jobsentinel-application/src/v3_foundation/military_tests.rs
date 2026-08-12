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
use uuid::Uuid;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 19).unwrap()
}

fn user_reference(
    case_file_id: &str,
    reference_id: &str,
    kind: CaseFileEventKind,
) -> CaseFileEventInput {
    CaseFileEventInput {
        case_file_id: case_file_id.to_string(),
        kind,
        origin: EventOrigin::User,
        user_action: true,
        privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        metadata: EventMetadata::LocalReference {
            reference_id: reference_id.to_string(),
        },
    }
}

async fn saved_snapshot(database: &Database, name: &str) -> ResumeEvidenceSnapshot {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(format!("{name}.txt"));
    std::fs::write(&path, "Army 25B user-authored military duties").unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume(name, path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap()
}

async fn confirmed_context() -> (
    Database,
    String,
    ResumeEvidenceSnapshot,
    ResumeEvidenceCitation,
) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-military", "Support Specialist", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-military")
        .await
        .unwrap();
    let snapshot = saved_snapshot(&database, "Military Resume").await;
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "military_info").unwrap();
    confirm_resume_evidence_for_case(
        &database,
        &snapshot,
        &citation,
        &user_reference(
            &case_file.case_file_id,
            &citation.evidence_id,
            CaseFileEventKind::EvidenceLinked,
        ),
    )
    .await
    .unwrap();
    (database, case_file.case_file_id, snapshot, citation)
}

async fn review(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
    citation: &ResumeEvidenceCitation,
) -> MilitaryOccupationReviewDraft {
    prepare_military_transition_review(
        database,
        case_file_id,
        MilitaryBranch::Army,
        transition_wording("25B", "Technical support specialist"),
        snapshot,
        citation,
        None,
        today(),
    )
    .await
    .unwrap()
}

fn transition_wording(
    occupation_code: impl Into<String>,
    civilian_role: impl Into<String>,
) -> MilitaryTransitionWording {
    MilitaryTransitionWording {
        occupation_code: occupation_code.into(),
        civilian_role: civilian_role.into(),
        responsibility_mappings: Vec::new(),
        credential_mappings: Vec::new(),
        current_clearance: None,
    }
}

async fn confirm(
    database: &Database,
    case_file_id: &str,
    review: MilitaryOccupationReviewDraft,
) -> Result<MilitaryOccupationSuggestion, FoundationError> {
    let receipt = user_reference(
        case_file_id,
        review.review_id(),
        CaseFileEventKind::PrivacyReceiptRecorded,
    );
    confirm_military_transition_review(database, review, &receipt).await
}

#[tokio::test]
async fn exact_review_draft_becomes_a_user_reviewed_suggestion_without_writes() {
    let (database, case_file_id, snapshot, citation) = confirmed_context().await;
    let before_links = read_case_file_evidence_links(&database, &case_file_id)
        .await
        .unwrap()
        .len();
    let before_events = database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .len();
    let review = review(&database, &case_file_id, &snapshot, &citation).await;

    assert_eq!(
        review.boundary(),
        MilitarySuggestionBoundary::AwaitingUserReview
    );
    assert_eq!(review.branch(), MilitaryBranch::Army);
    assert_eq!(review.occupation_code(), "25B");
    assert_eq!(review.civilian_role(), "Technical support specialist");
    assert_eq!(review.evidence_id(), citation.evidence_id);
    assert!(review.credential_review_resource().is_none());
    assert_eq!(review.user_confirmed_current_clearance(), None);
    assert_eq!(
        review.occupation_review_resource().intended_use(),
        VeteranResourceUse::OccupationCrosswalk
    );
    assert_eq!(
        review.occupation_review_resource().runtime_access(),
        VeteranResourceAccess::ManualReviewOnly
    );

    let suggestion = confirm(&database, &case_file_id, review).await.unwrap();
    assert_eq!(suggestion.case_file_id(), case_file_id);
    assert_eq!(suggestion.branch(), MilitaryBranch::Army);
    assert_eq!(suggestion.occupation_code(), "25B");
    assert_eq!(suggestion.civilian_role(), "Technical support specialist");
    assert_eq!(suggestion.evidence_id(), citation.evidence_id);
    assert_eq!(suggestion.clearance_evidence_id(), None);
    assert_eq!(
        suggestion.review_status(),
        MilitarySuggestionReviewStatus::UserReviewed
    );
    assert_eq!(
        suggestion.boundary(),
        MilitarySuggestionBoundary::SuggestionOnly
    );
    assert_eq!(
        suggestion.occupation_review_resource().resource_id(),
        "onet-military-crosswalk"
    );
    assert_eq!(
        suggestion.occupation_review_resource().display_name(),
        "O*NET Military Crosswalk"
    );
    assert_eq!(
        suggestion.occupation_review_resource().url(),
        "https://www.onetcenter.org/crosswalks.html"
    );
    assert_eq!(
        suggestion.occupation_review_resource().reviewed_on(),
        today()
    );
    assert_eq!(
        suggestion.occupation_review_resource().source_release(),
        Some("August 2024")
    );
    assert_eq!(
        read_case_file_evidence_links(&database, &case_file_id)
            .await
            .unwrap()
            .len(),
        before_links
    );
    assert_eq!(
        database
            .list_case_file_events(&case_file_id)
            .await
            .unwrap()
            .len(),
        before_events
    );
}

#[tokio::test]
async fn review_draft_rejects_unsafe_text_and_evidence_substitution() {
    let database = Database::connect_memory().await.unwrap();
    let snapshot = ResumeEvidenceSnapshot {
        source_id: "resume:1".to_string(),
        revision: "revision".to_string(),
    };
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "military_info").unwrap();
    for (code, role) in [
        ("".to_string(), "Support specialist".to_string()),
        (" 25B".to_string(), "Support specialist".to_string()),
        ("25B".to_string(), "Technical\nsupport".to_string()),
        ("X".repeat(33), "Support specialist".to_string()),
        ("25B".to_string(), "X".repeat(257)),
    ] {
        assert_eq!(
            prepare_military_transition_review(
                &database,
                "case-1",
                MilitaryBranch::Army,
                transition_wording(code, role),
                &snapshot,
                &citation,
                None,
                today(),
            )
            .await
            .err()
            .unwrap(),
            FoundationError::InvalidInput
        );
    }

    let mut tampered = citation.clone();
    let replacement = if tampered.evidence_id.starts_with('0') {
        "1"
    } else {
        "0"
    };
    tampered.evidence_id.replace_range(..1, replacement);
    let clearance = ResumeEvidenceCitation::for_field(&snapshot, "clearance").unwrap();
    let draft = ResumeEvidenceSnapshot {
        source_id: "resume-draft:1".to_string(),
        revision: snapshot.revision.clone(),
    };
    let draft_citation = ResumeEvidenceCitation::for_field(&draft, "military_info").unwrap();
    for (invalid_snapshot, invalid_citation) in [
        (&snapshot, &tampered),
        (&snapshot, &clearance),
        (&draft, &draft_citation),
    ] {
        assert_eq!(
            prepare_military_transition_review(
                &database,
                "case-1",
                MilitaryBranch::Army,
                transition_wording("25B", "Support specialist"),
                invalid_snapshot,
                invalid_citation,
                None,
                today(),
            )
            .await
            .err()
            .unwrap(),
            FoundationError::InvalidInput
        );
    }
}

#[tokio::test]
async fn confirmation_requires_exact_user_receipt_and_current_confirmed_evidence() {
    let (database, case_file_id, snapshot, citation) = confirmed_context().await;
    let wrong_review = review(&database, &case_file_id, &snapshot, &citation).await;
    let wrong_receipt = user_reference(
        &case_file_id,
        &Uuid::new_v4().to_string(),
        CaseFileEventKind::PrivacyReceiptRecorded,
    );
    assert_eq!(
        confirm_military_transition_review(&database, wrong_review, &wrong_receipt)
            .await
            .err()
            .unwrap(),
        FoundationError::InvalidInput
    );

    let system_review = review(&database, &case_file_id, &snapshot, &citation).await;
    let mut system_receipt = user_reference(
        &case_file_id,
        system_review.review_id(),
        CaseFileEventKind::PrivacyReceiptRecorded,
    );
    system_receipt.origin = EventOrigin::System;
    system_receipt.user_action = false;
    assert_eq!(
        confirm_military_transition_review(&database, system_review, &system_receipt)
            .await
            .err()
            .unwrap(),
        FoundationError::InvalidInput
    );

    let stale = ResumeEvidenceSnapshot {
        source_id: snapshot.source_id.clone(),
        revision: "stale-revision".to_string(),
    };
    let stale_citation = ResumeEvidenceCitation::for_field(&stale, "military_info").unwrap();
    assert_eq!(
        prepare_military_transition_review(
            &database,
            &case_file_id,
            MilitaryBranch::Army,
            transition_wording("25B", "Support specialist"),
            &stale,
            &stale_citation,
            None,
            today(),
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );

    let unconfirmed_snapshot = saved_snapshot(&database, "Unconfirmed Resume").await;
    let unconfirmed =
        ResumeEvidenceCitation::for_field(&unconfirmed_snapshot, "military_info").unwrap();
    assert_eq!(
        prepare_military_transition_review(
            &database,
            &case_file_id,
            MilitaryBranch::Army,
            transition_wording("25B", "Support specialist"),
            &unconfirmed_snapshot,
            &unconfirmed,
            None,
            today(),
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn preparation_rejects_evidence_confirmed_for_another_case() {
    let (database, _, snapshot, citation) = confirmed_context().await;
    database
        .insert_job_if_new(&test_job("job-other", "Technician", "Example"))
        .await
        .unwrap();
    let other_case = create_or_reuse_case_file(&database, "job-other")
        .await
        .unwrap();
    assert_eq!(
        prepare_military_transition_review(
            &database,
            &other_case.case_file_id,
            MilitaryBranch::Army,
            transition_wording("25B", "Support specialist"),
            &snapshot,
            &citation,
            None,
            today(),
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );
}
