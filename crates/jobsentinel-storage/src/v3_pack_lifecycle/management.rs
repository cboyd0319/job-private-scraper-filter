//! Persists and reads bounded signed facts for local pack-management review.

use std::collections::HashMap;

use jobsentinel_domain::{
    v3_manifests::{PackExecutionClass, PackType},
    v3_signed_packs::VerifiedPackRelease,
};
use serde::Serialize;
use sqlx::{FromRow, Sqlite, Transaction};

use super::*;
use crate::v3_foundation::parse_enum;

#[derive(FromRow)]
struct ManagementStreamRow {
    publisher_key_id: String,
    publisher_public_key_sha256: String,
    pack_id: String,
    high_water_sequence: i64,
    active_release_sequence: Option<i64>,
    rollback_release_sequence: Option<i64>,
    availability: String,
    generation: i64,
    cleanup_pending: bool,
}

#[derive(FromRow)]
struct ManagementReleaseRow {
    publisher_key_id: String,
    pack_id: String,
    release_sequence: i64,
    pack_version: String,
    pack_type: String,
    execution_class: String,
    lifecycle_state: String,
    quarantine_reason: Option<String>,
    artifact_cleanup_pending: bool,
    publisher_name: Option<String>,
    license: Option<String>,
    minimum_app_version: Option<String>,
    maximum_app_version: Option<String>,
    payload_bytes: Option<i64>,
    fixture_summary: Option<String>,
    privacy_labels_json: Option<String>,
    data_categories_json: Option<String>,
    task_kinds_json: Option<String>,
    actions_json: Option<String>,
    approval_gates_json: Option<String>,
    gateway_policy_id: Option<String>,
    external_destinations_json: Option<String>,
}

#[derive(Debug, PartialEq, Eq, FromRow)]
struct ReleaseReviewRow {
    publisher_name: String,
    license: String,
    minimum_app_version: String,
    maximum_app_version: String,
    payload_bytes: i64,
    fixture_summary: String,
    privacy_labels_json: String,
    data_categories_json: String,
    task_kinds_json: String,
    actions_json: String,
    approval_gates_json: String,
    gateway_policy_id: Option<String>,
    external_destinations_json: String,
}

impl Database {
    pub async fn list_pack_management(&self) -> Result<Vec<PackManagementStream>> {
        let mut transaction = self.pool().begin().await?;
        let streams = sqlx::query_as::<_, ManagementStreamRow>(
            "SELECT stream.publisher_key_id,
                    publisher.public_key_sha256 AS publisher_public_key_sha256,
                    stream.pack_id,
                    stream.high_water_sequence, stream.active_release_sequence,
                    stream.rollback_release_sequence, stream.availability,
                    stream.generation, EXISTS(
                        SELECT 1 FROM v3_pack_releases AS cleanup
                        WHERE cleanup.publisher_key_id = stream.publisher_key_id
                          AND cleanup.pack_id = stream.pack_id
                          AND cleanup.artifact_cleanup_pending = 1
                    ) AS cleanup_pending
             FROM v3_pack_streams AS stream
             JOIN v3_pack_publishers AS publisher
               ON publisher.publisher_key_id = stream.publisher_key_id
             ORDER BY stream.publisher_key_id, stream.pack_id",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let rows = sqlx::query_as::<_, ManagementReleaseRow>(
            "SELECT release.publisher_key_id, release.pack_id,
                    release.release_sequence, release.pack_version,
                    release.pack_type, release.execution_class,
                    release.lifecycle_state, release.quarantine_reason,
                    release.artifact_cleanup_pending,
                    review.publisher_name, review.license,
                    review.minimum_app_version, review.maximum_app_version,
                    review.payload_bytes, review.fixture_summary,
                    review.privacy_labels_json, review.data_categories_json,
                    review.task_kinds_json, review.actions_json,
                    review.approval_gates_json, review.gateway_policy_id,
                    review.external_destinations_json
             FROM v3_pack_releases AS release
             LEFT JOIN pack_release_reviews AS review
               ON review.publisher_key_id = release.publisher_key_id
              AND review.pack_id = release.pack_id
              AND review.release_sequence = release.release_sequence
             ORDER BY release.publisher_key_id, release.pack_id,
                      release.release_sequence",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let mut histories: HashMap<(String, String), Vec<PackManagementRelease>> = HashMap::new();
        for row in rows {
            let key = (row.publisher_key_id.clone(), row.pack_id.clone());
            histories
                .entry(key)
                .or_default()
                .push(management_release(row)?);
        }
        let mut management = Vec::with_capacity(streams.len());
        for row in streams {
            let key = (row.publisher_key_id.clone(), row.pack_id.clone());
            let releases = histories.remove(&key).ok_or_else(corrupt)?;
            management.push(PackManagementStream {
                publisher_key_id: row.publisher_key_id,
                publisher_public_key_sha256: row.publisher_public_key_sha256,
                pack_id: row.pack_id,
                high_water_sequence: u64::try_from(row.high_water_sequence)
                    .map_err(|_| corrupt())?,
                active_release_sequence: optional_sequence(row.active_release_sequence)?,
                rollback_release_sequence: optional_sequence(row.rollback_release_sequence)?,
                availability: pack_availability(&row.availability)?,
                generation: u64::try_from(row.generation).map_err(|_| corrupt())?,
                cleanup_pending: row.cleanup_pending,
                releases,
            });
        }
        if !histories.is_empty() {
            return Err(corrupt());
        }
        transaction.commit().await?;
        Ok(management)
    }
}

pub(super) async fn ensure_release_review(
    transaction: &mut Transaction<'_, Sqlite>,
    release: &VerifiedPackRelease,
) -> Result<()> {
    let expected = review_row(release)?;
    sqlx::query(
        "INSERT INTO pack_release_reviews (
            publisher_key_id, pack_id, release_sequence, publisher_name,
            license, minimum_app_version, maximum_app_version, payload_bytes,
            fixture_summary, privacy_labels_json, data_categories_json,
            task_kinds_json, actions_json, approval_gates_json,
            gateway_policy_id, external_destinations_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(publisher_key_id, pack_id, release_sequence) DO NOTHING",
    )
    .bind(release.publisher_key_id())
    .bind(&release.manifest().pack_id)
    .bind(i64::try_from(release.release_sequence()).map_err(|_| invalid())?)
    .bind(&expected.publisher_name)
    .bind(&expected.license)
    .bind(&expected.minimum_app_version)
    .bind(&expected.maximum_app_version)
    .bind(expected.payload_bytes)
    .bind(&expected.fixture_summary)
    .bind(&expected.privacy_labels_json)
    .bind(&expected.data_categories_json)
    .bind(&expected.task_kinds_json)
    .bind(&expected.actions_json)
    .bind(&expected.approval_gates_json)
    .bind(&expected.gateway_policy_id)
    .bind(&expected.external_destinations_json)
    .execute(&mut **transaction)
    .await?;
    let stored = sqlx::query_as::<_, ReleaseReviewRow>(
        "SELECT publisher_name, license, minimum_app_version,
                maximum_app_version, payload_bytes, fixture_summary,
                privacy_labels_json, data_categories_json, task_kinds_json,
                actions_json, approval_gates_json, gateway_policy_id,
                external_destinations_json
         FROM pack_release_reviews
         WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = ?",
    )
    .bind(release.publisher_key_id())
    .bind(&release.manifest().pack_id)
    .bind(i64::try_from(release.release_sequence()).map_err(|_| invalid())?)
    .fetch_one(&mut **transaction)
    .await?;
    if stored != expected {
        return Err(corrupt());
    }
    Ok(())
}

fn review_row(release: &VerifiedPackRelease) -> Result<ReleaseReviewRow> {
    let manifest = release.manifest();
    Ok(ReleaseReviewRow {
        publisher_name: release.publisher_name().to_string(),
        license: release.license().to_string(),
        minimum_app_version: release.minimum_app_version().to_string(),
        maximum_app_version: release.maximum_app_version().to_string(),
        payload_bytes: i64::try_from(release.payload_bytes()).map_err(|_| invalid())?,
        fixture_summary: release.fixture_summary().to_string(),
        privacy_labels_json: canonical_enum_json(&manifest.privacy_labels)?,
        data_categories_json: canonical_enum_json(&manifest.allowed_data_categories)?,
        task_kinds_json: canonical_enum_json(&manifest.allowed_task_kinds)?,
        actions_json: canonical_enum_json(&manifest.allowed_actions)?,
        approval_gates_json: canonical_enum_json(&manifest.approval_gates)?,
        gateway_policy_id: manifest.gateway_policy_id.clone(),
        external_destinations_json: serde_json::to_string(release.external_destinations())
            .map_err(|_| invalid())?,
    })
}

fn canonical_enum_json<T: Serialize>(values: &[T]) -> Result<String> {
    serde_json::to_string(values).map_err(|_| invalid())
}

fn management_release(row: ManagementReleaseRow) -> Result<PackManagementRelease> {
    Ok(PackManagementRelease {
        release_sequence: u64::try_from(row.release_sequence).map_err(|_| corrupt())?,
        pack_version: row.pack_version,
        pack_type: parse_enum::<PackType>(&row.pack_type).map_err(|_| corrupt())?,
        execution_class: parse_enum::<PackExecutionClass>(&row.execution_class)
            .map_err(|_| corrupt())?,
        lifecycle_state: pack_release_state(&row.lifecycle_state)?,
        quarantine_reason: row
            .quarantine_reason
            .as_deref()
            .map(pack_quarantine_reason)
            .transpose()?,
        artifact_cleanup_pending: row.artifact_cleanup_pending,
        publisher_name: row.publisher_name.ok_or_else(corrupt)?,
        license: row.license.ok_or_else(corrupt)?,
        minimum_app_version: row.minimum_app_version.ok_or_else(corrupt)?,
        maximum_app_version: row.maximum_app_version.ok_or_else(corrupt)?,
        payload_bytes: u64::try_from(row.payload_bytes.ok_or_else(corrupt)?)
            .map_err(|_| corrupt())?,
        fixture_summary: row.fixture_summary.ok_or_else(corrupt)?,
        privacy_labels: parse_json(row.privacy_labels_json)?,
        allowed_data_categories: parse_json(row.data_categories_json)?,
        allowed_task_kinds: parse_json(row.task_kinds_json)?,
        allowed_actions: parse_json(row.actions_json)?,
        approval_gates: parse_json(row.approval_gates_json)?,
        gateway_policy_id: row.gateway_policy_id,
        external_destinations: parse_json(row.external_destinations_json)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(value: Option<String>) -> Result<T> {
    serde_json::from_str(&value.ok_or_else(corrupt)?).map_err(|_| corrupt())
}

fn optional_sequence(value: Option<i64>) -> Result<Option<u64>> {
    value.map(u64::try_from).transpose().map_err(|_| corrupt())
}

fn pack_availability(value: &str) -> Result<PackAvailability> {
    match value {
        "ready" => Ok(PackAvailability::Ready),
        "disabled" => Ok(PackAvailability::Disabled),
        "quarantined" => Ok(PackAvailability::Quarantined),
        "removed" => Ok(PackAvailability::Removed),
        _ => Err(corrupt()),
    }
}
