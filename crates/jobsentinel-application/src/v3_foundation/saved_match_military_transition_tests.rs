use super::saved_match_military_transition::{
    confirm_saved_match_military_transition_review, prepare_saved_match_military_transition_review,
};
use super::*;
use crate::test_support::test_job;
use chrono::{Duration, Utc};
use jobsentinel_storage::{resume::NewSkill, Database};

const RESUME_TEXT: &str = "Army 25B\nConfigured tactical networks\nResolved service incidents\nCompTIA Security+\nCurrent Secret clearance";

fn wording() -> MilitaryTransitionWording {
    MilitaryTransitionWording {
        occupation_code: "25B".to_string(),
        civilian_role: "Technical support specialist".to_string(),
        responsibility_mappings: vec![
            MilitaryWordingMapping {
                military_evidence: "Configured tactical networks".to_string(),
                civilian_wording: "Maintained secure network services".to_string(),
            },
            MilitaryWordingMapping {
                military_evidence: "Resolved service incidents".to_string(),
                civilian_wording: "Resolved user support incidents".to_string(),
            },
        ],
        credential_mappings: vec![MilitaryWordingMapping {
            military_evidence: "CompTIA Security+".to_string(),
            civilian_wording: "CompTIA Security+".to_string(),
        }],
        current_clearance: Some("Secret".to_string()),
    }
}

async fn saved_military_match(name: &str) -> (Database, String, i64) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut job = test_job(name, "Support Specialist", "Example");
    job.description = Some("Required: network support".to_string());
    database.insert_job_if_new(&job).await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("military.txt");
    std::fs::write(&path, RESUME_TEXT).unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Military Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .match_resume_to_job(resume_id, &job.hash)
        .await
        .unwrap();
    (database, job.hash, resume_id)
}

async fn confirm_transition_evidence(database: &Database, job_hash: &str, resume_id: i64) {
    assert!(confirm_saved_match_military_evidence(
        database,
        job_hash,
        resume_id,
        SavedMatchMilitaryEvidenceKind::MilitaryService,
    )
    .await
    .unwrap());
    assert!(confirm_saved_match_military_evidence(
        database,
        job_hash,
        resume_id,
        SavedMatchMilitaryEvidenceKind::CurrentClearance,
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn saved_match_transition_requires_a_current_saved_match_at_prepare() {
    let (database, _, resume_id) = saved_military_match("saved-military-current-match").await;
    let mut unmatched = test_job("saved-military-unmatched", "Support Specialist", "Example");
    unmatched.description = Some("Required: network support".to_string());
    database.insert_job_if_new(&unmatched).await.unwrap();

    assert_eq!(
        prepare_pending_saved_match_military_transition_review(
            &database,
            &PendingMilitaryTransitionReviews::default(),
            &unmatched.hash,
            resume_id,
            MilitaryBranch::Army,
            wording(),
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn saved_match_transition_requires_exact_closed_military_confirmations_without_prepare_writes(
) {
    let (database, job_hash, resume_id) = saved_military_match("saved-military-required").await;
    let pending = PendingMilitaryTransitionReviews::default();
    assert_eq!(
        prepare_pending_saved_match_military_transition_review(
            &database,
            &pending,
            &job_hash,
            resume_id,
            MilitaryBranch::Army,
            wording(),
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );

    confirm_transition_evidence(&database, &job_hash, resume_id).await;
    let confirmed_before = database
        .read_saved_match_confirmed_evidence(&job_hash, resume_id)
        .await
        .unwrap();
    let case_file_id = confirmed_before.case_file_id().unwrap().to_string();
    let before = confirmed_before.evidence_ids().to_vec();
    let before_events = database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .len();
    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    let after = database
        .read_saved_match_confirmed_evidence(&job_hash, resume_id)
        .await
        .unwrap()
        .evidence_ids()
        .to_vec();
    assert_eq!(before, after);
    assert_eq!(
        before_events,
        database
            .list_case_file_events(&case_file_id)
            .await
            .unwrap()
            .len()
    );
    assert_eq!(pending.current_count(Utc::now()), 1);
    assert!(uuid::Uuid::parse_str(&token).is_ok());
}

#[tokio::test]
async fn saved_match_transition_consumes_one_safe_opaque_token_without_source_or_evidence_ids() {
    let (database, job_hash, resume_id) = saved_military_match("saved-military-safe").await;
    confirm_transition_evidence(&database, &job_hash, resume_id).await;
    let pending = PendingMilitaryTransitionReviews::default();
    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    let confirmation =
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap();
    assert_eq!(confirmation.civilian_role(), "Technical support specialist");
    assert_eq!(confirmation.civilian_responsibilities().len(), 2);
    assert_eq!(
        confirmation.user_confirmed_current_clearance(),
        Some("Secret")
    );
    let serialized = serde_json::to_value(&confirmation).unwrap();
    assert_eq!(
        serialized,
        serde_json::json!({
            "civilian_role": "Technical support specialist",
            "civilian_responsibilities": [
                "Maintained secure network services",
                "Resolved user support incidents",
            ],
            "credential_wording": ["CompTIA Security+"],
            "user_confirmed_current_clearance": "Secret",
            "boundary": "suggestion_only",
            "clearance_currentness": "not_verified",
            "military_civilian_equivalence": "not_verified",
        })
    );
    let serialized = serialized.to_string();
    for forbidden in [
        "case_file_id",
        "evidence_id",
        "resource",
        "occupation_code",
        "review_id",
        "military_evidence",
        "Configured tactical networks",
        "Army 25B",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "must not serialize {forbidden}"
        );
    }
    assert_eq!(
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn pending_saved_match_transition_replaces_expires_and_caps_move_only_reviews() {
    let (database, job_hash, resume_id) = saved_military_match("saved-military-pending").await;
    confirm_transition_evidence(&database, &job_hash, resume_id).await;
    let pending = PendingMilitaryTransitionReviews::default();
    let now = Utc::now();
    let first = prepare_saved_match_military_transition_review(
        &database,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    let first_token = pending.queue(first, now);
    let replacement = prepare_saved_match_military_transition_review(
        &database,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    let replacement_token = pending.queue(replacement, now + Duration::minutes(1));
    assert!(pending
        .take(&first_token, now + Duration::minutes(1))
        .is_none());
    assert!(pending
        .take(&replacement_token, now + Duration::minutes(31))
        .is_none());
    assert_eq!(PendingMilitaryTransitionReviews::capacity(), 20);

    let mut tokens = Vec::new();
    for index in 0..=PendingMilitaryTransitionReviews::capacity() {
        let name = format!("saved-military-cap-{index}");
        let (database, job_hash, resume_id) = saved_military_match(&name).await;
        confirm_transition_evidence(&database, &job_hash, resume_id).await;
        let review = prepare_saved_match_military_transition_review(
            &database,
            &job_hash,
            resume_id,
            MilitaryBranch::Army,
            wording(),
        )
        .await
        .unwrap();
        tokens.push(pending.queue(review, now + Duration::minutes(32)));
    }
    assert_eq!(
        pending.current_count(now + Duration::minutes(32)),
        PendingMilitaryTransitionReviews::capacity()
    );
    assert!(pending
        .take(&tokens[0], now + Duration::minutes(32))
        .is_none());
}

#[tokio::test]
async fn saved_match_transition_checks_saved_context_at_prepare_but_confirms_only_resume_evidence()
{
    let (database, job_hash, resume_id) = saved_military_match("saved-military-stale").await;
    confirm_transition_evidence(&database, &job_hash, resume_id).await;
    let wrong_review = prepare_saved_match_military_transition_review(
        &database,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    assert!(matches!(
        confirm_saved_match_military_transition_review(&database, wrong_review, "wrong-review-id")
            .await,
        Err(FoundationError::Conflict)
    ));

    let pending = PendingMilitaryTransitionReviews::default();
    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    let mut changed_job = database.get_job_by_hash(&job_hash).await.unwrap().unwrap();
    changed_job.description = Some("Changed after review".to_string());
    database.upsert_job(&changed_job).await.unwrap();
    assert_eq!(
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap()
            .civilian_role(),
        "Technical support specialist"
    );

    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    database
        .resume_matcher()
        .add_user_skill(
            resume_id,
            NewSkill {
                skill_name: "Changed after review".to_string(),
                ..NewSkill::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
    assert_eq!(
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn saved_match_transition_fails_closed_when_its_reviewed_resume_is_deleted() {
    let (database, job_hash, resume_id) = saved_military_match("saved-military-deleted").await;
    confirm_transition_evidence(&database, &job_hash, resume_id).await;
    let pending = PendingMilitaryTransitionReviews::default();
    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job_hash,
        resume_id,
        MilitaryBranch::Army,
        wording(),
    )
    .await
    .unwrap();
    database
        .resume_matcher()
        .delete_resume(resume_id)
        .await
        .unwrap();

    assert_eq!(
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
}
