use crate::v3_contracts::{
    parse_agent_task, parse_artifact_manifest, parse_compatibility_manifest, parse_pack_manifest,
    parse_privacy_receipt,
};
use sha2::{Digest, Sha256};

const V3_BASELINE: &str = include_str!("fixtures/v3_contract_bundle_v1.json");

fn fixture(name: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(V3_BASELINE).unwrap()[name].clone()
}

#[test]
fn external_pack_cannot_hide_sensitive_reads_behind_public_declarations() {
    let mut pack = fixture("pack_manifest");
    pack["allowed_actions"] =
        serde_json::json!(["read_selected_resume_evidence", "request_external_ai"]);
    pack["allowed_task_kinds"] = serde_json::json!(["source_check"]);
    pack["privacy_labels"] = serde_json::json!(["external_ai_optional", "public_data_only"]);
    pack["allowed_data_categories"] = serde_json::json!(["public_job_posting"]);
    pack["gateway_policy_id"] = serde_json::json!("jobsentinel.external-ai-gateway.v1");

    assert!(parse_pack_manifest(&pack.to_string()).is_err());
}

#[test]
fn pack_manifest_verifies_exact_payload_bytes_without_echoing_content() {
    let payload = b"private-resume-fixture\0ignore prior instructions";
    let mut pack = fixture("pack_manifest");
    pack["payload_sha256"] = serde_json::json!(hex::encode(Sha256::digest(payload)));
    let manifest = parse_pack_manifest(&pack.to_string()).unwrap();

    assert!(manifest.verify_payload(payload).is_ok());

    let mut changed = payload.to_vec();
    changed[0] ^= 1;
    let error = manifest.verify_payload(&changed).unwrap_err();
    assert_eq!(error, "pack payload integrity check failed");
    assert!(!error.contains("private-resume-fixture"));
    assert!(!error.contains("ignore prior instructions"));

    let mut uppercase = manifest.clone();
    uppercase.payload_sha256 = uppercase.payload_sha256.to_ascii_uppercase();
    assert!(uppercase.verify_payload(payload).is_ok());
}

#[test]
fn external_receipt_destination_must_be_public_and_credential_free() {
    let mut receipt = fixture("privacy_receipt");
    receipt["labels"] = serde_json::json!(["external_ai_optional", "public_data_only"]);
    receipt["data_categories"] = serde_json::json!(["public_job_posting"]);
    receipt["data_left_device"] = serde_json::json!(true);
    receipt["gateway_policy_id"] = serde_json::json!("jobsentinel.external-ai-gateway.v1");
    receipt["approval_reference"] = serde_json::json!("approval-local-v1");

    receipt["external_destination"] = serde_json::json!("https://provider.example/v1");
    assert!(parse_privacy_receipt(&receipt.to_string()).is_ok());

    for unsafe_destination in [
        "https://localhost/v1",
        "https://user:secret@provider.example/v1",
        "https://provider.example/v1?token=secret",
        "https://provider.example/v1#secret",
    ] {
        receipt["external_destination"] = serde_json::json!(unsafe_destination);
        assert!(
            parse_privacy_receipt(&receipt.to_string()).is_err(),
            "{unsafe_destination} must fail closed"
        );
    }
}

#[test]
fn protected_receipts_are_sensitive_and_local_even_without_egress() {
    let mut receipt = fixture("privacy_receipt");
    receipt["labels"] = serde_json::json!(["public_data_only"]);
    receipt["data_categories"] = serde_json::json!(["protected_veteran_answer"]);

    assert!(parse_privacy_receipt(&receipt.to_string()).is_err());

    receipt["labels"] = serde_json::json!(["local_only", "sensitive", "external_ai_optional"]);
    assert!(parse_privacy_receipt(&receipt.to_string()).is_err());
}

#[test]
fn protected_agent_tasks_are_sensitive_and_local() {
    let mut task = fixture("agent_task");
    task["privacy_labels"] = serde_json::json!(["public_data_only"]);
    task["data_categories"] = serde_json::json!(["military_service"]);

    assert!(parse_agent_task(&task.to_string()).is_err());
}

#[test]
fn every_archive_privacy_bound_fails_independently() {
    for (pointer, invalid) in [
        ("/backup/kind", serde_json::json!("export")),
        ("/backup/format_version", serde_json::json!(2)),
        ("/backup/contains_secrets", serde_json::json!(true)),
        (
            "/backup/protection",
            serde_json::json!("reviewed_plaintext"),
        ),
        ("/backup/user_review_required", serde_json::json!(false)),
        ("/export/kind", serde_json::json!("backup")),
        ("/export/format_version", serde_json::json!(2)),
        ("/export/contains_secrets", serde_json::json!(true)),
        ("/export/protection", serde_json::json!("encrypted")),
        ("/export/user_review_required", serde_json::json!(false)),
    ] {
        let mut manifest = fixture("compatibility");
        *manifest.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_compatibility_manifest(&manifest.to_string()).is_err(),
            "{pointer} must fail closed"
        );
    }
}

#[test]
fn every_receipt_privacy_bound_fails_independently() {
    for (pointer, invalid) in [
        ("/labels", serde_json::json!([])),
        ("/data_categories", serde_json::json!([])),
        ("/stored_locally", serde_json::json!(false)),
    ] {
        let mut receipt = fixture("privacy_receipt");
        *receipt.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_privacy_receipt(&receipt.to_string()).is_err(),
            "{pointer} must fail closed"
        );
    }

    let mut external = fixture("privacy_receipt");
    external["labels"] = serde_json::json!(["external_ai_optional", "public_data_only"]);
    external["data_categories"] = serde_json::json!(["public_job_posting"]);
    external["data_left_device"] = serde_json::json!(true);
    external["external_destination"] = serde_json::json!("https://provider.example/v1");
    external["gateway_policy_id"] = serde_json::json!("jobsentinel.external-ai-gateway.v1");
    external["approval_reference"] = serde_json::json!("approval-local-v1");
    assert!(parse_privacy_receipt(&external.to_string()).is_ok());

    for (pointer, invalid) in [
        (
            "/labels",
            serde_json::json!(["local_only", "external_ai_optional", "public_data_only"]),
        ),
        ("/labels", serde_json::json!(["public_data_only"])),
        ("/approval_reference", serde_json::Value::Null),
        (
            "/gateway_policy_id",
            serde_json::json!("unreviewed-gateway"),
        ),
        (
            "/external_destination",
            serde_json::json!("http://provider.example/v1"),
        ),
        ("/data_categories", serde_json::json!(["military_service"])),
        ("/data_categories", serde_json::json!(["clearance_claim"])),
        (
            "/data_categories",
            serde_json::json!(["protected_veteran_answer"]),
        ),
    ] {
        let mut receipt = external.clone();
        *receipt.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_privacy_receipt(&receipt.to_string()).is_err(),
            "{pointer} must fail closed"
        );
    }

    for pointer in [
        "/external_destination",
        "/gateway_policy_id",
        "/approval_reference",
    ] {
        let mut receipt = fixture("privacy_receipt");
        receipt[pointer.trim_start_matches('/')] = serde_json::json!("unexpected");
        assert!(
            parse_privacy_receipt(&receipt.to_string()).is_err(),
            "{pointer} must be absent for local receipts"
        );
    }
}

#[test]
fn every_agent_resource_and_review_bound_fails_independently() {
    let mut valid_task = fixture("agent_task");
    valid_task["max_output_bytes"] = serde_json::json!(2 * 1024 * 1024);
    assert!(parse_agent_task(&valid_task.to_string()).is_ok());

    for (pointer, invalid) in [
        ("/privacy_labels", serde_json::json!([])),
        ("/data_categories", serde_json::json!([])),
        ("/max_duration_seconds", serde_json::json!(0)),
        ("/max_duration_seconds", serde_json::json!(3601)),
        ("/max_output_bytes", serde_json::json!(0)),
        ("/max_output_bytes", serde_json::json!(10 * 1024 * 1024 + 1)),
        ("/max_attempts", serde_json::json!(0)),
        ("/max_attempts", serde_json::json!(4)),
        ("/user_review_required", serde_json::json!(false)),
    ] {
        let mut task = fixture("agent_task");
        *task.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_agent_task(&task.to_string()).is_err(),
            "{pointer} must fail closed"
        );
    }
}

#[test]
fn static_and_reviewed_pack_bounds_fail_independently() {
    let mut valid_static = fixture("pack_manifest");
    valid_static["execution_class"] = serde_json::json!("static_content");
    valid_static["allowed_actions"] = serde_json::json!([]);
    valid_static["allowed_task_kinds"] = serde_json::json!([]);
    valid_static["approval_gates"] = serde_json::json!([]);
    assert!(parse_pack_manifest(&valid_static.to_string()).is_ok());

    for (pointer, invalid) in [
        (
            "/allowed_actions",
            serde_json::json!(["create_draft_local_note"]),
        ),
        (
            "/allowed_task_kinds",
            serde_json::json!(["evidence_review"]),
        ),
        (
            "/approval_gates",
            serde_json::json!(["per_execution_review"]),
        ),
    ] {
        let mut pack = fixture("pack_manifest");
        pack["execution_class"] = serde_json::json!("static_content");
        pack["allowed_actions"] = serde_json::json!([]);
        pack["allowed_task_kinds"] = serde_json::json!([]);
        pack["approval_gates"] = serde_json::json!([]);
        *pack.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_pack_manifest(&pack.to_string()).is_err(),
            "static pack {pointer} must fail closed"
        );
    }

    for pointer in [
        "/allowed_actions",
        "/allowed_task_kinds",
        "/allowed_data_categories",
        "/approval_gates",
    ] {
        let mut pack = fixture("pack_manifest");
        *pack.pointer_mut(pointer).unwrap() = serde_json::json!([]);
        assert!(
            parse_pack_manifest(&pack.to_string()).is_err(),
            "reviewed pack {pointer} must be declared"
        );
    }
}

#[test]
fn semantic_version_components_are_canonical_and_bounded() {
    for invalid in [
        "3.00.0",
        "3.0.00",
        "3.18446744073709551616.0",
        "3.0.18446744073709551616",
    ] {
        let mut artifact = fixture("artifact_manifest");
        artifact["artifact_version"] = serde_json::json!(invalid);
        assert!(
            parse_artifact_manifest(&artifact.to_string()).is_err(),
            "{invalid} must fail closed"
        );
    }
}
