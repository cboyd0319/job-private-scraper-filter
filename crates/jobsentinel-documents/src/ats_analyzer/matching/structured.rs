// Matches ATS keywords against structured resume fields and evidence citations.

use super::{record_keyword_result, sort_keyword_matches, sort_missing_keywords, MatchedKeyword};
use crate::{
    ats_analyzer::{term_expansion, AtsAnalyzer},
    ats_types::{KeywordImportance, MissingKeyword, RegionalMatchingProfile},
    structured_resume::ResumeAnalysisInput,
};
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};

impl AtsAnalyzer {
    pub(in crate::ats_analyzer) fn find_keyword_matches(
        input: &ResumeAnalysisInput,
        job_keywords: &[(String, KeywordImportance)],
        region: Option<RegionalMatchingProfile>,
    ) -> (Vec<MatchedKeyword>, Vec<MissingKeyword>) {
        let resume = &input.resume;
        let evidence_snapshot = input.evidence_snapshot.as_ref();
        let mut matches = Vec::new();
        let mut missing = Vec::new();
        for (keyword, importance) in job_keywords {
            let mut found_in = Vec::new();
            let mut evidence_citations = Vec::new();
            let mut frequency = 0;
            let search_terms =
                term_expansion::conservative_keyword_search_terms(&keyword.to_lowercase(), region);
            if let Some(summary) = resume.summary.as_deref() {
                let count = Self::keyword_frequency_for_search_terms(
                    &summary.to_lowercase(),
                    &search_terms,
                );
                if count > 0 {
                    Self::add_evidence_section(&mut found_in, "summary");
                    Self::cite(
                        &mut evidence_citations,
                        evidence_snapshot,
                        "summary".to_string(),
                    );
                    frequency += count;
                }
            }

            for (field_path, section, content) in [
                ("clearance", "clearance", resume.clearance.as_deref()),
                (
                    "military_info",
                    "military service",
                    resume.military_info.as_deref(),
                ),
            ] {
                let Some(content) = content else {
                    continue;
                };
                let count = Self::keyword_frequency_for_search_terms(
                    &content.to_lowercase(),
                    &search_terms,
                );
                if count > 0 {
                    Self::add_evidence_section(&mut found_in, section);
                    Self::cite(
                        &mut evidence_citations,
                        evidence_snapshot,
                        field_path.to_string(),
                    );
                    frequency += count;
                }
            }

            for (index, experience) in resume.experience.iter().enumerate() {
                let mut fields = vec![experience.title.as_str(), experience.company.as_str()];
                fields.extend(experience.achievements.iter().map(String::as_str));
                let count = Self::keyword_frequency_across_fields(&fields, &search_terms);
                if count == 0 {
                    continue;
                }

                let section = Self::structured_experience_evidence_section(experience);
                Self::add_evidence_section(&mut found_in, section);
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("experience.{index}.title"),
                    &experience.title,
                    &search_terms,
                );
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("experience.{index}.company"),
                    &experience.company,
                    &search_terms,
                );
                for (achievement_index, achievement) in experience.achievements.iter().enumerate() {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("experience.{index}.achievements.{achievement_index}"),
                        achievement,
                        &search_terms,
                    );
                }
                frequency += Self::evidence_strength_adjusted_count(
                    count,
                    &fields.join(" ").to_lowercase(),
                    section,
                );
            }

            for (category_index, category) in resume.skills.iter().enumerate() {
                for (skill_index, skill) in category.skills.iter().enumerate() {
                    let skill_lower = skill.name.to_lowercase();
                    if search_terms.iter().any(|term| {
                        Self::keyword_appears_in_text(&skill_lower, term)
                            || Self::keyword_appears_in_text(term, &skill_lower)
                    }) {
                        Self::add_evidence_section(&mut found_in, "skills");
                        Self::cite(
                            &mut evidence_citations,
                            evidence_snapshot,
                            format!("skills.{category_index}.skills.{skill_index}.name"),
                        );
                        frequency += 1;
                    }
                }
            }

            for (index, education) in resume.education.iter().enumerate() {
                let mut fields = vec![
                    education.degree.as_str(),
                    education.institution.as_str(),
                    education.location.as_deref().unwrap_or_default(),
                ];
                fields.extend(education.honors.iter().map(String::as_str));
                let count = Self::keyword_frequency_across_fields(&fields, &search_terms);
                if count == 0 {
                    continue;
                }

                Self::add_evidence_section(&mut found_in, "education");
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("education.{index}.degree"),
                    &education.degree,
                    &search_terms,
                );
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("education.{index}.institution"),
                    &education.institution,
                    &search_terms,
                );
                if let Some(location) = education.location.as_deref() {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("education.{index}.location"),
                        location,
                        &search_terms,
                    );
                }
                for (honor_index, honor) in education.honors.iter().enumerate() {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("education.{index}.honors.{honor_index}"),
                        honor,
                        &search_terms,
                    );
                }
                frequency += count;
            }

            for (index, certification) in resume.certifications.iter().enumerate() {
                let fields = [
                    ("name", certification.name.as_str()),
                    ("issuer", certification.issuer.as_str()),
                    (
                        "date_obtained",
                        certification.date_obtained.as_deref().unwrap_or_default(),
                    ),
                    (
                        "expiration_date",
                        certification.expiration_date.as_deref().unwrap_or_default(),
                    ),
                    (
                        "credential_id",
                        certification.credential_id.as_deref().unwrap_or_default(),
                    ),
                ];
                let field_values = fields.map(|(_, value)| value);
                let count = Self::keyword_frequency_across_fields(&field_values, &search_terms);
                if count == 0 {
                    continue;
                }

                Self::add_evidence_section(&mut found_in, "certifications");
                for (field, content) in fields {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("certifications.{index}.{field}"),
                        content,
                        &search_terms,
                    );
                }
                frequency += count;
            }

            for (index, project) in resume.projects.iter().enumerate() {
                let mut fields = vec![
                    project.name.as_str(),
                    project.description.as_str(),
                    project.url.as_deref().unwrap_or_default(),
                ];
                fields.extend(project.technologies.iter().map(String::as_str));
                let count = Self::keyword_frequency_across_fields(&fields, &search_terms);
                if count == 0 {
                    continue;
                }

                Self::add_evidence_section(&mut found_in, "projects");
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("projects.{index}.name"),
                    &project.name,
                    &search_terms,
                );
                Self::cite_if_matched(
                    &mut evidence_citations,
                    evidence_snapshot,
                    format!("projects.{index}.description"),
                    &project.description,
                    &search_terms,
                );
                for (technology_index, technology) in project.technologies.iter().enumerate() {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("projects.{index}.technologies.{technology_index}"),
                        technology,
                        &search_terms,
                    );
                }
                if let Some(url) = project.url.as_deref() {
                    Self::cite_if_matched(
                        &mut evidence_citations,
                        evidence_snapshot,
                        format!("projects.{index}.url"),
                        url,
                        &search_terms,
                    );
                }
                frequency += Self::evidence_strength_adjusted_count(
                    count,
                    &fields.join(" ").to_lowercase(),
                    "projects",
                );
            }

            record_keyword_result(
                keyword,
                *importance,
                found_in,
                frequency,
                evidence_citations,
                &mut matches,
                &mut missing,
            );
        }

        sort_keyword_matches(&mut matches);
        sort_missing_keywords(&mut missing);
        (matches, missing)
    }

    fn cite_if_matched(
        citations: &mut Vec<ResumeEvidenceCitation>,
        snapshot: Option<&ResumeEvidenceSnapshot>,
        field_path: String,
        content: &str,
        search_terms: &[String],
    ) {
        if Self::keyword_frequency_for_search_terms(&content.to_lowercase(), search_terms) > 0 {
            Self::cite(citations, snapshot, field_path);
        }
    }

    fn cite(
        citations: &mut Vec<ResumeEvidenceCitation>,
        snapshot: Option<&ResumeEvidenceSnapshot>,
        field_path: String,
    ) {
        if let Some(citation) =
            snapshot.and_then(|snapshot| ResumeEvidenceCitation::for_field(snapshot, &field_path))
        {
            citations.push(citation);
        }
    }

    fn keyword_frequency_across_fields(fields: &[&str], search_terms: &[String]) -> usize {
        search_terms
            .iter()
            .map(|term| {
                fields
                    .iter()
                    .map(|field| Self::keyword_frequency(&field.to_lowercase(), term))
                    .sum()
            })
            .max()
            .unwrap_or_default()
    }
}
