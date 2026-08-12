//! Verifies source-policy migrations preserve governance and advance compatibility metadata.

use super::*;

#[tokio::test]
async fn migration_13_backfills_existing_policy_once() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(12, database.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO v3_source_policies (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision,
            restriction_reason_code, reviewed_at, created_at, updated_at
         ) VALUES (
            'existing-source', 'official_public_api', 'scheduled_public', 10,
            1, 'existing-policy', 4, NULL, ?, ?, ?
         )",
    )
    .bind("2026-07-19T00:00:00Z")
    .bind("2026-07-19T00:00:01Z")
    .bind("2026-07-19T00:00:02Z")
    .execute(database.pool())
    .await
    .unwrap();
    MIGRATOR.run(database.pool()).await.unwrap();

    let history: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT source_id, revision, recorded_at
         FROM v3_source_policy_ledger",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        history,
        vec![(
            "existing-source".to_string(),
            4,
            "2026-07-19T00:00:02Z".to_string(),
        )]
    );
    let migration_version: i64 =
        sqlx::query_scalar("SELECT migration_version FROM v3_compatibility_metadata")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(migration_version, 26);
}

#[tokio::test]
async fn migration_16_retires_yc_startup_source_metadata() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(15, database.pool()).await.unwrap();
    let before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM scraper_config WHERE scraper_name = 'yc_startup'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(before, 1);

    MIGRATOR.run(database.pool()).await.unwrap();

    let after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM scraper_config WHERE scraper_name = 'yc_startup'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(after, 0);
}

#[tokio::test]
async fn migration_17_disables_jobswithgpt_health_metadata() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(16, database.pool()).await.unwrap();
    let before: i64 = sqlx::query_scalar(
        "SELECT is_enabled FROM scraper_config WHERE scraper_name = 'jobswithgpt'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(before, 1);

    MIGRATOR.run(database.pool()).await.unwrap();

    let after: i64 = sqlx::query_scalar(
        "SELECT is_enabled FROM scraper_config WHERE scraper_name = 'jobswithgpt'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(after, 0);
    assert!(sqlx::query(
        "UPDATE scraper_config SET is_enabled = 1 WHERE scraper_name = 'jobswithgpt'"
    )
    .execute(database.pool())
    .await
    .is_err());
    assert!(sqlx::query(
        "UPDATE scraper_config SET is_enabled = NULL WHERE scraper_name = 'jobswithgpt'"
    )
    .execute(database.pool())
    .await
    .is_err());
}

#[tokio::test]
async fn migration_18_retires_restricted_source_health_metadata() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(17, database.pool()).await.unwrap();

    MIGRATOR.run(database.pool()).await.unwrap();

    for source_id in ["builtin", "dice", "simplyhired", "glassdoor"] {
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM scraper_config WHERE scraper_name = ?")
                .bind(source_id)
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(rows, 0, "{source_id} health metadata was not retired");
        assert!(
            sqlx::query("INSERT INTO scraper_config (scraper_name, display_name) VALUES (?, ?)")
                .bind(source_id)
                .bind(source_id)
                .execute(database.pool())
                .await
                .is_err(),
            "{source_id} health metadata could be restored"
        );
        assert!(
            sqlx::query(
                "UPDATE scraper_config SET scraper_name = ? WHERE scraper_name = 'greenhouse'"
            )
            .bind(source_id)
            .execute(database.pool())
            .await
            .is_err(),
            "{source_id} health metadata could be restored by renaming a row"
        );
    }

    let migration_version: i64 =
        sqlx::query_scalar("SELECT migration_version FROM v3_compatibility_metadata")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(migration_version, 26);
}

#[tokio::test]
async fn migration_19_preserves_scheduled_reviews_and_allows_user_opened_reviews() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(18, database.pool()).await.unwrap();
    sqlx::query(
        "INSERT INTO v3_source_policies (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision,
            restriction_reason_code, reviewed_at, created_at, updated_at
         ) VALUES (
            'migration-source', 'restricted_public_scheduled',
            'scheduled_public', 1, 1, 'migration-source-policy', 1,
            'terms-review', ?, ?, ?
         )",
    )
    .bind("2026-07-19T00:00:00Z")
    .bind("2026-07-19T00:00:00Z")
    .bind("2026-07-19T00:00:00Z")
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         ) VALUES (
            'migration-consent', NULL, 'migration-source', 'scheduled_check',
            1, 1, 'migration-source-policy', 1,
            'restricted_public_scheduled', '[\"public_job_posting\"]', ?, ?,
            'granted', ?
         )",
    )
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind("2026-07-19T00:00:00Z")
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         ) VALUES (
            'migration-revocation', 'migration-consent',
            'migration-source', 'scheduled_check', 1, 1,
            'migration-source-policy', 1, 'restricted_public_scheduled',
            '[\"public_job_posting\"]', ?, ?, 'revoked', ?
         )",
    )
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind("2026-07-19T00:00:01Z")
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_source_policies (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision,
            restriction_reason_code, reviewed_at, created_at, updated_at
         ) VALUES (
            'linkedin-workbench', 'restricted_user_opened', 'user_opened', 0,
            1, 'jobsentinel.source-policy.linkedin-workbench', 1,
            'account-backed-source', ?, ?, ?
         )",
    )
    .bind("2026-07-19T00:00:00Z")
    .bind("2026-07-19T00:00:00Z")
    .bind("2026-07-19T00:00:00Z")
    .execute(database.pool())
    .await
    .unwrap();

    MIGRATOR.run(database.pool()).await.unwrap();

    let operation_check: String = sqlx::query_scalar(
        "SELECT sql FROM sqlite_schema
         WHERE type = 'table' AND name = 'v3_source_consent_events'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert!(operation_check.contains("'restricted_workbench'"));
    let preserved: Vec<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT event_id, previous_event_id, decision
         FROM v3_source_consent_events
         WHERE source_id = 'migration-source'
         ORDER BY sequence",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        preserved,
        vec![
            ("migration-consent".to_string(), None, "granted".to_string()),
            (
                "migration-revocation".to_string(),
                Some("migration-consent".to_string()),
                "revoked".to_string(),
            ),
        ]
    );
    sqlx::query(
        "INSERT INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         ) VALUES (
            'migration-regrant', 'migration-revocation',
            'migration-source', 'scheduled_check', 1, 1,
            'migration-source-policy', 1, 'restricted_public_scheduled',
            '[\"public_job_posting\"]', ?, ?, 'granted', ?
         )",
    )
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind("2026-07-19T00:00:02Z")
    .execute(database.pool())
    .await
    .unwrap();
    let foreign_key_violations: Vec<(String, i64, String, i64)> =
        sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(database.pool())
            .await
            .unwrap();
    assert!(foreign_key_violations.is_empty());
    assert!(sqlx::query("DELETE FROM v3_source_consent_events")
        .execute(database.pool())
        .await
        .is_err());
    let migration_version: i64 =
        sqlx::query_scalar("SELECT migration_version FROM v3_compatibility_metadata")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(migration_version, 26);
}
