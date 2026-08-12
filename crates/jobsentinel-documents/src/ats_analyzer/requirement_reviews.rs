use super::super::ats_types::{
    HardConstraintCategory, KeywordImportance, KeywordMatch, MissingKeyword,
    ProfessionMatchingProfile, RequirementMatchState, RequirementReview,
};
use super::hard_constraints;
use super::matching::MatchedKeyword;
use super::matching_profiles;

pub(super) fn build_requirement_reviews(
    job_keywords: &[(String, KeywordImportance)],
    keyword_matches: &[MatchedKeyword],
    missing_keyword_details: &[MissingKeyword],
    profession: Option<ProfessionMatchingProfile>,
) -> Vec<RequirementReview> {
    let mut reviews = Vec::new();

    for (keyword, importance) in job_keywords {
        if let Some(matched) = keyword_matches
            .iter()
            .find(|item| item.keyword_match.keyword.eq_ignore_ascii_case(keyword))
        {
            let match_state = if hard_constraints::hard_constraint_category(keyword)
                == Some(HardConstraintCategory::SecurityClearance)
            {
                RequirementMatchState::Implied
            } else {
                classify_requirement_match_state(&matched.keyword_match)
            };
            let mut review = RequirementReview {
                keyword: keyword.clone(),
                importance: *importance,
                match_state,
                evidence_sections: matched.keyword_match.found_in.clone(),
                evidence_citations: matched.evidence_citations.clone(),
                hard_constraint: hard_constraints::hard_constraint_category(keyword).is_some(),
                profile_preferred_section: None,
                recommendation: requirement_recommendation(match_state),
            };
            review.profile_preferred_section = profession
                .map(|profile| matching_profiles::profile_prefers_section(profile, &review));
            reviews.push(review);
        } else if missing_keyword_details
            .iter()
            .any(|item| item.keyword.eq_ignore_ascii_case(keyword))
        {
            reviews.push(RequirementReview {
                keyword: keyword.clone(),
                importance: *importance,
                match_state: RequirementMatchState::Missing,
                evidence_sections: Vec::new(),
                evidence_citations: Vec::new(),
                hard_constraint: hard_constraints::hard_constraint_category(keyword).is_some(),
                profile_preferred_section: profession.map(|_| false),
                recommendation: requirement_recommendation(RequirementMatchState::Missing),
            });
        }
    }

    reviews.sort_by(|a, b| {
        let imp_order = |imp: KeywordImportance| match imp {
            KeywordImportance::Required => 0,
            KeywordImportance::Preferred => 1,
            KeywordImportance::Industry => 2,
        };
        let state_order = |state: RequirementMatchState| match state {
            RequirementMatchState::Missing => 0,
            RequirementMatchState::Partial => 1,
            RequirementMatchState::Implied => 2,
            RequirementMatchState::Direct => 3,
            RequirementMatchState::Strong => 4,
        };
        imp_order(a.importance)
            .cmp(&imp_order(b.importance))
            .then(state_order(a.match_state).cmp(&state_order(b.match_state)))
            .then(
                b.profile_preferred_section
                    .unwrap_or(false)
                    .cmp(&a.profile_preferred_section.unwrap_or(false)),
            )
            .then(a.keyword.cmp(&b.keyword))
    });

    reviews
}

fn classify_requirement_match_state(matched: &KeywordMatch) -> RequirementMatchState {
    let has_direct_evidence = matched.found_in.iter().any(|section| {
        matches!(
            section.as_str(),
            "resume text"
                | "experience"
                | "current experience"
                | "recent experience"
                | "summary"
                | "projects"
                | "education"
                | "certifications"
                | "licenses"
                | "languages"
                | "awards"
                | "publications"
                | "references"
        )
    });

    let evidence_section_count = matched
        .found_in
        .iter()
        .filter(|section| section.as_str() != "military service")
        .count();
    if has_direct_evidence && (matched.frequency > 1 || evidence_section_count > 1) {
        RequirementMatchState::Strong
    } else if has_direct_evidence {
        RequirementMatchState::Direct
    } else if matched.found_in.iter().any(|section| section == "skills") {
        RequirementMatchState::Partial
    } else {
        RequirementMatchState::Implied
    }
}

fn requirement_recommendation(match_state: RequirementMatchState) -> String {
    match match_state {
        RequirementMatchState::Strong => {
            "Strong visible evidence found. Keep it easy to see near the relevant role.".to_string()
        }
        RequirementMatchState::Direct => {
            "Found visible evidence. Keep it clear and tied to real work or credentials."
                .to_string()
        }
        RequirementMatchState::Partial => {
            "Found in a lighter evidence area. Add supporting evidence only if true.".to_string()
        }
        RequirementMatchState::Implied => {
            "Related evidence may exist, but the wording is not clear. Review before relying on it."
                .to_string()
        }
        RequirementMatchState::Missing => {
            "Only add it if true. If this is required and not true, treat the role as higher risk."
                .to_string()
        }
    }
}
