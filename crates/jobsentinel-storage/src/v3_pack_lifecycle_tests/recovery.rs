// Verifies pack lifecycle revocation and recovery behavior.

use super::*;

#[tokio::test]
async fn publisher_revocation_atomically_quarantines_every_pack_and_cannot_be_reversed() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let active_release = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(active_staged) = database
        .stage_verified_pack(&active_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("active release must stage");
    };
    let active_tested = parse_and_self_test_pack_payload(
        &active_release,
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .unwrap();
    let active_self_tested = database
        .record_pack_self_test(&active_tested, active_staged.generation)
        .await
        .unwrap();
    let active = database
        .activate_self_tested_pack(&active_tested, &publisher, active_self_tested.generation)
        .await
        .unwrap();

    let staged_release = release_for_pack(
        "jobsentinel.test.second-source",
        4,
        valid_source_payload(),
        &publisher,
    );
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&staged_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("second release must stage");
    };

    assert_eq!(database.revoke_pack_publisher(&publisher).await.unwrap(), 2);

    let streams: Vec<(String, String, Option<i64>, Option<i64>, i64)> = sqlx::query_as(
        "SELECT pack_id, availability, active_release_sequence,
                rollback_release_sequence, generation
         FROM v3_pack_streams WHERE publisher_key_id = ? ORDER BY pack_id",
    )
    .bind(PUBLISHER_ID)
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        streams,
        vec![
            (
                "jobsentinel.test.second-source".to_string(),
                "quarantined".to_string(),
                None,
                None,
                i64::try_from(staged.generation + 1).unwrap(),
            ),
            (
                PACK_ID.to_string(),
                "quarantined".to_string(),
                None,
                None,
                i64::try_from(active.generation + 1).unwrap(),
            ),
        ]
    );
    let releases: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT pack_id, lifecycle_state, quarantine_reason
         FROM v3_pack_releases WHERE publisher_key_id = ? ORDER BY pack_id",
    )
    .bind(PUBLISHER_ID)
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        releases,
        vec![
            (
                "jobsentinel.test.second-source".to_string(),
                "quarantined".to_string(),
                Some("trust_revoked".to_string()),
            ),
            (
                PACK_ID.to_string(),
                "quarantined".to_string(),
                Some("trust_revoked".to_string()),
            ),
        ]
    );
    let trust_state: String =
        sqlx::query_scalar("SELECT trust_state FROM v3_pack_publishers WHERE publisher_key_id = ?")
            .bind(PUBLISHER_ID)
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(trust_state, "revoked");
    assert_eq!(
        pack_lifecycle_error_kind(
            &database
                .stage_verified_pack(&release(5, valid_source_payload(), &publisher), &publisher)
                .await
                .unwrap_err()
        ),
        Some("revoked")
    );
    assert_eq!(
        pack_lifecycle_error_kind(
            &database
                .revoke_pack_publisher(&publisher)
                .await
                .unwrap_err()
        ),
        Some("revoked")
    );
}

#[tokio::test]
async fn uninstall_removes_every_release_but_retains_the_downgrade_tombstone() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let first = release(1, valid_source_payload(), &publisher);
    activate_release(&database, &first, &publisher).await;
    let third = release(3, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(third_staged) = database
        .stage_verified_pack(&third, &publisher)
        .await
        .unwrap()
    else {
        panic!("third release must stage");
    };
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

    let removed = database
        .uninstall_pack(PUBLISHER_ID, PACK_ID, third_active.generation)
        .await
        .unwrap();

    assert_eq!(removed.availability, PackAvailability::Removed);
    assert_eq!(removed.high_water_sequence, 3);
    assert_eq!(removed.active_release_sequence, None);
    assert_eq!(removed.rollback_release_sequence, None);
    assert_eq!(removed.generation, third_active.generation + 1);
    let states: Vec<(i64, String)> = sqlx::query_as(
        "SELECT release_sequence, lifecycle_state FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? ORDER BY release_sequence",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        states,
        vec![(1, "removed".to_string()), (3, "removed".to_string())]
    );
    assert_eq!(
        pack_lifecycle_error_kind(
            &database
                .uninstall_pack(PUBLISHER_ID, PACK_ID, third_active.generation)
                .await
                .unwrap_err()
        ),
        Some("stale")
    );
    assert_eq!(
        database
            .stage_verified_pack(&third, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(removed)
    );
}

#[tokio::test]
async fn a_newer_release_restores_an_uninstalled_stream_to_quarantine_once() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let first = release(1, valid_source_payload(), &publisher);
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&first, &publisher)
        .await
        .unwrap()
    else {
        panic!("first release must stage");
    };
    let removed = database
        .uninstall_pack(PUBLISHER_ID, PACK_ID, staged.generation)
        .await
        .unwrap();

    let PackStageOutcome::Staged(restaged) = database
        .stage_verified_pack(&release(2, valid_source_payload(), &publisher), &publisher)
        .await
        .unwrap()
    else {
        panic!("newer release must stage");
    };

    assert_eq!(restaged.availability, PackAvailability::Quarantined);
    assert_eq!(restaged.high_water_sequence, 2);
    assert_eq!(restaged.active_release_sequence, None);
    assert_eq!(restaged.rollback_release_sequence, None);
    assert_eq!(restaged.generation, removed.generation + 1);
}

#[tokio::test]
async fn startup_reconciliation_quarantines_interrupted_candidates_once() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let staged_release = release_for_pack(
        "jobsentinel.test.interrupted-a",
        1,
        valid_source_payload(),
        &publisher,
    );
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&staged_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("first interrupted release must stage");
    };
    let tested_release = release_for_pack(
        "jobsentinel.test.interrupted-b",
        1,
        valid_source_payload(),
        &publisher,
    );
    let PackStageOutcome::Staged(tested_staged) = database
        .stage_verified_pack(&tested_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("second interrupted release must stage");
    };
    let tested_proof = parse_and_self_test_pack_payload(
        &tested_release,
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .unwrap();
    let tested = database
        .record_pack_self_test(&tested_proof, tested_staged.generation)
        .await
        .unwrap();

    let active_release = release_for_pack(
        "jobsentinel.test.interrupted-c",
        1,
        valid_source_payload(),
        &publisher,
    );
    let PackStageOutcome::Staged(active_staged) = database
        .stage_verified_pack(&active_release, &publisher)
        .await
        .unwrap()
    else {
        panic!("active release must stage");
    };
    let active_proof = parse_and_self_test_pack_payload(
        &active_release,
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .unwrap();
    let active_tested = database
        .record_pack_self_test(&active_proof, active_staged.generation)
        .await
        .unwrap();
    database
        .activate_self_tested_pack(&active_proof, &publisher, active_tested.generation)
        .await
        .unwrap();
    let interrupted_update = release_for_pack(
        "jobsentinel.test.interrupted-c",
        2,
        valid_source_payload(),
        &publisher,
    );
    let PackStageOutcome::Staged(update_staged) = database
        .stage_verified_pack(&interrupted_update, &publisher)
        .await
        .unwrap()
    else {
        panic!("interrupted update must stage");
    };

    assert_eq!(
        database
            .reconcile_interrupted_pack_lifecycle()
            .await
            .unwrap(),
        3
    );

    let releases: Vec<(String, i64, String, Option<String>)> = sqlx::query_as(
        "SELECT pack_id, release_sequence, lifecycle_state, quarantine_reason
         FROM v3_pack_releases WHERE pack_id LIKE 'jobsentinel.test.interrupted-%'
         ORDER BY pack_id, release_sequence",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        releases,
        vec![
            (
                "jobsentinel.test.interrupted-a".to_string(),
                1,
                "quarantined".to_string(),
                Some("interrupted".to_string()),
            ),
            (
                "jobsentinel.test.interrupted-b".to_string(),
                1,
                "quarantined".to_string(),
                Some("interrupted".to_string()),
            ),
            (
                "jobsentinel.test.interrupted-c".to_string(),
                1,
                "ready".to_string(),
                None,
            ),
            (
                "jobsentinel.test.interrupted-c".to_string(),
                2,
                "quarantined".to_string(),
                Some("interrupted".to_string()),
            ),
        ]
    );
    let streams: Vec<(String, String, Option<i64>, i64)> = sqlx::query_as(
        "SELECT pack_id, availability, active_release_sequence, generation
         FROM v3_pack_streams WHERE pack_id LIKE 'jobsentinel.test.interrupted-%'
         ORDER BY pack_id",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        streams,
        vec![
            (
                "jobsentinel.test.interrupted-a".to_string(),
                "quarantined".to_string(),
                None,
                i64::try_from(staged.generation + 1).unwrap(),
            ),
            (
                "jobsentinel.test.interrupted-b".to_string(),
                "quarantined".to_string(),
                None,
                i64::try_from(tested.generation + 1).unwrap(),
            ),
            (
                "jobsentinel.test.interrupted-c".to_string(),
                "ready".to_string(),
                Some(1),
                i64::try_from(update_staged.generation + 1).unwrap(),
            ),
        ]
    );
    assert_eq!(
        database
            .reconcile_interrupted_pack_lifecycle()
            .await
            .unwrap(),
        0
    );
    let unchanged_generations: Vec<i64> = sqlx::query_scalar(
        "SELECT generation FROM v3_pack_streams
         WHERE pack_id LIKE 'jobsentinel.test.interrupted-%' ORDER BY pack_id",
    )
    .fetch_all(database.pool())
    .await
    .unwrap();
    assert_eq!(
        unchanged_generations,
        streams
            .iter()
            .map(|(_, _, _, generation)| *generation)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn database_startup_runs_pack_reconciliation_before_returning() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);
    database
        .stage_verified_pack(&release, &publisher)
        .await
        .unwrap();

    database.migrate().await.unwrap();

    let state: (String, Option<String>) = sqlx::query_as(
        "SELECT lifecycle_state, quarantine_reason FROM v3_pack_releases
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = 1",
    )
    .bind(PUBLISHER_ID)
    .bind(PACK_ID)
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(
        state,
        ("quarantined".to_string(), Some("interrupted".to_string()))
    );
}
