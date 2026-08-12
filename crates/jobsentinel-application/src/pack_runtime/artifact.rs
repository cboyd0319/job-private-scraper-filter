//! Loads, verifies, persists, and removes signed pack artifacts in private storage.

use std::path::Path;

use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_pack_payloads::{parse_and_self_test_pack_payload, SelfTestedPackRelease},
    v3_signed_packs::{TrustedPublisherKey, VerifiedPackRelease},
};
use jobsentinel_storage::v3_pack_lifecycle::StoredPackRelease;
use sha2::{Digest, Sha256};

use super::verify_pack;

mod secure;

#[derive(Clone, Copy)]
pub(super) enum ArtifactLoadError {
    Missing,
    Invalid,
    TrustRevoked,
}

pub(super) fn load_tested_artifact<'a>(
    artifact_root: &Path,
    stored: &StoredPackRelease,
    trusted_publishers: &'a [TrustedPublisherKey],
    today: NaiveDate,
) -> std::result::Result<
    (
        VerifiedPackRelease,
        SelfTestedPackRelease,
        &'a TrustedPublisherKey,
    ),
    ArtifactLoadError,
> {
    let publisher = trusted_publishers
        .iter()
        .find(|publisher| publisher.publisher_key_id == stored.publisher_key_id)
        .filter(|publisher| !publisher.revoked)
        .ok_or(ArtifactLoadError::TrustRevoked)?;
    let (publisher_dir, pack_dir, file_name) = artifact_parts(
        &stored.publisher_key_id,
        &stored.pack_id,
        stored.release_sequence,
        &stored.signed_release_sha256,
    );
    let envelope = match secure::read(artifact_root, &publisher_dir, &pack_dir, &file_name) {
        Ok(envelope) => envelope,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(ArtifactLoadError::Missing)
        }
        Err(_) => return Err(ArtifactLoadError::Invalid),
    };
    let release =
        verify_pack(&envelope, trusted_publishers).map_err(|_| ArtifactLoadError::Invalid)?;
    if release.publisher_key_id() != stored.publisher_key_id
        || release.manifest().pack_id != stored.pack_id
        || release.release_sequence() != stored.release_sequence
        || release.signed_release_sha256() != stored.signed_release_sha256
    {
        return Err(ArtifactLoadError::Invalid);
    }
    let tested = parse_and_self_test_pack_payload(&release, today)
        .map_err(|_| ArtifactLoadError::Invalid)?;
    Ok((release, tested, publisher))
}

pub(super) fn persist_artifact(
    artifact_root: &Path,
    release: &VerifiedPackRelease,
    envelope: &[u8],
) -> std::io::Result<()> {
    let (publisher_dir, pack_dir, file_name) = artifact_parts(
        release.publisher_key_id(),
        &release.manifest().pack_id,
        release.release_sequence(),
        release.signed_release_sha256(),
    );
    secure::persist(
        artifact_root,
        &publisher_dir,
        &pack_dir,
        &file_name,
        envelope,
    )
}

pub(super) fn remove_owned_artifact(
    artifact_root: &Path,
    release: &StoredPackRelease,
) -> std::io::Result<()> {
    let (publisher_dir, pack_dir, file_name) = artifact_parts(
        &release.publisher_key_id,
        &release.pack_id,
        release.release_sequence,
        &release.signed_release_sha256,
    );
    secure::remove(artifact_root, &publisher_dir, &pack_dir, &file_name)
}

fn sha256(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn artifact_parts(
    publisher_key_id: &str,
    pack_id: &str,
    release_sequence: u64,
    signed_release_sha256: &str,
) -> (String, String, String) {
    (
        sha256(publisher_key_id),
        sha256(pack_id),
        format!("{release_sequence}-{signed_release_sha256}.jspack"),
    )
}
