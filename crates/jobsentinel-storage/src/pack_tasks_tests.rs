//! Exercises pack-task persistence, transition, replay, lifecycle, and audit guarantees.

use chrono::{Duration, Utc};
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{AgentTaskKind, DataCategory, PrivacyLabel, PrivacyReceipt},
};
use sqlx::SqlitePool;

use crate::test_support::migrated_database;

mod expiry;
mod input_guard;

async fn assert_started_without_audit(database: &crate::Database, run_id: &str) {
    assert_eq!(
        database
            .get_pack_task(run_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        crate::pack_tasks::PackTaskStatus::Started
    );
    let receipt_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_privacy_receipts")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let event_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_job_events")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!((receipt_count, event_count), (0, 0));
}

#[tokio::test]
async fn creates_an_immutable_pending_evidence_review() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;

    let created = database.create_pack_task(&context).await.unwrap();

    assert_eq!(created.context, context);
    assert_eq!(created.status, crate::pack_tasks::PackTaskStatus::Pending);
    assert!(created.receipt_id.is_none());

    let mut replay = context;
    replay.run_id = "run-2".to_string();
    assert!(database.create_pack_task(&replay).await.is_err());
}

#[tokio::test]
async fn start_requires_the_exact_unexpired_context_and_is_single_use() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();

    let mut wrong = context.clone();
    wrong.input_sha256 = "c".repeat(64);
    assert!(database.start_pack_task(&wrong).await.is_err());

    let started = database.start_pack_task(&context).await.unwrap();
    assert_eq!(started.status, crate::pack_tasks::PackTaskStatus::Started);
    assert!(started.started_at.is_some());
    assert!(database.start_pack_task(&context).await.is_err());
}

#[tokio::test]
async fn cancel_is_idempotent_only_for_the_cancelled_terminal_state() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();

    let cancelled = database.cancel_pack_task(&context.run_id).await.unwrap();
    assert_eq!(
        cancelled.status,
        crate::pack_tasks::PackTaskStatus::Cancelled
    );
    assert_eq!(
        database.cancel_pack_task(&context.run_id).await.unwrap(),
        cancelled
    );
    assert!(database.start_pack_task(&context).await.is_err());
}

#[tokio::test]
async fn changed_or_disabled_pack_state_refuses_start() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();

    sqlx::query(
        "UPDATE v3_pack_streams
         SET availability = 'disabled', generation = generation + 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap();

    assert!(database.start_pack_task(&context).await.is_err());
}

#[tokio::test]
async fn revoked_or_removed_pack_refuses_start() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    quarantine_stream(database.pool()).await;
    sqlx::query(
        "UPDATE v3_pack_publishers SET trust_state = 'revoked', revoked_at = ?, updated_at = ?
         WHERE publisher_key_id = 'publisher-1'",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap();
    assert!(database.start_pack_task(&context).await.is_err());

    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    sqlx::query(
        "UPDATE v3_pack_streams
         SET active_release_sequence = NULL, rollback_release_sequence = NULL,
             availability = 'removed', generation = generation + 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases
         SET lifecycle_state = 'removed', artifact_cleanup_pending = 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap();
    assert!(database.start_pack_task(&context).await.is_err());
}

#[tokio::test]
async fn rollback_replacing_the_active_release_refuses_start() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO v3_pack_releases (
            publisher_key_id, pack_id, release_sequence, release_id,
            signed_release_sha256, payload_sha256, pack_version, pack_type,
            execution_class, lifecycle_state, quarantine_reason, self_tested_at,
            created_at, updated_at
         ) VALUES ('publisher-1', 'pack-1', 2, 'release-2', ?, ?, '1.0.1',
                   'agent', 'reviewed_typed_workflow', 'staged', NULL, NULL, ?, ?)",
    )
    .bind("c".repeat(64))
    .bind("d".repeat(64))
    .bind(&now)
    .bind(&now)
    .execute(database.pool())
    .await
    .unwrap();
    insert_review(database.pool(), 2).await;
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'self_tested', self_tested_at = ?, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1' AND release_sequence = 2",
    )
    .bind(&now)
    .bind(&now)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'ready', updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1' AND release_sequence = 2",
    )
    .bind(&now)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_streams
         SET active_release_sequence = 2, rollback_release_sequence = 1,
             generation = generation + 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .execute(database.pool())
    .await
    .unwrap();

    assert!(database.start_pack_task(&context).await.is_err());
}

#[tokio::test]
async fn successful_evidence_review_records_one_receipt_and_user_action_atomically() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    crate::test_support::insert_test_job(
        database.pool(),
        "job-1",
        "Coordinator",
        Some("Example"),
        None,
        &Utc::now().to_rfc3339(),
    )
    .await;
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();
    let receipt = local_receipt(&context);

    let succeeded = database
        .complete_reviewed_pack_task(&context.run_id, &receipt, "job-1", None)
        .await
        .unwrap();

    assert_eq!(
        succeeded.status,
        crate::pack_tasks::PackTaskStatus::Succeeded
    );
    assert_eq!(succeeded.receipt_id.as_deref(), Some("receipt-1"));
    let receipt_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_privacy_receipts")
        .fetch_one(database.pool())
        .await
        .unwrap();
    let events: Vec<(String, String, bool)> =
        sqlx::query_as("SELECT event_kind, origin, user_action FROM v3_job_events")
            .fetch_all(database.pool())
            .await
            .unwrap();
    assert_eq!(receipt_count, 1);
    assert_eq!(
        events,
        vec![(
            "privacy_receipt_recorded".to_string(),
            "user".to_string(),
            true
        )]
    );
}

#[tokio::test]
async fn invalid_completion_leaves_started_run_and_audit_empty() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();
    let mut receipt = local_receipt(&context);
    receipt.task_id = "wrong-task".to_string();

    assert!(database
        .complete_reviewed_pack_task(&context.run_id, &receipt, "missing-job", None)
        .await
        .is_err());
    assert_started_without_audit(&database, &context.run_id).await;
}

#[tokio::test]
async fn missing_completion_case_leaves_started_run_and_audit_empty() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();

    assert!(database
        .complete_reviewed_pack_task(
            &context.run_id,
            &local_receipt(&context),
            "missing-job",
            None,
        )
        .await
        .is_err());
    assert_started_without_audit(&database, &context.run_id).await;
}

#[tokio::test]
async fn database_triggers_reject_context_mutation_deletion_and_invalid_transitions() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();

    for attack in [
        "UPDATE pack_task_runs SET input_sha256 = 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc' WHERE run_id = 'run-1'",
        "UPDATE pack_task_runs SET status = 'succeeded', receipt_id = 'receipt-1', completed_at = '2026-07-22T00:00:00+00:00' WHERE run_id = 'run-1'",
        "DELETE FROM pack_task_runs WHERE run_id = 'run-1'",
    ] {
        assert!(sqlx::query(attack).execute(database.pool()).await.is_err(), "{attack}");
    }
}

async fn ready_context(pool: &SqlitePool) -> crate::pack_tasks::PackTaskContext {
    let now = Utc::now().to_rfc3339();
    let digest = "a".repeat(64);
    sqlx::query(
        "INSERT INTO v3_pack_publishers (
            publisher_key_id, public_key_sha256, trust_state, revoked_at, created_at, updated_at
         ) VALUES ('publisher-1', ?, 'trusted', NULL, ?, ?)",
    )
    .bind(&digest)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_pack_streams (
            publisher_key_id, pack_id, high_water_sequence,
            high_water_signed_release_sha256, active_release_sequence,
            rollback_release_sequence, availability, generation, created_at, updated_at
         ) VALUES ('publisher-1', 'pack-1', 0, NULL, NULL, NULL, 'quarantined', 0, ?, ?)",
    )
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_pack_releases (
            publisher_key_id, pack_id, release_sequence, release_id,
            signed_release_sha256, payload_sha256, pack_version, pack_type,
            execution_class, lifecycle_state, quarantine_reason, self_tested_at,
            created_at, updated_at
         ) VALUES ('publisher-1', 'pack-1', 1, 'release-1', ?, ?, '1.0.0',
                   'agent', 'reviewed_typed_workflow', 'staged', NULL, NULL, ?, ?)",
    )
    .bind(&digest)
    .bind("b".repeat(64))
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    insert_review(pool, 1).await;
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'self_tested', self_tested_at = ?, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'ready', updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_streams
         SET active_release_sequence = 1, availability = 'ready', generation = generation + 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    crate::pack_tasks::PackTaskContext {
        run_id: "run-1".to_string(),
        approval_reference: "approval-1".to_string(),
        publisher_key_id: "publisher-1".to_string(),
        pack_id: "pack-1".to_string(),
        release_sequence: 1,
        signed_release_sha256: "a".repeat(64),
        stream_generation: 2,
        task_kind: AgentTaskKind::EvidenceReview,
        task_id: "evidence-review-1".to_string(),
        input_sha256: "b".repeat(64),
        privacy_labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        data_categories: vec![DataCategory::ResumeEvidence],
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(5),
    }
}

async fn insert_review(pool: &SqlitePool, release_sequence: i64) {
    sqlx::query(
        r#"INSERT INTO pack_release_reviews (
            publisher_key_id, pack_id, release_sequence, publisher_name,
            license, minimum_app_version, maximum_app_version, payload_bytes,
            fixture_summary, privacy_labels_json, data_categories_json,
            task_kinds_json, actions_json, approval_gates_json,
            gateway_policy_id, external_destinations_json
         ) VALUES (
            'publisher-1', 'pack-1', ?, 'Publisher', 'MIT', '3.0.0',
            '3.0.0', 1, 'Reviewed task fixture', '["local_only","sensitive"]',
            '["resume_evidence"]', '["evidence_review"]',
            '["read_selected_resume_evidence","write_local_event"]',
            '["per_execution_review"]', NULL, '[]'
         )"#,
    )
    .bind(release_sequence)
    .execute(pool)
    .await
    .unwrap();
}

fn local_receipt(context: &crate::pack_tasks::PackTaskContext) -> PrivacyReceipt {
    PrivacyReceipt {
        schema: SchemaId::PrivacyReceiptV1,
        receipt_id: "receipt-1".to_string(),
        task_id: context.task_id.clone(),
        pack_id: Some(context.pack_id.clone()),
        labels: context.privacy_labels.clone(),
        data_categories: context.data_categories.clone(),
        stored_locally: true,
        data_left_device: false,
        external_destination: None,
        gateway_policy_id: None,
        approval_reference: None,
        delete_or_revoke_action: "delete-local-receipt".to_string(),
    }
}

async fn quarantine_stream(pool: &SqlitePool) {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE v3_pack_streams
         SET active_release_sequence = NULL, rollback_release_sequence = NULL,
             availability = 'quarantined', generation = generation + 1, updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases
         SET lifecycle_state = 'quarantined', quarantine_reason = 'trust_revoked', updated_at = ?
         WHERE publisher_key_id = 'publisher-1' AND pack_id = 'pack-1'",
    )
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
}
