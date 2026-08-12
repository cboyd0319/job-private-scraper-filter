//! Proves pack lifecycle schema migration and database-level transition guards.

use super::*;

async fn seed_pack_lifecycle(database: &Database, release_digest: &str, payload_digest: &str) {
    sqlx::query(
        "INSERT INTO v3_pack_publishers (
            publisher_key_id, public_key_sha256, trust_state,
            revoked_at, created_at, updated_at
         ) VALUES ('publisher', ?, 'trusted', NULL, 'now', 'now')",
    )
    .bind(release_digest)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_pack_streams (
            publisher_key_id, pack_id, high_water_sequence,
            high_water_signed_release_sha256, active_release_sequence,
            rollback_release_sequence, availability, generation,
            created_at, updated_at
         ) VALUES ('publisher', 'pack', 0, NULL, NULL, NULL,
                   'quarantined', 0, 'now', 'now')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_pack_releases (
            publisher_key_id, pack_id, release_sequence, release_id,
            signed_release_sha256, payload_sha256, pack_version,
            pack_type, execution_class, lifecycle_state,
            quarantine_reason, self_tested_at, created_at, updated_at
         ) VALUES ('publisher', 'pack', 1, 'release', ?, ?, '1.0.0',
                   'source', 'static_content', 'staged', NULL, NULL, 'now', 'now')",
    )
    .bind(release_digest)
    .bind(payload_digest)
    .execute(database.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn migration_23_adds_transactional_pack_lifecycle_tables() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(22, database.pool()).await.unwrap();

    MIGRATOR.run_to(23, database.pool()).await.unwrap();

    let tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table'
           AND name IN (
               'v3_pack_publishers',
               'v3_pack_streams',
               'v3_pack_releases'
           )",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    let migration_version: i64 =
        sqlx::query_scalar("SELECT migration_version FROM v3_compatibility_metadata")
            .fetch_one(database.pool())
            .await
            .unwrap();

    assert_eq!(tables, 3);
    assert_eq!(migration_version, 23);
}

#[tokio::test]
async fn migration_26_conservatively_marks_removed_release_cleanup_pending() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run_to(25, database.pool()).await.unwrap();
    let digest = "a".repeat(64);
    let payload_digest = "b".repeat(64);
    seed_pack_lifecycle(&database, &digest, &payload_digest).await;
    sqlx::query(
        "UPDATE v3_pack_streams
         SET availability = 'removed', generation = generation + 1
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'removed'
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();

    MIGRATOR.run_to(26, database.pool()).await.unwrap();

    let cleanup_pending: bool = sqlx::query_scalar(
        "SELECT artifact_cleanup_pending FROM v3_pack_releases
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    let migration_version: i64 =
        sqlx::query_scalar("SELECT migration_version FROM v3_compatibility_metadata")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(cleanup_pending);
    assert_eq!(migration_version, 26);
}

#[tokio::test]
async fn pack_lifecycle_triggers_reject_state_bypass_history_loss_and_retrust() {
    let database = Database::connect_memory().await.unwrap();
    MIGRATOR.run(database.pool()).await.unwrap();
    let digest = "a".repeat(64);
    let payload_digest = "b".repeat(64);
    seed_pack_lifecycle(&database, &digest, &payload_digest).await;
    sqlx::query(
        r#"INSERT INTO pack_release_reviews (
            publisher_key_id, pack_id, release_sequence, publisher_name,
            license, minimum_app_version, maximum_app_version, payload_bytes,
            fixture_summary, privacy_labels_json, data_categories_json,
            task_kinds_json, actions_json, approval_gates_json,
            gateway_policy_id, external_destinations_json
         ) VALUES (
            'publisher', 'pack', 1, 'Publisher', 'MIT', '3.0.0', '3.0.0',
            1, 'Source fixture', '["local_only"]', '[]', '[]', '[]', '[]',
            NULL, '[]'
         )"#,
    )
    .execute(database.pool())
    .await
    .unwrap();

    assert!(sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'ready',
                self_tested_at = 'now'
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'"
    )
    .execute(database.pool())
    .await
    .is_err());
    assert!(sqlx::query(
        "UPDATE v3_pack_streams SET active_release_sequence = 1,
                availability = 'ready', generation = generation + 1
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'"
    )
    .execute(database.pool())
    .await
    .is_err());
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'self_tested',
                self_tested_at = 'now'
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'ready'
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_streams SET active_release_sequence = 1,
                availability = 'ready', generation = generation + 1
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();

    for attack in [
        "UPDATE v3_pack_streams SET generation = generation + 2 WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
        "UPDATE v3_pack_streams SET high_water_sequence = 0, high_water_signed_release_sha256 = NULL, generation = generation + 1 WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
        "UPDATE v3_pack_releases SET signed_release_sha256 = 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc' WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
        "UPDATE v3_pack_publishers SET trust_state = 'revoked', revoked_at = 'now' WHERE publisher_key_id = 'publisher'",
        "DELETE FROM v3_pack_streams WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
        "DELETE FROM v3_pack_releases WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
        "DELETE FROM v3_pack_publishers WHERE publisher_key_id = 'publisher'",
    ] {
        assert!(sqlx::query(attack).execute(database.pool()).await.is_err());
    }

    sqlx::query(
        "UPDATE v3_pack_streams SET active_release_sequence = NULL,
                availability = 'quarantined', generation = generation + 1
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_releases SET lifecycle_state = 'quarantined',
                quarantine_reason = 'trust_revoked'
         WHERE publisher_key_id = 'publisher' AND pack_id = 'pack'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE v3_pack_publishers SET trust_state = 'revoked',
                revoked_at = 'now' WHERE publisher_key_id = 'publisher'",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert!(sqlx::query(
        "UPDATE v3_pack_publishers SET trust_state = 'trusted', revoked_at = NULL
         WHERE publisher_key_id = 'publisher'"
    )
    .execute(database.pool())
    .await
    .is_err());
}
