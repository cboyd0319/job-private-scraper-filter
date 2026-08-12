// Verifies durable pack lifecycle transitions and concurrency guards.

use super::*;

#[tokio::test]
async fn self_test_proof_persists_without_activating_the_pack() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage before self-test");
    };
    let tested =
        parse_and_self_test_pack_payload(&release, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();

    let persisted = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();

    assert_eq!(persisted.generation, staged.generation + 1);
    assert_eq!(persisted.availability, PackAvailability::Quarantined);
    assert_eq!(persisted.active_release_sequence, None);
    let (state, tested_at): (String, Option<String>) = sqlx::query_as(
        "SELECT lifecycle_state, self_tested_at
         FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(state, "self_tested");
    assert!(tested_at.is_some());
}

#[tokio::test]
async fn failed_self_test_is_quarantined_without_activating_the_pack() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, "invalid typed source payload".to_string(), &publisher);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage before self-test");
    };

    let quarantined = database
        .quarantine_failed_pack_self_test(&release, staged.generation)
        .await
        .unwrap();

    assert_eq!(quarantined.generation, staged.generation + 1);
    assert_eq!(quarantined.availability, PackAvailability::Quarantined);
    assert_eq!(quarantined.active_release_sequence, None);
    let (state, reason, tested_at): (String, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT lifecycle_state, quarantine_reason, self_tested_at
         FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(state, "quarantined");
    assert_eq!(reason.as_deref(), Some("self_test_failed"));
    assert!(tested_at.is_none());
}

#[tokio::test]
async fn activation_requires_self_test_proof_and_is_explicit() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage before activation");
    };
    let tested =
        parse_and_self_test_pack_payload(&release, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    let self_tested = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();

    let active = database
        .activate_self_tested_pack(&tested, &publisher, self_tested.generation)
        .await
        .unwrap();

    assert_eq!(active.generation, self_tested.generation + 1);
    assert_eq!(active.availability, PackAvailability::Ready);
    assert_eq!(active.active_release_sequence, Some(1));
    assert_eq!(active.rollback_release_sequence, None);
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle_state FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(lifecycle, "ready");
    assert_eq!(
        database
            .stage_verified_pack(&release, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(active)
    );
}

#[tokio::test]
async fn disable_preserves_the_ready_release_and_rejects_stale_generation() {
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
    let self_tested = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();
    let active = database
        .activate_self_tested_pack(&tested, &publisher, self_tested.generation)
        .await
        .unwrap();

    let disabled = database
        .disable_pack(PUBLISHER_ID, PACK_ID, active.generation)
        .await
        .unwrap();

    assert_eq!(disabled.availability, PackAvailability::Disabled);
    assert_eq!(disabled.active_release_sequence, Some(1));
    assert_eq!(disabled.generation, active.generation + 1);
    let stale = database
        .disable_pack(PUBLISHER_ID, PACK_ID, active.generation)
        .await
        .unwrap_err();
    assert_eq!(pack_lifecycle_error_kind(&stale), Some("stale"));
    let enabled = database
        .enable_pack(&tested, &publisher, disabled.generation)
        .await
        .unwrap();
    assert_eq!(enabled.availability, PackAvailability::Ready);
    assert_eq!(enabled.active_release_sequence, Some(1));
    assert_eq!(enabled.generation, disabled.generation + 1);
    let lifecycle: String = sqlx::query_scalar(
        "SELECT lifecycle_state FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(lifecycle, "ready");
}

#[tokio::test]
async fn concurrent_disable_accepts_one_generation_compare_and_swap() {
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
    let self_tested = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();
    let active = database
        .activate_self_tested_pack(&tested, &publisher, self_tested.generation)
        .await
        .unwrap();

    let (left, right) = tokio::join!(
        database.disable_pack(PUBLISHER_ID, PACK_ID, active.generation),
        database.disable_pack(PUBLISHER_ID, PACK_ID, active.generation),
    );

    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    let error = left.err().or_else(|| right.err()).unwrap();
    assert_eq!(pack_lifecycle_error_kind(&error), Some("stale"));
    let state: (String, i64) = sqlx::query_as(
        "SELECT availability, generation FROM v3_pack_streams
         WHERE publisher_key_id = ? AND pack_id = ?",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(state.0, "disabled");
    assert_eq!(state.1, i64::try_from(active.generation + 1).unwrap());
}

#[tokio::test]
async fn rollback_swaps_only_retained_ready_releases_without_lowering_high_water() {
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
    let first_self_tested = database
        .record_pack_self_test(&first_tested, first_staged.generation)
        .await
        .unwrap();
    let first_active = database
        .activate_self_tested_pack(&first_tested, &publisher, first_self_tested.generation)
        .await
        .unwrap();
    let third = release(3, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(third_staged) = database
        .stage_verified_pack(&third, &publisher)
        .await
        .unwrap()
    else {
        panic!("update release must stage");
    };
    assert_eq!(third_staged.active_release_sequence, Some(1));
    assert_eq!(third_staged.availability, PackAvailability::Ready);
    let third_tested =
        parse_and_self_test_pack_payload(&third, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    let third_self_tested = database
        .record_pack_self_test(&third_tested, third_staged.generation)
        .await
        .unwrap();
    let third_active = database
        .activate_self_tested_pack(&third_tested, &publisher, third_self_tested.generation)
        .await
        .unwrap();
    assert_eq!(third_active.active_release_sequence, Some(3));
    assert_eq!(third_active.rollback_release_sequence, Some(1));

    let rolled_back = database
        .rollback_pack(&first_tested, &publisher, third_active.generation)
        .await
        .unwrap();

    assert_eq!(rolled_back.active_release_sequence, Some(1));
    assert_eq!(rolled_back.rollback_release_sequence, Some(3));
    assert_eq!(rolled_back.high_water_sequence, 3);
    assert_eq!(rolled_back.generation, third_active.generation + 1);
    assert_eq!(
        database
            .stage_verified_pack(&first, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(rolled_back.clone())
    );
    assert_eq!(
        database
            .stage_verified_pack(&third, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(rolled_back.clone())
    );
    let downgrade = database
        .stage_verified_pack(&release(2, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap_err();
    assert_eq!(pack_lifecycle_error_kind(&downgrade), Some("downgrade"));
    assert!(rolled_back.generation > first_active.generation);
}
