//! Reconciles runnable pack streams against their verified on-disk artifacts.

use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use jobsentinel_domain::v3_signed_packs::TrustedPublisherKey;
use jobsentinel_storage::{v3_pack_lifecycle::PackQuarantineReason, Database};
use serde::Serialize;

use super::artifact::{load_tested_artifact, ArtifactLoadError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackArtifactReconciliation {
    pub checked: u64,
    pub rolled_back: u64,
    pub quarantined: u64,
}

pub(crate) async fn reconcile_active_pack_artifacts(
    database: &Database,
    artifact_root: &Path,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<PackArtifactReconciliation> {
    let mut outcome = PackArtifactReconciliation::default();
    for stream in database.list_runnable_pack_streams().await? {
        outcome.checked += 1;
        let active_sequence = stream
            .active_release_sequence
            .ok_or_else(|| anyhow!("runnable pack has no active release"))?;
        let active = database
            .get_stored_pack_release(&stream.publisher_key_id, &stream.pack_id, active_sequence)
            .await?;
        let active_error =
            match load_tested_artifact(artifact_root, &active, trusted_publishers, today) {
                Ok(_) => continue,
                Err(error) => error,
            };
        if let Some(rollback_sequence) = stream.rollback_release_sequence {
            let rollback = database
                .get_stored_pack_release(
                    &stream.publisher_key_id,
                    &stream.pack_id,
                    rollback_sequence,
                )
                .await?;
            match load_tested_artifact(artifact_root, &rollback, trusted_publishers, today) {
                Ok((_, tested, publisher)) => {
                    match active_error {
                        ArtifactLoadError::Missing => {
                            database
                                .replace_unavailable_active_pack_with_rollback(
                                    &active,
                                    &tested,
                                    publisher,
                                    stream.generation,
                                )
                                .await?
                        }
                        ArtifactLoadError::Invalid => {
                            database
                                .replace_invalid_active_pack_with_rollback(
                                    &active,
                                    &tested,
                                    publisher,
                                    stream.generation,
                                )
                                .await?
                        }
                        ArtifactLoadError::TrustRevoked => {
                            database
                                .quarantine_active_and_rollback_pack_artifacts(
                                    &active,
                                    PackQuarantineReason::TrustRevoked,
                                    &rollback,
                                    PackQuarantineReason::TrustRevoked,
                                    stream.generation,
                                )
                                .await?;
                            outcome.quarantined += 1;
                            continue;
                        }
                    };
                    outcome.rolled_back += 1;
                }
                Err(rollback_error) => {
                    database
                        .quarantine_active_and_rollback_pack_artifacts(
                            &active,
                            quarantine_reason(active_error),
                            &rollback,
                            quarantine_reason(rollback_error),
                            stream.generation,
                        )
                        .await?;
                    outcome.quarantined += 1;
                }
            }
            continue;
        }
        match active_error {
            ArtifactLoadError::Missing => {
                database
                    .quarantine_unavailable_active_pack_artifact(&active, stream.generation)
                    .await?
            }
            ArtifactLoadError::Invalid => {
                database
                    .quarantine_invalid_active_pack_artifact(&active, stream.generation)
                    .await?
            }
            ArtifactLoadError::TrustRevoked => {
                database
                    .quarantine_trust_revoked_active_pack_artifact(&active, stream.generation)
                    .await?
            }
        };
        outcome.quarantined += 1;
    }
    Ok(outcome)
}

const fn quarantine_reason(error: ArtifactLoadError) -> PackQuarantineReason {
    match error {
        ArtifactLoadError::Missing => PackQuarantineReason::ArtifactMissing,
        ArtifactLoadError::Invalid => PackQuarantineReason::IntegrityFailed,
        ArtifactLoadError::TrustRevoked => PackQuarantineReason::TrustRevoked,
    }
}
