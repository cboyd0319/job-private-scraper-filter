//! Proves durable pack-management review facts and artifact-cleanup truth.

use super::*;
use crate::v3_pack_lifecycle::PackReleaseState;

#[tokio::test]
async fn stage_persists_immutable_verified_review() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);

    database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap();

    let review: (String, String, String, String, String, String, String) = sqlx::query_as(
        "SELECT publisher_name, license, privacy_labels_json,
                data_categories_json, task_kinds_json, actions_json,
                approval_gates_json
         FROM pack_release_reviews
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(
        review,
        (
            "JobSentinel Test".to_string(),
            "MIT".to_string(),
            r#"["local_only"]"#.to_string(),
            "[]".to_string(),
            "[]".to_string(),
            "[]".to_string(),
            "[]".to_string(),
        )
    );
    assert!(sqlx::query(
        "UPDATE pack_release_reviews SET publisher_name = 'Altered'
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .execute(database.pool())
    .await
    .is_err());
    assert!(sqlx::query(
        "DELETE FROM pack_release_reviews
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .execute(database.pool())
    .await
    .is_err());

    assert!(matches!(
        database
            .stage_verified_pack(&release, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(_)
    ));
}

#[tokio::test]
async fn removed_release_cleanup_is_durable_and_release_scoped() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    database
        .stage_verified_pack(&release(1, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap();
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release(3, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap()
    else {
        panic!("new release must stage");
    };

    database
        .uninstall_pack(PUBLISHER_ID, PACK_ID, staged.generation)
        .await
        .unwrap();
    let removed = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert!(removed
        .iter()
        .all(|release| release.artifact_cleanup_pending));

    database
        .complete_pack_release_cleanup(&removed[0])
        .await
        .unwrap();
    let partly_cleaned = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert!(!partly_cleaned[0].artifact_cleanup_pending);
    assert!(partly_cleaned[1].artifact_cleanup_pending);

    database
        .stage_verified_pack(&release(4, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap();
    let after_restage = database
        .list_stored_pack_releases(PUBLISHER_ID, PACK_ID)
        .await
        .unwrap();
    assert_eq!(
        after_restage
            .iter()
            .map(|release| release.artifact_cleanup_pending)
            .collect::<Vec<_>>(),
        vec![false, true, false]
    );
    assert!(sqlx::query(
        "UPDATE v3_pack_releases SET artifact_cleanup_pending = 1
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 4",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .execute(database.pool())
    .await
    .is_err());
}

#[tokio::test]
async fn concurrent_exact_cleanup_completion_is_idempotent() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release(1, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage");
    };
    database
        .uninstall_pack(PUBLISHER_ID, PACK_ID, staged.generation)
        .await
        .unwrap();
    let release = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();

    let (first, second) = tokio::join!(
        database.complete_pack_release_cleanup(&release),
        database.complete_pack_release_cleanup(&release),
    );

    assert!(first.is_ok());
    assert!(second.is_ok());
    assert!(
        !database
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
            .await
            .unwrap()
            .artifact_cleanup_pending
    );
}

#[tokio::test]
async fn management_lists_every_stream_and_release_state() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let first = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(first_staged) = database
        .stage_verified_pack(&first, &publisher)
        .await
        .unwrap()
    else {
        panic!("first release must stage");
    };
    let first_tested =
        parse_and_self_test_pack_payload(&first, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    let first_reviewed = database
        .record_pack_self_test(&first_tested, first_staged.generation)
        .await
        .unwrap();
    database
        .activate_self_tested_pack(&first_tested, &publisher, first_reviewed.generation)
        .await
        .unwrap();

    let update = release(3, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(update_staged) = database
        .stage_verified_pack(&update, &publisher)
        .await
        .unwrap()
    else {
        panic!("update must stage");
    };
    let update_tested =
        parse_and_self_test_pack_payload(&update, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    database
        .record_pack_self_test(&update_tested, update_staged.generation)
        .await
        .unwrap();

    let removed_pack_id = "jobsentinel.test.removed-source";
    let removed_release = release_for_pack(removed_pack_id, 1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(removed_staged) = database
        .stage_verified_pack(&removed_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("removed release must stage");
    };
    database
        .uninstall_pack(PUBLISHER_ID, removed_pack_id, removed_staged.generation)
        .await
        .unwrap();

    let management = database.list_pack_management().await.unwrap();

    assert_eq!(management.len(), 2);
    let active = management
        .iter()
        .find(|stream| stream.pack_id == PACK_ID)
        .unwrap();
    assert_eq!(
        active.publisher_public_key_sha256,
        hex::encode(Sha256::digest(publisher.public_key))
    );
    assert_eq!(active.availability, PackAvailability::Ready);
    assert_eq!(active.active_release_sequence, Some(1));
    assert_eq!(active.high_water_sequence, 3);
    assert!(!active.cleanup_pending);
    assert_eq!(active.releases.len(), 2);
    assert_eq!(active.releases[0].lifecycle_state, PackReleaseState::Ready);
    assert_eq!(
        active.releases[1].lifecycle_state,
        PackReleaseState::SelfTested
    );
    assert_eq!(active.releases[1].publisher_name, "JobSentinel Test");
    assert_eq!(active.releases[1].minimum_app_version, "3.0.0");
    assert_eq!(active.releases[1].maximum_app_version, "3.0.0");
    assert_eq!(active.releases[1].privacy_labels, [PrivacyLabel::LocalOnly]);

    let removed = management
        .iter()
        .find(|stream| stream.pack_id == removed_pack_id)
        .unwrap();
    assert_eq!(removed.availability, PackAvailability::Removed);
    assert!(removed.cleanup_pending);
    assert!(removed.releases[0].artifact_cleanup_pending);
    assert_eq!(
        removed.releases[0].lifecycle_state,
        PackReleaseState::Removed
    );
}

#[tokio::test]
async fn release_without_verified_review_cannot_become_actionable() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage");
    };
    let tested =
        parse_and_self_test_pack_payload(&release, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    let reviewed = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER pack_release_reviews_no_delete")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query(
        "DELETE FROM pack_release_reviews
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .execute(database.pool())
    .await
    .unwrap();

    assert!(database
        .activate_self_tested_pack(&tested, &publisher, reviewed.generation)
        .await
        .is_err());
    assert!(database.list_pack_management().await.is_err());

    assert!(matches!(
        database
            .stage_verified_pack(&release, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(_)
    ));
    assert!(database.list_pack_management().await.is_ok());
    database
        .activate_self_tested_pack(&tested, &publisher, reviewed.generation)
        .await
        .unwrap();
}
