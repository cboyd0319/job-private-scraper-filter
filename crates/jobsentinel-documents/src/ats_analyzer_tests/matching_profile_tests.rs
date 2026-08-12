use crate::{
    AtsAnalyzer, HardConstraintCategory, ProfessionMatchingProfile, RegionalMatchingProfile,
    RequirementMatchState, ResumeMatchingProfile,
};
use serde::Deserialize;
use std::collections::BTreeSet;

const MATCHING_PROFILE_EVALS: &str = include_str!("../eval_fixtures/matching_profiles_v1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatchingProfileEvaluationSet {
    schema: String,
    schema_version: u32,
    revision: String,
    data_origin: String,
    contains_personal_data: bool,
    profession_case: ProfessionEvaluationCase,
    regional_cases: Vec<RegionalEvaluationCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfessionEvaluationCase {
    resume_text: String,
    skills: Vec<String>,
    job_description: String,
    profiles: Vec<ProfessionExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfessionExpectation {
    profession: ProfessionMatchingProfile,
    expected_preferred_keywords: Vec<String>,
    expected_review_order: Vec<String>,
    expected_keyword_score_delta: f64,
    expected_overall_score_delta: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegionalEvaluationCase {
    id: String,
    region: RegionalMatchingProfile,
    resume_text: String,
    skills: Vec<String>,
    job_description: String,
    expected_matches: Vec<String>,
    expected_missing: Vec<String>,
    expected_hard_constraint_categories: Vec<HardConstraintCategory>,
    expected_keyword_score_delta: f64,
    expected_overall_score_delta: f64,
}

fn analyze(
    resume_text: &str,
    skills: &[String],
    job_description: &str,
    profession: ProfessionMatchingProfile,
    region: RegionalMatchingProfile,
) -> crate::AtsAnalysisResult {
    AtsAnalyzer::analyze_text_for_job_with_profile(
        resume_text,
        skills,
        job_description,
        ResumeMatchingProfile { profession, region },
    )
}

#[test]
fn explicit_profession_profile_marks_preferred_sections_without_changing_match_state() {
    let profile = ResumeMatchingProfile {
        profession: ProfessionMatchingProfile::Technical,
        region: RegionalMatchingProfile::UnitedKingdom,
    };
    let result = AtsAnalyzer::analyze_text_for_job_with_profile(
        "Projects\nKubernetes\n\nSkills\nExcel",
        &["Excel".to_string()],
        "Required: Kubernetes, Excel",
        profile,
    );

    assert_eq!(result.matching_profile, Some(profile));
    let kubernetes = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "kubernetes")
        .unwrap();
    assert_eq!(kubernetes.profile_preferred_section, Some(true));
    assert_eq!(kubernetes.match_state, RequirementMatchState::Direct);

    let excel = result
        .requirement_reviews
        .iter()
        .find(|review| review.keyword == "excel")
        .unwrap();
    assert_eq!(excel.profile_preferred_section, Some(false));
    assert_eq!(excel.match_state, RequirementMatchState::Partial);

    let unprofiled = AtsAnalyzer::analyze_text_for_job(
        "Projects\nKubernetes\n\nSkills\nExcel",
        &["Excel".to_string()],
        "Required: Kubernetes, Excel",
    );
    assert_eq!(result.keyword_score, unprofiled.keyword_score);
    assert_eq!(result.overall_score, unprofiled.overall_score);
}

#[test]
fn regional_profiles_match_reviewed_spelling_variants_but_us_does_not() {
    for region in [
        RegionalMatchingProfile::UnitedKingdom,
        RegionalMatchingProfile::EuropeanUnion,
        RegionalMatchingProfile::India,
    ] {
        let result = analyze(
            "Experience\nLed program evaluation.",
            &[],
            "Required: programme evaluation",
            ProfessionMatchingProfile::Operations,
            region,
        );
        assert_eq!(result.keyword_matches.len(), 1);
        assert_eq!(result.keyword_matches[0].keyword, "program evaluation");
    }

    let us = analyze(
        "Experience\nLed program evaluation.",
        &[],
        "Required: programme evaluation",
        ProfessionMatchingProfile::Operations,
        RegionalMatchingProfile::UnitedStates,
    );
    assert!(us.keyword_matches.is_empty());
}

#[test]
fn regional_profile_does_not_turn_a_different_concept_into_evidence() {
    let result = analyze(
        "Experience\nLed project evaluation.",
        &[],
        "Required: programme evaluation",
        ProfessionMatchingProfile::Operations,
        RegionalMatchingProfile::UnitedKingdom,
    );

    assert!(result.keyword_matches.is_empty());
    assert_eq!(result.missing_keywords, ["program evaluation"]);
}

#[test]
fn regional_spelling_keeps_missing_licence_as_a_hard_constraint() {
    let result = analyze(
        "Experience\nLed program evaluation.",
        &[],
        "Required: driver's licence, programme evaluation",
        ProfessionMatchingProfile::Operations,
        RegionalMatchingProfile::UnitedKingdom,
    );

    assert!(result
        .missing_keywords
        .iter()
        .any(|keyword| keyword == "driver's license"));
    assert!(result.hard_constraint_risks.iter().any(|risk| {
        risk.requirement == "driver's license"
            && risk.category == HardConstraintCategory::LicenseOrCertification
    }));
}

#[test]
fn existing_unprofiled_analysis_neither_infers_nor_serializes_a_profile() {
    let result = AtsAnalyzer::analyze_text_for_job(
        "Experience\nLed program evaluation.",
        &[],
        "Required: program evaluation",
    );

    assert!(result.matching_profile.is_none());
    assert!(result
        .requirement_reviews
        .iter()
        .all(|review| review.profile_preferred_section.is_none()));
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("matching_profile"));
    assert!(!serialized.contains("profile_preferred_section"));
}

#[test]
fn matching_profiles_have_stable_wire_names() {
    let profile = ResumeMatchingProfile {
        profession: ProfessionMatchingProfile::EarlyCareer,
        region: RegionalMatchingProfile::India,
    };

    assert_eq!(
        serde_json::to_string(&profile).unwrap(),
        r#"{"profession":"early_career","region":"india"}"#
    );
}

fn matching_profile_evaluations() -> MatchingProfileEvaluationSet {
    serde_json::from_str(MATCHING_PROFILE_EVALS).expect("matching profile eval fixture must parse")
}

fn profile_wire_name(profile: ProfessionMatchingProfile) -> &'static str {
    match profile {
        ProfessionMatchingProfile::Technical => "technical",
        ProfessionMatchingProfile::Content => "content",
        ProfessionMatchingProfile::Operations => "operations",
        ProfessionMatchingProfile::Healthcare => "healthcare",
        ProfessionMatchingProfile::Service => "service",
        ProfessionMatchingProfile::Trades => "trades",
        ProfessionMatchingProfile::Education => "education",
        ProfessionMatchingProfile::Sales => "sales",
        ProfessionMatchingProfile::EarlyCareer => "early_career",
    }
}

fn region_wire_name(region: RegionalMatchingProfile) -> &'static str {
    match region {
        RegionalMatchingProfile::UnitedStates => "us",
        RegionalMatchingProfile::UnitedKingdom => "uk",
        RegionalMatchingProfile::EuropeanUnion => "eu",
        RegionalMatchingProfile::India => "india",
    }
}

fn requirement_states(result: &crate::AtsAnalysisResult) -> Vec<(String, RequirementMatchState)> {
    let mut states = result
        .requirement_reviews
        .iter()
        .map(|review| (review.keyword.clone(), review.match_state))
        .collect::<Vec<_>>();
    states.sort_by(|a, b| a.0.cmp(&b.0));
    states
}

fn assert_delta(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < f64::EPSILON,
        "expected delta {expected}, got {actual}"
    );
}

#[test]
fn frozen_profession_evals_cover_every_profile_without_score_or_state_drift() {
    let evals = matching_profile_evaluations();

    assert_eq!(evals.schema, "jobsentinel.resume-matching-profile-eval");
    assert_eq!(evals.schema_version, 1);
    assert_eq!(evals.revision, "matching-profiles-v1");
    assert_eq!(evals.data_origin, "synthetic");
    assert!(!evals.contains_personal_data);

    let expected_profiles = [
        "technical",
        "content",
        "operations",
        "healthcare",
        "service",
        "trades",
        "education",
        "sales",
        "early_career",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let actual_profiles = evals
        .profession_case
        .profiles
        .iter()
        .map(|expectation| profile_wire_name(expectation.profession))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_profiles, expected_profiles);
    assert_eq!(
        actual_profiles.len(),
        evals.profession_case.profiles.len(),
        "profession eval profiles must be unique"
    );

    let baseline = AtsAnalyzer::analyze_text_for_job(
        &evals.profession_case.resume_text,
        &evals.profession_case.skills,
        &evals.profession_case.job_description,
    );
    let baseline_states = requirement_states(&baseline);
    let baseline_review_order = baseline
        .requirement_reviews
        .iter()
        .map(|review| review.keyword.clone())
        .collect::<Vec<_>>();
    for expectation in &evals.profession_case.profiles {
        let result = analyze(
            &evals.profession_case.resume_text,
            &evals.profession_case.skills,
            &evals.profession_case.job_description,
            expectation.profession,
            RegionalMatchingProfile::UnitedStates,
        );
        let mut preferred = result
            .requirement_reviews
            .iter()
            .filter(|review| review.profile_preferred_section == Some(true))
            .map(|review| review.keyword.clone())
            .collect::<Vec<_>>();
        preferred.sort();
        let review_order = result
            .requirement_reviews
            .iter()
            .map(|review| review.keyword.clone())
            .collect::<Vec<_>>();

        assert_eq!(preferred, expectation.expected_preferred_keywords);
        assert_eq!(review_order, expectation.expected_review_order);
        assert_ne!(review_order, baseline_review_order);
        assert_eq!(requirement_states(&result), baseline_states);
        assert_delta(
            result.keyword_score - baseline.keyword_score,
            expectation.expected_keyword_score_delta,
        );
        assert_delta(
            result.overall_score - baseline.overall_score,
            expectation.expected_overall_score_delta,
        );
    }
}

#[test]
fn frozen_regional_evals_exercise_every_declared_profile_and_exact_score_deltas() {
    let evals = matching_profile_evaluations();
    let actual_regions = evals
        .regional_cases
        .iter()
        .map(|case| region_wire_name(case.region))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_regions,
        ["us", "uk", "eu", "india"].into_iter().collect()
    );
    assert_eq!(
        evals
            .regional_cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        evals.regional_cases.len(),
        "regional eval case ids must be unique"
    );

    for case in &evals.regional_cases {
        let baseline = AtsAnalyzer::analyze_text_for_job(
            &case.resume_text,
            &case.skills,
            &case.job_description,
        );
        let result = analyze(
            &case.resume_text,
            &case.skills,
            &case.job_description,
            ProfessionMatchingProfile::Operations,
            case.region,
        );
        let mut matches = result
            .keyword_matches
            .iter()
            .map(|item| item.keyword.clone())
            .collect::<Vec<_>>();
        let mut missing = result.missing_keywords.clone();
        let mut hard_constraints = result
            .hard_constraint_risks
            .iter()
            .map(|risk| risk.category)
            .collect::<Vec<_>>();
        matches.sort();
        missing.sort();
        hard_constraints.sort_by_key(|category| format!("{category:?}"));

        assert_eq!(matches, case.expected_matches, "case {}", case.id);
        assert_eq!(missing, case.expected_missing, "case {}", case.id);
        assert_eq!(
            hard_constraints, case.expected_hard_constraint_categories,
            "case {}",
            case.id
        );
        assert_delta(
            result.keyword_score - baseline.keyword_score,
            case.expected_keyword_score_delta,
        );
        assert_delta(
            result.overall_score - baseline.overall_score,
            case.expected_overall_score_delta,
        );
        assert_eq!(result.format_score, baseline.format_score);
        assert_eq!(result.completeness_score, baseline.completeness_score);
    }
}
