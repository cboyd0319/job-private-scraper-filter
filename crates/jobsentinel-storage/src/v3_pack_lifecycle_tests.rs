//! Exercises signed pack staging, lifecycle, recovery, integrity, and management state.

use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{PackExecutionClass, PackManifest, PackType, PrivacyLabel},
    v3_pack_payloads::{parse_and_self_test_pack_payload, SelfTestedPackRelease},
    v3_signed_packs::{parse_signed_pack_release, TrustedPublisherKey, VerifiedPackRelease},
};
use jobsentinel_security::sign_ed25519_for_test;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    pack_lifecycle_error_kind,
    test_support::migrated_database,
    v3_pack_lifecycle::{PackAvailability, PackStageOutcome, PackStream},
};

const PUBLISHER_ID: &str = "jobsentinel-test-source-v1";
const PACK_ID: &str = "jobsentinel.test.synthetic-source";

fn publisher(seed: u8) -> TrustedPublisherKey {
    let (public_key, _) = sign_ed25519_for_test(&[seed; 32], &[]).unwrap();
    TrustedPublisherKey {
        publisher_key_id: PUBLISHER_ID.to_string(),
        public_key,
        revoked: false,
        pack_type: PackType::Source,
        execution_class: PackExecutionClass::StaticContent,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly],
        allowed_data_categories: vec![],
        allowed_task_kinds: vec![],
        allowed_actions: vec![],
        allowed_approval_gates: vec![],
        allow_gateway_external_ai: false,
    }
}

fn release(sequence: u64, payload: String, publisher: &TrustedPublisherKey) -> VerifiedPackRelease {
    release_for_pack(PACK_ID, sequence, payload, publisher)
}

fn release_for_pack(
    pack_id: &str,
    sequence: u64,
    payload: String,
    publisher: &TrustedPublisherKey,
) -> VerifiedPackRelease {
    let manifest = PackManifest {
        schema: SchemaId::PackManifestV1,
        pack_id: pack_id.to_string(),
        pack_type: PackType::Source,
        execution_class: PackExecutionClass::StaticContent,
        publisher_key_id: PUBLISHER_ID.to_string(),
        payload_sha256: hex::encode(Sha256::digest(payload.as_bytes())),
        privacy_labels: vec![PrivacyLabel::LocalOnly],
        allowed_data_categories: vec![],
        allowed_task_kinds: vec![],
        allowed_actions: vec![],
        approval_gates: vec![],
        gateway_policy_id: None,
    };
    let signed_release = serde_json::to_string(&json!({
        "release_id": format!("{PUBLISHER_ID}:{pack_id}:{sequence}"),
        "pack_version": format!("1.0.{sequence}"),
        "min_v3_app_version": "3.0.0",
        "max_v3_app_version": "3.0.0",
        "release_sequence": sequence,
        "publisher_name": "JobSentinel Test",
        "license": "MIT",
        "manifest_json": serde_json::to_string(&manifest).unwrap(),
        "payload": payload,
        "payload_bytes": payload.len(),
        "fixture_summary": "Synthetic source fixtures",
        "external_destinations": []
    }))
    .unwrap();
    let mut signing_bytes = b"jobsentinel.pack-envelope.v1\0".to_vec();
    for value in [PUBLISHER_ID.as_bytes(), signed_release.as_bytes()] {
        signing_bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        signing_bytes.extend_from_slice(value);
    }
    let (public_key, signature) = sign_ed25519_for_test(&[7; 32], &signing_bytes).unwrap();
    assert_eq!(public_key, publisher.public_key);
    let envelope = serde_json::to_vec(&json!({
        "schema": "jobsentinel.v3.signed-pack-envelope.v1",
        "publisher_key_id": PUBLISHER_ID,
        "signed_release": signed_release,
        "signature": hex::encode(signature)
    }))
    .unwrap();
    parse_signed_pack_release(&envelope, std::slice::from_ref(publisher)).unwrap()
}

fn valid_source_payload() -> String {
    serde_json::to_string(&json!({
        "schema": "jobsentinel.v3.pack-payload.v1",
        "pack_type": "source",
        "policy": {
            "source_id": "synthetic-official-jobs",
            "source_class": "official_public_api",
            "access": "disabled",
            "request_limit_per_hour": 0,
            "user_review_required": false,
            "policy_ref": "synthetic-official-jobs-v1",
            "revision": 1,
            "restriction_reason_code": null,
            "reviewed_at": "2026-07-19T00:00:00Z"
        },
        "manifest_json": include_str!("../../jobsentinel-domain/src/fixtures/v3_source_manifest_v1.json"),
        "fixtures": [
            {
                "path": "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_list.json",
                "content": include_str!("../../jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_list.json")
            },
            {
                "path": "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_detail.json",
                "content": include_str!("../../jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_detail.json")
            },
            {
                "path": "crates/jobsentinel-domain/src/fixtures/source_reviews/synthetic_official_v1.json",
                "content": include_str!("../../jobsentinel-domain/src/fixtures/source_reviews/synthetic_official_v1.json")
            }
        ]
    }))
    .unwrap()
}

async fn activate_release(
    database: &crate::Database,
    release: &VerifiedPackRelease,
    publisher: &TrustedPublisherKey,
) -> (SelfTestedPackRelease, PackStream) {
    let PackStageOutcome::Staged(staged) = database
        .stage_verified_pack(release, publisher)
        .await
        .unwrap()
    else {
        panic!("release must stage before activation");
    };
    let tested =
        parse_and_self_test_pack_payload(release, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap())
            .unwrap();
    let self_tested = database
        .record_pack_self_test(&tested, staged.generation)
        .await
        .unwrap();
    let active = database
        .activate_self_tested_pack(&tested, publisher, self_tested.generation)
        .await
        .unwrap();
    (tested, active)
}

mod artifacts;
mod lifecycle;
mod management;
mod recovery;
mod staging;
