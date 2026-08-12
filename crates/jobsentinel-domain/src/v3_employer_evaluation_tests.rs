//! Proves employer evaluation fixtures require dated attribution and prohibit verdicts.

use crate::v3_evaluation_tests::{case_index, fixture, validation_error};

const REQUIRED: &[&str] = &[
    "employer_provenance",
    "employer_evidence_date_visible",
    "employer_uncertainty_visible",
    "employer_safe_next_action",
    "stale_evidence_separately_attributed",
    "conflicting_evidence_separately_attributed",
];
const FORBIDDEN: &[&str] = &[
    "published_user_observation",
    "centralized_user_observation",
    "employer_verdict",
    "definitive_scam_verdict",
    "legal_compliance_verdict",
    "composite_employer_verdict",
];

#[test]
fn employer_dossier_requires_visible_attribution_and_no_verdicts() -> Result<(), String> {
    let value = fixture()?;
    let dossier = case_index(&value, "employer-dossier-mixed-evidence-v1")?;
    assert_eq!(
        value["cases"][dossier]["oracle"]["required_assertions"],
        serde_json::json!(REQUIRED)
    );
    assert_eq!(
        value["cases"][dossier]["oracle"]["forbidden_assertions"],
        serde_json::json!(FORBIDDEN)
    );
    Ok(())
}

#[test]
fn employer_dossier_safety_assertions_are_schema_mandatory() -> Result<(), String> {
    for (field, assertions) in [
        ("required_assertions", REQUIRED),
        ("forbidden_assertions", FORBIDDEN),
    ] {
        for assertion in assertions {
            let mut changed = fixture()?;
            let index = case_index(&changed, "employer-dossier-mixed-evidence-v1")?;
            let values = changed["cases"][index]["oracle"][field]
                .as_array_mut()
                .ok_or_else(|| "assertions must be an array".to_string())?;
            let position = values
                .iter()
                .position(|value| value == assertion)
                .ok_or_else(|| format!("missing employer safety assertion {assertion}"))?;
            values.remove(position);
            assert!(validation_error(&changed)?.contains("safety assertions"));
        }
    }
    Ok(())
}

#[test]
fn employer_dossier_requires_uniquely_dated_evidence() -> Result<(), String> {
    let mut invalid_date = fixture()?;
    let dossier = case_index(&invalid_date, "employer-dossier-mixed-evidence-v1")?;
    invalid_date["cases"][dossier]["input"]["evidence"][0]["observed_on"] =
        serde_json::json!("not-a-date");
    assert!(validation_error(&invalid_date)?.contains("uniquely dated"));

    let mut duplicate_source = fixture()?;
    let dossier = case_index(&duplicate_source, "employer-dossier-mixed-evidence-v1")?;
    duplicate_source["cases"][dossier]["input"]["evidence"][1]["source_class"] =
        serde_json::json!("official_domain");
    assert!(validation_error(&duplicate_source)?.contains("uniquely dated"));

    for evidence_index in [1, 2] {
        let mut missing_attribution = fixture()?;
        let dossier = case_index(&missing_attribution, "employer-dossier-mixed-evidence-v1")?;
        missing_attribution["cases"][dossier]["input"]["evidence"][evidence_index]["state"] =
            serde_json::json!("current");
        assert!(validation_error(&missing_attribution)?.contains("safety assertions"));
    }
    Ok(())
}
