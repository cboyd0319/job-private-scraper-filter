//! Proves reviewed packet tasks create only bounded, local, user-submitted draft material.

use super::*;
use crate::{
    pack_runtime::{execute_draft_packet_task, prepare_draft_packet_task},
    v3_foundation::{
        confirm_saved_match_debugger_evidence, prepare_saved_match_debugger,
        save_saved_match_evidence_packet_claim,
    },
};
use jobsentinel_storage::pack_tasks::PackTaskStatus;

const PACKET_PUBLISHER_ID: &str = "jobsentinel-test-packet-agent-v1";
const PACKET_PACK_ID: &str = "jobsentinel.test.packet-builder";

mod guards;

fn signed_packet_builder_pack(
    sequence: u64,
    max_output_bytes: u64,
) -> (TrustedPublisherKey, Vec<u8>) {
    let (public_key, _) = sign_ed25519_for_test(&[9; 32], &[]).unwrap();
    let actions = vec![
        PackAction::ReadSelectedCaseFile,
        PackAction::ReadSelectedResumeEvidence,
        PackAction::ReadPublicJobPosting,
        PackAction::CreateDraftApplicationPacket,
        PackAction::WriteLocalEvent,
    ];
    let publisher = TrustedPublisherKey {
        publisher_key_id: PACKET_PUBLISHER_ID.to_string(),
        public_key,
        revoked: false,
        pack_type: PackType::Agent,
        execution_class: PackExecutionClass::ReviewedTypedWorkflow,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        allowed_data_categories: vec![DataCategory::PublicJobPosting, DataCategory::ResumeEvidence],
        allowed_task_kinds: vec![AgentTaskKind::DraftPacket],
        allowed_actions: actions.clone(),
        allowed_approval_gates: vec![ApprovalGate::PerExecutionReview],
        allow_gateway_external_ai: false,
    };
    let payload = serde_json::to_string(&json!({
        "schema": "jobsentinel.v3.pack-payload.v1",
        "pack_type": "agent",
        "task": {
            "schema": "jobsentinel.v3.agent-task.v1",
            "task_id": "prepare-reviewed-application-packet",
            "pack_id": PACKET_PACK_ID,
            "kind": "draft_packet",
            "privacy_labels": ["local_only", "sensitive"],
            "data_categories": ["public_job_posting", "resume_evidence"],
            "max_duration_seconds": 60,
            "max_output_bytes": max_output_bytes,
            "max_attempts": 1,
            "user_review_required": true
        },
        "plan": [
            {"action": "read_selected_case_file", "label": "Read the selected case file"},
            {"action": "read_selected_resume_evidence", "label": "Read selected resume evidence"},
            {"action": "read_public_job_posting", "label": "Read the public job posting"},
            {"action": "create_draft_application_packet", "label": "Create a local draft application packet"},
            {"action": "write_local_event", "label": "Record a local audit receipt"}
        ],
        "failure_message": "No draft was prepared. Review the case and retry."
    }))
    .unwrap();
    let manifest = PackManifest {
        schema: SchemaId::PackManifestV1,
        pack_id: PACKET_PACK_ID.to_string(),
        pack_type: PackType::Agent,
        execution_class: PackExecutionClass::ReviewedTypedWorkflow,
        publisher_key_id: PACKET_PUBLISHER_ID.to_string(),
        payload_sha256: hex::encode(Sha256::digest(payload.as_bytes())),
        privacy_labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        allowed_data_categories: vec![DataCategory::PublicJobPosting, DataCategory::ResumeEvidence],
        allowed_task_kinds: vec![AgentTaskKind::DraftPacket],
        allowed_actions: actions,
        approval_gates: vec![ApprovalGate::PerExecutionReview],
        gateway_policy_id: None,
    };
    let envelope = signed_envelope(
        [9; 32],
        PACKET_PUBLISHER_ID,
        PACKET_PACK_ID,
        sequence,
        &manifest,
        &payload,
        "Deterministic reviewed packet preparation",
    );
    (publisher, envelope)
}

async fn activated_packet_pack(
    database: &Database,
    max_output_bytes: u64,
) -> (tempfile::TempDir, TrustedPublisherKey, u64) {
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, envelope) = signed_packet_builder_pack(1, max_output_bytes);
    let staged = stage_pack_artifact(
        database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();
    let active = activate_pack_artifact(
        database,
        artifact_root.path(),
        PACKET_PUBLISHER_ID,
        PACKET_PACK_ID,
        1,
        staged.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();
    (artifact_root, publisher, active.generation)
}

async fn confirm_first_evidence(database: &Database, job_hash: &str, resume_id: i64) -> String {
    let debugger = prepare_saved_match_debugger(database, job_hash, resume_id)
        .await
        .unwrap();
    let evidence_id = debugger
        .requirements()
        .iter()
        .flat_map(|requirement| requirement.evidence())
        .next()
        .unwrap()
        .evidence_id()
        .to_string();
    confirm_saved_match_debugger_evidence(
        database,
        job_hash,
        resume_id,
        debugger.debugger_id(),
        &evidence_id,
    )
    .await
    .unwrap();
    evidence_id
}

#[tokio::test]
async fn exact_reviewed_draft_packet_is_single_use_bounded_and_audited() {
    let (database, job_hash, resume_id) = saved_match().await;
    let evidence_id = confirm_first_evidence(&database, &job_hash, resume_id).await;
    save_saved_match_evidence_packet_claim(
        &database,
        &job_hash,
        resume_id,
        "Managed support-team scheduling.".to_string(),
        vec![evidence_id],
    )
    .await
    .unwrap();
    let (artifact_root, publisher, generation) = activated_packet_pack(&database, 524_288).await;
    let management = crate::pack_runtime::list_pack_management_reviews(&database)
        .await
        .unwrap();
    assert_eq!(
        management[0].current_release.purpose,
        crate::pack_runtime::PackPurpose::DraftApplicationPacket
    );

    let prepared = prepare_draft_packet_task(
        &database,
        artifact_root.path(),
        PACKET_PUBLISHER_ID,
        PACKET_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &job_hash,
        resume_id,
    )
    .await
    .unwrap();

    assert_eq!(prepared.task.task_kind, AgentTaskKind::DraftPacket);
    assert_eq!(prepared.task.plan.len(), 5);
    assert_eq!(prepared.task.max_duration_seconds, 60);
    assert_eq!(prepared.task.max_output_bytes, 524_288);
    assert_eq!(prepared.packet.job_title, "Office Assistant");
    assert_eq!(prepared.packet.company, "Example");
    assert_eq!(prepared.packet.resume_name, "Resume");
    assert_eq!(prepared.packet.reviewed_claims.len(), 1);
    assert_eq!(prepared.packet.attachment_checklist.len(), 5);
    assert!(prepared.packet.screening_answers_require_manual_review);
    assert!(prepared.packet.final_submission_by_user);

    let result = execute_draft_packet_task(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &prepared.task.run_id,
        &prepared.task.approval_reference,
        &job_hash,
        resume_id,
    )
    .await
    .unwrap();
    assert_eq!(result.packet, prepared.packet);
    let encoded = serde_json::to_vec(&result).unwrap();
    assert!(encoded.len() <= prepared.task.max_output_bytes as usize);
    for private in ["Managed scheduling for a support team.", "resume.txt"] {
        assert!(!encoded
            .windows(private.len())
            .any(|value| value == private.as_bytes()));
    }
    let run = database
        .get_pack_task(&prepared.task.run_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.status, PackTaskStatus::Succeeded);
    assert_eq!(run.receipt_id.as_deref(), Some(result.receipt_id.as_str()));
    assert!(execute_draft_packet_task(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &prepared.task.run_id,
        &prepared.task.approval_reference,
        &job_hash,
        resume_id,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn packet_preflight_failure_cancels_the_authenticated_pending_approval() {
    let (database, job_hash, resume_id) = saved_match().await;
    confirm_first_evidence(&database, &job_hash, resume_id).await;
    let (artifact_root, publisher, generation) = activated_packet_pack(&database, 524_288).await;
    let prepared = prepare_draft_packet_task(
        &database,
        artifact_root.path(),
        PACKET_PUBLISHER_ID,
        PACKET_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &job_hash,
        resume_id,
    )
    .await
    .unwrap();
    disable_pack_artifact(&database, PACKET_PUBLISHER_ID, PACKET_PACK_ID, generation)
        .await
        .unwrap();

    assert!(execute_draft_packet_task(
        &database,
        artifact_root.path(),
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &prepared.task.run_id,
        &prepared.task.approval_reference,
        &job_hash,
        resume_id,
    )
    .await
    .is_err());
    assert_eq!(
        database
            .get_pack_task(&prepared.task.run_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        PackTaskStatus::Cancelled
    );
}

#[tokio::test]
async fn draft_packet_is_not_offered_when_the_signed_output_limit_cannot_hold_it() {
    let (database, job_hash, resume_id) = saved_match().await;
    confirm_first_evidence(&database, &job_hash, resume_id).await;
    let (artifact_root, publisher, generation) = activated_packet_pack(&database, 64).await;

    let error = prepare_draft_packet_task(
        &database,
        artifact_root.path(),
        PACKET_PUBLISHER_ID,
        PACKET_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &job_hash,
        resume_id,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "draft packet exceeds the signed output limit"
    );
}
