use super::*;

#[tokio::test]
async fn staging_is_monotonic_and_exact_replay_is_inert() {
    let database = migrated_database().await;
    let publisher = publisher(7);
    let first = release(1, "synthetic source payload one".to_string(), &publisher);

    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(&first, &publisher)
        .await
        .unwrap()
    else {
        panic!("first release must be staged");
    };
    assert_eq!(staged.high_water_sequence, 1);
    assert_eq!(staged.availability, PackAvailability::Quarantined);
    assert_eq!(staged.generation, 1);

    assert_eq!(
        database
            .stage_verified_pack(&first, &publisher)
            .await
            .unwrap(),
        PackStageOutcome::Replay(staged.clone())
    );

    let equivocation = database
        .stage_verified_pack(
            &release(
                1,
                "synthetic source payload different".to_string(),
                &publisher,
            ),
            &publisher,
        )
        .await
        .unwrap_err();
    assert_eq!(
        pack_lifecycle_error_kind(&equivocation),
        Some("equivocation")
    );

    let third = database
        .stage_verified_pack(
            &release(3, "synthetic source payload three".to_string(), &publisher),
            &publisher,
        )
        .await
        .unwrap();
    assert!(matches!(third, PackStageOutcome::Staged(_)));
    let downgrade = database
        .stage_verified_pack(
            &release(2, "synthetic source payload two".to_string(), &publisher),
            &publisher,
        )
        .await
        .unwrap_err();
    assert_eq!(pack_lifecycle_error_kind(&downgrade), Some("downgrade"));

    let PackStageOutcome::Replay(after_replay) = database
        .stage_verified_pack(&first, &publisher)
        .await
        .unwrap()
    else {
        panic!("retained old release must be an inert replay");
    };
    assert_eq!(after_replay.high_water_sequence, 3);
    assert_eq!(after_replay.active_release_sequence, None);
}

#[tokio::test]
async fn staging_requires_the_publisher_key_used_for_verification() {
    let database = migrated_database().await;
    let verified_publisher = publisher(7);
    let verified = release(
        1,
        "synthetic source payload one".to_string(),
        &verified_publisher,
    );

    let error = database
        .stage_verified_pack(&verified, &publisher(8))
        .await
        .unwrap_err();

    assert_eq!(pack_lifecycle_error_kind(&error), Some("invalid"));
}
