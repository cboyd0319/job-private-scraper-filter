use std::collections::BTreeSet;

use crate::{
    parse_v3_evaluation_set,
    v3_contracts::{parse_region_manifest, PayPeriod},
    v3_manifests::SourceClass,
};
use chrono::NaiveDate;
use serde_json::json;

const UK: &str = include_str!("fixtures/region_manifests/uk_v1.json");
const EU: &str = include_str!("fixtures/region_manifests/eu_v1.json");
const INDIA: &str = include_str!("fixtures/region_manifests/india_v1.json");
const EVALUATIONS: &str = include_str!("fixtures/v3_evaluation_set_v1.json");
const CONTRACT_BASELINE: &str = include_str!("fixtures/v3_contract_bundle_v1.json");

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 7, 19).unwrap()
}

fn parse(input: &str) -> crate::v3_manifests::RegionManifest {
    parse_region_manifest(input, today()).unwrap()
}

#[test]
fn starter_regions_are_incomplete_metadata_without_native_source_claims() {
    for (input, region_id) in [(UK, "uk"), (EU, "eu"), (INDIA, "india")] {
        let manifest = parse(input);

        assert_eq!(manifest.region_id, region_id);
        assert_eq!(manifest.reviewed_on, today());
        assert!(manifest.incomplete_coverage);
        assert_eq!(
            manifest.source_classes,
            [SourceClass::RegionalBoard, SourceClass::UserImport]
        );
        assert!(manifest
            .policy_note_refs
            .iter()
            .any(|reference| reference == "docs/plans/v3/regional-readiness-framework.md"));
        assert!(manifest.provenance_refs.iter().any(|reference| reference
            .strip_prefix("https://")
            .is_some_and(|path| !path.is_empty())));
    }
}

#[test]
fn starter_regions_declare_research_profiles_and_pay_semantics() {
    let uk = parse(UK);
    assert_eq!(uk.country_codes, ["GB"]);
    assert_eq!(uk.languages, ["en"]);
    assert_eq!(uk.currencies, ["GBP"]);
    assert!(uk.pay_periods.contains(&PayPeriod::Daily));
    assert!(uk.pay_periods.contains(&PayPeriod::NotDisclosed));
    assert_eq!(uk.cv_profiles, ["uk_cv_research"]);
    assert_eq!(uk.taxonomy_ids, ["uk_soc_2020_research"]);

    let eu = parse(EU);
    assert_eq!(
        eu.country_codes,
        [
            "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE", "GR", "HU", "IE",
            "IT", "LV", "LT", "LU", "MT", "NL", "PL", "PT", "RO", "SK", "SI", "ES", "SE"
        ]
    );
    assert_eq!(
        eu.country_codes.iter().collect::<BTreeSet<_>>().len(),
        eu.country_codes.len()
    );
    assert_eq!(eu.languages, ["en"]);
    assert_eq!(
        eu.currencies,
        ["EUR", "CZK", "DKK", "HUF", "PLN", "RON", "SEK"]
    );
    assert_eq!(eu.cv_profiles, ["europass_cv_research"]);
    assert_eq!(eu.taxonomy_ids, ["esco_research"]);

    let india = parse(INDIA);
    assert_eq!(india.country_codes, ["IN"]);
    assert_eq!(india.languages, ["en"]);
    assert_eq!(india.currencies, ["INR"]);
    assert!(india.pay_periods.contains(&PayPeriod::Stipend));
    assert!(india.pay_periods.contains(&PayPeriod::NotDisclosed));
    assert_eq!(india.cv_profiles, ["india_cv_research"]);
    assert_eq!(
        india.taxonomy_ids,
        ["india_nco_2015_research", "india_nsqf_nos_research"]
    );
}

#[test]
fn starter_regions_reference_real_frozen_evaluations() {
    let evaluation_ids = parse_v3_evaluation_set(EVALUATIONS)
        .unwrap()
        .cases
        .into_iter()
        .map(|case| case.id)
        .collect::<BTreeSet<_>>();

    for input in [UK, EU, INDIA] {
        let manifest = parse(input);
        assert!(manifest
            .evaluation_fixture_ids
            .iter()
            .all(|id| evaluation_ids.contains(id)));
    }

    let baseline: serde_json::Value = serde_json::from_str(CONTRACT_BASELINE).unwrap();
    let us = parse_region_manifest(&baseline["region_manifest"].to_string(), today()).unwrap();
    assert_eq!(us.reviewed_on, today());
    assert!(us
        .evaluation_fixture_ids
        .iter()
        .all(|id| evaluation_ids.contains(id)));
}

#[test]
fn starter_regions_reject_future_reviews_and_missing_provenance() {
    let mut future: serde_json::Value = serde_json::from_str(UK).unwrap();
    future["reviewed_on"] = json!("2026-07-20");
    assert!(parse_region_manifest(&future.to_string(), today()).is_err());

    let mut missing: serde_json::Value = serde_json::from_str(UK).unwrap();
    missing["provenance_refs"] = json!([]);
    assert!(parse_region_manifest(&missing.to_string(), today()).is_err());

    for invalid in [
        "",
        "http://example.com/research",
        "https:///research",
        "https://user:secret@example.com/research",
    ] {
        let mut unsafe_ref: serde_json::Value = serde_json::from_str(UK).unwrap();
        unsafe_ref["provenance_refs"] = json!([invalid]);
        assert!(
            parse_region_manifest(&unsafe_ref.to_string(), today()).is_err(),
            "{invalid} must fail closed"
        );
    }
}
