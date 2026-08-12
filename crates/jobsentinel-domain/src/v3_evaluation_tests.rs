//! Proves the versioned v3 evaluation set remains complete and fail closed.

use std::collections::BTreeSet;

use crate::{
    parse_v3_evaluation_set, EvaluationAssertion, EvaluationCategory, EvaluationEvidenceTarget,
    EvaluationVerificationTarget,
};

pub(crate) const V3_EVALUATIONS: &str = include_str!("fixtures/v3_evaluation_set_v1.json");

pub(crate) fn fixture() -> Result<serde_json::Value, String> {
    serde_json::from_str(V3_EVALUATIONS).map_err(|error| error.to_string())
}

pub(crate) fn case_index(value: &serde_json::Value, id: &str) -> Result<usize, String> {
    value["cases"]
        .as_array()
        .and_then(|cases| cases.iter().position(|case| case["id"] == id))
        .ok_or_else(|| format!("missing evaluation case {id}"))
}

pub(crate) fn validation_error(value: &serde_json::Value) -> Result<String, String> {
    parse_v3_evaluation_set(&value.to_string())
        .err()
        .ok_or_else(|| "fixture unexpectedly validated".to_string())
}

#[test]
fn synthetic_baseline_covers_every_v3_evaluation_category() -> Result<(), String> {
    let set = parse_v3_evaluation_set(V3_EVALUATIONS)?;
    let actual = set
        .cases
        .iter()
        .map(|case| case.category)
        .collect::<BTreeSet<_>>();

    assert_eq!(actual, EvaluationCategory::ALL.into_iter().collect());
    assert!(!set.contains_personal_data);
    assert!(set.cases.iter().all(|case| {
        case.privacy_boundary.local_only
            && !case.privacy_boundary.external_ai_allowed
            && !case.privacy_boundary.contains_personal_data
            && !case.evidence_capture.captures_fixture_content
            && !case.oracle.required_assertions.is_empty()
            && !case.oracle.forbidden_assertions.is_empty()
    }));
    assert!(set.cases.iter().any(|case| {
        case.verification_target == EvaluationVerificationTarget::SourceGraph
            && case.evidence_capture.target == EvaluationEvidenceTarget::StructuredTestResult
    }));
    Ok(())
}

#[test]
fn evaluation_set_rejects_missing_categories_and_duplicate_ids() -> Result<(), String> {
    let mut missing = fixture()?;
    missing["cases"]
        .as_array_mut()
        .ok_or_else(|| "cases must be an array".to_string())?
        .pop();
    assert!(validation_error(&missing)?.contains("all 11"));

    let mut duplicate = fixture()?;
    let first = duplicate["cases"][0].clone();
    duplicate["cases"]
        .as_array_mut()
        .ok_or_else(|| "cases must be an array".to_string())?
        .push(first);
    assert!(validation_error(&duplicate)?.contains("unique"));
    Ok(())
}

#[test]
fn every_case_requires_owner_privacy_safe_evidence_and_concrete_oracle() -> Result<(), String> {
    let mut missing_owner = fixture()?;
    missing_owner["cases"][0]
        .as_object_mut()
        .ok_or_else(|| "case must be an object".to_string())?
        .remove("verification_target");
    assert!(parse_v3_evaluation_set(&missing_owner.to_string()).is_err());

    let mut external = fixture()?;
    external["cases"][0]["privacy_boundary"]["external_ai_allowed"] = serde_json::json!(true);
    assert!(validation_error(&external)?.contains("local synthetic"));

    let mut raw_capture = fixture()?;
    raw_capture["cases"][0]["evidence_capture"]["captures_fixture_content"] =
        serde_json::json!(true);
    assert!(validation_error(&raw_capture)?.contains("fixture content"));

    let mut vacuous = fixture()?;
    vacuous["cases"][0]["oracle"]["required_assertions"] = serde_json::json!(["pass"]);
    assert!(parse_v3_evaluation_set(&vacuous.to_string()).is_err());

    let mut duplicate_forbidden = fixture()?;
    let duplicate = duplicate_forbidden["cases"][0]["oracle"]["forbidden_assertions"][0].clone();
    duplicate_forbidden["cases"][0]["oracle"]["forbidden_assertions"]
        .as_array_mut()
        .ok_or_else(|| "forbidden assertions must be an array".to_string())?
        .push(duplicate);
    assert!(validation_error(&duplicate_forbidden)?.contains("unique"));
    Ok(())
}

#[test]
fn every_category_rejects_cross_category_assertion_pairs() -> Result<(), String> {
    let baseline = fixture()?;
    let ids = baseline["cases"]
        .as_array()
        .ok_or_else(|| "cases must be an array".to_string())?
        .iter()
        .filter_map(|case| case["id"].as_str().map(str::to_string))
        .collect::<Vec<_>>();

    for id in ids {
        let replacements = if id == "source-truth-official-absence-v1" {
            ("commercial_dimensions_measured", "silent_data_loss")
        } else {
            ("source_truth_unverified", "fabricated_posting_verdict")
        };
        for (field, assertion) in [
            ("required_assertions", replacements.0),
            ("forbidden_assertions", replacements.1),
        ] {
            let mut changed = fixture()?;
            let index = case_index(&changed, &id)?;
            changed["cases"][index]["oracle"][field] = serde_json::json!([assertion]);
            assert!(parse_v3_evaluation_set(&changed.to_string()).is_err());
        }
    }

    let mut source_swap = fixture()?;
    let source = case_index(&source_swap, "source-truth-official-absence-v1")?;
    source_swap["cases"][source]["oracle"]["required_assertions"] =
        serde_json::json!(["commercial_dimensions_measured"]);
    source_swap["cases"][source]["oracle"]["forbidden_assertions"] =
        serde_json::json!(["silent_data_loss"]);
    assert!(parse_v3_evaluation_set(&source_swap.to_string()).is_err());
    Ok(())
}

#[test]
fn evaluation_revalidates_a_mutated_case_before_comparing_observations() -> Result<(), String> {
    let mut set = parse_v3_evaluation_set(V3_EVALUATIONS)?;
    let case = set
        .cases
        .iter_mut()
        .find(|case| case.id == "source-truth-official-absence-v1")
        .ok_or_else(|| "missing source evaluation".to_string())?;
    case.oracle.required_assertions = vec![EvaluationAssertion::CommercialDimensionsMeasured];
    case.oracle.forbidden_assertions = vec![EvaluationAssertion::SilentDataLoss];

    assert!(!case.evaluate(&[EvaluationAssertion::CommercialDimensionsMeasured]));
    Ok(())
}

#[test]
fn evaluation_fails_on_missing_wrong_or_forbidden_observed_assertions() -> Result<(), String> {
    let set = parse_v3_evaluation_set(V3_EVALUATIONS)?;
    let case = set
        .cases
        .iter()
        .find(|case| case.id == "military-civilian-suggestion-v1")
        .ok_or_else(|| "missing military evaluation".to_string())?;
    let mut observed = case.oracle.required_assertions.clone();

    assert!(case.evaluate(&observed));
    observed.pop();
    assert!(!case.evaluate(&observed));
    assert!(!case.evaluate(&[EvaluationAssertion::ResumeEvidenceGap]));

    let mut forbidden = case.oracle.required_assertions.clone();
    forbidden.push(EvaluationAssertion::CivilianEquivalenceClaim);
    assert!(!case.evaluate(&forbidden));

    let mut unexpected = case.oracle.required_assertions.clone();
    unexpected.push(EvaluationAssertion::ResumeEvidenceGap);
    assert!(!case.evaluate(&unexpected));

    let mut duplicate = case.oracle.required_assertions.clone();
    duplicate.push(case.oracle.required_assertions[0]);
    assert!(!case.evaluate(&duplicate));
    Ok(())
}

#[test]
fn harmful_assertions_cannot_be_required_and_safe_assertions_cannot_be_forbidden(
) -> Result<(), String> {
    for id in [
        "source-truth-official-absence-v1",
        "military-civilian-suggestion-v1",
        "protected-veteran-unanswered-v1",
        "protected-veteran-user-selected-v1",
    ] {
        for assertion in [
            "inferred_veteran_status",
            "inferred_clearance",
            "inferred_eligibility",
            "inferred_protected_status",
        ] {
            let mut changed = fixture()?;
            let index = case_index(&changed, id)?;
            let forbidden = changed["cases"][index]["oracle"]["forbidden_assertions"]
                .as_array_mut()
                .ok_or_else(|| "forbidden assertions must be an array".to_string())?;
            forbidden.retain(|value| value != assertion);
            changed["cases"][index]["oracle"]["required_assertions"]
                .as_array_mut()
                .ok_or_else(|| "required assertions must be an array".to_string())?
                .push(serde_json::json!(assertion));
            assert!(validation_error(&changed)?.contains("harmful assertions"));
        }
    }

    let mut safe_forbidden = fixture()?;
    safe_forbidden["cases"][0]["oracle"]["forbidden_assertions"]
        .as_array_mut()
        .ok_or_else(|| "forbidden assertions must be an array".to_string())?
        .push(serde_json::json!("pay_context_preserved"));
    assert!(validation_error(&safe_forbidden)?.contains("safe assertions"));
    Ok(())
}

#[test]
fn evaluation_set_rejects_personal_data_and_inferred_status() -> Result<(), String> {
    let mut personal = fixture()?;
    personal["contains_personal_data"] = serde_json::json!(true);
    assert!(validation_error(&personal)?.contains("personal data"));

    for (id, field) in [
        ("military-civilian-suggestion-v1", "military_service_basis"),
        (
            "protected-veteran-unanswered-v1",
            "protected_veteran_answer_basis",
        ),
    ] {
        let mut inferred = fixture()?;
        let index = case_index(&inferred, id)?;
        inferred["cases"][index][field] = serde_json::json!("inferred");
        assert!(parse_v3_evaluation_set(&inferred.to_string()).is_err());
    }
    Ok(())
}

#[test]
fn military_mapping_requires_confirmed_duties_and_evidence() -> Result<(), String> {
    for (field, message) in [
        ("user_confirmed_duties", "user-confirmed duties"),
        ("evidence_refs", "evidence references"),
    ] {
        let mut missing = fixture()?;
        let index = case_index(&missing, "military-civilian-suggestion-v1")?;
        missing["cases"][index]["input"][field] = serde_json::json!([]);
        assert!(validation_error(&missing)?.contains(message));
    }

    Ok(())
}

#[test]
fn veteran_safety_assertion_subsets_are_schema_mandatory() -> Result<(), String> {
    for (id, field, assertions) in [
        (
            "military-civilian-suggestion-v1",
            "required_assertions",
            &[
                "civilian_role_suggestion",
                "confirmed_duty_evidence",
                "mapping_provenance",
                "mapping_uncertainty",
            ][..],
        ),
        (
            "military-civilian-suggestion-v1",
            "forbidden_assertions",
            &[
                "civilian_equivalence_claim",
                "invented_credential",
                "invented_certification",
                "inferred_clearance",
                "inferred_veteran_status",
                "inferred_eligibility",
            ][..],
        ),
        (
            "protected-veteran-unanswered-v1",
            "required_assertions",
            &["preserve_unanswered"][..],
        ),
        (
            "protected-veteran-unanswered-v1",
            "forbidden_assertions",
            &[
                "inferred_protected_status",
                "inferred_answer_from_military_evidence",
            ][..],
        ),
        (
            "protected-veteran-user-selected-v1",
            "required_assertions",
            &["preserve_user_selection"][..],
        ),
        (
            "protected-veteran-user-selected-v1",
            "forbidden_assertions",
            &["inferred_protected_status", "overridden_user_selection"][..],
        ),
    ] {
        for assertion in assertions {
            for replacement in [None, Some("resume_evidence_gap")] {
                let mut changed = fixture()?;
                let index = case_index(&changed, id)?;
                let values = changed["cases"][index]["oracle"][field]
                    .as_array_mut()
                    .ok_or_else(|| "assertions must be an array".to_string())?;
                let position = values
                    .iter()
                    .position(|value| value == assertion)
                    .ok_or_else(|| format!("missing safety assertion {assertion}"))?;
                if let Some(value) = replacement {
                    values[position] = serde_json::json!(value);
                } else {
                    values.remove(position);
                }
                assert!(parse_v3_evaluation_set(&changed.to_string()).is_err());
            }
        }
    }

    for (id, field, assertion) in [
        (
            "military-civilian-suggestion-v1",
            "required_assertions",
            "resume_evidence_gap",
        ),
        (
            "military-civilian-suggestion-v1",
            "forbidden_assertions",
            "definitive_scam_verdict",
        ),
        (
            "protected-veteran-unanswered-v1",
            "required_assertions",
            "resume_evidence_gap",
        ),
        (
            "protected-veteran-user-selected-v1",
            "forbidden_assertions",
            "definitive_scam_verdict",
        ),
    ] {
        let mut extra = fixture()?;
        let index = case_index(&extra, id)?;
        extra["cases"][index]["oracle"][field]
            .as_array_mut()
            .ok_or_else(|| "assertions must be an array".to_string())?
            .push(serde_json::json!(assertion));
        assert!(validation_error(&extra)?.contains("safety assertions"));
    }
    Ok(())
}

#[test]
fn protected_veteran_evaluations_require_unanswered_and_user_selected_cases() -> Result<(), String>
{
    let mut missing_selected = fixture()?;
    let selected = case_index(&missing_selected, "protected-veteran-user-selected-v1")?;
    missing_selected["cases"]
        .as_array_mut()
        .ok_or_else(|| "cases must be an array".to_string())?
        .remove(selected);
    assert!(validation_error(&missing_selected)?.contains("Unanswered and UserSelected"));

    let mut selected_without_answer = fixture()?;
    let index = case_index(
        &selected_without_answer,
        "protected-veteran-user-selected-v1",
    )?;
    selected_without_answer["cases"][index]["input"]["user_selection"] = serde_json::Value::Null;
    assert!(validation_error(&selected_without_answer)?.contains("selected answer"));
    Ok(())
}

#[test]
fn commercial_comparison_requires_a_measurable_baseline() -> Result<(), String> {
    let mut no_dimensions = fixture()?;
    let index = case_index(&no_dimensions, "commercial-safe-path-v1")?;
    no_dimensions["cases"][index]["input"]["dimensions"] = serde_json::json!([]);
    assert!(validation_error(&no_dimensions)?.contains("comparison dimensions"));

    let mut no_improvement = fixture()?;
    let index = case_index(&no_improvement, "commercial-safe-path-v1")?;
    let baseline =
        no_improvement["cases"][index]["input"]["dimensions"][0]["baseline_value"].clone();
    no_improvement["cases"][index]["input"]["dimensions"][0]["jobsentinel_target_value"] = baseline;
    assert!(validation_error(&no_improvement)?.contains("measurable improvement"));
    Ok(())
}

#[test]
fn evaluation_set_rejects_unknown_nested_fields_and_newer_schema() -> Result<(), String> {
    let mut unknown = fixture()?;
    unknown["cases"][0]["input"]["private_profile"] = serde_json::json!("do not accept");
    assert!(parse_v3_evaluation_set(&unknown.to_string()).is_err());

    let mut newer = fixture()?;
    newer["schema_version"] = serde_json::json!(2);
    assert!(validation_error(&newer)?.contains("unsupported evaluation schema version"));
    Ok(())
}
