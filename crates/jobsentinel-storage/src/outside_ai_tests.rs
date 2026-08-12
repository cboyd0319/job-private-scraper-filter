use chrono::{Duration, Utc};
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{DataCategory, PrivacyLabel, PrivacyReceipt, EXTERNAL_AI_GATEWAY_POLICY},
};

use crate::{
    outside_ai::{OutsideAiCompletion, OutsideAiContext, OutsideAiStatus},
    test_support::migrated_database,
    Database, ReviewedExportSelection,
};

const DESTINATION: &str = "https://api.provider.example/requests";

fn context() -> OutsideAiContext {
    OutsideAiContext {
        feature_id: "job-description-summary".to_string(),
        provider_id: "provider-a".to_string(),
        destination: DESTINATION.to_string(),
        request_sha256: "a".repeat(64),
        labels: vec![
            PrivacyLabel::ExternalAiOptional,
            PrivacyLabel::PublicDataOnly,
        ],
        data_categories: vec![DataCategory::PublicJobPosting],
        gateway_policy_revision: EXTERNAL_AI_GATEWAY_POLICY.to_string(),
    }
}

fn external_receipt(operation_id: &str, approval_reference: &str) -> PrivacyReceipt {
    PrivacyReceipt {
        schema: SchemaId::PrivacyReceiptV1,
        receipt_id: format!("receipt-{operation_id}"),
        task_id: operation_id.to_string(),
        pack_id: None,
        labels: context().labels,
        data_categories: context().data_categories,
        stored_locally: true,
        data_left_device: true,
        external_destination: Some(DESTINATION.to_string()),
        gateway_policy_id: Some(EXTERNAL_AI_GATEWAY_POLICY.to_string()),
        approval_reference: Some(approval_reference.to_string()),
        delete_or_revoke_action: "delete_local_receipt".to_string(),
    }
}

#[tokio::test]
async fn reviewed_request_starts_once_only_for_the_exact_unexpired_context() {
    let database = migrated_database().await;
    let reviewed = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();
    assert_eq!(reviewed.status, OutsideAiStatus::Pending);
    assert_eq!(
        database
            .get_outside_ai_operation_by_approval(&reviewed.approval_reference)
            .await
            .unwrap(),
        Some(reviewed.clone())
    );

    let mut mismatch = context();
    mismatch.request_sha256 = "b".repeat(64);
    assert!(database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &mismatch,
        )
        .await
        .is_err());

    let started = database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context(),
        )
        .await
        .unwrap();
    assert_eq!(started.status, OutsideAiStatus::Started);
    assert!(started.started_at.is_some());
    assert!(database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context(),
        )
        .await
        .is_err());
}

#[tokio::test]
async fn reviews_accept_only_the_current_public_summary_scope() {
    let database = migrated_database().await;
    let expires_at = Utc::now() + Duration::minutes(10);

    for destination in [
        "http://provider.example/v1",
        "https://localhost/v1",
        "https://user:secret@provider.example/v1",
        "https://provider.example/v1?query=private",
    ] {
        let mut invalid = context();
        invalid.destination = destination.to_string();
        assert!(database
            .create_outside_ai_review(&invalid, expires_at)
            .await
            .is_err());
    }
    let mut invalid = context();
    invalid.feature_id = "resume-draft".to_string();
    assert!(database
        .create_outside_ai_review(&invalid, expires_at)
        .await
        .is_err());
    invalid = context();
    invalid.labels = vec![PrivacyLabel::ExternalAiOptional, PrivacyLabel::Sensitive];
    invalid.data_categories = vec![DataCategory::ResumeEvidence];
    assert!(database
        .create_outside_ai_review(&invalid, expires_at)
        .await
        .is_err());
    assert!(database
        .create_outside_ai_review(&context(), Utc::now() - Duration::seconds(1))
        .await
        .is_err());
}

#[tokio::test]
async fn outside_ai_context_and_lifecycle_fail_closed_in_sql() {
    let database = migrated_database().await;
    let reviewed = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();

    assert!(sqlx::query(
        "UPDATE v3_outside_ai_operations SET provider_id = 'different-provider'
         WHERE operation_id = ?",
    )
    .bind(&reviewed.operation_id)
    .execute(database.pool())
    .await
    .is_err());
    assert!(database
        .finish_outside_ai_request(&reviewed.operation_id, OutsideAiCompletion::Succeeded)
        .await
        .is_err());
    assert!(sqlx::query(
        "UPDATE v3_outside_ai_operations
         SET status = 'started', started_at = ?
         WHERE operation_id = ?",
    )
    .bind((reviewed.expires_at + Duration::minutes(1)).to_rfc3339())
    .bind(&reviewed.operation_id)
    .execute(database.pool())
    .await
    .is_err());

    database
        .cancel_outside_ai_review(&reviewed.operation_id)
        .await
        .unwrap();
    assert!(database
        .finish_outside_ai_request(&reviewed.operation_id, OutsideAiCompletion::Failed)
        .await
        .is_err());
    assert!(
        sqlx::query("DELETE FROM v3_outside_ai_operations WHERE operation_id = ?",)
            .bind(&reviewed.operation_id)
            .execute(database.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn external_receipts_require_the_exact_started_operation_and_single_use_approval() {
    let database = migrated_database().await;
    let reviewed = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();
    assert!(database
        .store_privacy_receipt(&external_receipt(
            &reviewed.operation_id,
            &reviewed.approval_reference,
        ))
        .await
        .is_err());
    database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context(),
        )
        .await
        .unwrap();

    let mut injected = serde_json::to_value(external_receipt(
        &reviewed.operation_id,
        &reviewed.approval_reference,
    ))
    .unwrap();
    injected["payload"] = serde_json::json!("PRIVATE");
    assert!(direct_receipt_json_insert(&database, &injected, true)
        .await
        .is_err());

    let mut mismatch = external_receipt(&reviewed.operation_id, &reviewed.approval_reference);
    mismatch.external_destination = Some("https://other.example/different-request".to_string());
    assert!(database.store_privacy_receipt(&mismatch).await.is_err());

    let receipt = external_receipt(&reviewed.operation_id, &reviewed.approval_reference);
    database.store_privacy_receipt(&receipt).await.unwrap();
    assert!(sqlx::query(
        "UPDATE v3_privacy_receipts
         SET receipt_json = json_set(
             receipt_json, '$.approval_reference', 'fabricated-approval'
         )
         WHERE receipt_id = ?",
    )
    .bind(&receipt.receipt_id)
    .execute(database.pool())
    .await
    .is_err());
    assert!(
        sqlx::query("DELETE FROM v3_privacy_receipts WHERE receipt_id = ?")
            .bind(&receipt.receipt_id)
            .execute(database.pool())
            .await
            .is_err()
    );
    let mut duplicate = receipt.clone();
    duplicate.receipt_id = "different-receipt".to_string();
    assert!(database.store_privacy_receipt(&duplicate).await.is_err());

    database
        .finish_outside_ai_request(&reviewed.operation_id, OutsideAiCompletion::Succeeded)
        .await
        .unwrap();
    assert_eq!(
        database
            .get_outside_ai_operation(&reviewed.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Succeeded
    );
}

#[tokio::test]
async fn fabricated_external_refs_and_local_approval_refs_fail_in_sql() {
    let database = migrated_database().await;
    let external = external_receipt("fabricated-operation", "fabricated-approval");
    assert!(direct_receipt_insert(&database, &external).await.is_err());

    let mut local = external;
    local.receipt_id = "local-with-external-approval".to_string();
    local.task_id = "local-task".to_string();
    local.labels = vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive];
    local.data_left_device = false;
    local.external_destination = None;
    local.gateway_policy_id = None;
    assert!(direct_receipt_insert(&database, &local).await.is_err());
}

#[tokio::test]
async fn reconciliation_marks_started_ambiguous_and_expired_pending_cancelled() {
    let database = migrated_database().await;
    let started = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();
    database
        .start_reviewed_outside_ai_request(
            &started.operation_id,
            &started.approval_reference,
            &context(),
        )
        .await
        .unwrap();
    insert_expired_pending(&database).await;

    let reconciled = database.reconcile_outside_ai_operations().await.unwrap();
    assert_eq!(reconciled.ambiguous, 1);
    assert_eq!(reconciled.cancelled, 1);
    let states: Vec<String> =
        sqlx::query_scalar("SELECT status FROM v3_outside_ai_operations ORDER BY status")
            .fetch_all(database.pool())
            .await
            .unwrap();
    assert_eq!(states, ["ambiguous", "cancelled"]);
}

#[tokio::test]
async fn durable_cancellation_wins_against_late_provider_success() {
    let database = migrated_database().await;
    let reviewed = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();
    database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context(),
        )
        .await
        .unwrap();

    assert_eq!(
        database
            .cancel_outside_ai_operation(&reviewed.operation_id)
            .await
            .unwrap(),
        OutsideAiStatus::Ambiguous
    );
    assert!(database
        .finish_outside_ai_request(&reviewed.operation_id, OutsideAiCompletion::Succeeded)
        .await
        .is_err());
}

#[tokio::test]
async fn recent_activity_is_bounded_status_only_recovery_metadata() {
    let database = migrated_database().await;
    let reviewed = database
        .create_outside_ai_review(&context(), Utc::now() + Duration::minutes(10))
        .await
        .unwrap();
    database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context(),
        )
        .await
        .unwrap();
    database
        .finish_outside_ai_request(&reviewed.operation_id, OutsideAiCompletion::Succeeded)
        .await
        .unwrap_err();
    database
        .cancel_outside_ai_operation(&reviewed.operation_id)
        .await
        .unwrap();

    let activity = database.list_recent_outside_ai_activity(20).await.unwrap();

    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].provider_id, "provider-a");
    assert_eq!(activity[0].destination, DESTINATION);
    assert_eq!(activity[0].status, OutsideAiStatus::Ambiguous);
    assert!(activity[0].completed_at.is_some());
    assert!(database.list_recent_outside_ai_activity(0).await.is_err());
    assert!(database.list_recent_outside_ai_activity(51).await.is_err());
}

#[tokio::test]
async fn outside_ai_ledger_is_metadata_only_and_excluded_from_reviewed_export() {
    let database = migrated_database().await;
    let marker = "private-provider-marker";
    let mut marked = context();
    marked.provider_id = marker.to_string();
    database
        .create_outside_ai_review(&marked, Utc::now() + Duration::minutes(10))
        .await
        .unwrap();

    let columns: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('v3_outside_ai_operations')")
            .fetch_all(database.pool())
            .await
            .unwrap();
    for forbidden in [
        "payload",
        "response",
        "credential",
        "prompt",
        "model",
        "private_path",
        "query",
        "raw_error",
    ] {
        assert!(!columns.iter().any(|column| column.contains(forbidden)));
    }

    let temp_dir = tempfile::tempdir().unwrap();
    let export_path = temp_dir.path().join("reviewed.jsonl");
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();
    database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap();
    assert!(!std::fs::read_to_string(export_path)
        .unwrap()
        .contains(marker));
}

async fn direct_receipt_insert(
    database: &Database,
    receipt: &PrivacyReceipt,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    direct_receipt_json_insert(
        database,
        &serde_json::to_value(receipt).unwrap(),
        receipt.data_left_device,
    )
    .await
}

async fn direct_receipt_json_insert(
    database: &Database,
    receipt: &serde_json::Value,
    data_left_device: bool,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO v3_privacy_receipts (
            receipt_id, schema, receipt_json, stored_locally,
            data_left_device, created_at
         ) VALUES (?, 'jobsentinel.v3.privacy-receipt.v1', ?, 1, ?, ?)",
    )
    .bind(receipt["receipt_id"].as_str().unwrap())
    .bind(serde_json::to_string(receipt).unwrap())
    .bind(data_left_device)
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
}

async fn insert_expired_pending(database: &Database) {
    sqlx::query(
        "INSERT INTO v3_outside_ai_operations (
            operation_id, approval_reference, feature_id, provider_id,
            destination, request_sha256, labels_json, data_categories_json,
            gateway_policy_revision, status, created_at, expires_at
         ) VALUES (
            'outside-ai:expired', 'outside-ai-approval:expired',
            'job-description-summary', 'provider-a',
            ?, ?, '[\"external_ai_optional\",\"public_data_only\"]',
            '[\"public_job_posting\"]', 'jobsentinel.external-ai-gateway.v1',
            'pending', ?, ?
         )",
    )
    .bind(DESTINATION)
    .bind("a".repeat(64))
    .bind((Utc::now() - Duration::minutes(20)).to_rfc3339())
    .bind((Utc::now() - Duration::minutes(10)).to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap();
}
