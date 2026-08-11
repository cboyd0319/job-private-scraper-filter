//! Proves an approved pack task cannot commit after its review window expires.

use std::time::Duration as StdDuration;

use super::*;

#[tokio::test]
async fn startup_reconciliation_cancels_expired_pending_and_fails_interrupted_started_runs() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();
    let started_id = context.run_id.clone();
    let mut expired = context.clone();
    expired.run_id = "run-2".to_string();
    expired.approval_reference = "approval-2".to_string();
    expired.expires_at = Utc::now() - Duration::minutes(1);
    expired.created_at = expired.expires_at - Duration::minutes(5);
    insert_pending_directly(database.pool(), &expired).await;

    let repaired = database.reconcile_pack_tasks().await.unwrap();

    assert_eq!(repaired.expired_pending, 1);
    assert_eq!(repaired.interrupted_started, 1);
    assert_eq!(
        database
            .get_pack_task(&started_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        crate::pack_tasks::PackTaskStatus::Failed
    );
    assert_eq!(
        database
            .get_pack_task(&expired.run_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        crate::pack_tasks::PackTaskStatus::Cancelled
    );
}

#[tokio::test]
async fn database_startup_reconciles_interrupted_reviewed_tasks_before_returning() {
    let database = migrated_database().await;
    let context = ready_context(database.pool()).await;
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();

    database.migrate().await.unwrap();

    assert_eq!(
        database
            .get_pack_task(&context.run_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        crate::pack_tasks::PackTaskStatus::Failed
    );
}

#[tokio::test]
async fn approval_must_remain_unexpired_at_completion() {
    let database = migrated_database().await;
    let mut context = ready_context(database.pool()).await;
    context.expires_at = Utc::now() + Duration::seconds(1);
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
    tokio::time::sleep(StdDuration::from_millis(1_100)).await;

    assert!(database
        .complete_reviewed_pack_task(&context.run_id, &local_receipt(&context), "job-1", None,)
        .await
        .is_err());
    assert_eq!(
        database
            .get_pack_task(&context.run_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        crate::pack_tasks::PackTaskStatus::Started
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM v3_privacy_receipts")
            .fetch_one(database.pool())
            .await
            .unwrap(),
        0
    );
}

async fn insert_pending_directly(pool: &SqlitePool, context: &crate::pack_tasks::PackTaskContext) {
    sqlx::query(
        "INSERT INTO pack_task_runs (
            run_id, approval_reference, publisher_key_id, pack_id, release_sequence,
            signed_release_sha256, stream_generation, task_kind, task_id, input_sha256,
            privacy_labels_json, data_categories_json, status, receipt_id, created_at,
            expires_at, started_at, completed_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, 'evidence_review', ?, ?,
                   '[\"local_only\",\"sensitive\"]', '[\"resume_evidence\"]',
                   'pending', NULL, ?, ?, NULL, NULL)",
    )
    .bind(&context.run_id)
    .bind(&context.approval_reference)
    .bind(&context.publisher_key_id)
    .bind(&context.pack_id)
    .bind(i64::try_from(context.release_sequence).unwrap())
    .bind(&context.signed_release_sha256)
    .bind(i64::try_from(context.stream_generation).unwrap())
    .bind(&context.task_id)
    .bind(&context.input_sha256)
    .bind(context.created_at.to_rfc3339())
    .bind(context.expires_at.to_rfc3339())
    .execute(pool)
    .await
    .unwrap();
}
