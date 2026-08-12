use std::sync::OnceLock;

use regex::Regex;

use super::super::ats_types::{
    ProfessionMatchingProfile, RegionalMatchingProfile, RequirementReview,
};

fn regional_spellings() -> &'static [(Regex, &'static str); 4] {
    static SPELLINGS: OnceLock<[(Regex, &'static str); 4]> = OnceLock::new();
    SPELLINGS.get_or_init(|| {
        [
            (
                Regex::new(r"\bprogramme evaluation\b").unwrap(),
                "program evaluation",
            ),
            (
                Regex::new(r"\bdriver's licence\b").unwrap(),
                "driver's license",
            ),
            (
                Regex::new(r"\bdrivers licence\b").unwrap(),
                "drivers license",
            ),
            (Regex::new(r"\bdriver licence\b").unwrap(), "driver license"),
        ]
    })
}

pub(super) fn normalize_job_description(
    job_description: &str,
    region: RegionalMatchingProfile,
) -> String {
    let mut normalized = job_description.to_lowercase();
    if matches!(region, RegionalMatchingProfile::UnitedStates) {
        return normalized;
    }
    for (regional, canonical) in regional_spellings() {
        normalized = regional.replace_all(&normalized, *canonical).into_owned();
    }
    normalized
}

pub(super) fn extend_regional_search_terms(
    keyword: &str,
    region: RegionalMatchingProfile,
    terms: &mut Vec<String>,
) {
    if matches!(region, RegionalMatchingProfile::UnitedStates) {
        return;
    }
    let regional = match keyword {
        "program evaluation" => Some("programme evaluation"),
        "driver's license" => Some("driver's licence"),
        "drivers license" => Some("drivers licence"),
        "driver license" => Some("driver licence"),
        _ => None,
    };
    if let Some(regional) = regional.filter(|term| !terms.iter().any(|item| item == term)) {
        terms.push(regional.to_string());
    }
}

pub(super) fn profile_prefers_section(
    profession: ProfessionMatchingProfile,
    review: &RequirementReview,
) -> bool {
    review.evidence_sections.iter().any(|section| {
        let section = section.as_str();
        match profession {
            ProfessionMatchingProfile::Technical => matches!(
                section,
                "experience"
                    | "current experience"
                    | "recent experience"
                    | "projects"
                    | "certifications"
            ),
            ProfessionMatchingProfile::Content => matches!(
                section,
                "experience"
                    | "current experience"
                    | "recent experience"
                    | "projects"
                    | "publications"
            ),
            ProfessionMatchingProfile::Operations
            | ProfessionMatchingProfile::Service
            | ProfessionMatchingProfile::Sales => matches!(
                section,
                "experience" | "current experience" | "recent experience" | "projects"
            ),
            ProfessionMatchingProfile::Healthcare
            | ProfessionMatchingProfile::Trades
            | ProfessionMatchingProfile::Education => matches!(
                section,
                "experience"
                    | "current experience"
                    | "recent experience"
                    | "education"
                    | "certifications"
                    | "licenses"
            ),
            ProfessionMatchingProfile::EarlyCareer => {
                matches!(section, "projects" | "education" | "certifications")
            }
        }
    })
}
