//! Proves draft-packet source changes cannot interleave with atomic audit completion.

use super::*;

#[tokio::test]
async fn draft_packet_completion_rolls_back_a_change_during_receipt_insertion() {
    let database = migrated_database().await;
    let (context, guard) = started_draft_packet(&database).await;
    sqlx::query(
        "CREATE TRIGGER mutate_packet_source_before_receipt
         BEFORE INSERT ON v3_privacy_receipts
         BEGIN
             UPDATE jobs
             SET title = 'Changed during completion', updated_at = '2026-07-22T12:01:00+00:00'
             WHERE hash = 'job-1';
         END",
    )
    .execute(database.pool())
    .await
    .unwrap();

    assert!(database
        .complete_reviewed_pack_task(
            &context.run_id,
            &local_receipt(&context),
            "job-1",
            Some(&guard),
        )
        .await
        .is_err());
    let run = database
        .get_pack_task(&context.run_id)
        .await
        .unwrap()
        .unwrap();
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM v3_privacy_receipts)
              + (SELECT COUNT(*) FROM v3_job_events)",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    let title: String = sqlx::query_scalar("SELECT title FROM jobs WHERE hash = 'job-1'")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(run.status, crate::pack_tasks::PackTaskStatus::Started);
    assert!(run.receipt_id.is_none());
    assert_eq!(audit_rows, 0);
    assert_eq!(title, "Coordinator");
}

#[tokio::test]
async fn current_guard_cannot_replace_the_guard_bound_to_the_approval() {
    let database = migrated_database().await;
    let (context, _) = started_draft_packet(&database).await;
    sqlx::query(
        "UPDATE jobs
         SET title = 'Changed after approval', updated_at = '2026-07-22T12:01:00+00:00'
         WHERE hash = 'job-1'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    let replacement = database
        .read_draft_packet_input_guard("job-1", 1)
        .await
        .unwrap();

    assert!(database
        .complete_reviewed_pack_task(
            &context.run_id,
            &local_receipt(&context),
            "job-1",
            Some(&replacement),
        )
        .await
        .is_err());
    let run = database
        .get_pack_task(&context.run_id)
        .await
        .unwrap()
        .unwrap();
    let audit_rows: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM v3_privacy_receipts)
              + (SELECT COUNT(*) FROM v3_job_events)",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(run.status, crate::pack_tasks::PackTaskStatus::Started);
    assert!(run.receipt_id.is_none());
    assert_eq!(audit_rows, 0);
}

async fn started_draft_packet(
    database: &crate::Database,
) -> (
    crate::pack_tasks::PackTaskContext,
    crate::pack_tasks::DraftPacketInputGuard,
) {
    let mut context = ready_context(database.pool()).await;
    context.task_kind = AgentTaskKind::DraftPacket;
    context.task_id = "draft-packet-1".to_string();
    context.data_categories = vec![DataCategory::PublicJobPosting, DataCategory::ResumeEvidence];
    packet_source(database).await;
    let guard = database
        .read_draft_packet_input_guard("job-1", 1)
        .await
        .unwrap();
    context.input_sha256 = guard.digest().to_string();
    database.create_pack_task(&context).await.unwrap();
    database.start_pack_task(&context).await.unwrap();
    (context, guard)
}

async fn packet_source(database: &crate::Database) {
    crate::test_support::insert_test_job(
        database.pool(),
        "job-1",
        "Coordinator",
        Some("Example"),
        None,
        "2026-07-22T12:00:00+00:00",
    )
    .await;
    sqlx::query("UPDATE jobs SET description = 'Required: scheduling' WHERE hash = 'job-1'")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO resumes (id, name, file_path, parsed_text, updated_at)
         VALUES (1, 'Resume', 'resume.txt', 'Managed scheduling.', '2026-07-22T12:00:00+00:00')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO resume_job_matches (id, resume_id, job_hash, overall_match_score)
         VALUES (1, 1, 'job-1', 0.8)",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO opportunity_case_files (case_file_id, job_hash, created_at)
         VALUES ('00000000-0000-4000-8000-000000000001', 'job-1', '2026-07-22T12:00:00+00:00')",
    )
    .execute(database.pool())
    .await
    .unwrap();
}
