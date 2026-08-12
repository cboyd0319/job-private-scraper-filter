// Verifies pack artifact quarantine, cleanup, and secure retention transitions.

use super::*;
use crate::v3_pack_lifecycle::{PackQuarantineReason, PackReleaseState};

#[tokio::test]
async fn invalid_active_artifact_is_unlinked_and_quarantined_atomically() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let release = release(1, valid_source_payload(), &publisher);
    let (_, active) = activate_release(&database, &release, &publisher).await;
    let stored = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();

    let quarantined = database
        .quarantine_invalid_active_pack_artifact(&stored, active.generation)
        .await
        .unwrap();

    assert_eq!(quarantined.availability, PackAvailability::Quarantined);
    assert_eq!(quarantined.active_release_sequence, None);
    assert_eq!(quarantined.rollback_release_sequence, None);
    assert_eq!(quarantined.high_water_sequence, 1);
    assert_eq!(quarantined.generation, active.generation + 1);
    let stored = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 1)
        .await
        .unwrap();
    assert_eq!(stored.lifecycle_state, PackReleaseState::Quarantined);
    assert_eq!(
        stored.quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}

#[tokio::test]
async fn valid_retained_artifact_atomically_replaces_an_invalid_active_release() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let first = release(1, valid_source_payload(), &publisher);
    let third = release(3, valid_source_payload(), &publisher);
    let mut generation = 0;
    for release in [&first, &third] {
        let PackStageOutcome::Staged(staged) = database
            .stage_verified_pack(release, &publisher)
            .await
            .unwrap()
        else {
            panic!("release must stage");
        };
        let tested = parse_and_self_test_pack_payload(
            release,
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        )
        .unwrap();
        let self_tested = database
            .record_pack_self_test(&tested, staged.generation)
            .await
            .unwrap();
        generation = database
            .activate_self_tested_pack(&tested, &publisher, self_tested.generation)
            .await
            .unwrap()
            .generation;
    }
    let invalid = database
        .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
        .await
        .unwrap();
    let rollback =
        parse_and_self_test_pack_payload(&first, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();

    let restored = database
        .replace_invalid_active_pack_with_rollback(&invalid, &rollback, &publisher, generation)
        .await
        .unwrap();

    assert_eq!(restored.availability, PackAvailability::Ready);
    assert_eq!(restored.active_release_sequence, Some(1));
    assert_eq!(restored.rollback_release_sequence, None);
    assert_eq!(restored.high_water_sequence, 3);
    assert_eq!(restored.generation, generation + 1);
    assert_eq!(
        database
            .get_stored_pack_release(PUBLISHER_ID, PACK_ID, 3)
            .await
            .unwrap()
            .quarantine_reason,
        Some(PackQuarantineReason::IntegrityFailed)
    );
}
