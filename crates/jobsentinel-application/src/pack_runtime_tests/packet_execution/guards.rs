//! Proves draft-packet approval fails when rendered or non-rendered source state changes.

use super::*;

async fn prepared_packet_review() -> (
    Database,
    String,
    i64,
    tempfile::TempDir,
    TrustedPublisherKey,
    String,
    crate::pack_runtime::DraftPacketTaskReview,
) {
    let (database, job_hash, resume_id) = saved_match().await;
    let evidence_id = confirm_first_evidence(&database, &job_hash, resume_id).await;
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
    (
        database,
        job_hash,
        resume_id,
        artifact_root,
        publisher,
        evidence_id,
        prepared,
    )
}

async fn assert_prepared_packet_fails(
    database: &Database,
    artifact_root: &std::path::Path,
    publisher: &TrustedPublisherKey,
    prepared: &crate::pack_runtime::DraftPacketTaskReview,
    job_hash: &str,
    resume_id: i64,
) {
    assert!(execute_draft_packet_task(
        database,
        artifact_root,
        std::slice::from_ref(publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        &prepared.task.run_id,
        &prepared.task.approval_reference,
        job_hash,
        resume_id,
    )
    .await
    .is_err());
    assert_pack_task_without_receipt(database, &prepared.task.run_id, PackTaskStatus::Failed).await;
}

#[tokio::test]
async fn changed_reviewed_claim_fails_without_committing_a_receipt() {
    let (database, job_hash, resume_id, artifact_root, publisher, evidence_id, prepared) =
        prepared_packet_review().await;
    save_saved_match_evidence_packet_claim(
        &database,
        &job_hash,
        resume_id,
        "Added after the user reviewed the packet.".to_string(),
        vec![evidence_id],
    )
    .await
    .unwrap();

    assert_prepared_packet_fails(
        &database,
        artifact_root.path(),
        &publisher,
        &prepared,
        &job_hash,
        resume_id,
    )
    .await;
}

#[tokio::test]
async fn changed_non_rendered_case_state_fails_without_committing_a_receipt() {
    let (database, job_hash, resume_id, artifact_root, publisher, _, prepared) =
        prepared_packet_review().await;
    database
        .application_tracker()
        .create_application(&job_hash)
        .await
        .unwrap();

    assert_prepared_packet_fails(
        &database,
        artifact_root.path(),
        &publisher,
        &prepared,
        &job_hash,
        resume_id,
    )
    .await;
}
