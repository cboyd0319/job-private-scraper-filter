//! Proves signed pack verification, compatibility, authority, and bounded metadata.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    v3_manifests::{
        AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackType,
        PrivacyLabel, EXTERNAL_AI_GATEWAY_POLICY,
    },
    v3_signed_packs::{
        parse_signed_pack_release, parse_signed_pack_release_for_runtime_test,
        parse_verified_signed_release_for_runtime_test, parse_verified_signed_release_for_test,
        TrustedPublisherKey, CURRENT_V3_RUNTIME_VERSION, MAX_SIGNED_PACK_BYTES,
    },
};

const KEY_ID: &str = "jobsentinel-test-agent-v1";
const PACK_ID: &str = "jobsentinel.test.evidence-review";
const PUBLIC_KEY: &str = "7b7a10c233f71f9ddc725d837ead48592b1fee5b5aec33758d9c885c3f2736ff";
const SIGNATURE: &str = "3508eabc1aff2e475721238410b9c3e1fdede592f685d373c361d26d7e92e1071b1604b91b040fe4b0322b7592c8944cd93db0034f331877eec32f2663155207";
const NODE_SIGNER_PUBLIC_KEY: &str =
    "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c";
const NODE_SIGNER_SIGNATURE: &str = "a52171ad052e4ecfafe343947cb54354a73147678df5dbef1fe43b00be2dfead037a3eeddf8cc3fe02d5c9321ee8c758ac4c65171893565fd41c56ca48d18609";
const SIGNED_RELEASE: &str = r#"{"external_destinations":[],"fixture_summary":"One deterministic local note fixture.","license":"MIT","manifest_json":"{\"allowed_actions\":[\"read_selected_resume_evidence\",\"create_draft_local_note\"],\"allowed_data_categories\":[\"resume_evidence\"],\"allowed_task_kinds\":[\"evidence_review\"],\"approval_gates\":[\"per_execution_review\"],\"execution_class\":\"reviewed_typed_workflow\",\"gateway_policy_id\":null,\"pack_id\":\"jobsentinel.test.evidence-review\",\"pack_type\":\"agent\",\"payload_sha256\":\"5cc2c1542c744e40e81dcbf6cbbeae0c53779d9cddcf6d993de4ec5133f3eaaa\",\"privacy_labels\":[\"local_only\",\"sensitive\"],\"publisher_key_id\":\"jobsentinel-test-agent-v1\",\"schema\":\"jobsentinel.v3.pack-manifest.v1\"}","max_v3_app_version":"3.0.0","min_v3_app_version":"3.0.0","pack_version":"1.0.0","payload":"Create a local draft note for selected evidence.","payload_bytes":48,"publisher_name":"JobSentinel","release_id":"jobsentinel-test-agent-v1:jobsentinel.test.evidence-review:1","release_sequence":1}"#;

fn base_manifest(payload: &str) -> Value {
    json!({
        "schema": "jobsentinel.v3.pack-manifest.v1",
        "pack_id": PACK_ID,
        "pack_type": "agent",
        "execution_class": "reviewed_typed_workflow",
        "publisher_key_id": KEY_ID,
        "payload_sha256": hex::encode(Sha256::digest(payload.as_bytes())),
        "privacy_labels": ["local_only", "sensitive"],
        "allowed_data_categories": ["resume_evidence"],
        "allowed_task_kinds": ["evidence_review"],
        "allowed_actions": ["read_selected_resume_evidence", "create_draft_local_note"],
        "approval_gates": ["per_execution_review"],
        "gateway_policy_id": null
    })
}

fn base_release() -> Value {
    let payload = "Create a local draft note for selected evidence.";
    json!({
        "release_id": format!("{KEY_ID}:{PACK_ID}:1"),
        "pack_version": "1.0.0",
        "min_v3_app_version": "3.0.0",
        "max_v3_app_version": "3.0.0",
        "release_sequence": 1,
        "publisher_name": "JobSentinel",
        "license": "MIT",
        "manifest_json": serde_json::to_string(&base_manifest(payload)).unwrap(),
        "payload": payload,
        "payload_bytes": payload.len(),
        "fixture_summary": "One deterministic local note fixture.",
        "external_destinations": []
    })
}

fn agent_key() -> TrustedPublisherKey {
    TrustedPublisherKey {
        publisher_key_id: KEY_ID.to_string(),
        public_key: hex::decode(PUBLIC_KEY).unwrap().try_into().unwrap(),
        revoked: false,
        pack_type: PackType::Agent,
        execution_class: PackExecutionClass::ReviewedTypedWorkflow,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        allowed_data_categories: vec![DataCategory::ResumeEvidence],
        allowed_task_kinds: vec![AgentTaskKind::EvidenceReview],
        allowed_actions: vec![
            PackAction::ReadSelectedResumeEvidence,
            PackAction::CreateDraftLocalNote,
        ],
        allowed_approval_gates: vec![ApprovalGate::PerExecutionReview],
        allow_gateway_external_ai: false,
    }
}

fn external_ai_key() -> TrustedPublisherKey {
    TrustedPublisherKey {
        allowed_privacy_labels: vec![
            PrivacyLabel::ExternalAiOptional,
            PrivacyLabel::PublicDataOnly,
        ],
        allowed_data_categories: vec![DataCategory::PublicJobPosting],
        allowed_task_kinds: vec![AgentTaskKind::SourceCheck],
        allowed_actions: vec![
            PackAction::ReadPublicJobPosting,
            PackAction::RequestExternalAi,
        ],
        allow_gateway_external_ai: true,
        ..agent_key()
    }
}

fn valid_envelope() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema": "jobsentinel.v3.signed-pack-envelope.v1",
        "publisher_key_id": KEY_ID,
        "signed_release": SIGNED_RELEASE,
        "signature": SIGNATURE
    }))
    .unwrap()
}

fn verify_release(release: &Value, key: &TrustedPublisherKey) -> Result<(), String> {
    parse_verified_signed_release_for_test(&serde_json::to_string(release).unwrap(), key).map(drop)
}

fn replace_manifest(release: &mut Value, update: impl FnOnce(&mut Value)) {
    let mut manifest: Value =
        serde_json::from_str(release["manifest_json"].as_str().unwrap()).unwrap();
    update(&mut manifest);
    release["manifest_json"] = json!(serde_json::to_string(&manifest).unwrap());
}

#[test]
fn valid_signed_release_is_verified_before_any_pack_data_is_exposed() {
    let verified =
        parse_signed_pack_release_for_runtime_test(&valid_envelope(), &[agent_key()], "3.0.0")
            .unwrap();

    assert_eq!(verified.pack_version, "1.0.0");
    assert_eq!(verified.release_sequence, 1);
    assert_eq!(
        verified.signed_release_sha256,
        hex::encode(Sha256::digest(SIGNED_RELEASE.as_bytes()))
    );
    assert_eq!(
        verified.publisher_public_key_sha256,
        hex::encode(Sha256::digest(agent_key().public_key))
    );
    assert_eq!(verified.runtime_version, "3.0.0");
    assert_eq!(verified.minimum_app_version(), "3.0.0");
    assert_eq!(verified.maximum_app_version(), "3.0.0");
    assert_eq!(verified.payload_bytes(), 48);
    assert_eq!(
        verified.fixture_summary(),
        "One deterministic local note fixture."
    );
    assert!(verified.external_destinations().is_empty());
    assert_eq!(verified.manifest.pack_id, PACK_ID);
}

#[test]
fn v3_binary_accepts_an_exact_compatible_pack_release() {
    let verified = parse_signed_pack_release(&valid_envelope(), &[agent_key()])
        .expect("the compiled v3 runtime must accept an exact compatible release");
    assert_eq!(CURRENT_V3_RUNTIME_VERSION, "3.0.0");
    assert_eq!(verified.runtime_version, CURRENT_V3_RUNTIME_VERSION);
}

#[test]
fn rust_verifier_accepts_the_node_release_signer_vector() {
    let envelope = serde_json::to_vec(&json!({
        "schema": "jobsentinel.v3.signed-pack-envelope.v1",
        "publisher_key_id": KEY_ID,
        "signed_release": SIGNED_RELEASE,
        "signature": NODE_SIGNER_SIGNATURE
    }))
    .unwrap();
    let mut key = agent_key();
    key.public_key = hex::decode(NODE_SIGNER_PUBLIC_KEY)
        .unwrap()
        .try_into()
        .unwrap();

    let verified = parse_signed_pack_release(&envelope, &[key]).unwrap();

    assert_eq!(verified.release_sequence(), 1);
    assert_eq!(verified.publisher_key_id(), KEY_ID);
}

#[test]
fn external_ai_release_must_disclose_the_exact_gateway_destination() {
    let mut release = base_release();
    replace_manifest(&mut release, |manifest| {
        manifest["privacy_labels"] = json!(["external_ai_optional", "public_data_only"]);
        manifest["allowed_data_categories"] = json!(["public_job_posting"]);
        manifest["allowed_task_kinds"] = json!(["source_check"]);
        manifest["allowed_actions"] = json!(["read_public_job_posting", "request_external_ai"]);
        manifest["gateway_policy_id"] = json!(EXTERNAL_AI_GATEWAY_POLICY);
    });

    assert!(verify_release(&release, &external_ai_key()).is_err());
    release["external_destinations"] = json!([EXTERNAL_AI_GATEWAY_POLICY]);
    assert!(verify_release(&release, &external_ai_key()).is_ok());
}

#[test]
fn signed_release_identity_is_bound_to_pack_and_sequence() {
    let mut release = base_release();
    release["release_id"] = json!("jobsentinel.test.different-pack:1");
    assert!(verify_release(&release, &agent_key()).is_err());
}

#[test]
fn release_identity_includes_the_trusted_publisher() {
    let mut release = base_release();
    release["release_id"] = json!(format!("{KEY_ID}:{PACK_ID}:1"));
    let first = parse_verified_signed_release_for_test(
        &serde_json::to_string(&release).unwrap(),
        &agent_key(),
    )
    .unwrap();

    let second_key_id = "jobsentinel-test-agent-v2";
    let mut second_release = release;
    second_release["release_id"] = json!(format!("{second_key_id}:{PACK_ID}:1"));
    replace_manifest(&mut second_release, |manifest| {
        manifest["publisher_key_id"] = json!(second_key_id);
    });
    let mut second_key = agent_key();
    second_key.publisher_key_id = second_key_id.to_string();
    let second = parse_verified_signed_release_for_test(
        &serde_json::to_string(&second_release).unwrap(),
        &second_key,
    )
    .unwrap();

    assert_ne!(first.release_id, second.release_id);
}

#[test]
fn compatibility_uses_the_compiled_application_version() {
    assert_eq!(CURRENT_V3_RUNTIME_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn publisher_and_license_are_bounded_display_text() {
    let mut release = base_release();
    release["publisher_name"] = json!("JobSentinel Project");
    release["license"] = json!("Apache-2.0 OR MIT");
    assert!(verify_release(&release, &agent_key()).is_ok());
}

#[test]
fn signed_display_metadata_rejects_control_characters() {
    let mut release = base_release();
    release["fixture_summary"] = json!("Safe summary\nforged status");
    assert!(verify_release(&release, &agent_key()).is_err());
}

#[test]
fn signature_and_trust_failures_expose_no_untrusted_content() {
    let envelope = valid_envelope();
    let mut revoked = agent_key();
    revoked.revoked = true;
    let duplicate = agent_key();
    for (candidate, keys) in [
        (envelope.clone(), Vec::new()),
        (envelope.clone(), vec![revoked]),
        (envelope.clone(), vec![duplicate.clone(), duplicate]),
        (
            {
                let mut value: Value = serde_json::from_slice(&envelope).unwrap();
                value["signature"] = json!("00".repeat(64));
                serde_json::to_vec(&value).unwrap()
            },
            vec![agent_key()],
        ),
        (
            {
                let mut value: Value = serde_json::from_slice(&envelope).unwrap();
                value["signed_release"] = json!(format!("{SIGNED_RELEASE} "));
                serde_json::to_vec(&value).unwrap()
            },
            vec![agent_key()],
        ),
    ] {
        let error = parse_signed_pack_release(&candidate, &keys).unwrap_err();
        assert!(!error.contains("Create a local draft"));
        assert!(!error.contains("00".repeat(64).as_str()));
        assert!(!error.contains('/'));
    }
}

#[test]
fn verified_release_cannot_hide_payload_or_publisher_changes() {
    let mut wrong_payload = base_release();
    wrong_payload["payload"] = json!("Changed after the manifest hash was declared.");
    wrong_payload["payload_bytes"] = json!(wrong_payload["payload"].as_str().unwrap().len());
    assert!(verify_release(&wrong_payload, &agent_key()).is_err());

    let mut wrong_publisher = base_release();
    replace_manifest(&mut wrong_publisher, |manifest| {
        manifest["publisher_key_id"] = json!("jobsentinel-test-other-v1");
    });
    assert!(verify_release(&wrong_publisher, &agent_key()).is_err());
}

#[test]
fn publisher_authority_is_an_exact_ceiling() {
    let mut narrower = agent_key();
    narrower.allowed_actions = vec![PackAction::ReadSelectedResumeEvidence];
    assert!(verify_release(&base_release(), &narrower).is_err());

    let mut source_only = agent_key();
    source_only.pack_type = PackType::Source;
    source_only.execution_class = PackExecutionClass::StaticContent;
    source_only.allowed_data_categories.clear();
    source_only.allowed_task_kinds.clear();
    source_only.allowed_actions.clear();
    source_only.allowed_approval_gates.clear();
    assert!(verify_release(&base_release(), &source_only).is_err());
}

#[test]
fn duplicate_and_unknown_signed_fields_fail_closed() {
    let release = serde_json::to_string(&base_release()).unwrap();
    let duplicate_release = format!("{{\"release_id\":\"duplicate\",{}", &release[1..]);
    assert!(parse_verified_signed_release_for_test(&duplicate_release, &agent_key()).is_err());

    let mut duplicate_manifest = base_release();
    let manifest = duplicate_manifest["manifest_json"].as_str().unwrap();
    duplicate_manifest["manifest_json"] =
        json!(format!("{{\"pack_id\":\"duplicate\",{}", &manifest[1..]));
    assert!(verify_release(&duplicate_manifest, &agent_key()).is_err());

    let envelope = String::from_utf8(valid_envelope()).unwrap();
    let duplicate_outer = format!("{{\"schema\":\"duplicate\",{}", &envelope[1..]);
    assert!(parse_signed_pack_release(duplicate_outer.as_bytes(), &[agent_key()]).is_err());

    let mut unknown_action = base_release();
    replace_manifest(&mut unknown_action, |manifest| {
        manifest["allowed_actions"] = json!(["unrestricted_shell"]);
    });
    assert!(verify_release(&unknown_action, &agent_key()).is_err());
}

#[test]
fn release_bounds_and_compatibility_fail_before_install() {
    for (field, value) in [
        ("release_sequence", json!(0)),
        ("payload_bytes", json!(1)),
        ("min_v3_app_version", json!("3.0.1")),
        ("max_v3_app_version", json!("2.9.9")),
    ] {
        let mut release = base_release();
        release[field] = value;
        assert!(verify_release(&release, &agent_key()).is_err());
    }
    assert!(parse_signed_pack_release(&[0xff], &[agent_key()]).is_err());
    assert!(
        parse_signed_pack_release(&vec![b' '; MAX_SIGNED_PACK_BYTES + 1], &[agent_key()],).is_err()
    );
}

#[test]
fn compatibility_accepts_exact_and_bounded_patch_versions() {
    let release = base_release();
    assert!(parse_verified_signed_release_for_runtime_test(
        &serde_json::to_string(&release).unwrap(),
        &agent_key(),
        "3.0.0",
    )
    .is_ok());

    let mut patch_range = release;
    patch_range["max_v3_app_version"] = json!("3.0.2");
    let patch_range = serde_json::to_string(&patch_range).unwrap();
    assert!(
        parse_verified_signed_release_for_runtime_test(&patch_range, &agent_key(), "3.0.1",)
            .is_ok()
    );
    assert!(
        parse_verified_signed_release_for_runtime_test(&patch_range, &agent_key(), "2.9.9",)
            .is_err()
    );
    assert!(
        parse_verified_signed_release_for_runtime_test(&patch_range, &agent_key(), "3.0.3",)
            .is_err()
    );
}

#[test]
fn capability_lists_and_direct_external_routes_are_not_ambiguous() {
    let mut duplicate_action = base_release();
    replace_manifest(&mut duplicate_action, |manifest| {
        manifest["allowed_actions"] = json!([
            "read_selected_resume_evidence",
            "create_draft_local_note",
            "create_draft_local_note"
        ]);
    });
    assert!(verify_release(&duplicate_action, &agent_key()).is_err());

    let mut direct_route = base_release();
    direct_route["external_destinations"] = json!(["https://example.com/upload"]);
    assert!(verify_release(&direct_route, &agent_key()).is_err());
}
