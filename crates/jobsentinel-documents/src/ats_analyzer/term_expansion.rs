use super::super::ats_types::RegionalMatchingProfile;

pub(super) fn conservative_keyword_search_terms(
    keyword_lower: &str,
    region: Option<RegionalMatchingProfile>,
) -> Vec<String> {
    let mut terms = vec![keyword_lower.to_string()];
    for group in super::requirement_rules::conservative_search_term_groups() {
        if group.iter().any(|term| term == keyword_lower) {
            for term in group {
                if !terms.iter().any(|existing| existing == term) {
                    terms.push(term.clone());
                }
            }
        }
    }
    for term in super::requirement_rules::supplemental_keyword_search_terms(keyword_lower) {
        if !terms.iter().any(|existing| existing == &term) {
            terms.push(term);
        }
    }
    for term in super::requirement_rules::credential_keyword_search_terms(keyword_lower) {
        if !terms.iter().any(|existing| existing == &term) {
            terms.push(term);
        }
    }
    terms.extend(super::requirement_rules::physical_weight_search_terms(
        keyword_lower,
    ));
    extend_language_fluency_terms(keyword_lower, &mut terms);
    if let Some(region) = region {
        super::matching_profiles::extend_regional_search_terms(keyword_lower, region, &mut terms);
    }

    match keyword_lower {
        "senior-level experience" => {
            terms.extend(
                [
                    "senior", "sr.", "lead", "5 years", "5+ years", "5 yrs", "5+ yrs",
                ]
                .into_iter()
                .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(6));
        }
        "mid-level experience" => {
            terms.extend(
                [
                    "mid-level",
                    "intermediate",
                    "3 years",
                    "3+ years",
                    "3 yrs",
                    "3+ yrs",
                ]
                .into_iter()
                .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(4));
        }
        "lead-level experience" => {
            terms.extend(
                [
                    "lead",
                    "team lead",
                    "shift lead",
                    "crew lead",
                    "lead worker",
                    "lead experience",
                    "leadership experience",
                    "supervised",
                    "supervisor",
                ]
                .into_iter()
                .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(5));
        }
        "staff/principal-level experience" => {
            terms.extend(
                ["staff", "principal", "architect", "10 years", "10+ years"]
                    .into_iter()
                    .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(11));
        }
        "management experience" => {
            terms.extend(
                [
                    "management",
                    "manager",
                    "managed",
                    "managed a team",
                    "managed team",
                    "managed staff",
                    "managed people",
                    "managed employees",
                    "people management",
                    "supervisor experience",
                    "supervised",
                    "supervised staff",
                    "supervising staff",
                    "supervisor",
                    "team supervision",
                ]
                .into_iter()
                .map(str::to_string),
            );
        }
        "director-level experience" => {
            terms.extend(
                [
                    "director",
                    "head of",
                    "department lead",
                    "10 years",
                    "10+ years",
                ]
                .into_iter()
                .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(11));
        }
        "executive-level experience" => {
            terms.extend(
                [
                    "executive",
                    "vp",
                    "vice president",
                    "chief",
                    "c-level",
                    "10 years",
                    "10+ years",
                ]
                .into_iter()
                .map(str::to_string),
            );
            terms.extend(experience_year_search_terms(11));
        }
        "degree or equivalent experience" => {
            terms.extend(
                [
                    "degree",
                    "bachelor's degree",
                    "bachelor degree",
                    "bachelor",
                    "ba",
                    "bs",
                    "master's degree",
                    "master degree",
                    "master",
                    "ma",
                    "ms",
                    "equivalent experience",
                    "work experience",
                    "experience",
                ]
                .into_iter()
                .map(str::to_string),
            );
        }
        _ => {}
    }

    terms
}

fn experience_year_search_terms(min_years: usize) -> Vec<String> {
    let mut terms = Vec::new();
    for years in min_years..=50 {
        terms.push(format!("{years} years"));
        terms.push(format!("{years}+ years"));
        terms.push(format!("{years} yrs"));
        terms.push(format!("{years}+ yrs"));
    }
    terms
}

pub(super) fn known_human_language_requirement(lower: &str) -> bool {
    if lower.contains("bilingual") {
        return true;
    }

    super::requirement_rules::human_languages()
        .iter()
        .any(|language| {
            lower.contains(&format!("{language} fluency"))
                || lower.contains(&format!("fluent {language}"))
                || lower.contains(&format!("fluent in {language}"))
                || lower.contains(&format!("{language} language"))
                || lower.contains(&format!("english/{language}"))
                || lower.contains(&format!("english and {language}"))
        })
}

fn extend_language_fluency_terms(keyword_lower: &str, terms: &mut Vec<String>) {
    for language in super::requirement_rules::human_languages() {
        if !keyword_lower.contains(language) {
            continue;
        }

        for term in [
            format!("bilingual {language}"),
            format!("{language} fluency"),
            format!("fluent {language}"),
            format!("fluent in {language}"),
            format!("{language} language"),
            format!("english/{language}"),
            format!("english and {language}"),
            language.clone(),
        ] {
            if !terms.iter().any(|existing| existing == &term) {
                terms.push(term);
            }
        }
    }
}
