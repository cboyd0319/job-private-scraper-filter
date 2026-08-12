use chrono::Utc;
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata,
        EventOrigin, GraphProvenance, SourceAccess, SourceGraphLink, SourcePolicy, SourceRelation,
    },
    v3_manifests::{DataCategory, PrivacyLabel, PrivacyReceipt, SourceClass},
};

use crate::test_support::{insert_test_job, migrated_database};

async fn case_file_database() -> crate::Database {
    let database = migrated_database().await;
    insert_test_job(
        database.pool(),
        "job-1",
        "Office Assistant",
        Some("Example"),
        None,
        "2026-07-19T00:00:00Z",
    )
    .await;
    database
}

fn local_receipt() -> PrivacyReceipt {
    PrivacyReceipt {
        schema: SchemaId::PrivacyReceiptV1,
        receipt_id: "receipt-1".to_string(),
        task_id: "task-1".to_string(),
        pack_id: None,
        labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        data_categories: vec![DataCategory::ResumeEvidence],
        stored_locally: true,
        data_left_device: false,
        external_destination: None,
        gateway_policy_id: None,
        approval_reference: None,
        delete_or_revoke_action: "delete_local_receipt".to_string(),
    }
}

#[tokio::test]
async fn case_file_create_is_atomic_and_reuses_the_job() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = crate::Database::connect(&temp_dir.path().join("jobs.db"))
        .await
        .unwrap();
    database.migrate().await.unwrap();
    insert_test_job(
        database.pool(),
        "job-1",
        "Office Assistant",
        Some("Example"),
        None,
        "2026-07-19T00:00:00Z",
    )
    .await;
    let (first, second) = tokio::join!(
        database.ensure_case_file("job-1"),
        database.ensure_case_file("job-1")
    );

    assert_eq!(first.unwrap().case_file_id, second.unwrap().case_file_id);
    assert!(database.ensure_case_file("missing-job").await.is_err());
}

#[tokio::test]
async fn generic_typed_events_round_trip_without_private_payload_fields() {
    let database = case_file_database().await;
    let case_file = database.ensure_case_file("job-1").await.unwrap();
    database
        .append_case_file_event(&CaseFileEventInput {
            case_file_id: case_file.case_file_id.clone(),
            kind: CaseFileEventKind::PrivacyReceiptRecorded,
            origin: EventOrigin::User,
            user_action: true,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::LocalReference {
                reference_id: "receipt-1".to_string(),
            },
        })
        .await
        .unwrap();

    let events = database
        .list_case_file_events(&case_file.case_file_id)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].metadata,
        EventMetadata::LocalReference {
            reference_id: "receipt-1".to_string()
        }
    );
    let stored: String = sqlx::query_scalar("SELECT metadata_json FROM v3_job_events LIMIT 1")
        .fetch_one(database.pool())
        .await
        .unwrap();
    for forbidden in ["resume", "note", "credential", "cookie", "provider_payload"] {
        assert!(!stored.contains(forbidden));
    }
}

#[tokio::test]
async fn database_rejects_unknown_and_oversized_event_metadata() {
    let database = case_file_database().await;
    let case_file = database.ensure_case_file("job-1").await.unwrap();
    let padded_empty = |target_bytes: usize| {
        let mut json = r#"{"kind":"empty""#.to_string();
        json.push_str(&" ".repeat(target_bytes - json.len() - 1));
        json.push('}');
        json
    };
    for (event_id, size) in [
        ("event-boundary-minus-one", 2_047),
        ("event-boundary", 2_048),
    ] {
        sqlx::query(
            "INSERT INTO v3_job_events
             (event_id, case_file_id, event_kind, origin, user_action, local_only, sensitive, metadata_json, created_at)
             VALUES (?, ?, 'case_created', 'system', 0, 1, 1, ?, ?)",
        )
        .bind(event_id)
        .bind(&case_file.case_file_id)
        .bind(padded_empty(size))
        .bind(Utc::now().to_rfc3339())
        .execute(database.pool())
        .await
        .unwrap();
    }
    let invalid_privacy = sqlx::query(
        "INSERT INTO v3_job_events
         (event_id, case_file_id, event_kind, origin, user_action, local_only, sensitive, metadata_json, created_at)
         VALUES ('event-public', ?, 'case_created', 'system', 0, 0, 1, '{\"kind\":\"empty\"}', ?)",
    )
    .bind(&case_file.case_file_id)
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await;
    assert!(invalid_privacy.is_err());

    for (event_id, event_kind, metadata) in [
        (
            "event-unknown",
            "evidence_linked",
            r#"{"kind":"local_reference","reference_id":"safe","note":"private"}"#.to_string(),
        ),
        ("event-oversized", "case_created", padded_empty(2_049)),
        (
            "event-unicode",
            "evidence_linked",
            r#"{"kind":"local_reference","reference_id":"résumé"}"#.to_string(),
        ),
        (
            "event-mismatch",
            "status_changed",
            r#"{"kind":"empty"}"#.to_string(),
        ),
    ] {
        let result = sqlx::query(
            "INSERT INTO v3_job_events
             (event_id, case_file_id, event_kind, origin, user_action, local_only, sensitive, metadata_json, created_at)
             VALUES (?, ?, ?, 'system', 0, 1, 1, ?, ?)",
        )
        .bind(event_id)
        .bind(&case_file.case_file_id)
        .bind(event_kind)
        .bind(metadata)
        .bind(Utc::now().to_rfc3339())
        .execute(database.pool())
        .await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn graph_links_are_typed_and_duplicate_safe() {
    let database = migrated_database().await;
    let career = CareerGraphLink {
        link_id: "career-link-1".to_string(),
        subject_id: "skill-1".to_string(),
        relation: CareerRelation::Related,
        object_id: "skill-2".to_string(),
        provenance: GraphProvenance::UserConfirmed,
        provenance_ref: None,
    };
    let source = SourceGraphLink {
        link_id: "source-link-1".to_string(),
        source_id: "source-1".to_string(),
        relation: SourceRelation::Policy,
        related_id: "policy-1".to_string(),
        provenance: GraphProvenance::PublicSource,
        provenance_ref: Some("review-1".to_string()),
    };

    database.insert_career_graph_link(&career).await.unwrap();
    database.insert_source_graph_link(&source).await.unwrap();
    assert!(database.insert_career_graph_link(&career).await.is_err());
    assert!(database.insert_source_graph_link(&source).await.is_err());

    let direct = sqlx::query(
        "INSERT INTO career_graph_links (
            link_id, subject_id, relation, object_id,
            provenance, provenance_ref, created_at
         ) VALUES ('private note', 'skill-1', 'related', 'skill-2',
                   'public_source', NULL, ?)",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await;
    assert!(direct.is_err());
}

#[tokio::test]
async fn receipts_round_trip_only_after_contract_validation() {
    let database = migrated_database().await;
    let receipt = local_receipt();

    database.store_privacy_receipt(&receipt).await.unwrap();
    assert_eq!(
        database
            .get_privacy_receipt(&receipt.receipt_id)
            .await
            .unwrap(),
        Some(receipt)
    );

    let mut invalid = local_receipt();
    invalid.receipt_id = "receipt-2".to_string();
    invalid.labels = vec![PrivacyLabel::PublicDataOnly];
    invalid.data_categories = vec![DataCategory::ProtectedVeteranAnswer];
    assert!(database.store_privacy_receipt(&invalid).await.is_err());

    let mismatched = sqlx::query(
        "INSERT INTO v3_privacy_receipts (
            receipt_id, schema, receipt_json, stored_locally,
            data_left_device, created_at
         ) VALUES ('different-id', 'jobsentinel.v3.privacy-receipt.v1', ?, 1, 0, ?)",
    )
    .bind(serde_json::to_string(&local_receipt()).unwrap())
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await;
    assert!(mismatched.is_err());
}

#[tokio::test]
async fn source_policy_upsert_is_revision_monotonic() {
    let database = migrated_database().await;
    let mut policy = SourcePolicy {
        source_id: "source-1".to_string(),
        source_class: SourceClass::OfficialPublicApi,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour: 30,
        user_review_required: true,
        policy_ref: "policy-1".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: Utc::now(),
    };
    database.upsert_source_policy(&policy).await.unwrap();
    policy.revision = 2;
    policy.request_limit_per_hour = 15;
    database.upsert_source_policy(&policy).await.unwrap();
    assert_eq!(
        database.get_source_policy(&policy.source_id).await.unwrap(),
        Some(policy.clone())
    );

    policy.revision = 1;
    assert!(database.upsert_source_policy(&policy).await.is_err());
    policy.revision = 2;
    policy.request_limit_per_hour = 99;
    assert!(database.upsert_source_policy(&policy).await.is_err());

    let private_identifier = sqlx::query(
        "INSERT INTO v3_source_policies (
            source_id, source_class, access, request_limit_per_hour,
            user_review_required, policy_ref, revision,
            restriction_reason_code, reviewed_at, created_at, updated_at
         ) VALUES (
            'private source note', 'official_public_api', 'disabled', 0,
            1, 'policy-raw', 1, NULL, ?, ?, ?
         )",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .bind(Utc::now().to_rfc3339())
    .execute(database.pool())
    .await;
    assert!(private_identifier.is_err());
}

#[tokio::test]
async fn concurrent_policy_writers_cannot_replace_the_same_revision() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = crate::Database::connect(&temp_dir.path().join("jobs.db"))
        .await
        .unwrap();
    database.migrate().await.unwrap();
    let base = SourcePolicy {
        source_id: "source-1".to_string(),
        source_class: SourceClass::OfficialPublicApi,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour: 30,
        user_review_required: true,
        policy_ref: "policy-1".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: Utc::now(),
    };
    database.upsert_source_policy(&base).await.unwrap();
    let mut slower = base.clone();
    slower.revision = 2;
    slower.request_limit_per_hour = 10;
    let mut faster = slower.clone();
    faster.request_limit_per_hour = 20;

    let (left, right) = tokio::join!(
        database.upsert_source_policy(&slower),
        database.upsert_source_policy(&faster)
    );

    assert_ne!(left.is_ok(), right.is_ok());
    assert_eq!(
        database
            .get_source_policy(&base.source_id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
}

#[tokio::test]
async fn deleting_a_job_detaches_but_does_not_delete_its_case_history() {
    let database = case_file_database().await;
    let case_file = database.ensure_case_file("job-1").await.unwrap();
    database
        .append_case_file_event(&CaseFileEventInput {
            case_file_id: case_file.case_file_id.clone(),
            kind: CaseFileEventKind::CaseCreated,
            origin: EventOrigin::System,
            user_action: false,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::Empty,
        })
        .await
        .unwrap();

    sqlx::query("DELETE FROM jobs WHERE hash = 'job-1'")
        .execute(database.pool())
        .await
        .unwrap();

    let detached: Option<String> =
        sqlx::query_scalar("SELECT job_hash FROM opportunity_case_files WHERE case_file_id = ?")
            .bind(&case_file.case_file_id)
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert!(detached.is_none());
    assert_eq!(
        database
            .list_case_file_events(&case_file.case_file_id)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn fresh_database_exposes_typed_compatibility_metadata() {
    let database = migrated_database().await;
    let metadata = database.read_compatibility_metadata().await.unwrap();

    metadata.validate().unwrap();
    assert_eq!(metadata.compatibility_line, 3);
    assert_eq!(metadata.database_schema, 2);
    let current: i64 = sqlx::query_scalar("SELECT MAX(version) FROM _sqlx_migrations")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(metadata.migration_version, current);
}
