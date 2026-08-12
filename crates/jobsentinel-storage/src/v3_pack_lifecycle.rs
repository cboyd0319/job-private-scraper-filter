//! Persists signed pack identity, trust, lifecycle, review, and artifact ownership.

use std::{error::Error, fmt};

use anyhow::{anyhow, Result};
use chrono::Utc;
use jobsentinel_domain::{
    v3_manifests::{PackExecutionClass, PackType},
    v3_pack_payloads::SelfTestedPackRelease,
    v3_signed_packs::{TrustedPublisherKey, VerifiedPackRelease},
};
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};

use crate::Database;

mod artifacts;
mod lifecycle;
mod management;
mod transitions;
mod types;

use management::ensure_release_review;
pub use types::*;

#[derive(Debug)]
enum PackLifecycleError {
    Invalid,
    Revoked,
    Downgrade,
    Equivocation,
    NotFound,
    Stale,
    InvalidState,
    Corrupt,
}

impl fmt::Display for PackLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "pack lifecycle input is invalid",
            Self::Revoked => "pack publisher is revoked",
            Self::Downgrade => "pack release is older than the retained high-water mark",
            Self::Equivocation => "pack publisher reused a release sequence",
            Self::NotFound => "pack lifecycle state was not found",
            Self::Stale => "pack lifecycle generation is stale",
            Self::InvalidState => "pack lifecycle transition is not allowed",
            Self::Corrupt => "stored pack lifecycle state is invalid",
        })
    }
}

impl Error for PackLifecycleError {}

pub fn pack_lifecycle_error_kind(error: &anyhow::Error) -> Option<&'static str> {
    Some(match error.downcast_ref::<PackLifecycleError>()? {
        PackLifecycleError::Invalid => "invalid",
        PackLifecycleError::Revoked => "revoked",
        PackLifecycleError::Downgrade => "downgrade",
        PackLifecycleError::Equivocation => "equivocation",
        PackLifecycleError::NotFound => "not_found",
        PackLifecycleError::Stale => "stale",
        PackLifecycleError::InvalidState => "invalid_state",
        PackLifecycleError::Corrupt => "corrupt",
    })
}

impl Database {
    pub async fn stage_verified_pack(
        &self,
        release: &VerifiedPackRelease,
        publisher: &TrustedPublisherKey,
    ) -> Result<PackStageOutcome> {
        validate_stage_input(release, publisher)?;
        let sequence = i64::try_from(release.release_sequence()).map_err(|_| invalid())?;
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;

        sqlx::query(
            "INSERT INTO v3_pack_publishers (
                publisher_key_id, public_key_sha256, trust_state,
                revoked_at, created_at, updated_at
             ) VALUES (?, ?, 'trusted', NULL, ?, ?)
             ON CONFLICT(publisher_key_id) DO NOTHING",
        )
        .bind(&publisher.publisher_key_id)
        .bind(&public_key_sha256)
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?;
        let (stored_key_sha256, trust_state): (String, String) = sqlx::query_as(
            "SELECT public_key_sha256, trust_state
             FROM v3_pack_publishers
             WHERE publisher_key_id = ?",
        )
        .bind(&publisher.publisher_key_id)
        .fetch_one(&mut *transaction)
        .await?;
        if stored_key_sha256 != public_key_sha256 {
            return Err(equivocation());
        }
        if trust_state != "trusted" {
            return Err(revoked());
        }

        if let Some(stored_digest) = sqlx::query_scalar::<_, String>(
            "SELECT signed_release_sha256
             FROM v3_pack_releases
             WHERE publisher_key_id = ?
               AND pack_id = ?
               AND release_sequence = ?",
        )
        .bind(release.publisher_key_id())
        .bind(&release.manifest().pack_id)
        .bind(sequence)
        .fetch_optional(&mut *transaction)
        .await?
        {
            if stored_digest != release.signed_release_sha256() {
                return Err(equivocation());
            }
            ensure_release_review(&mut transaction, release).await?;
            let stream = fetch_stream(&mut transaction, release).await?;
            transaction.commit().await?;
            return Ok(PackStageOutcome::Replay(stream));
        }

        let high_water = sqlx::query_scalar::<_, i64>(
            "SELECT high_water_sequence
             FROM v3_pack_streams
             WHERE publisher_key_id = ? AND pack_id = ?",
        )
        .bind(release.publisher_key_id())
        .bind(&release.manifest().pack_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if high_water.is_some_and(|value| sequence <= value) {
            return Err(downgrade());
        }

        sqlx::query(
            "INSERT INTO v3_pack_streams (
                publisher_key_id, pack_id, high_water_sequence,
                high_water_signed_release_sha256, active_release_sequence,
                rollback_release_sequence, availability, generation,
                created_at, updated_at
             ) VALUES (?, ?, 0, NULL, NULL, NULL, 'quarantined', 0, ?, ?)
             ON CONFLICT(publisher_key_id, pack_id) DO NOTHING",
        )
        .bind(release.publisher_key_id())
        .bind(&release.manifest().pack_id)
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "INSERT INTO v3_pack_releases (
                publisher_key_id, pack_id, release_sequence, release_id,
                signed_release_sha256, payload_sha256, pack_version,
                pack_type, execution_class, lifecycle_state,
                quarantine_reason, self_tested_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'staged', NULL, NULL, ?, ?)",
        )
        .bind(release.publisher_key_id())
        .bind(&release.manifest().pack_id)
        .bind(sequence)
        .bind(release.release_id())
        .bind(release.signed_release_sha256())
        .bind(&release.manifest().payload_sha256)
        .bind(release.pack_version())
        .bind(pack_type_text(release.manifest().pack_type)?)
        .bind(execution_class_text(release.manifest().execution_class))
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?;

        ensure_release_review(&mut transaction, release).await?;

        let stream = fetch_stream(&mut transaction, release).await?;
        transaction.commit().await?;
        Ok(PackStageOutcome::Staged(stream))
    }

    pub async fn get_pack_stream(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
    ) -> Result<PackStream> {
        let mut transaction = self.pool().begin().await?;
        let stream = fetch_stream_by_id(&mut transaction, publisher_key_id, pack_id).await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn get_stored_pack_release(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
        release_sequence: u64,
    ) -> Result<StoredPackRelease> {
        let sequence = i64::try_from(release_sequence).map_err(|_| invalid())?;
        let row = sqlx::query_as::<_, StoredPackReleaseRow>(
            "SELECT publisher_key_id, pack_id, release_sequence,
                    signed_release_sha256, lifecycle_state, quarantine_reason,
                    artifact_cleanup_pending
             FROM v3_pack_releases
             WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = ?",
        )
        .bind(publisher_key_id)
        .bind(pack_id)
        .bind(sequence)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(not_found)?;
        row.try_into()
    }
}

async fn fetch_stream(
    transaction: &mut Transaction<'_, Sqlite>,
    release: &VerifiedPackRelease,
) -> Result<PackStream> {
    fetch_stream_by_id(
        transaction,
        release.publisher_key_id(),
        &release.manifest().pack_id,
    )
    .await
}

async fn fetch_stream_by_id(
    transaction: &mut Transaction<'_, Sqlite>,
    publisher_key_id: &str,
    pack_id: &str,
) -> Result<PackStream> {
    let row = sqlx::query_as::<_, PackStreamRow>(
        "SELECT publisher_key_id, pack_id, high_water_sequence,
                active_release_sequence, rollback_release_sequence,
                availability, generation
         FROM v3_pack_streams
         WHERE publisher_key_id = ? AND pack_id = ?",
    )
    .bind(publisher_key_id)
    .bind(pack_id)
    .fetch_one(&mut **transaction)
    .await?;
    stream_from_row(row)
}

async fn stream_guard_error(
    transaction: &mut Transaction<'_, Sqlite>,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: i64,
) -> Result<anyhow::Error> {
    let generation = sqlx::query_scalar::<_, i64>(
        "SELECT generation FROM v3_pack_streams
         WHERE publisher_key_id = ? AND pack_id = ?",
    )
    .bind(publisher_key_id)
    .bind(pack_id)
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(match generation {
        None => not_found(),
        Some(current) if current != expected_generation => stale(),
        Some(_) => invalid_state(),
    })
}

async fn require_current_candidate(
    transaction: &mut Transaction<'_, Sqlite>,
    publisher_key_id: &str,
    pack_id: &str,
    release_sequence: i64,
    signed_release_sha256: &str,
    expected_generation: i64,
) -> Result<()> {
    let current = sqlx::query_as::<_, (i64, i64, Option<String>)>(
        "SELECT generation, high_water_sequence, high_water_signed_release_sha256
         FROM v3_pack_streams
         WHERE publisher_key_id = ? AND pack_id = ?",
    )
    .bind(publisher_key_id)
    .bind(pack_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some((generation, high_water, digest)) = current else {
        return Err(not_found());
    };
    if generation != expected_generation {
        return Err(stale());
    }
    if high_water != release_sequence || digest.as_deref() != Some(signed_release_sha256) {
        return Err(invalid_state());
    }
    Ok(())
}

async fn require_trusted_publisher(
    transaction: &mut Transaction<'_, Sqlite>,
    publisher_key_id: &str,
    public_key_sha256: &str,
) -> Result<()> {
    let publisher = sqlx::query_as::<_, (String, String)>(
        "SELECT public_key_sha256, trust_state
         FROM v3_pack_publishers WHERE publisher_key_id = ?",
    )
    .bind(publisher_key_id)
    .fetch_optional(&mut **transaction)
    .await?;
    match publisher {
        None => Err(not_found()),
        Some((stored, _)) if stored != public_key_sha256 => Err(equivocation()),
        Some((_, state)) if state != "trusted" => Err(revoked()),
        Some(_) => Ok(()),
    }
}

fn stream_from_row(row: PackStreamRow) -> Result<PackStream> {
    Ok(PackStream {
        publisher_key_id: row.publisher_key_id,
        pack_id: row.pack_id,
        high_water_sequence: u64::try_from(row.high_water_sequence).map_err(|_| corrupt())?,
        active_release_sequence: row
            .active_release_sequence
            .map(u64::try_from)
            .transpose()
            .map_err(|_| corrupt())?,
        rollback_release_sequence: row
            .rollback_release_sequence
            .map(u64::try_from)
            .transpose()
            .map_err(|_| corrupt())?,
        availability: match row.availability.as_str() {
            "ready" => PackAvailability::Ready,
            "disabled" => PackAvailability::Disabled,
            "quarantined" => PackAvailability::Quarantined,
            "removed" => PackAvailability::Removed,
            _ => return Err(corrupt()),
        },
        generation: u64::try_from(row.generation).map_err(|_| corrupt())?,
    })
}

fn validate_stage_input(
    release: &VerifiedPackRelease,
    publisher: &TrustedPublisherKey,
) -> Result<()> {
    release.manifest().validate().map_err(|_| invalid())?;
    release
        .manifest()
        .verify_payload(release.payload().as_bytes())
        .map_err(|_| invalid())?;
    if publisher.revoked {
        return Err(revoked());
    }
    if publisher.publisher_key_id != release.publisher_key_id()
        || release.publisher_key_id() != release.manifest().publisher_key_id
        || hex::encode(Sha256::digest(publisher.public_key))
            != release.publisher_public_key_sha256()
        || publisher.pack_type != release.manifest().pack_type
        || publisher.execution_class != release.manifest().execution_class
        || release.release_sequence() == 0
        || release.release_id()
            != format!(
                "{}:{}:{}",
                release.publisher_key_id(),
                release.manifest().pack_id,
                release.release_sequence()
            )
        || !is_sha256(release.signed_release_sha256())
        || !is_sha256(release.publisher_public_key_sha256())
    {
        return Err(invalid());
    }
    Ok(())
}

fn pack_type_text(pack_type: PackType) -> Result<&'static str> {
    match pack_type {
        PackType::Skill => Ok("skill"),
        PackType::Agent => Ok("agent"),
        PackType::Workflow => Ok("workflow"),
        PackType::Source => Ok("source"),
        PackType::Evaluation => Ok("evaluation"),
        PackType::Role
        | PackType::Region
        | PackType::Rubric
        | PackType::Template
        | PackType::OsHelper => Err(invalid()),
    }
}

const fn execution_class_text(execution_class: PackExecutionClass) -> &'static str {
    match execution_class {
        PackExecutionClass::StaticContent => "static_content",
        PackExecutionClass::ReviewedTypedWorkflow => "reviewed_typed_workflow",
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn invalid() -> anyhow::Error {
    anyhow!(PackLifecycleError::Invalid)
}

fn revoked() -> anyhow::Error {
    anyhow!(PackLifecycleError::Revoked)
}

fn downgrade() -> anyhow::Error {
    anyhow!(PackLifecycleError::Downgrade)
}

fn equivocation() -> anyhow::Error {
    anyhow!(PackLifecycleError::Equivocation)
}

fn not_found() -> anyhow::Error {
    anyhow!(PackLifecycleError::NotFound)
}

fn stale() -> anyhow::Error {
    anyhow!(PackLifecycleError::Stale)
}

fn invalid_state() -> anyhow::Error {
    anyhow!(PackLifecycleError::InvalidState)
}

fn corrupt() -> anyhow::Error {
    anyhow!(PackLifecycleError::Corrupt)
}
