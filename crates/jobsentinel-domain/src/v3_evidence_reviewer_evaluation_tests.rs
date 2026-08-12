//! Proves the Evidence Reviewer target accepts only one bounded resume-evidence case.

use chrono::NaiveDate;
use serde_json::json;

use crate::{
    v3_manifests::PackType,
    v3_pack_payloads::{parse_and_self_test_pack_payload, SelfTestedPackPayload},
};

use super::v3_pack_payload_tests::{release, EVALUATION_SET};

fn payload(evaluation: &serde_json::Value) -> String {
    serde_json::to_string(&json!({
        "schema": "jobsentinel.v3.pack-payload.v1",
        "pack_type": "evaluation",
        "target": "evidence_reviewer_resume_requirement_v1",
        "evaluation_set_json": serde_json::to_string(evaluation).unwrap()
    }))
    .unwrap()
}

#[test]
fn evidence_reviewer_target_rejects_full_and_mixed_evaluation_sets() {
    let full: serde_json::Value = serde_json::from_str(EVALUATION_SET).unwrap();
    let mut mixed = full.clone();
    mixed["cases"].as_array_mut().unwrap().retain(|case| {
        matches!(
            case["category"].as_str(),
            Some("resume_evidence" | "pay_clarity")
        )
    });
    for evaluation in [full, mixed] {
        assert!(parse_and_self_test_pack_payload(
            &release(payload(&evaluation), PackType::Evaluation),
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        )
        .is_err());
    }
}

#[test]
fn evidence_reviewer_target_accepts_one_resume_evidence_case() {
    let mut targeted: serde_json::Value = serde_json::from_str(EVALUATION_SET).unwrap();
    targeted["cases"]
        .as_array_mut()
        .unwrap()
        .retain(|case| case["category"] == "resume_evidence");
    targeted["revision"] = json!("v3.m7.evidence-reviewer.1");
    let tested = parse_and_self_test_pack_payload(
        &release(payload(&targeted), PackType::Evaluation),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .unwrap()
    .into_payload();
    assert!(matches!(
        tested,
        SelfTestedPackPayload::EvidenceReviewerResumeRequirementEvaluation { .. }
    ));
    assert!(!format!("{tested:?}").contains("production Rust experience"));
}
