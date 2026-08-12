// Verifies saved-match debugger evidence identity, confirmation, and stale-input rejection.

use super::*;
use crate::test_support::test_job;
use jobsentinel_documents::{RequirementMatchState, ResumeMatchFeedbackLabel};
use jobsentinel_storage::{resume::NewSkill, Database};

fn scheduling_evidence_id(debugger: &SavedMatchDebugger) -> &str {
    debugger
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap()
        .evidence()[0]
        .evidence_id()
}

pub(super) async fn saved_match_context() -> (Database, String, i64) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut job = test_job("saved-match-debugger", "Office Assistant", "Example");
    job.description = Some("Required: scheduling\nPreferred: CRM".to_string());
    database.insert_job_if_new(&job).await.unwrap();

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resume.txt");
    std::fs::write(
        &path,
        "Experience\nManaged scheduling for a support team.\nImproved scheduling.",
    )
    .unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .match_resume_to_job(resume_id, &job.hash)
        .await
        .unwrap();

    (database, job.hash, resume_id)
}

#[tokio::test]
async fn saved_match_debugger_marks_each_current_citation_confirmed_after_reload() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let first = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let scheduling = first
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap();
    assert_eq!(scheduling.evidence().len(), 2);
    assert!(scheduling
        .evidence()
        .iter()
        .all(|evidence| !evidence.confirmed()));
    let evidence_id = scheduling.evidence()[0].evidence_id().to_string();

    assert!(confirm_saved_match_debugger_evidence(
        &database,
        &job_hash,
        resume_id,
        first.debugger_id(),
        &evidence_id,
    )
    .await
    .unwrap());
    let refreshed = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let evidence = refreshed
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap()
        .evidence();
    assert!(evidence[0].confirmed());
    assert!(!evidence[1].confirmed());
}

#[tokio::test]
async fn saved_match_debugger_rejects_an_unmatched_saved_job_and_resume() {
    let (database, _, resume_id) = saved_match_context().await;
    let mut unmatched_job = test_job("saved-match-debugger-unmatched", "Receptionist", "Example");
    unmatched_job.description = Some("Required: reports".to_string());
    database.insert_job_if_new(&unmatched_job).await.unwrap();

    assert_eq!(
        prepare_saved_match_debugger(&database, &unmatched_job.hash, resume_id)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
}

#[tokio::test]
async fn saved_match_debugger_ignores_saved_match_feedback() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let debugger = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let saved_match = database
        .resume_matcher()
        .get_match_result(resume_id, &job_hash)
        .await
        .unwrap()
        .unwrap();

    database
        .resume_matcher()
        .set_match_feedback(saved_match.id, Some(ResumeMatchFeedbackLabel::Useful))
        .await
        .unwrap();

    assert_eq!(
        prepare_saved_match_debugger(&database, &job_hash, resume_id)
            .await
            .unwrap(),
        debugger
    );
}

#[tokio::test]
async fn saved_match_debugger_returns_only_deterministic_opaque_current_evidence() {
    let (database, job_hash, resume_id) = saved_match_context().await;

    let first = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let repeated = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();

    assert_eq!(first, repeated);
    let scheduling = first
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap();
    assert!(!scheduling.evidence().is_empty());
    assert_eq!(scheduling.match_state(), RequirementMatchState::Strong);

    let encoded = serde_json::to_string(&first).unwrap();
    assert!(!encoded.contains("case_file_id"));
    assert!(!encoded.contains("resume_text.1"));
    assert!(!encoded.contains("resume:"));
    assert!(!encoded.contains("Managed scheduling for a support team."));
    assert!(!encoded.contains("Required: scheduling"));
}

#[tokio::test]
async fn saved_match_debugger_confirms_only_exact_current_evidence_after_explicit_action() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let debugger = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let evidence_id = debugger
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap()
        .evidence()[0]
        .evidence_id()
        .to_string();

    assert!(confirm_saved_match_debugger_evidence(
        &database,
        &job_hash,
        resume_id,
        debugger.debugger_id(),
        &evidence_id,
    )
    .await
    .unwrap());
    assert!(!confirm_saved_match_debugger_evidence(
        &database,
        &job_hash,
        resume_id,
        debugger.debugger_id(),
        &evidence_id,
    )
    .await
    .unwrap());

    let case_file = create_or_reuse_case_file(&database, &job_hash)
        .await
        .unwrap();
    let links = read_case_file_evidence_links(&database, &case_file.case_file_id)
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].object_id, evidence_id);
}

#[tokio::test]
async fn saved_match_debugger_fails_closed_for_changed_deleted_or_unknown_saved_inputs() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let debugger = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let evidence_id = debugger
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap()
        .evidence()[0]
        .evidence_id()
        .to_string();

    database
        .resume_matcher()
        .add_user_skill(
            resume_id,
            NewSkill {
                skill_name: "CRM".to_string(),
                ..NewSkill::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        confirm_saved_match_debugger_evidence(
            &database,
            &job_hash,
            resume_id,
            debugger.debugger_id(),
            &evidence_id,
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );

    let current = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let mut changed_job = database.get_job_by_hash(&job_hash).await.unwrap().unwrap();
    changed_job.description =
        Some("Required: scheduling\nPreferred: CRM\nRequired: reports".to_string());
    database.upsert_job(&changed_job).await.unwrap();
    assert_eq!(
        confirm_saved_match_debugger_evidence(
            &database,
            &job_hash,
            resume_id,
            current.debugger_id(),
            scheduling_evidence_id(&current),
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );

    let current = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    database
        .resume_matcher()
        .delete_resume(resume_id)
        .await
        .unwrap();
    assert_eq!(
        prepare_saved_match_debugger(&database, &job_hash, resume_id)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
    assert_eq!(
        confirm_saved_match_debugger_evidence(
            &database,
            &job_hash,
            resume_id,
            current.debugger_id(),
            scheduling_evidence_id(&current),
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );
    assert_eq!(
        prepare_saved_match_debugger(&database, "missing-job", 1)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
}
