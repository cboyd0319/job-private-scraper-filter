//! Resume readability analyzer for candidate-side explainability
//!
//! This module analyzes resumes for job-word coverage, readable structure,
//! and suggestions that improve truthful application clarity.
#![allow(clippy::unwrap_used, clippy::expect_used)] // Regex patterns are compile-time constants

use super::ats_types::*;
use super::structured_resume::ResumeAnalysisInput;
#[cfg(test)]
use super::structured_resume::{
    ResumeEducation, ResumeExperience, ResumePersonalInfo, ResumeSkill, ResumeSkillCategory,
    StructuredResume,
};
use jobsentinel_domain::ResumeEvidenceSnapshot;

mod bullet_prompts;
mod format_result;
mod hard_constraints;
mod keyword_catalog;
mod matching;
mod matching_profiles;
mod plain_text_format;
mod requirement_reviews;
mod requirement_rules;
mod structured_format;
mod term_expansion;

// ============================================================================
// Resume Readability Analyzer
// ============================================================================

pub struct AtsAnalyzer;

impl AtsAnalyzer {
    pub(super) const SECTION_BOUNDARY_HEADERS: &'static [&'static str] = &[
        "required",
        "must have",
        "requirements",
        "qualifications",
        "preferred",
        "nice to have",
        "bonus",
        "plus",
        "responsibilities",
        "about the role",
        "what you will do",
        "benefits",
        "education",
        "experience",
    ];

    /// Analyze resume against a specific job description
    pub fn analyze_for_job(
        resume: &ResumeAnalysisInput,
        job_description: &str,
    ) -> AtsAnalysisResult {
        Self::analyze_for_job_context(resume, job_description, None)
    }

    pub fn analyze_for_job_with_profile(
        resume: &ResumeAnalysisInput,
        job_description: &str,
        profile: ResumeMatchingProfile,
    ) -> AtsAnalysisResult {
        Self::analyze_for_job_context(resume, job_description, Some(profile))
    }

    fn analyze_for_job_context(
        resume: &ResumeAnalysisInput,
        job_description: &str,
        profile: Option<ResumeMatchingProfile>,
    ) -> AtsAnalysisResult {
        let normalized_job = profile.map(|profile| {
            matching_profiles::normalize_job_description(job_description, profile.region)
        });
        let job_keywords =
            Self::extract_job_keywords(normalized_job.as_deref().unwrap_or(job_description));
        let format_result = Self::analyze_format(resume);

        // Find keyword matches
        let (keyword_matches, missing_keyword_details) = Self::find_keyword_matches(
            resume,
            &job_keywords,
            profile.map(|profile| profile.region),
        );
        Self::build_job_analysis_result(
            job_description,
            &job_keywords,
            format_result,
            keyword_matches,
            missing_keyword_details,
            profile,
        )
    }

    /// Analyze plain resume text against a job description without returning raw text.
    pub fn analyze_text_for_job(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(resume_text, skills, job_description, None, None, None)
    }

    pub fn analyze_text_for_job_with_profile(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        profile: ResumeMatchingProfile,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(
            resume_text,
            skills,
            job_description,
            None,
            None,
            Some(profile),
        )
    }

    pub fn analyze_text_for_job_with_snapshot(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        evidence_snapshot: &ResumeEvidenceSnapshot,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(
            resume_text,
            skills,
            job_description,
            None,
            Some(evidence_snapshot),
            None,
        )
    }

    /// Analyze readable resume text against a job description with optional source markup for
    /// format-only checks.
    pub fn analyze_text_for_job_with_source(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        source_text: Option<&str>,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(
            resume_text,
            skills,
            job_description,
            source_text,
            None,
            None,
        )
    }

    pub fn analyze_text_for_job_with_source_and_snapshot(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        source_text: Option<&str>,
        evidence_snapshot: &ResumeEvidenceSnapshot,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(
            resume_text,
            skills,
            job_description,
            source_text,
            Some(evidence_snapshot),
            None,
        )
    }

    pub fn analyze_text_for_job_with_source_snapshot_and_profile(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        source_text: Option<&str>,
        evidence_snapshot: &ResumeEvidenceSnapshot,
        profile: ResumeMatchingProfile,
    ) -> AtsAnalysisResult {
        Self::analyze_text_for_job_context(
            resume_text,
            skills,
            job_description,
            source_text,
            Some(evidence_snapshot),
            Some(profile),
        )
    }

    fn analyze_text_for_job_context(
        resume_text: &str,
        skills: &[String],
        job_description: &str,
        source_text: Option<&str>,
        evidence_snapshot: Option<&ResumeEvidenceSnapshot>,
        profile: Option<ResumeMatchingProfile>,
    ) -> AtsAnalysisResult {
        let normalized_job = profile.map(|profile| {
            matching_profiles::normalize_job_description(job_description, profile.region)
        });
        let job_keywords =
            Self::extract_job_keywords(normalized_job.as_deref().unwrap_or(job_description));
        let format_result = if source_text.is_some() {
            plain_text_format::analyze_plain_text_format_with_source(resume_text, source_text)
        } else {
            plain_text_format::analyze_plain_text_format(resume_text)
        };
        let (keyword_matches, missing_keyword_details) = Self::find_keyword_matches_in_text(
            resume_text,
            skills,
            &job_keywords,
            evidence_snapshot,
            profile.map(|profile| profile.region),
        );

        Self::build_job_analysis_result(
            job_description,
            &job_keywords,
            format_result,
            keyword_matches,
            missing_keyword_details,
            profile,
        )
    }

    /// Analyze resume format without job context
    pub fn analyze_format(resume: &ResumeAnalysisInput) -> AtsAnalysisResult {
        structured_format::analyze_format(resume)
    }

    /// Extract keywords from job description
    pub fn extract_job_keywords(job_description: &str) -> Vec<(String, KeywordImportance)> {
        let mut keywords = Vec::new();
        let lower = job_description.to_lowercase();

        // Split into sections
        let required_section = Self::extract_section(
            &lower,
            &["required", "must have", "requirements", "qualifications"],
        );
        let preferred_section =
            Self::extract_section(&lower, &["preferred", "nice to have", "bonus", "plus"]);

        // Extract from required section
        for keyword in keyword_catalog::extract_keywords_from_text(&required_section) {
            keywords.push((keyword, KeywordImportance::Required));
        }
        for keyword in hard_constraints::extract_hard_constraint_keywords(&required_section) {
            if !keywords.iter().any(|(k, _)| k == &keyword) {
                keywords.push((keyword, KeywordImportance::Required));
            }
        }

        // Extract from preferred section
        for keyword in keyword_catalog::extract_keywords_from_text(&preferred_section) {
            if !keywords.iter().any(|(k, _)| k == &keyword) {
                keywords.push((keyword, KeywordImportance::Preferred));
            }
        }
        for keyword in hard_constraints::extract_hard_constraint_keywords(&preferred_section) {
            if !keywords.iter().any(|(k, _)| k == &keyword) {
                keywords.push((keyword, KeywordImportance::Preferred));
            }
        }

        // Add industry keywords if found
        for keyword in keyword_catalog::industry_keywords() {
            let canonical_keyword = keyword_catalog::canonical_requirement_keyword(&keyword);
            if Self::keyword_appears_in_text(&lower, &keyword)
                && !keywords.iter().any(|(k, _)| k == &canonical_keyword)
            {
                keywords.push((canonical_keyword, KeywordImportance::Industry));
            }
        }

        keywords
    }

    /// Get power words for action verbs
    pub fn get_power_words() -> Vec<&'static str> {
        bullet_prompts::get_power_words()
    }

    /// Suggest improved bullet point
    pub fn improve_bullet(bullet: &str, job_context: Option<&str>) -> String {
        let mut improved = bullet.trim().to_string();

        // Ensure starts with power word
        let power_words = Self::get_power_words();
        let starts_with_power_word = power_words
            .iter()
            .any(|&word| improved.to_lowercase().starts_with(word));

        if !starts_with_power_word {
            // Prompt for a clearer verb without upgrading the user's claim.
            let lower = improved.to_lowercase();
            let vague_action = ["was responsible for", "worked on", "helped with"]
                .iter()
                .any(|phrase| lower.contains(phrase));

            if vague_action {
                improved.push_str(" (choose a clearer action verb only if it is true)");
            }
        }

        // Add quantification if missing
        if !improved.contains(|c: char| c.is_ascii_digit()) && !improved.contains('%') {
            improved.push_str(" (add a true number, outcome, or concrete detail if you have one)");
        }

        // Add relevant keywords from job context
        if let Some(job_desc) = job_context {
            let keywords = Self::extract_job_keywords(job_desc);
            let important_keywords: Vec<_> = keywords
                .iter()
                .filter(|(_, imp)| *imp == KeywordImportance::Required)
                .take(2)
                .map(|(k, _)| k.as_str())
                .collect();

            if !important_keywords.is_empty()
                && !important_keywords
                    .iter()
                    .any(|&k| improved.to_lowercase().contains(&k.to_lowercase()))
            {
                improved.push_str(&format!(
                    " (review if these are true and worth making visible: {})",
                    important_keywords.join(", ")
                ));
            }

            bullet_prompts::append_role_specific_evidence_prompt(&mut improved, job_desc);
        }

        bullet_prompts::append_interview_defense_prompt(&mut improved);

        improved
    }

    fn build_job_analysis_result(
        job_description: &str,
        job_keywords: &[(String, KeywordImportance)],
        mut format_result: AtsAnalysisResult,
        keyword_matches: Vec<matching::MatchedKeyword>,
        missing_keyword_details: Vec<MissingKeyword>,
        profile: Option<ResumeMatchingProfile>,
    ) -> AtsAnalysisResult {
        let missing_keywords = missing_keyword_details
            .iter()
            .map(|gap| gap.keyword.clone())
            .collect::<Vec<_>>();

        let total_keywords = job_keywords.len();
        let matched_keywords = keyword_matches.len();
        let keyword_score = if total_keywords > 0 {
            (matched_keywords as f64 / total_keywords as f64) * 100.0
        } else {
            0.0
        };

        if total_keywords == 0 && !job_description.trim().is_empty() {
            format_result.format_issues.push(FormatIssue {
                severity: IssueSeverity::Info,
                issue: "Not enough job-post detail recognized to score fit confidently."
                    .to_string(),
                fix: "Paste a fuller job post with responsibilities, requirements, or preferred qualifications."
                    .to_string(),
            });
        }

        let mut suggestions = format_result.suggestions.clone();
        for gap in &missing_keyword_details {
            let impact = match gap.importance {
                KeywordImportance::Required => {
                    "Required job-post language is easier to compare when real evidence is visible."
                }
                KeywordImportance::Preferred => {
                    "Preferred job-post language can help when it honestly fits your background."
                }
                KeywordImportance::Industry => {
                    "Role language can improve clarity when it accurately describes your work."
                }
            };

            suggestions.push(AtsSuggestion {
                category: SuggestionCategory::AddKeyword,
                suggestion: format!(
                    "Review whether '{}' is true for your background and worth making visible",
                    gap.keyword
                ),
                impact: impact.to_string(),
            });
        }

        let requirement_reviews = requirement_reviews::build_requirement_reviews(
            job_keywords,
            &keyword_matches,
            &missing_keyword_details,
            profile.map(|profile| profile.profession),
        );
        let hard_constraint_risks =
            hard_constraints::build_hard_constraint_risks(&requirement_reviews);
        let score_cap = hard_constraint_risks
            .iter()
            .map(|risk| risk.score_cap)
            .min_by(f64::total_cmp);
        let mut overall_score = (keyword_score * 0.4)
            + (format_result.format_score * 0.3)
            + (format_result.completeness_score * 0.3);
        if let Some(cap) = score_cap {
            overall_score = overall_score.min(cap);
        }
        let keyword_matches = keyword_matches
            .into_iter()
            .map(|matched| matched.keyword_match)
            .collect();

        AtsAnalysisResult {
            overall_score,
            keyword_score,
            format_score: format_result.format_score,
            completeness_score: format_result.completeness_score,
            keyword_matches,
            missing_keywords,
            missing_keyword_details,
            format_issues: format_result.format_issues,
            requirement_reviews,
            hard_constraint_risks,
            suggestions,
            matching_profile: profile,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "ats_analyzer_tests.rs"]
mod tests;
