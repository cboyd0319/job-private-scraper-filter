use chrono::{NaiveDate, TimeZone, Utc};
use jobsentinel_domain::{
    v3_foundation::{GraphProvenance, SourceAccess, SourceGraphLink, SourcePolicy, SourceRelation},
    v3_manifests::SourceClass,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{parse_source_manifest, SourceManifest, SourceOperation},
};

use crate::test_support::migrated_database;

const SOURCE_MANIFEST: &str =
    include_str!("../../jobsentinel-domain/src/fixtures/v3_source_manifest_v1.json");

fn policy(revision: u32) -> SourcePolicy {
    SourcePolicy {
        source_id: "synthetic-official-jobs".to_string(),
        source_class: SourceClass::OfficialPublicApi,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour: 60,
        user_review_required: false,
        policy_ref: "synthetic-official-jobs-v1".to_string(),
        revision,
        restriction_reason_code: None,
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    }
}

fn manifest(policy: &SourcePolicy) -> SourceManifest {
    let mut value: serde_json::Value = serde_json::from_str(SOURCE_MANIFEST).unwrap();
    value["policy_revision"] = serde_json::json!(policy.revision);
    parse_source_manifest(&value.to_string(), policy).unwrap()
}

#[tokio::test]
async fn source_manifest_round_trips_and_same_revision_is_idempotent() {
    let database = migrated_database().await;
    let policy = policy(1);
    database.upsert_source_policy(&policy).await.unwrap();
    let manifest = manifest(&policy);

    database.store_source_manifest(&manifest).await.unwrap();
    database.store_source_manifest(&manifest).await.unwrap();

    assert_eq!(
        database
            .get_source_manifest(&policy.source_id)
            .await
            .unwrap(),
        Some(manifest)
    );
}

#[tokio::test]
async fn source_manifest_replacement_requires_a_new_current_policy_revision() {
    let database = migrated_database().await;
    let first_policy = policy(1);
    database.upsert_source_policy(&first_policy).await.unwrap();
    let first = manifest(&first_policy);
    database.store_source_manifest(&first).await.unwrap();

    let mut conflicting = first.clone();
    conflicting.display_name = "Conflicting Same Revision".to_string();
    assert!(database.store_source_manifest(&conflicting).await.is_err());

    let second_policy = policy(2);
    database.upsert_source_policy(&second_policy).await.unwrap();
    assert_eq!(
        database
            .get_source_manifest(&first.source_id)
            .await
            .unwrap(),
        Some(first.clone()),
        "policy drift must remain visible to authorization"
    );
    assert_eq!(
        first
            .authorize(
                &second_policy,
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Blocked(
            jobsentinel_domain::v3_source_manifest::SourceStopCondition::PolicyChanged
        )
    );
    assert!(database.store_source_manifest(&first).await.is_err());

    let second = manifest(&second_policy);
    database.store_source_manifest(&second).await.unwrap();
    assert_eq!(
        database
            .get_source_manifest(&second.source_id)
            .await
            .unwrap(),
        Some(second)
    );
}

#[tokio::test]
async fn database_rejects_malformed_manifest_rows_and_deletion() {
    let database = migrated_database().await;
    let policy = policy(1);
    database.upsert_source_policy(&policy).await.unwrap();
    let now = Utc::now().to_rfc3339();

    let malformed = sqlx::query(
        "INSERT INTO v3_source_manifests (
            source_id, policy_ref, policy_revision, manifest_json, created_at, updated_at
         ) VALUES (?, ?, 1, '{}', ?, ?)",
    )
    .bind(&policy.source_id)
    .bind(&policy.policy_ref)
    .bind(&now)
    .bind(&now)
    .execute(database.pool())
    .await;
    assert!(malformed.is_err());

    database
        .store_source_manifest(&manifest(&policy))
        .await
        .unwrap();
    assert!(
        sqlx::query("DELETE FROM v3_source_manifests WHERE source_id = ?")
            .bind(&policy.source_id)
            .execute(database.pool())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn database_rejects_stale_and_same_revision_manifest_bypasses() {
    let database = migrated_database().await;
    let first_policy = policy(1);
    database.upsert_source_policy(&first_policy).await.unwrap();
    let first = manifest(&first_policy);
    let first_json = serde_json::to_string(&first).unwrap();
    database.upsert_source_policy(&policy(2)).await.unwrap();
    let now = Utc::now().to_rfc3339();
    let stale = sqlx::query(
        "INSERT INTO v3_source_manifests (
            source_id, policy_ref, policy_revision, manifest_json, created_at, updated_at
         ) VALUES (?, ?, 1, ?, ?, ?)",
    )
    .bind(&first.source_id)
    .bind(&first.policy_ref)
    .bind(&first_json)
    .bind(&now)
    .bind(&now)
    .execute(database.pool())
    .await;
    assert!(stale.is_err());

    let current_policy = policy(2);
    let current = manifest(&current_policy);
    database.store_source_manifest(&current).await.unwrap();
    let mut conflicting = current.clone();
    conflicting.display_name = "Replacement Attack".to_string();
    let conflicting_json = serde_json::to_string(&conflicting).unwrap();
    let replace = sqlx::query(
        "INSERT OR REPLACE INTO v3_source_manifests (
            source_id, policy_ref, policy_revision, manifest_json, created_at, updated_at
         ) VALUES (?, ?, 2, ?, ?, ?)",
    )
    .bind(&conflicting.source_id)
    .bind(&conflicting.policy_ref)
    .bind(&conflicting_json)
    .bind(&now)
    .bind(&now)
    .execute(database.pool())
    .await;
    assert!(replace.is_err());

    let update = sqlx::query(
        "UPDATE v3_source_manifests
         SET manifest_json = ?
         WHERE source_id = ?",
    )
    .bind(conflicting_json)
    .bind(&conflicting.source_id)
    .execute(database.pool())
    .await;
    assert!(update.is_err());
}

#[tokio::test]
async fn manifest_lineage_is_the_typed_source_graph_projection() {
    let database = migrated_database().await;
    let first_policy = policy(1);
    database.upsert_source_policy(&first_policy).await.unwrap();
    let first = manifest(&first_policy);
    database.store_source_manifest(&first).await.unwrap();
    assert_eq!(
        database
            .list_source_graph_links(&first.source_id)
            .await
            .unwrap(),
        first.lineage
    );
    let unrelated = SourceGraphLink {
        link_id: "source-related-user-note".to_string(),
        source_id: first.source_id.clone(),
        relation: SourceRelation::Related,
        related_id: "user-confirmed-source-note".to_string(),
        provenance: GraphProvenance::UserConfirmed,
        provenance_ref: None,
    };
    database.insert_source_graph_link(&unrelated).await.unwrap();
    let forbidden_lineage = SourceGraphLink {
        link_id: "source-lineage-user-note".to_string(),
        relation: SourceRelation::Lineage,
        related_id: "user-confirmed-lineage-note".to_string(),
        ..unrelated.clone()
    };
    assert!(database
        .insert_source_graph_link(&forbidden_lineage)
        .await
        .is_err());

    let second_policy = policy(2);
    database.upsert_source_policy(&second_policy).await.unwrap();
    let mut second = manifest(&second_policy);
    second.lineage[0].link_id = "source-lineage-synthetic-official-jobs-v2".to_string();
    second.lineage[0].related_id = "official-docs-synthetic-jobs-v2".to_string();
    database.store_source_manifest(&second).await.unwrap();
    let mut expected = second.lineage.clone();
    expected.push(unrelated);
    expected.sort_by(|left, right| left.link_id.cmp(&right.link_id));
    assert_eq!(
        database
            .list_source_graph_links(&second.source_id)
            .await
            .unwrap(),
        expected
    );
}
