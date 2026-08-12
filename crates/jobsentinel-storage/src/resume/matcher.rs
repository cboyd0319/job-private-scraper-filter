//! Resume-Job Matching Algorithm
//!
//! Compares resume skills against job requirements and generates match scores.
#![allow(clippy::unwrap_used, clippy::expect_used)] // Regex patterns are compile-time constants

use super::skills::SkillExtractor;
use super::types::{DegreeLevel, EducationRequirement, ExperienceRequirement};
use super::{MatchResult, UserSkill};
use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use sqlx::{Row, SqlitePool};
use std::collections::HashSet;

mod gap_analysis;
mod queries;

struct JobInfo {
    title: String,
    description: String,
}

pub(super) struct JobMatcher {
    db: SqlitePool,
    skill_extractor: SkillExtractor,
}

impl JobMatcher {
    pub(super) fn new(db: SqlitePool) -> Self {
        Self {
            db,
            skill_extractor: SkillExtractor::new(),
        }
    }

    /// Extract skills from job description and store in database
    pub(super) async fn extract_job_skills(&self, job_hash: &str) -> Result<Vec<String>> {
        // Get job details
        let job = self.get_job(job_hash).await?;

        // Extract skills from description
        let job_text = format!("{} {}", job.title, job.description);
        let extracted_skills = self.skill_extractor.extract_skills(&job_text);

        // Insert skills into database
        for skill in &extracted_skills {
            sqlx::query(
                r#"
                INSERT INTO job_skills (job_hash, skill_name, is_required, skill_category)
                VALUES (?, ?, 1, ?)
                ON CONFLICT(job_hash, skill_name) DO UPDATE SET
                    skill_category = excluded.skill_category
                "#,
            )
            .bind(job_hash)
            .bind(&skill.skill_name)
            .bind(&skill.skill_category)
            .execute(&self.db)
            .await?;
        }

        Ok(extracted_skills
            .iter()
            .map(|s| s.skill_name.clone())
            .collect())
    }

    /// Extract experience requirements from job description
    /// Patterns: "5+ years Python", "3-5 years experience", "Senior (7+ years)"
    pub(super) fn extract_experience_requirements(&self, text: &str) -> Vec<ExperienceRequirement> {
        let mut requirements = Vec::new();
        let lower = text.to_lowercase();

        // Pattern 1: "X+ years [of] [SKILL/experience]" - e.g., "5+ years of Python"
        let pattern1 =
            Regex::new(r"(\d+)\+?\s*(?:years?|yrs?)\s+(?:of\s+)?(?:experience\s+(?:with|in)\s+)?([a-zA-Z][a-zA-Z0-9+#/.]*(?:\s+[a-zA-Z][a-zA-Z0-9+#/.]*)?)")
                .unwrap();

        // Pattern 2: "X-Y years [of] experience/SKILL" - e.g., "3-5 years experience"
        let pattern2 = Regex::new(
            r"(\d+)\s*[-–]\s*(\d+)\s*(?:years?|yrs?)\s+(?:of\s+)?([a-zA-Z][a-zA-Z0-9+#/.\s]*)?",
        )
        .unwrap();

        // Pattern 3: Level indicators - "Senior", "Mid-level", etc.
        let seniority_patterns = [
            (
                r"(?i)\bsenior\b|\bsr\.\b|\blead\b|\bprincipal\b|\bstaff\b",
                5.0,
            ),
            (r"(?i)\bmid[- ]?level\b|\bintermediate\b", 3.0),
            (r"(?i)\bjunior\b|\bjr\.\b|\bentry[- ]?level\b", 0.0),
        ];

        // Extract from pattern 1
        for cap in pattern1.captures_iter(text) {
            let years: f64 = cap[1].parse().unwrap_or(0.0);
            let skill_text = cap[2].trim().to_string();

            // Skip generic words
            if !["experience", "relevant", "professional", "industry"]
                .contains(&skill_text.to_lowercase().as_str())
            {
                requirements.push(ExperienceRequirement {
                    skill: Some(skill_text),
                    min_years: years,
                    max_years: None,
                    is_required: !lower.contains("preferred") && !lower.contains("nice to have"),
                });
            }
        }

        // Extract from pattern 2 (ranges)
        for cap in pattern2.captures_iter(text) {
            let min_years: f64 = cap[1].parse().unwrap_or(0.0);
            let max_years: f64 = cap[2].parse().unwrap_or(0.0);
            let skill_text = cap.get(3).map(|m| m.as_str().trim().to_string());

            // Check if skill is generic "experience" or specific
            let skill = match skill_text {
                Some(s)
                    if !["experience", "relevant", "professional"]
                        .contains(&s.to_lowercase().as_str()) =>
                {
                    Some(s)
                }
                _ => None,
            };

            requirements.push(ExperienceRequirement {
                skill,
                min_years,
                max_years: Some(max_years),
                is_required: true,
            });
        }

        // Check for seniority indicators if no explicit years found
        if requirements.is_empty() {
            for (pattern, years) in seniority_patterns {
                if Regex::new(pattern).unwrap().is_match(text) {
                    requirements.push(ExperienceRequirement {
                        skill: None,
                        min_years: years,
                        max_years: None,
                        is_required: true,
                    });
                    break;
                }
            }
        }

        requirements
    }

    /// Extract education requirements from job description
    pub(super) fn extract_education_requirements(
        &self,
        text: &str,
    ) -> Option<EducationRequirement> {
        let lower = text.to_lowercase();

        // Check if education is explicitly NOT required
        if lower.contains("no degree required")
            || lower.contains("degree not required")
            || lower.contains("no formal education")
        {
            return None;
        }

        // Look for degree mentions with context
        let degree_patterns = [
            // PhD patterns
            (
                r"(?i)(?:\bph\.?d\b|\bdoctorate\b)\s*(?:degree)?(?:\s+in\s+([a-zA-Z\s,]+))?",
                DegreeLevel::PhD,
            ),
            // Master's patterns
            (
                r"(?i)(?:\bmaster'?s?\b|\bm\.?s\.?(?=\s|$)|\bm\.?a\.?(?=\s|$)|\bmba\b)\s*(?:degree)?(?:\s+in\s+([a-zA-Z\s,]+))?",
                DegreeLevel::Master,
            ),
            // Bachelor's patterns
            (
                r"(?i)(?:\bbachelor'?s?\b|\bb\.?s\.?(?=\s|$)|\bb\.?a\.?(?=\s|$)|\bundergraduate\b)\s*(?:degree)?(?:\s+in\s+([a-zA-Z\s,]+))?",
                DegreeLevel::Bachelor,
            ),
            // Associate patterns
            (
                r"(?i)(?:\bassociate'?s?\b|\ba\.?s\.?(?=\s|$)|\ba\.?a\.?(?=\s|$))\s*(?:degree)?(?:\s+in\s+([a-zA-Z\s,]+))?",
                DegreeLevel::Associate,
            ),
        ];

        let mut best_match: Option<(DegreeLevel, Vec<String>)> = None;

        for (pattern, level) in degree_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(cap) = re.captures(text) {
                    // Extract fields of study if present
                    let fields: Vec<String> = cap
                        .get(1)
                        .map(|m| {
                            m.as_str()
                                .split(|c| c == ',' || c == '/')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect()
                        })
                        .unwrap_or_default();

                    // Take highest degree level found
                    if best_match.is_none()
                        || level
                            > best_match
                                .as_ref()
                                .map(|(l, _)| *l)
                                .unwrap_or(DegreeLevel::None)
                    {
                        best_match = Some((level, fields));
                    }
                }
            }
        }

        // Also check for "degree required" without specific type (assume Bachelor's)
        if best_match.is_none()
            && (lower.contains("degree required") || lower.contains("college degree"))
        {
            best_match = Some((DegreeLevel::Bachelor, vec![]));
        }

        // Determine if required or preferred
        let is_required = !lower.contains("preferred")
            && !lower.contains("or equivalent")
            && !lower.contains("nice to have");

        best_match.map(|(level, fields)| EducationRequirement {
            degree_level: level,
            fields,
            is_required,
        })
    }

    /// Calculate experience match score
    fn calculate_experience_match(
        &self,
        user_skills: &[UserSkill],
        requirements: &[ExperienceRequirement],
    ) -> f64 {
        if requirements.is_empty() {
            return 1.0; // No requirements = full match
        }

        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for req in requirements {
            let weight = if req.is_required { 1.0 } else { 0.5 };
            total_weight += weight;

            // Find matching user skill
            let user_years = if let Some(skill_name) = &req.skill {
                // Look for specific skill
                user_skills
                    .iter()
                    .find(|s| s.skill_name.to_lowercase() == skill_name.to_lowercase())
                    .and_then(|s| s.years_experience)
                    .unwrap_or(0.0)
            } else {
                // General experience - use max years from any skill
                user_skills
                    .iter()
                    .filter_map(|s| s.years_experience)
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            };

            // Calculate score for this requirement
            let score = if user_years >= req.min_years {
                1.0 // Meets or exceeds requirement
            } else if user_years > 0.0 {
                user_years / req.min_years // Partial credit
            } else {
                0.0 // No experience
            };

            total_score += score * weight;
        }

        if total_weight > 0.0 {
            total_score / total_weight
        } else {
            1.0
        }
    }

    /// Calculate education match score
    fn calculate_education_match(
        &self,
        user_education: Option<DegreeLevel>,
        requirement: Option<&EducationRequirement>,
    ) -> f64 {
        match (user_education, requirement) {
            (_, None) => 1.0,       // No requirement = full match
            (None, Some(_)) => 0.0, // Requirement but no user education
            (Some(user), Some(req)) => {
                if user >= req.degree_level {
                    1.0 // Meets or exceeds
                } else {
                    // Partial credit based on level difference
                    let user_level = user as i32;
                    let req_level = req.degree_level as i32;
                    if req_level > 0 {
                        (user_level as f64) / (req_level as f64)
                    } else {
                        1.0
                    }
                }
            }
        }
    }

    /// Calculate match between resume and job
    pub(super) async fn calculate_match(
        &self,
        resume_id: i64,
        job_hash: &str,
    ) -> Result<MatchResult> {
        // Get job details for experience/education extraction
        let job = self.get_job(job_hash).await?;
        let job_text = format!("{} {}", job.title, job.description);

        // Get user skills
        let user_skills = self.get_user_skills(resume_id).await?;
        let user_skill_names: HashSet<String> = user_skills
            .iter()
            .map(|s| s.skill_name.to_lowercase())
            .collect();

        // Get job skills
        let job_skills = self.get_job_skills(job_hash).await?;
        let job_skill_names: HashSet<String> =
            job_skills.iter().map(|s| s.to_lowercase()).collect();

        // Calculate matching skills
        let matching_skills: Vec<String> = job_skill_names
            .intersection(&user_skill_names)
            .map(|s| {
                // Find original casing from job_skills
                job_skills
                    .iter()
                    .find(|js| js.to_lowercase() == *s)
                    .unwrap_or(s)
                    .clone()
            })
            .collect();

        // Calculate missing skills
        let missing_skills: Vec<String> = job_skill_names
            .difference(&user_skill_names)
            .map(|s| {
                job_skills
                    .iter()
                    .find(|js| js.to_lowercase() == *s)
                    .unwrap_or(s)
                    .clone()
            })
            .collect();

        // Calculate skills match score
        let skills_match_score = if !job_skill_names.is_empty() {
            matching_skills.len() as f64 / job_skill_names.len() as f64
        } else {
            0.0 // No recognized job skills means insufficient evidence, not a perfect match.
        };

        // Extract and calculate experience match
        let experience_reqs = self.extract_experience_requirements(&job_text);
        let experience_match_score =
            self.calculate_experience_match(&user_skills, &experience_reqs);

        // Extract and calculate education match
        // For now, we'll try to detect user's education from their resume text
        let education_req = self.extract_education_requirements(&job_text);
        let user_education = if education_req.is_some() {
            self.get_user_education(resume_id).await?
        } else {
            None
        };
        let education_match_score =
            self.calculate_education_match(user_education, education_req.as_ref());

        let match_score = jobsentinel_documents::calculate_resume_match_score(
            &matching_skills,
            &missing_skills,
            skills_match_score,
            experience_match_score,
            &experience_reqs,
            education_match_score,
            education_req.as_ref(),
        );
        let overall_match_score = match_score.score;

        // Generate enhanced gap analysis
        let gap_analysis = gap_analysis::generate_enhanced_gap_analysis(
            &matching_skills,
            &missing_skills,
            skills_match_score,
            experience_match_score,
            &experience_reqs,
            education_match_score,
            education_req.as_ref(),
            overall_match_score,
            match_score.blocker.as_deref(),
            &match_score.sources,
        );

        Ok(MatchResult {
            id: 0, // Will be set by caller
            resume_id,
            job_hash: job_hash.to_string(),
            overall_match_score,
            skills_match_score: Some(skills_match_score),
            experience_match_score: Some(experience_match_score),
            education_match_score: Some(education_match_score),
            missing_skills,
            matching_skills,
            gap_analysis: Some(gap_analysis),
            created_at: Utc::now(),
        })
    }

    /// Generate human-readable gap analysis (legacy - kept for backward compatibility)
    #[cfg(test)]
    fn generate_gap_analysis(
        &self,
        matching_skills: &[String],
        missing_skills: &[String],
        overall_score: f64,
    ) -> String {
        gap_analysis::generate_gap_analysis(matching_skills, missing_skills, overall_score)
    }
}

#[cfg(test)]
#[path = "matcher_tests.rs"]
mod tests;
