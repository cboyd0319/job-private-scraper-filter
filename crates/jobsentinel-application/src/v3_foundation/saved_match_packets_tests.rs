use super::saved_match_debugger_tests::saved_match_context;
use super::*;
use jobsentinel_storage::resume::NewSkill;
use uuid::Uuid;

async fn current_evidence_id(database: &Database, job_hash: &str, resume_id: i64) -> String {
    prepare_saved_match_debugger(database, job_hash, resume_id)
        .await
        .unwrap()
        .requirements()
        .iter()
        .find(|requirement| requirement.requirement() == "scheduling")
        .unwrap()
        .evidence()[0]
        .evidence_id()
        .to_string()
}

async fn confirm_current_evidence(database: &Database, job_hash: &str, resume_id: i64) -> String {
    let debugger = prepare_saved_match_debugger(database, job_hash, resume_id)
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
        database,
        job_hash,
        resume_id,
        debugger.debugger_id(),
        &evidence_id,
    )
    .await
    .unwrap());
    evidence_id
}

#[tokio::test]
async fn saved_match_packet_saves_reloads_and_serializes_only_reviewed_data() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let evidence_id = confirm_current_evidence(&database, &job_hash, resume_id).await;

    let saved = save_saved_match_evidence_packet_claim(
        &database,
        &job_hash,
        resume_id,
        "Managed support-team scheduling.".to_string(),
        vec![evidence_id.clone()],
    )
    .await
    .unwrap();
    assert!(Uuid::parse_str(saved.claim_id()).is_ok());
    assert_eq!(saved.evidence_ids(), [evidence_id]);
    assert!(saved.boundaries().is_empty());
    assert_eq!(
        list_saved_match_evidence_packet_claims(&database, &job_hash, resume_id)
            .await
            .unwrap(),
        vec![saved.clone()]
    );

    let encoded = serde_json::to_string(&saved).unwrap();
    for forbidden in [
        "case_file_id",
        "packet_id",
        "revision",
        "resume_text.1",
        "Managed scheduling for a support team.",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

#[tokio::test]
async fn saved_match_packet_refuses_unconfirmed_or_invalid_claim_inputs_without_a_packet() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let evidence_id = current_evidence_id(&database, &job_hash, resume_id).await;
    let valid_text = "Managed support-team scheduling.".to_string();

    assert_eq!(
        save_saved_match_evidence_packet_claim(
            &database,
            &job_hash,
            resume_id,
            valid_text.clone(),
            vec![evidence_id.clone()],
        )
        .await
        .unwrap_err(),
        FoundationError::Conflict
    );
    for (text, evidence_ids) in [
        (valid_text.clone(), vec!["f".repeat(64)]),
        (
            valid_text.clone(),
            vec![evidence_id.clone(), evidence_id.clone()],
        ),
        (valid_text.clone(), vec!["f".repeat(65)]),
        (" ".to_string(), vec![evidence_id.clone()]),
        ("x".repeat(8_193), vec![evidence_id]),
    ] {
        assert_eq!(
            save_saved_match_evidence_packet_claim(
                &database,
                &job_hash,
                resume_id,
                text,
                evidence_ids,
            )
            .await
            .unwrap_err(),
            FoundationError::InvalidInput
        );
    }
    assert!(
        list_saved_match_evidence_packet_claims(&database, &job_hash, resume_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn saved_match_packet_refuses_stale_resume_skills_and_job_data() {
    let (database, job_hash, resume_id) = saved_match_context().await;
    let evidence_id = confirm_current_evidence(&database, &job_hash, resume_id).await;
    let saved = save_saved_match_evidence_packet_claim(
        &database,
        &job_hash,
        resume_id,
        "Managed support-team scheduling.".to_string(),
        vec![evidence_id.clone()],
    )
    .await
    .unwrap();

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
        save_saved_match_evidence_packet_claim(
            &database,
            &job_hash,
            resume_id,
            "Stale claim.".to_string(),
            vec![evidence_id],
        )
        .await
        .unwrap_err(),
        FoundationError::InvalidInput
    );
    assert_eq!(
        list_saved_match_evidence_packet_claims(&database, &job_hash, resume_id)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );

    let mut changed_job = database.get_job_by_hash(&job_hash).await.unwrap().unwrap();
    changed_job.description = Some("Required: scheduling\nRequired: reports".to_string());
    database.upsert_job(&changed_job).await.unwrap();
    assert_eq!(
        list_saved_match_evidence_packet_claims(&database, &job_hash, resume_id)
            .await
            .unwrap_err(),
        FoundationError::Conflict
    );
    assert_eq!(saved.evidence_ids().len(), 1);
}
