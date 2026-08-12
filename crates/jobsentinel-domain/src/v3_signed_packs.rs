//! Strict verification for bounded, signed pack releases.

use jobsentinel_security::verify_ed25519_signature;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::v3_manifests::{
    AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackManifest,
    PackType, PrivacyLabel, EXTERNAL_AI_GATEWAY_POLICY,
};

pub const MAX_SIGNED_PACK_BYTES: usize = 4 * 1024 * 1024;
pub const CURRENT_V3_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

const SIGNED_PACK_SCHEMA: &str = "jobsentinel.v3.signed-pack-envelope.v1";
const SIGNING_DOMAIN: &[u8] = b"jobsentinel.pack-envelope.v1\0";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_VERSION_BYTES: usize = 64;
const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_PAYLOAD_BYTES: usize = 3 * 1024 * 1024;
const MAX_FIXTURE_SUMMARY_BYTES: usize = 4096;
const MAX_CAPABILITY_ITEMS: usize = 16;
const MAX_EXTERNAL_DESTINATIONS: usize = 4;
const VERIFICATION_ERROR: &str = "signed pack verification failed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPublisherKey {
    pub publisher_key_id: String,
    pub public_key: [u8; 32],
    pub revoked: bool,
    pub pack_type: PackType,
    pub execution_class: PackExecutionClass,
    pub allowed_privacy_labels: Vec<PrivacyLabel>,
    pub allowed_data_categories: Vec<DataCategory>,
    pub allowed_task_kinds: Vec<AgentTaskKind>,
    pub allowed_actions: Vec<PackAction>,
    pub allowed_approval_gates: Vec<ApprovalGate>,
    pub allow_gateway_external_ai: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPackRelease {
    pub(crate) release_id: String,
    pub(crate) pack_version: String,
    pub(crate) minimum_app_version: String,
    pub(crate) maximum_app_version: String,
    pub(crate) release_sequence: u64,
    pub(crate) signed_release_sha256: String,
    pub(crate) publisher_public_key_sha256: String,
    pub(crate) publisher_key_id: String,
    pub(crate) publisher_name: String,
    pub(crate) license: String,
    pub(crate) manifest: PackManifest,
    pub(crate) payload: String,
    pub(crate) payload_bytes: u64,
    pub(crate) fixture_summary: String,
    pub(crate) external_destinations: Vec<String>,
    pub(crate) runtime_version: &'static str,
}

impl VerifiedPackRelease {
    pub fn release_id(&self) -> &str {
        &self.release_id
    }

    pub fn pack_version(&self) -> &str {
        &self.pack_version
    }

    pub fn minimum_app_version(&self) -> &str {
        &self.minimum_app_version
    }

    pub fn maximum_app_version(&self) -> &str {
        &self.maximum_app_version
    }

    pub const fn release_sequence(&self) -> u64 {
        self.release_sequence
    }

    pub fn signed_release_sha256(&self) -> &str {
        &self.signed_release_sha256
    }

    pub fn publisher_public_key_sha256(&self) -> &str {
        &self.publisher_public_key_sha256
    }

    pub fn publisher_key_id(&self) -> &str {
        &self.publisher_key_id
    }

    pub fn publisher_name(&self) -> &str {
        &self.publisher_name
    }

    pub fn license(&self) -> &str {
        &self.license
    }

    pub const fn manifest(&self) -> &PackManifest {
        &self.manifest
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }

    pub const fn payload_bytes(&self) -> u64 {
        self.payload_bytes
    }

    pub fn fixture_summary(&self) -> &str {
        &self.fixture_summary
    }

    pub fn external_destinations(&self) -> &[String] {
        &self.external_destinations
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedPackEnvelope {
    schema: String,
    publisher_key_id: String,
    signed_release: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedPackRelease {
    release_id: String,
    pack_version: String,
    min_v3_app_version: String,
    max_v3_app_version: String,
    release_sequence: u64,
    publisher_name: String,
    license: String,
    manifest_json: String,
    payload: String,
    payload_bytes: u64,
    fixture_summary: String,
    external_destinations: Vec<String>,
}

/// Verify an untrusted signed pack release without installing or executing it.
pub fn parse_signed_pack_release(
    input: &[u8],
    trusted_keys: &[TrustedPublisherKey],
) -> Result<VerifiedPackRelease, String> {
    parse_signed_pack_release_at_runtime(input, trusted_keys, CURRENT_V3_RUNTIME_VERSION)
}

fn parse_signed_pack_release_at_runtime(
    input: &[u8],
    trusted_keys: &[TrustedPublisherKey],
    runtime_version: &'static str,
) -> Result<VerifiedPackRelease, String> {
    if input.len() > MAX_SIGNED_PACK_BYTES {
        return Err("signed pack exceeds the byte limit".to_string());
    }
    let input = std::str::from_utf8(input).map_err(|_| VERIFICATION_ERROR.to_string())?;
    let envelope: SignedPackEnvelope =
        serde_json::from_str(input).map_err(|_| VERIFICATION_ERROR.to_string())?;
    if envelope.schema != SIGNED_PACK_SCHEMA
        || !is_identifier(&envelope.publisher_key_id)
        || envelope.signed_release.len() > MAX_SIGNED_PACK_BYTES
    {
        return Err(VERIFICATION_ERROR.to_string());
    }
    let key = trusted_key(&envelope.publisher_key_id, trusted_keys)?;
    let signing_bytes = signing_bytes(&envelope.publisher_key_id, &envelope.signed_release);
    verify_ed25519_signature(&key.public_key, &signing_bytes, &envelope.signature)
        .map_err(|_| VERIFICATION_ERROR.to_string())?;

    parse_verified_release(
        &envelope.publisher_key_id,
        &envelope.signed_release,
        key,
        runtime_version,
    )
}

fn parse_verified_release(
    publisher_key_id: &str,
    signed_release: &str,
    key: &TrustedPublisherKey,
    runtime_version: &'static str,
) -> Result<VerifiedPackRelease, String> {
    let release: SignedPackRelease = serde_json::from_str(signed_release)
        .map_err(|_| "signed pack release is invalid".to_string())?;
    validate_release(&release, runtime_version)?;
    let manifest: PackManifest = serde_json::from_str(&release.manifest_json)
        .map_err(|_| "signed pack release is invalid".to_string())?;
    manifest
        .validate()
        .map_err(|_| "signed pack release is invalid".to_string())?;
    validate_manifest_bounds(&manifest)?;
    if manifest.publisher_key_id != publisher_key_id
        || release.release_id
            != format!(
                "{publisher_key_id}:{}:{}",
                manifest.pack_id, release.release_sequence
            )
    {
        return Err("signed pack identity is inconsistent".to_string());
    }
    manifest
        .verify_payload(release.payload.as_bytes())
        .map_err(|_| "signed pack payload is invalid".to_string())?;
    if !key_allows_manifest(key, &manifest) {
        return Err("signed pack authority is not allowed".to_string());
    }
    validate_external_destinations(&release.external_destinations, &manifest)?;

    Ok(VerifiedPackRelease {
        release_id: release.release_id,
        pack_version: release.pack_version,
        minimum_app_version: release.min_v3_app_version,
        maximum_app_version: release.max_v3_app_version,
        release_sequence: release.release_sequence,
        signed_release_sha256: hex::encode(Sha256::digest(signed_release.as_bytes())),
        publisher_public_key_sha256: hex::encode(Sha256::digest(key.public_key)),
        publisher_key_id: publisher_key_id.to_string(),
        publisher_name: release.publisher_name,
        license: release.license,
        manifest,
        payload: release.payload,
        payload_bytes: release.payload_bytes,
        fixture_summary: release.fixture_summary,
        external_destinations: release.external_destinations,
        runtime_version,
    })
}

#[cfg(test)]
pub(crate) fn parse_verified_signed_release_for_test(
    signed_release: &str,
    key: &TrustedPublisherKey,
) -> Result<VerifiedPackRelease, String> {
    parse_verified_release(&key.publisher_key_id, signed_release, key, "3.0.0")
}

#[cfg(any(test, feature = "test-support"))]
pub fn parse_signed_pack_release_for_runtime_test(
    input: &[u8],
    trusted_keys: &[TrustedPublisherKey],
    runtime_version: &'static str,
) -> Result<VerifiedPackRelease, String> {
    parse_signed_pack_release_at_runtime(input, trusted_keys, runtime_version)
}

#[cfg(test)]
pub(crate) fn parse_verified_signed_release_for_runtime_test(
    signed_release: &str,
    key: &TrustedPublisherKey,
    runtime_version: &'static str,
) -> Result<VerifiedPackRelease, String> {
    parse_verified_release(&key.publisher_key_id, signed_release, key, runtime_version)
}

fn trusted_key<'a>(
    publisher_key_id: &str,
    trusted_keys: &'a [TrustedPublisherKey],
) -> Result<&'a TrustedPublisherKey, String> {
    let mut matching = trusted_keys
        .iter()
        .filter(|key| key.publisher_key_id == publisher_key_id);
    let Some(key) = matching.next() else {
        return Err(VERIFICATION_ERROR.to_string());
    };
    if matching.next().is_some() || key.revoked || !is_identifier(&key.publisher_key_id) {
        return Err(VERIFICATION_ERROR.to_string());
    }
    Ok(key)
}

fn signing_bytes(publisher_key_id: &str, signed_release: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(
        SIGNING_DOMAIN.len() + 16 + publisher_key_id.len() + signed_release.len(),
    );
    bytes.extend_from_slice(SIGNING_DOMAIN);
    append_length_delimited(&mut bytes, publisher_key_id.as_bytes());
    append_length_delimited(&mut bytes, signed_release.as_bytes());
    bytes
}

fn append_length_delimited(target: &mut Vec<u8>, value: &[u8]) {
    target.extend_from_slice(&(value.len() as u64).to_le_bytes());
    target.extend_from_slice(value);
}

fn validate_release(
    release: &SignedPackRelease,
    runtime_version: &'static str,
) -> Result<(), String> {
    require_identifier(&release.release_id, "signed pack release is invalid")?;
    require_bounded_text(
        &release.pack_version,
        MAX_VERSION_BYTES,
        "signed pack release is invalid",
    )?;
    require_display_text(&release.publisher_name, MAX_IDENTIFIER_BYTES)?;
    require_display_text(&release.license, MAX_IDENTIFIER_BYTES)?;
    require_display_text(&release.fixture_summary, MAX_FIXTURE_SUMMARY_BYTES)?;
    if release.manifest_json.len() > MAX_MANIFEST_BYTES
        || release.payload.len() > MAX_PAYLOAD_BYTES
        || release.release_sequence == 0
        || u64::try_from(release.payload.len()).ok() != Some(release.payload_bytes)
        || release.external_destinations.len() > MAX_EXTERNAL_DESTINATIONS
        || !is_semver(&release.pack_version)
        || !is_v3_semver(&release.min_v3_app_version)
        || !is_v3_semver(&release.max_v3_app_version)
    {
        return Err("signed pack release is invalid".to_string());
    }
    let minimum = semver_parts(&release.min_v3_app_version)
        .ok_or_else(|| "signed pack release is invalid".to_string())?;
    let maximum = semver_parts(&release.max_v3_app_version)
        .ok_or_else(|| "signed pack release is invalid".to_string())?;
    let current = semver_parts(runtime_version)
        .ok_or_else(|| "signed pack release is invalid".to_string())?;
    if minimum > maximum || current < minimum || current > maximum {
        return Err("signed pack is not compatible".to_string());
    }
    if !release
        .external_destinations
        .iter()
        .all(|value| value.len() <= MAX_IDENTIFIER_BYTES && !value.is_empty())
    {
        return Err("signed pack release is invalid".to_string());
    }
    Ok(())
}

fn validate_manifest_bounds(manifest: &PackManifest) -> Result<(), String> {
    if !is_identifier(&manifest.pack_id)
        || !is_identifier(&manifest.publisher_key_id)
        || manifest.privacy_labels.len() > MAX_CAPABILITY_ITEMS
        || manifest.allowed_data_categories.len() > MAX_CAPABILITY_ITEMS
        || manifest.allowed_task_kinds.len() > MAX_CAPABILITY_ITEMS
        || manifest.allowed_actions.len() > MAX_CAPABILITY_ITEMS
        || manifest.approval_gates.len() > MAX_CAPABILITY_ITEMS
        || manifest
            .gateway_policy_id
            .as_deref()
            .is_some_and(|value| !is_identifier(value))
        || contains_duplicate(&manifest.privacy_labels)
        || contains_duplicate(&manifest.allowed_data_categories)
        || contains_duplicate(&manifest.allowed_task_kinds)
        || contains_duplicate(&manifest.allowed_actions)
        || contains_duplicate(&manifest.approval_gates)
    {
        return Err("signed pack release is invalid".to_string());
    }
    Ok(())
}

fn key_allows_manifest(key: &TrustedPublisherKey, manifest: &PackManifest) -> bool {
    key.pack_type == manifest.pack_type
        && key.execution_class == manifest.execution_class
        && is_subset(&manifest.privacy_labels, &key.allowed_privacy_labels)
        && is_subset(
            &manifest.allowed_data_categories,
            &key.allowed_data_categories,
        )
        && is_subset(&manifest.allowed_task_kinds, &key.allowed_task_kinds)
        && is_subset(&manifest.allowed_actions, &key.allowed_actions)
        && is_subset(&manifest.approval_gates, &key.allowed_approval_gates)
        && (!manifest
            .allowed_actions
            .contains(&PackAction::RequestExternalAi)
            || key.allow_gateway_external_ai)
}

fn validate_external_destinations(
    destinations: &[String],
    manifest: &PackManifest,
) -> Result<(), String> {
    let requests_external_ai = manifest
        .allowed_actions
        .contains(&PackAction::RequestExternalAi);
    if (requests_external_ai
        && (manifest.gateway_policy_id.as_deref() != Some(EXTERNAL_AI_GATEWAY_POLICY)
            || destinations != [EXTERNAL_AI_GATEWAY_POLICY]))
        || (!requests_external_ai && !destinations.is_empty())
    {
        return Err("signed pack external route is not allowed".to_string());
    }
    Ok(())
}

fn is_subset<T: PartialEq>(requested: &[T], ceiling: &[T]) -> bool {
    requested
        .iter()
        .all(|requested_value| ceiling.contains(requested_value))
}

fn contains_duplicate<T: PartialEq>(values: &[T]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[(index + 1)..].contains(value))
}

fn require_identifier(value: &str, error: &str) -> Result<(), String> {
    if is_identifier(value) {
        Ok(())
    } else {
        Err(error.to_string())
    }
}

fn require_bounded_text(value: &str, max_bytes: usize, error: &str) -> Result<(), String> {
    if !value.trim().is_empty() && value.len() <= max_bytes {
        Ok(())
    } else {
        Err(error.to_string())
    }
}

fn require_display_text(value: &str, max_bytes: usize) -> Result<(), String> {
    if value.trim() == value
        && value.len() <= max_bytes
        && !value.is_empty()
        && !value.chars().any(char::is_control)
    {
        Ok(())
    } else {
        Err("signed pack release is invalid".to_string())
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_v3_semver(value: &str) -> bool {
    semver_parts(value).is_some_and(|parts| parts[0] == 3)
}

fn is_semver(value: &str) -> bool {
    semver_parts(value).is_some()
}

fn semver_parts(value: &str) -> Option<[u64; 3]> {
    let mut parts = value.split('.');
    let components = [parts.next()?, parts.next()?, parts.next()?];
    if parts.next().is_some() {
        return None;
    }
    let parsed = components.map(|part| {
        (!part.is_empty()
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part.len() == 1 || !part.starts_with('0')))
        .then(|| part.parse().ok())
        .flatten()
    });
    Some([parsed[0]?, parsed[1]?, parsed[2]?])
}
