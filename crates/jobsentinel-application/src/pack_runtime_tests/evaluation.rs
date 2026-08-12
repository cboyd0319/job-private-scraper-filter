//! Proves active packs run closed ATS and Evidence Reviewer targets without fixture exposure.

use super::*;
use crate::pack_runtime::{
    evaluate_active_ats_resume_requirement_pack,
    evaluate_active_evidence_reviewer_resume_requirement_pack,
};

const EVALUATION_PUBLISHER_ID: &str = "jobsentinel-test-evaluation-v1";
const EVALUATION_PACK_ID: &str = "jobsentinel.eval.synthetic-baseline";
const EVALUATION_CASE_ID: &str = "resume-evidence-hard-negative-v1";

fn signed_evaluation_pack(
    has_direct_evidence: bool,
    requirement: &str,
    target: &str,
) -> (TrustedPublisherKey, Vec<u8>) {
    let seed = [11; 32];
    let (public_key, _) = sign_ed25519_for_test(&seed, &[]).unwrap();
    let publisher = TrustedPublisherKey {
        publisher_key_id: EVALUATION_PUBLISHER_ID.to_string(),
        public_key,
        revoked: false,
        pack_type: PackType::Evaluation,
        execution_class: PackExecutionClass::StaticContent,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly],
        allowed_data_categories: vec![],
        allowed_task_kinds: vec![],
        allowed_actions: vec![],
        allowed_approval_gates: vec![],
        allow_gateway_external_ai: false,
    };
    let mut evaluation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../jobsentinel-domain/src/fixtures/v3_evaluation_set_v1.json"
    ))
    .unwrap();
    evaluation["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|case| case["id"] == EVALUATION_CASE_ID);
    evaluation["revision"] = json!("fixture-secret-id");
    evaluation["cases"][0]["input"]["requirement"] = json!(requirement);
    if has_direct_evidence {
        let case = evaluation["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["id"] == EVALUATION_CASE_ID)
            .unwrap();
        case["input"]["evidence"] =
            json!(["Built production Rust experience across shipped desktop systems."]);
    }
    let payload = serde_json::to_string(&json!({
        "schema": "jobsentinel.v3.pack-payload.v1",
        "pack_type": "evaluation",
        "target": target,
        "evaluation_set_json": serde_json::to_string(&evaluation).unwrap()
    }))
    .unwrap();
    let manifest = PackManifest {
        schema: SchemaId::PackManifestV1,
        pack_id: EVALUATION_PACK_ID.to_string(),
        pack_type: PackType::Evaluation,
        execution_class: PackExecutionClass::StaticContent,
        publisher_key_id: EVALUATION_PUBLISHER_ID.to_string(),
        payload_sha256: hex::encode(Sha256::digest(payload.as_bytes())),
        privacy_labels: vec![PrivacyLabel::LocalOnly],
        allowed_data_categories: vec![],
        allowed_task_kinds: vec![],
        allowed_actions: vec![],
        approval_gates: vec![],
        gateway_policy_id: None,
    };
    let envelope = signed_envelope(
        seed,
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        1,
        &manifest,
        &payload,
        "Complete synthetic v3 evaluation set.",
    );
    (publisher, envelope)
}

async fn activated_evaluation_pack(
    database: &Database,
    has_direct_evidence: bool,
    requirement: &str,
    target: &str,
) -> (tempfile::TempDir, TrustedPublisherKey, u64) {
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, envelope) = signed_evaluation_pack(has_direct_evidence, requirement, target);
    let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    let staged = stage_pack_artifact(
        database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        today,
    )
    .await
    .unwrap();
    let active = activate_pack_artifact(
        database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        1,
        staged.generation,
        std::slice::from_ref(&publisher),
        today,
    )
    .await
    .unwrap();
    (artifact_root, publisher, active.generation)
}

async fn evaluate(
    has_direct_evidence: bool,
    requirement: &str,
) -> anyhow::Result<crate::pack_runtime::AtsResumeRequirementEvaluation> {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_evaluation_pack(
        &database,
        has_direct_evidence,
        requirement,
        "ats_plain_text_resume_requirement_v1",
    )
    .await;

    evaluate_active_ats_resume_requirement_pack(
        &database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
}

async fn evaluate_evidence_reviewer(
    has_direct_evidence: bool,
    requirement: &str,
) -> anyhow::Result<crate::pack_runtime::EvidenceReviewerResumeRequirementEvaluation> {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_evaluation_pack(
        &database,
        has_direct_evidence,
        requirement,
        "evidence_reviewer_resume_requirement_v1",
    )
    .await;

    evaluate_active_evidence_reviewer_resume_requirement_pack(
        &database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
}

#[tokio::test]
async fn active_evaluation_pack_runs_the_plain_text_ats_target_opaquely() {
    let passing = evaluate(false, "production Rust experience").await.unwrap();
    assert_eq!(passing.target, "ats_plain_text_resume_requirement_v1");
    assert_eq!((passing.passed_cases, passing.failed_cases), (1, 0));

    let failing = evaluate(true, "production Rust experience").await.unwrap();
    assert_eq!((failing.passed_cases, failing.failed_cases), (0, 1));

    for result in [passing, failing] {
        let value = serde_json::to_value(&result).unwrap();
        let encoded = value.to_string();
        let fields = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(fields, ["failedCases", "passedCases", "target"]);
        assert!(!encoded.contains("production Rust experience"));
        assert!(!encoded.contains(EVALUATION_CASE_ID));
        assert!(!encoded.contains("fixture-secret-id"));
        assert!(!encoded.contains("receipt"));
        assert!(!encoded.contains("task"));
        assert!(!format!("{result:?}").contains("production Rust experience"));
        assert!(!format!("{result:?}").contains("fixture-secret-id"));
    }
}

#[tokio::test]
async fn evidence_reviewer_evaluation_leaves_caller_user_state_empty() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_evaluation_pack(
        &database,
        false,
        "production Rust experience",
        "evidence_reviewer_resume_requirement_v1",
    )
    .await;

    evaluate_active_evidence_reviewer_resume_requirement_pack(
        &database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert!(database.get_recent_jobs(1).await.unwrap().is_empty());
    assert!(database
        .resume_matcher()
        .get_active_resume()
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn altered_active_evidence_reviewer_evaluation_is_quarantined() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_evaluation_pack(
        &database,
        false,
        "production Rust experience",
        "evidence_reviewer_resume_requirement_v1",
    )
    .await;
    std::fs::write(walk_files(artifact_root.path()).pop().unwrap(), b"altered").unwrap();

    let error = evaluate_active_evidence_reviewer_resume_requirement_pack(
        &database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "pack artifact verification failed");
    let stream = database
        .get_pack_stream(EVALUATION_PUBLISHER_ID, EVALUATION_PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
}

#[tokio::test]
async fn active_evaluation_pack_runs_the_evidence_reviewer_target_opaquely() {
    let passing = evaluate_evidence_reviewer(false, "production Rust experience")
        .await
        .unwrap();
    assert_eq!(passing.target, "evidence_reviewer_resume_requirement_v1");
    assert_eq!((passing.passed_cases, passing.failed_cases), (1, 0));

    let failing = evaluate_evidence_reviewer(true, "production Rust experience")
        .await
        .unwrap();
    assert_eq!((failing.passed_cases, failing.failed_cases), (0, 1));

    for result in [passing, failing] {
        let value = serde_json::to_value(&result).unwrap();
        let encoded = value.to_string();
        let fields = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(fields, ["failedCases", "passedCases", "target"]);
        assert!(!encoded.contains("production Rust experience"));
        assert!(!encoded.contains(EVALUATION_CASE_ID));
        assert!(!encoded.contains("fixture-secret-id"));
        assert!(!encoded.contains("receipt"));
        assert!(!encoded.contains("task"));
        assert!(!format!("{result:?}").contains("production Rust experience"));
        assert!(!format!("{result:?}").contains("fixture-secret-id"));
    }
}

#[tokio::test]
async fn active_evaluation_pack_rejects_ambiguous_ats_requirements() {
    assert!(evaluate(false, "Rust, Python").await.is_err());
}

#[tokio::test]
async fn altered_active_evaluation_is_quarantined_without_exposing_fixture_data() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_evaluation_pack(
        &database,
        false,
        "production Rust experience",
        "ats_plain_text_resume_requirement_v1",
    )
    .await;
    std::fs::write(walk_files(artifact_root.path()).pop().unwrap(), b"altered").unwrap();

    let error = evaluate_active_ats_resume_requirement_pack(
        &database,
        artifact_root.path(),
        EVALUATION_PUBLISHER_ID,
        EVALUATION_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "pack artifact verification failed");
    assert!(!error.to_string().contains("production Rust experience"));
    assert!(!error.to_string().contains(EVALUATION_CASE_ID));
    assert!(!error.to_string().contains("fixture-secret-id"));
    let stream = database
        .get_pack_stream(EVALUATION_PUBLISHER_ID, EVALUATION_PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
    let release = database
        .get_stored_pack_release(EVALUATION_PUBLISHER_ID, EVALUATION_PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(
        release.quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}
