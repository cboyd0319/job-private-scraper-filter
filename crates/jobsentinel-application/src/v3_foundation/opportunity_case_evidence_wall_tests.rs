use super::*;
use crate::{ats::ApplicationStatus, test_support::test_job};
use jobsentinel_storage::Database;

async fn saved_match_with(
    job_hash: &str,
    description: &str,
    resume_text: &str,
) -> (Database, String, i64) {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut job = test_job(job_hash, "Office Assistant", "Example");
    job.description = Some(description.to_string());
    database.insert_job_if_new(&job).await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resume.txt");
    std::fs::write(&path, resume_text).unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();
    (database, job_hash.to_string(), resume_id)
}

async fn confirm_all_match_evidence(database: &Database, job_hash: &str, resume_id: i64) {
    let debugger = prepare_saved_match_debugger(database, job_hash, resume_id)
        .await
        .unwrap();
    for evidence_id in debugger
        .requirements()
        .iter()
        .flat_map(|requirement| requirement.evidence())
        .map(|evidence| evidence.evidence_id().to_string())
    {
        assert!(confirm_saved_match_debugger_evidence(
            database,
            job_hash,
            resume_id,
            debugger.debugger_id(),
            &evidence_id,
        )
        .await
        .unwrap());
    }
}

#[tokio::test]
async fn opportunity_case_links_active_saved_match_requirements_to_safe_evidence() {
    let (database, job_hash, resume_id) =
        super::saved_match_debugger_tests::saved_match_context().await;
    let debugger = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let scheduling = debugger
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap();
    let evidence_id = scheduling.evidence()[0].evidence_id();
    assert!(confirm_saved_match_debugger_evidence(
        &database,
        &job_hash,
        resume_id,
        debugger.debugger_id(),
        evidence_id,
    )
    .await
    .unwrap());

    let snapshot = open_opportunity_case(&database, &job_hash, 60)
        .await
        .unwrap();
    let value = serde_json::to_value(snapshot).unwrap();
    let requirements = value["evidence"]["requirements"].as_array().unwrap();
    let scheduling = requirements
        .iter()
        .find(|requirement| requirement["requirement"] == "scheduling")
        .unwrap();
    let crm = requirements
        .iter()
        .find(|requirement| {
            requirement["requirement"]
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("CRM"))
        })
        .unwrap();

    assert_eq!(value["evidence"]["review_status"], "ready");
    assert!(scheduling["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| {
            evidence["confirmed"] == true
                && matches!(
                    evidence["kind"].as_str(),
                    Some(
                        "resume_bullet" | "project" | "skill" | "certification" | "resume_evidence"
                    )
                )
        }));
    assert_eq!(crm["match_state"], "missing");
    assert_eq!(crm["why_not"], "missing_evidence");
    assert_eq!(value["decision"]["kind"], "maybe");
    assert!(value["decision"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().to_lowercase().contains("crm")));

    let serialized = value.to_string();
    assert!(!serialized.contains(evidence_id));
    assert!(!serialized.contains("file_path"));
    assert!(!serialized.contains("parsed_text"));
}

#[tokio::test]
async fn opportunity_case_does_not_silently_use_a_non_active_saved_match() {
    let (database, job_hash, _) = super::saved_match_debugger_tests::saved_match_context().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("active-resume.txt");
    std::fs::write(&path, "Current resume without a saved comparison.").unwrap();
    let active_resume_id = database
        .resume_matcher()
        .upload_resume("Current Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .set_active_resume(active_resume_id)
        .await
        .unwrap();

    let snapshot = open_opportunity_case(&database, &job_hash, 60)
        .await
        .unwrap();
    let value = serde_json::to_value(snapshot).unwrap();

    assert_eq!(value["evidence"]["review_status"], "no_saved_match");
    assert_eq!(value["evidence"]["requirements"], serde_json::json!([]));
    assert_eq!(value["decision"]["kind"], "research_more");
    assert_eq!(
        value["decision"]["reasons"],
        serde_json::json!(["No current saved-resume evidence review is available."])
    );
}

#[tokio::test]
async fn opportunity_case_recommends_apply_only_after_current_evidence_is_confirmed() {
    let (database, job_hash, resume_id) = saved_match_with(
        "case-apply",
        "Required: scheduling",
        "Experience\nManaged scheduling for a support team.",
    )
    .await;
    confirm_all_match_evidence(&database, &job_hash, resume_id).await;

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job_hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["decision"]["kind"], "apply");
    assert_eq!(
        value["decision"]["reasons"],
        serde_json::json!(["Current confirmed evidence supports every reviewed requirement."])
    );
}

#[tokio::test]
async fn opportunity_case_keeps_confirmed_military_evidence_out_of_apply() {
    let (database, job_hash, resume_id) = saved_match_with(
        "case-military-evidence",
        "Required: logistics",
        "Military Experience\nArmy 25B with logistics duties.",
    )
    .await;
    confirm_all_match_evidence(&database, &job_hash, resume_id).await;

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job_hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["decision"]["kind"], "research_more");
    assert!(value["decision"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason.as_str().unwrap().contains("civilian qualification")));
    let serialized = value.to_string();
    assert!(!serialized.contains("Army 25B"));
    assert!(!serialized.contains("military_evidence"));
    assert!(!serialized.contains("requires_review"));
}

#[tokio::test]
async fn opportunity_case_keeps_unverified_hard_requirements_in_research_more() {
    let (database, job_hash, _) = saved_match_with(
        "case-hard-blocker",
        "Required: security clearance",
        "Experience\nManaged scheduling for a support team.",
    )
    .await;

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job_hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();
    let requirement = value["evidence"]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|requirement| {
            requirement["requirement"]
                .as_str()
                .is_some_and(|value| value.to_lowercase().contains("clearance"))
        })
        .unwrap();

    assert_eq!(requirement["blocking"], true);
    assert_eq!(value["decision"]["kind"], "research_more");
    assert!(value["decision"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("clearance")));
}

#[tokio::test]
async fn opportunity_case_uses_skip_only_for_a_closed_negative_outcome() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let job = test_job("case-closed", "Office Assistant", "Example");
    database.insert_job_if_new(&job).await.unwrap();
    let application_id = database
        .application_tracker()
        .create_application(&job.hash)
        .await
        .unwrap();
    database
        .application_tracker()
        .update_status(application_id, ApplicationStatus::Withdrawn)
        .await
        .unwrap();

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job.hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["decision"]["kind"], "skip");
    assert_eq!(
        value["decision"]["reasons"],
        serde_json::json!(["This opportunity already has a closed outcome."])
    );
}

#[tokio::test]
async fn opportunity_case_treats_an_accepted_offer_as_closed_success() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let job = test_job("case-accepted", "Office Assistant", "Example");
    database.insert_job_if_new(&job).await.unwrap();
    let application_id = database
        .application_tracker()
        .create_application(&job.hash)
        .await
        .unwrap();
    database
        .application_tracker()
        .update_status(application_id, ApplicationStatus::OfferAccepted)
        .await
        .unwrap();

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job.hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["decision"]["kind"], "skip");
    assert_eq!(
        value["decision"]["reasons"],
        serde_json::json!(["This opportunity closed with an accepted offer."])
    );
}

#[tokio::test]
async fn opportunity_case_marks_a_changed_active_match_for_refresh() {
    let (database, job_hash, _) = saved_match_with(
        "case-needs-refresh",
        "Required: scheduling",
        "Experience\nManaged scheduling for a support team.",
    )
    .await;
    let mut changed_job = database.get_job_by_hash(&job_hash).await.unwrap().unwrap();
    changed_job.description = Some(String::new());
    changed_job.updated_at += chrono::Duration::seconds(1);
    database.upsert_job(&changed_job).await.unwrap();

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job_hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["evidence"]["review_status"], "needs_refresh");
    assert_eq!(value["decision"]["kind"], "research_more");
}

#[tokio::test]
async fn opportunity_case_requires_research_before_reusing_a_stale_packet() {
    let (database, job_hash, resume_id) = saved_match_with(
        "case-stale-packet",
        "Required: scheduling",
        "Experience\nManaged scheduling for a support team.",
    )
    .await;
    let debugger = prepare_saved_match_debugger(&database, &job_hash, resume_id)
        .await
        .unwrap();
    let evidence_id = debugger.requirements()[0].evidence()[0]
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
    save_saved_match_evidence_packet_claim(
        &database,
        &job_hash,
        resume_id,
        "Reviewed scheduling claim.".to_string(),
        vec![evidence_id],
    )
    .await
    .unwrap();
    let mut changed_job = database.get_job_by_hash(&job_hash).await.unwrap().unwrap();
    changed_job.updated_at += chrono::Duration::seconds(1);
    database.upsert_job(&changed_job).await.unwrap();

    let value = serde_json::to_value(
        open_opportunity_case(&database, &job_hash, 60)
            .await
            .unwrap(),
    )
    .unwrap();

    assert_eq!(value["evidence"]["stale_packet_count"], 1);
    assert_eq!(value["decision"]["kind"], "research_more");
    assert!(value["decision"]["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason == "Reviewed evidence needs confirmation before reuse."));
}
