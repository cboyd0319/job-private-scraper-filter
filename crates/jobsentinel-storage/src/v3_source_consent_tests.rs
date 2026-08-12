use chrono::Utc;
use jobsentinel_domain::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::{DataCategory, SourceClass},
    v3_source_consent::{
        SourceConsentContext, SourceConsentOperation, SourceConsentReviewReason,
        SourceConsentStatus,
    },
};

use crate::test_support::migrated_database;

fn policy(revision: u32) -> SourcePolicy {
    SourcePolicy {
        source_id: "dice".to_string(),
        source_class: SourceClass::RestrictedPublicScheduled,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour: 1,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.dice".to_string(),
        revision,
        restriction_reason_code: Some("terms-review".to_string()),
        reviewed_at: Utc::now(),
    }
}

fn context(revision: u32) -> SourceConsentContext {
    SourceConsentContext {
        source_id: "dice".to_string(),
        operation: SourceConsentOperation::ScheduledCheck,
        warning_version: 1,
        behavior_revision: revision,
        policy_ref: "jobsentinel.source-policy.dice".to_string(),
        policy_revision: revision,
        source_class: SourceClass::RestrictedPublicScheduled,
        data_categories: vec![
            DataCategory::PublicJobPosting,
            DataCategory::CareerGoals,
            DataCategory::LocationPreferences,
        ],
        destination_sha256: "a".repeat(64),
        request_sha256: "b".repeat(64),
    }
}

fn workbench_policy() -> SourcePolicy {
    SourcePolicy {
        source_id: "linkedin-workbench".to_string(),
        source_class: SourceClass::RestrictedUserOpened,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.linkedin-workbench".to_string(),
        revision: 1,
        restriction_reason_code: Some("account-backed-source".to_string()),
        reviewed_at: Utc::now(),
    }
}

fn workbench_context() -> SourceConsentContext {
    SourceConsentContext {
        source_id: "linkedin-workbench".to_string(),
        operation: SourceConsentOperation::RestrictedWorkbench,
        warning_version: 1,
        behavior_revision: 1,
        policy_ref: "jobsentinel.source-policy.linkedin-workbench".to_string(),
        policy_revision: 1,
        source_class: SourceClass::RestrictedUserOpened,
        data_categories: vec![
            DataCategory::PublicJobPosting,
            DataCategory::ApplicationHistory,
            DataCategory::CareerGoals,
        ],
        destination_sha256: "c".repeat(64),
        request_sha256: "d".repeat(64),
    }
}

#[tokio::test]
async fn policy_history_is_bounded_ordered_and_database_immutable() {
    let database = migrated_database().await;
    database.upsert_source_policy(&policy(1)).await.unwrap();
    database.upsert_source_policy(&policy(2)).await.unwrap();

    let history = database.source_policy_history("dice", 100).await.unwrap();
    assert_eq!(
        history
            .iter()
            .map(|entry| entry.policy.revision)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert!(history[0].sequence > history[1].sequence);
    assert_eq!(
        database
            .source_policy_history("dice", 1)
            .await
            .unwrap()
            .len(),
        1
    );

    assert!(
        sqlx::query("UPDATE v3_source_policy_ledger SET policy_ref = 'changed'")
            .execute(database.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM v3_source_policy_ledger")
        .execute(database.pool())
        .await
        .is_err());
    assert!(
        sqlx::query("UPDATE v3_source_policies SET source_id = 'dice-renamed', revision = 3")
            .execute(database.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM v3_source_policies")
        .execute(database.pool())
        .await
        .is_err());
    assert!(sqlx::query(
        "INSERT INTO v3_source_policy_ledger (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision, restriction_reason_code,
            reviewed_at, recorded_at
         ) VALUES (
            'poison', 'restricted_public_scheduled', 'disabled', 1,
            0, 'policy', 1, NULL, ?, ?
         )"
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .is_err());
    assert!(sqlx::query(
        "INSERT OR REPLACE INTO v3_source_policy_ledger (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision, restriction_reason_code,
            reviewed_at, recorded_at
         )
         SELECT source_id, source_class, access, request_limit_per_hour,
                user_review_required, policy_ref, revision, restriction_reason_code,
                reviewed_at, recorded_at
         FROM v3_source_policy_ledger
         WHERE source_id = 'dice' AND revision = 2"
    )
    .execute(database.pool())
    .await
    .is_err());
}

#[tokio::test]
async fn exact_source_consent_resets_on_policy_drift_and_regrants_after_revoke() {
    let database = migrated_database().await;
    database.upsert_source_policy(&policy(1)).await.unwrap();
    let first_context = context(1);

    assert_eq!(
        database
            .source_consent_status(&first_context)
            .await
            .unwrap(),
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::Missing,
            latest_event_id: None,
        }
    );
    let grant = database
        .grant_source_consent(&first_context, None)
        .await
        .unwrap();
    assert_eq!(
        database
            .source_consent_status(&first_context)
            .await
            .unwrap(),
        SourceConsentStatus::Remembered {
            event_id: grant.event_id.clone(),
        }
    );

    database.upsert_source_policy(&policy(2)).await.unwrap();
    assert_eq!(
        database
            .source_consent_status(&first_context)
            .await
            .unwrap(),
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::ContextChanged,
            latest_event_id: Some(grant.event_id),
        }
    );

    assert!(database
        .revoke_source_consent("dice", SourceConsentOperation::ScheduledCheck)
        .await
        .unwrap());
    assert!(!database
        .revoke_source_consent("dice", SourceConsentOperation::ScheduledCheck)
        .await
        .unwrap());
    let revoked = database.source_consent_status(&context(2)).await.unwrap();
    let SourceConsentStatus::ReviewRequired {
        reason: SourceConsentReviewReason::Revoked,
        latest_event_id: Some(revocation_id),
    } = revoked
    else {
        panic!("revocation must require review");
    };

    let regrant = database
        .grant_source_consent(&context(2), Some(&revocation_id))
        .await
        .unwrap();
    assert!(matches!(
        database.source_consent_status(&context(2)).await.unwrap(),
        SourceConsentStatus::Remembered { .. }
    ));

    let mut disabled = policy(3);
    disabled.access = SourceAccess::Disabled;
    disabled.request_limit_per_hour = 0;
    database.upsert_source_policy(&disabled).await.unwrap();
    assert!(matches!(
        database.source_consent_status(&context(2)).await.unwrap(),
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::ContextChanged,
            ..
        }
    ));
    assert!(database
        .grant_source_consent(&context(3), None)
        .await
        .is_err());
    let policy_race = sqlx::query(
        "INSERT INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         ) VALUES (
            'policy-race', ?, 'dice', 'scheduled_check',
            1, 3, 'jobsentinel.source-policy.dice', 3,
            'restricted_public_scheduled',
            '[\"public_job_posting\",\"career_goals\",\"location_preferences\"]',
            ?, ?, 'granted', ?
         )",
    )
    .bind(regrant.event_id)
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .unwrap_err();
    assert_eq!(
        super::error_kind(&super::map_consent_write_error(policy_race)),
        Some("conflict")
    );
    assert_eq!(
        database.source_policy_history("dice", 1).await.unwrap()[0]
            .policy
            .access,
        SourceAccess::Disabled
    );
}

#[tokio::test]
async fn restricted_workbench_review_is_append_only_and_revocable() {
    let database = migrated_database().await;
    database
        .upsert_source_policy(&workbench_policy())
        .await
        .unwrap();
    let context = workbench_context();

    let grant = database.grant_source_consent(&context, None).await.unwrap();
    assert!(matches!(
        database.source_consent_status(&context).await.unwrap(),
        SourceConsentStatus::Remembered { .. }
    ));

    assert!(database
        .revoke_source_consent(
            "linkedin-workbench",
            SourceConsentOperation::RestrictedWorkbench,
        )
        .await
        .unwrap());
    assert!(matches!(
        database.source_consent_status(&context).await.unwrap(),
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::Revoked,
            ..
        }
    ));
    assert!(database
        .grant_source_consent(
            &context,
            database
                .source_consent_history("linkedin-workbench", 1)
                .await
                .unwrap()
                .first()
                .map(|event| event.event_id.as_str()),
        )
        .await
        .is_ok());
    assert!(!grant.event_id.is_empty());
}

#[tokio::test]
async fn stale_or_concurrent_grants_cannot_override_newer_consent_state() {
    let database = migrated_database().await;
    database.upsert_source_policy(&policy(1)).await.unwrap();
    let context = context(1);
    let (left, right) = tokio::join!(
        database.grant_source_consent(&context, None),
        database.grant_source_consent(&context, None)
    );
    assert_ne!(left.is_ok(), right.is_ok());
    let grant_id = left.or(right).unwrap().event_id;

    assert!(database
        .revoke_source_consent("dice", SourceConsentOperation::ScheduledCheck)
        .await
        .unwrap());
    assert!(database
        .grant_source_consent(&context, Some(&grant_id))
        .await
        .is_err());
    assert!(matches!(
        database.source_consent_status(&context).await.unwrap(),
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::Revoked,
            ..
        }
    ));
}

#[tokio::test]
async fn consent_ledger_rejects_private_and_unreviewed_payload_material() {
    let database = migrated_database().await;
    database.upsert_source_policy(&policy(1)).await.unwrap();
    for (event_id, source_id, categories) in [
        (
            "event-private",
            "dice private query",
            r#"["public_job_posting"]"#,
        ),
        ("event-protected", "dice", r#"["protected_veteran_answer"]"#),
        ("event-resume", "dice", r#"["resume_evidence"]"#),
    ] {
        let inserted = sqlx::query(
            "INSERT INTO v3_source_consent_events (
                event_id, previous_event_id, source_id, operation,
                warning_version, behavior_revision, policy_ref, policy_revision,
                source_class, data_categories_json, destination_sha256,
                request_sha256, decision, recorded_at
             ) VALUES (?, NULL, ?, 'scheduled_check', 1, 1, ?, 1,
                'restricted_public_scheduled', ?, ?, ?, 'granted', ?)",
        )
        .bind(event_id)
        .bind(source_id)
        .bind("jobsentinel.source-policy.dice")
        .bind(categories)
        .bind("a".repeat(64))
        .bind("b".repeat(64))
        .bind(Utc::now().to_rfc3339())
        .execute(database.pool())
        .await;
        assert!(inserted.is_err());
    }
    database
        .upsert_source_policy(&workbench_policy())
        .await
        .unwrap();
    assert!(sqlx::query(
        "INSERT INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         ) VALUES (
            'workbench-protected', NULL, 'linkedin-workbench',
            'restricted_workbench', 1, 1,
            'jobsentinel.source-policy.linkedin-workbench', 1,
            'restricted_user_opened', '[\"protected_veteran_answer\"]',
            ?, ?, 'granted', ?
         )"
    )
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await
    .is_err());

    let grant = database
        .grant_source_consent(&context(1), None)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE v3_source_consent_events SET decision = 'revoked'")
            .execute(database.pool())
            .await
            .is_err()
    );
    assert!(sqlx::query("DELETE FROM v3_source_consent_events")
        .execute(database.pool())
        .await
        .is_err());
    assert_eq!(
        database.source_consent_history("dice", 100).await.unwrap()[0].event_id,
        grant.event_id
    );
    assert!(sqlx::query(
        "INSERT OR REPLACE INTO v3_source_consent_events (
            event_id, previous_event_id, source_id, operation,
            warning_version, behavior_revision, policy_ref, policy_revision,
            source_class, data_categories_json, destination_sha256,
            request_sha256, decision, recorded_at
         )
         SELECT event_id, event_id, source_id, operation,
                warning_version, behavior_revision, policy_ref, policy_revision,
                source_class, data_categories_json, destination_sha256,
                request_sha256, decision, recorded_at
         FROM v3_source_consent_events
         WHERE event_id = ?"
    )
    .bind(&grant.event_id)
    .execute(database.pool())
    .await
    .is_err());
    assert!(database.source_consent_history("dice", 0).await.is_err());
    assert!(database.source_consent_history("dice", 101).await.is_err());
}

#[tokio::test]
async fn corrupt_stored_consent_has_a_storage_error_kind() {
    let database = migrated_database().await;
    database.upsert_source_policy(&policy(1)).await.unwrap();
    database
        .grant_source_consent(&context(1), None)
        .await
        .unwrap();
    sqlx::query("DROP TRIGGER v3_source_consent_no_update")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query("PRAGMA ignore_check_constraints = ON")
        .execute(database.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE v3_source_consent_events SET request_sha256 = 'invalid'")
        .execute(database.pool())
        .await
        .unwrap();

    let error = database
        .source_consent_status(&context(1))
        .await
        .unwrap_err();
    assert_eq!(super::error_kind(&error), Some("corrupt"));
}
