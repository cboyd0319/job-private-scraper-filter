use super::evidence_tests::{confirm_field, saved_snapshot_with_text};
use super::*;
use crate::test_support::test_job;
use jobsentinel_documents::{AtsAnalyzer, KeywordImportance, RequirementMatchState};
use jobsentinel_domain::ResumeEvidenceSnapshot;
use jobsentinel_storage::{resume::NewSkill, Database};

async fn context(
    job_id: &str,
    job_description: &str,
    resume_text: &str,
) -> (Database, String, ResumeEvidenceSnapshot) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut job = test_job(job_id, "Security Engineer", "Example");
    job.description = Some(job_description.to_string());
    database.insert_job_if_new(&job).await.unwrap();
    let case_file = create_or_reuse_case_file(&database, job_id).await.unwrap();
    let snapshot = saved_snapshot_with_text(&database, resume_text).await;
    (database, case_file.case_file_id, snapshot)
}

fn resume_id(snapshot: &ResumeEvidenceSnapshot) -> i64 {
    snapshot
        .source_id
        .strip_prefix("resume:")
        .unwrap()
        .parse()
        .unwrap()
}

async fn confirm_analyzed_requirement(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
    resume_text: &str,
    job_description: &str,
    requirement: &str,
) -> RequirementMatchState {
    let skills = database
        .resume_matcher()
        .get_user_skills(resume_id(snapshot))
        .await
        .unwrap()
        .into_iter()
        .map(|skill| skill.skill_name)
        .collect::<Vec<_>>();
    let result = AtsAnalyzer::analyze_text_for_job_with_snapshot(
        resume_text,
        &skills,
        job_description,
        snapshot,
    );
    let review = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == requirement)
        .unwrap();
    for citation in &review.evidence_citations {
        assert_eq!(
            confirm_field(database, case_file_id, snapshot, &citation.field_path).await,
            *citation
        );
    }
    review.match_state
}

#[tokio::test]
async fn requirement_diagnostic_recomputes_supported_states_from_current_inputs() {
    for (job_id, resume_text, expected_state) in [
        (
            "job-diagnostic-direct",
            "Experience\nManaged scheduling for a support team.",
            RequirementMatchState::Direct,
        ),
        (
            "job-diagnostic-strong",
            "Experience\nManaged scheduling.\nImproved scheduling.",
            RequirementMatchState::Strong,
        ),
    ] {
        let job_description = "Required: scheduling";
        let (database, case_file_id, snapshot) =
            context(job_id, job_description, resume_text).await;
        assert_eq!(
            confirm_analyzed_requirement(
                &database,
                &case_file_id,
                &snapshot,
                resume_text,
                job_description,
                "scheduling",
            )
            .await,
            expected_state
        );
        let before_events = database
            .list_case_file_events(&case_file_id)
            .await
            .unwrap()
            .len();
        let diagnostic = prepare_evidence_bound_requirement_diagnostic(
            &database,
            &case_file_id,
            &snapshot,
            "scheduling",
        )
        .await
        .unwrap();
        assert_eq!(diagnostic.case_file_id(), case_file_id);
        assert_eq!(diagnostic.requirement(), "scheduling");
        assert!(!diagnostic.job_revision().is_empty());
        assert_eq!(diagnostic.match_state(), expected_state);
        assert_eq!(diagnostic.importance(), KeywordImportance::Required);
        assert!(!diagnostic.hard_constraint());
        assert!(!diagnostic.evidence_ids().is_empty());
        assert_eq!(diagnostic.why_not(), None);
        assert!(!diagnostic.is_blocking());
        assert_eq!(
            database
                .list_case_file_events(&case_file_id)
                .await
                .unwrap()
                .len(),
            before_events
        );
    }
}

#[tokio::test]
async fn requirement_diagnostic_keeps_partial_implied_and_missing_reasons_bounded() {
    let (database, case_file_id, initial_snapshot) = context(
        "job-diagnostic-partial",
        "Required: CRM",
        "Customer service",
    )
    .await;
    database
        .resume_matcher()
        .add_user_skill(
            resume_id(&initial_snapshot),
            NewSkill {
                skill_name: "CRM".to_string(),
                ..NewSkill::default()
            },
        )
        .await
        .unwrap();
    let snapshot = database
        .resume_matcher()
        .get_resume_evidence_snapshot(resume_id(&initial_snapshot))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        confirm_analyzed_requirement(
            &database,
            &case_file_id,
            &snapshot,
            "Customer service",
            "Required: CRM",
            "crm",
        )
        .await,
        RequirementMatchState::Partial
    );
    let partial =
        prepare_evidence_bound_requirement_diagnostic(&database, &case_file_id, &snapshot, "crm")
            .await
            .unwrap();
    assert_eq!(partial.why_not(), Some(RequirementWhyNot::PartialEvidence));
    assert!(!partial.is_blocking());

    let resume_text = "Experience\nHeld a security clearance.";
    let job_description = "Required: security clearance";
    let (database, case_file_id, snapshot) =
        context("job-diagnostic-implied", job_description, resume_text).await;
    assert_eq!(
        confirm_analyzed_requirement(
            &database,
            &case_file_id,
            &snapshot,
            resume_text,
            job_description,
            "security clearance",
        )
        .await,
        RequirementMatchState::Implied
    );
    let implied = prepare_evidence_bound_requirement_diagnostic(
        &database,
        &case_file_id,
        &snapshot,
        "security clearance",
    )
    .await
    .unwrap();
    assert_eq!(implied.why_not(), Some(RequirementWhyNot::ImpliedEvidence));
    assert!(implied.hard_constraint());
    assert!(implied.is_blocking());

    let (database, case_file_id, snapshot) = context(
        "job-diagnostic-missing",
        "Required: security clearance",
        "Customer service",
    )
    .await;
    let unrelated = confirm_field(&database, &case_file_id, &snapshot, "resume_text.0").await;
    let missing = prepare_evidence_bound_requirement_diagnostic(
        &database,
        &case_file_id,
        &snapshot,
        "security clearance",
    )
    .await
    .unwrap();
    assert_eq!(missing.match_state(), RequirementMatchState::Missing);
    assert_eq!(missing.why_not(), Some(RequirementWhyNot::MissingEvidence));
    assert!(missing.hard_constraint());
    assert!(missing.is_blocking());
    assert!(missing.evidence_ids().is_empty());
    assert!(!missing.evidence_ids().contains(&unrelated.evidence_id));
}

#[tokio::test]
async fn requirement_diagnostic_rejects_unconfirmed_cross_case_stale_and_unknown_inputs() {
    let resume_text = "Experience\nManaged scheduling for a support team.";
    let job_description = "Required: scheduling";
    let (database, case_file_id, snapshot) =
        context("job-diagnostic-guards", job_description, resume_text).await;
    assert_eq!(
        prepare_evidence_bound_requirement_diagnostic(
            &database,
            &case_file_id,
            &snapshot,
            "scheduling",
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );
    confirm_analyzed_requirement(
        &database,
        &case_file_id,
        &snapshot,
        resume_text,
        job_description,
        "scheduling",
    )
    .await;

    let mut other_job = test_job("job-diagnostic-other", "Other", "Example");
    other_job.description = Some(job_description.to_string());
    database.insert_job_if_new(&other_job).await.unwrap();
    let other_case = create_or_reuse_case_file(&database, "job-diagnostic-other")
        .await
        .unwrap();
    assert_eq!(
        prepare_evidence_bound_requirement_diagnostic(
            &database,
            &other_case.case_file_id,
            &snapshot,
            "scheduling",
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );

    database
        .resume_matcher()
        .add_user_skill(
            resume_id(&snapshot),
            NewSkill {
                skill_name: "Case management".to_string(),
                ..NewSkill::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        prepare_evidence_bound_requirement_diagnostic(
            &database,
            &case_file_id,
            &snapshot,
            "scheduling",
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );

    let current = database
        .resume_matcher()
        .get_resume_evidence_snapshot(resume_id(&snapshot))
        .await
        .unwrap()
        .unwrap();
    for requirement in ["", " \n ", "unsafe\nrequirement", "not in this job"] {
        assert_eq!(
            prepare_evidence_bound_requirement_diagnostic(
                &database,
                &case_file_id,
                &current,
                requirement,
            )
            .await
            .err()
            .unwrap(),
            FoundationError::InvalidInput
        );
    }
}
