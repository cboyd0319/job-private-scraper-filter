use std::collections::HashSet;

use super::super::ats_types::{
    HardConstraintCategory, HardConstraintRisk, KeywordImportance, RequirementMatchState,
    RequirementReview,
};
use super::requirement_rules;
use super::term_expansion;

pub(super) fn build_hard_constraint_risks(
    reviews: &[RequirementReview],
) -> Vec<HardConstraintRisk> {
    let mut risks = reviews
        .iter()
        .filter(|review| {
            review.importance == KeywordImportance::Required
                && hard_constraint_review_needed(review.match_state)
        })
        .filter_map(|review| {
            let category = hard_constraint_category(&review.keyword)?;
            let score_cap = hard_constraint_score_cap(category);
            Some(HardConstraintRisk {
                requirement: review.keyword.clone(),
                category,
                score_cap,
                reason: if category == HardConstraintCategory::SecurityClearance {
                    "Visible clearance wording is not confirmed-current evidence.".to_string()
                } else {
                    "A required hard constraint was not clearly found in the resume.".to_string()
                },
                action: hard_constraint_action(&review.keyword, category),
            })
        })
        .collect::<Vec<_>>();

    risks.sort_by(|a, b| {
        a.score_cap
            .total_cmp(&b.score_cap)
            .then(a.requirement.cmp(&b.requirement))
    });
    risks
}

fn hard_constraint_review_needed(match_state: RequirementMatchState) -> bool {
    matches!(
        match_state,
        RequirementMatchState::Missing
            | RequirementMatchState::Partial
            | RequirementMatchState::Implied
    )
}

pub(super) fn hard_constraint_score_cap(category: HardConstraintCategory) -> f64 {
    match category {
        HardConstraintCategory::WorkAuthorization => 50.0,
        HardConstraintCategory::Citizenship => 50.0,
        HardConstraintCategory::SecurityClearance => 60.0,
        HardConstraintCategory::LicenseOrCertification => 60.0,
        HardConstraintCategory::Education => 65.0,
        HardConstraintCategory::Experience => 65.0,
        HardConstraintCategory::Language => 65.0,
        HardConstraintCategory::Age => 70.0,
        HardConstraintCategory::BackgroundScreening => 70.0,
        HardConstraintCategory::PhysicalRequirement => 70.0,
        HardConstraintCategory::Location => 70.0,
    }
}

pub(super) fn hard_constraint_action(keyword: &str, category: HardConstraintCategory) -> String {
    if category == HardConstraintCategory::Experience && seniority_level_constraint_keyword(keyword)
    {
        return "Check whether your visible level matches this role; lower-title or fewer-years evidence may not satisfy it. Do not round up, stretch titles, or imply more experience than you have."
            .to_string();
    }
    if category == HardConstraintCategory::Citizenship
        || (category == HardConstraintCategory::WorkAuthorization
            && citizenship_constraint_keyword(keyword))
    {
        return "Check citizenship before tailoring. If it is not true for you, do not claim it. Do not treat work authorization as citizenship."
            .to_string();
    }
    if driving_record_constraint_keyword(keyword) || vehicle_insurance_constraint_keyword(keyword) {
        return "Check driving record, vehicle, or auto insurance before tailoring. If it is not current, workable, or true for you, do not claim it."
            .to_string();
    }

    let action = match category {
        HardConstraintCategory::WorkAuthorization => {
            "Check work authorization before tailoring. If it is not true for you, do not claim it."
        }
        HardConstraintCategory::Citizenship => {
            "Check citizenship before tailoring. If it is not true for you, do not claim it. Do not treat work authorization as citizenship."
        }
        HardConstraintCategory::SecurityClearance => {
            "Check clearance before tailoring. If it is not current or true for you, do not claim it."
        }
        HardConstraintCategory::LicenseOrCertification => {
            "Check license or certification before tailoring. If it is not current or true for you, do not claim it."
        }
        HardConstraintCategory::Education => {
            "Check the degree or education requirement before tailoring. If it is not true for you, do not claim it."
        }
        HardConstraintCategory::Experience => {
            "Check years or level before tailoring. Do not round up, stretch titles, or imply more experience than you have."
        }
        HardConstraintCategory::Language => {
            "Check language fluency before tailoring. If it is not true for you, do not claim it."
        }
        HardConstraintCategory::Age => {
            "Check the minimum-age or legal work-age requirement before tailoring. If it is not true for you, do not claim it."
        }
        HardConstraintCategory::BackgroundScreening => {
            "Check background, drug, or pre-employment screening before tailoring. If it is not workable or true for you, do not claim or imply that it is."
        }
        HardConstraintCategory::PhysicalRequirement => {
            "Check this physical demand before tailoring. If it is not workable or safe for you, do not claim it."
        }
        HardConstraintCategory::Location => {
            "Check location, schedule, availability, or travel before tailoring. If it is not workable for you, do not claim it."
        }
    };
    action.to_string()
}

fn driving_record_constraint_keyword(keyword: &str) -> bool {
    requirement_rules::hard_constraint_driving_record_keyword(keyword)
}

fn vehicle_insurance_constraint_keyword(keyword: &str) -> bool {
    requirement_rules::hard_constraint_vehicle_insurance_keyword(keyword)
}

pub(super) fn citizenship_constraint_keyword(keyword: &str) -> bool {
    requirement_rules::hard_constraint_citizenship_keyword(keyword)
}

pub(super) fn seniority_level_constraint_keyword(keyword: &str) -> bool {
    requirement_rules::hard_constraint_seniority_level_keyword(keyword)
}

pub(super) fn hard_constraint_category(keyword: &str) -> Option<HardConstraintCategory> {
    let lower = keyword.to_lowercase();
    if requirement_rules::hard_constraint_degree_equivalent_exclusion(&lower) {
        return None;
    }
    if citizenship_constraint_keyword(&lower) {
        return Some(HardConstraintCategory::Citizenship);
    }
    if requirement_rules::hard_constraint_work_authorization_keyword(&lower) {
        return Some(HardConstraintCategory::WorkAuthorization);
    }
    if requirement_rules::hard_constraint_security_clearance_keyword(&lower) {
        return Some(HardConstraintCategory::SecurityClearance);
    }
    if requirement_rules::hard_constraint_license_or_certification_keyword(&lower)
        || requirement_rules::is_credential_keyword(&lower)
    {
        return Some(HardConstraintCategory::LicenseOrCertification);
    }
    if requirement_rules::hard_constraint_education_keyword(&lower) {
        return Some(HardConstraintCategory::Education);
    }
    if age_requirement_keyword(&lower) {
        return Some(HardConstraintCategory::Age);
    }
    if requirement_rules::hard_constraint_experience_keyword(&lower) {
        return Some(HardConstraintCategory::Experience);
    }
    if term_expansion::known_human_language_requirement(&lower) {
        return Some(HardConstraintCategory::Language);
    }
    if requirement_rules::hard_constraint_background_screening_keyword(&lower)
        || driving_record_constraint_keyword(&lower)
    {
        return Some(HardConstraintCategory::BackgroundScreening);
    }
    if requirement_rules::is_physical_weight_requirement(&lower)
        || requirement_rules::hard_constraint_physical_requirement_keyword(&lower)
    {
        return Some(HardConstraintCategory::PhysicalRequirement);
    }
    if requirement_rules::hard_constraint_location_keyword(&lower)
        || vehicle_insurance_constraint_keyword(&lower)
    {
        return Some(HardConstraintCategory::Location);
    }
    None
}

pub(super) fn extract_hard_constraint_keywords(text: &str) -> Vec<String> {
    let mut keywords = HashSet::new();
    let degree_equivalent_re = regex::Regex::new(
            r"(?i)\b(?:ph\.?d\.?(?:\s+degree)?|doctorate(?:\s+degree)?|doctoral degree|associate'?s degree|associate degree|baccalaureate degree|bachelor'?s degree|bachelor degree|master'?s degree|master degree|degree)\s+(?:or|/)\s+(?:(?:equivalent|commensurate)\s+(?:work\s+)?experience|equivalent\s+combination\s+of\s+education\s+and\s+experience)\b",
        )
        .unwrap();
    let has_degree_equivalent = degree_equivalent_re.is_match(text);
    if has_degree_equivalent {
        keywords.insert("degree or equivalent experience".to_string());
    }

    let language_alternation = requirement_rules::human_languages().join("|");
    let hard_constraint_patterns = [
        r"(?i)\b(work authorization|authorized to work|visa sponsorship|u\.?s\.?\s+citizenship|u\.?s\.?\s+citizen|citizenship required)\b".to_string(),
        r"(?i)\b(security clearance|clearance)\b".to_string(),
        r"(?i)\b(clean driving record|acceptable driving record|driving record|mvr|motor vehicle record)\b".to_string(),
        r"(?i)\b(certification|food[- ]handler'?s?\s+(?:certification|certificate|permit|card))\b".to_string(),
        r"(?i)\b(ph\.?d\.?(?:\s+degree)?|doctorate(?:\s+degree)?|doctoral degree|associate'?s degree|associate degree|baccalaureate degree|bachelor'?s degree|bachelor degree|master'?s degree|master degree|degree|high[- ]school diploma|high[- ]school degree|ged|high[- ]school equivalency|general education development)\b".to_string(),
        r"(?i)\b\d+\+?\s*(?:years?|yrs?)\s+(?:of\s+)?(?:experience\s+(?:with|in)\s+)?[a-zA-Z][a-zA-Z0-9+#/.-]*(?:\s+[a-zA-Z][a-zA-Z0-9+#/.-]*){0,3}\b".to_string(),
        r"(?i)\b(?:minimum age(?:\s+is)?\s*)?\d{2}\s*(?:\+|(?:years?|yrs?)\s+(?:old|of\s+age))\b".to_string(),
        r"(?i)\b(?:minimum age|age requirement|legal work age)\b".to_string(),
        format!(r"(?i)\b(bilingual(?:\s+(?:english|{language_alternation}))?|(?:{language_alternation})\s+fluency|fluent(?:\s+in)?\s+(?:{language_alternation})|(?:{language_alternation})\s+language|english/(?:{language_alternation})|english and (?:{language_alternation}))\b"),
        r"(?i)\b(background checks?|background screenings?|pre[- ]employment screenings?|drug screens?|drug screenings?|drug tests?|drug testing)\b".to_string(),
        r"(?i)\b((?:stand|standing) for long periods?|climb(?:ing)? ladders?|physical requirements?|physical demands?)\b".to_string(),
        r"(?i)\b(onsite|on-site|on site|remote(?:[- ](?:work|role|position|job))?|hybrid(?:[- ](?:work|role|schedule|position|job))?|reliable internet(?:\s+connection)?|high[- ]speed internet(?:\s+connection)?|home office|quiet workspace|dedicated workspace|relocation|relocate|willing to relocate|travel|reliable transportation|own transportation|reliable vehicle|insured vehicle|proof of auto insurance|proof of insurance|auto insurance|car insurance|vehicle insurance|commute|commuting|full[- ]time(?:\s+availability)?|part[- ]time(?:\s+availability)?|availability|available|schedule|weekend availability|weekend shifts?|night shift|overnight shift|third shift|3rd shift|evening shift|second shift|2nd shift|day shift|first shift|1st shift|overtime(?:\s+(?:availability|shifts?|hours?))?|holiday(?:\s+(?:availability|shifts?|hours?))?)\b".to_string(),
    ];

    for keyword in requirement_rules::extract_physical_weight_keywords(text) {
        keywords.insert(keyword);
    }
    for keyword in requirement_rules::extract_credential_keywords(text) {
        keywords.insert(keyword);
    }

    for pattern in &hard_constraint_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            for cap in re.captures_iter(text) {
                if let Some(m) = cap.get(0) {
                    keywords.insert(m.as_str().to_lowercase());
                }
            }
        }
    }
    if has_degree_equivalent {
        for exact_degree in [
            "degree",
            "associate's degree",
            "associate degree",
            "associates degree",
            "baccalaureate degree",
            "bachelor's degree",
            "bachelor degree",
            "bachelors degree",
            "master's degree",
            "master degree",
            "masters degree",
            "phd",
            "ph.d",
            "ph.d.",
            "phd degree",
            "ph.d degree",
            "ph.d. degree",
            "doctorate",
            "doctorate degree",
            "doctoral degree",
        ] {
            keywords.remove(exact_degree);
        }
    }
    if keywords.iter().any(|keyword| {
        matches!(
            keyword.as_str(),
            "commercial driver's license"
                | "commercial drivers license"
                | "commercial driver license"
                | "cdl"
        )
    }) {
        for generic_license in ["driver's license", "drivers license", "driver license"] {
            keywords.remove(generic_license);
        }
    }
    let specific_certification_keywords = requirement_rules::specific_credential_keywords();
    if keywords
        .iter()
        .any(|keyword| specific_certification_keywords.contains(keyword))
    {
        keywords.remove("certification");
    }
    for keyword in extract_seniority_constraint_keywords(text) {
        keywords.insert(keyword);
    }

    let mut sorted_keywords = keywords.into_iter().collect::<Vec<_>>();
    sorted_keywords.sort();
    sorted_keywords
}

fn age_requirement_keyword(keyword: &str) -> bool {
    requirement_rules::hard_constraint_age_requirement_keyword(keyword)
}

pub(super) fn extract_seniority_constraint_keywords(text: &str) -> Vec<String> {
    let mut keywords = HashSet::new();
    let seniority_patterns = [
        (
            r"(?i)\b(senior[- ]level|senior|sr\.)\b",
            "senior-level experience",
        ),
        (
            r"(?i)\b(lead[- ]level|team lead|shift lead|crew lead|lead worker|lead experience|leadership experience)\b",
            "lead-level experience",
        ),
        (
            r"(?i)\b(staff[- ]level|principal[- ]level|staff engineer|principal engineer|principal consultant)\b",
            "staff/principal-level experience",
        ),
        (
            r"(?i)\b(people management|management experience|manager[- ]level|supervisor[- ]level|supervisor experience|supervisory experience|supervision experience|team management|team supervision|supervising staff|supervised staff|managed\s+(?:a\s+)?team|managed staff|managed people|managed employees)\b",
            "management experience",
        ),
        (
            r"(?i)\b(director[- ]level|director experience|department director)\b",
            "director-level experience",
        ),
        (
            r"(?i)\b(executive[- ]level|executive leadership|c-suite|vice president|vp)\b",
            "executive-level experience",
        ),
        (
            r"(?i)\b(mid[- ]level|intermediate)\b",
            "mid-level experience",
        ),
    ];

    for (pattern, keyword) in seniority_patterns {
        if regex::Regex::new(pattern).unwrap().is_match(text) {
            keywords.insert(keyword.to_string());
        }
    }

    let mut sorted_keywords = keywords.into_iter().collect::<Vec<_>>();
    sorted_keywords.sort();
    sorted_keywords
}
