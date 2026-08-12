//! AI Resume-Job Matcher
//!
//! Automatically parse resumes, extract skills, and match against job postings.
//!
//! ## Features
//!
//! - **Local Resume Parsing** - Extract text from PDF, DOCX, TXT, Markdown, and HTML resumes
//! - **Skill Extraction** - Identify technical, workplace, and role-specific skills
//! - **Semantic Matching** - Compare resume skills against job requirements
//! - **Gap Analysis** - Identify missing skills and strengths
//! - **Application-readable Templates** - 5 professional resume templates
//! - **Resume Builder** - Interactive resume creation with CRUD operations
//! - **Resume Readability Analyzer** - job-word extraction and format clarity checks
//!
//! ## Usage
//!
//! ```rust,ignore
//! use jobsentinel_core::resume::ResumeMatcher;
//!
//! let matcher = ResumeMatcher::new(db_pool);
//!
//! // Upload and parse resume
//! let resume_id = matcher.upload_resume("My Resume.docx", "/path/to/resume.docx").await?;
//!
//! // Extract skills automatically
//! let skills = matcher.extract_skills(resume_id).await?;
//!
//! // Match against a job
//! let match_result = matcher.match_resume_to_job(resume_id, "job_hash_123").await?;
//! println!("Match score: {}%", match_result.overall_match_score * 100.0);
//! println!("Missing skills: {:?}", match_result.missing_skills);
//! ```

use crate::sqlite_time::parse_sqlite_datetime;
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::path::Path;

mod builder;
mod json_import;
mod json_resume;
mod management;
mod matcher;
mod skill_store;

use jobsentinel_documents::{ResumeParser, SkillExtractor};
use matcher::JobMatcher;

pub(crate) mod skills {
    pub(crate) use jobsentinel_documents::SkillExtractor;
}

pub(crate) mod types {
    #[cfg(test)]
    pub(crate) use jobsentinel_documents::UserSkill;
    pub(crate) use jobsentinel_documents::{
        DegreeLevel, EducationRequirement, ExperienceRequirement, NewSkill, NullableFieldUpdate,
        SkillUpdate,
    };
}

pub use builder::{DraftEducation, DraftExperience, DraftSkill, ResumeBuilder, ResumeDraft};

pub use jobsentinel_documents::{
    AtsAnalysisResult, AtsAnalyzer, AtsSuggestion, DegreeLevel, EducationRequirement,
    ExperienceRequirement, FormatIssue, HardConstraintCategory, HardConstraintRisk, IssueSeverity,
    JobSkill, KeywordImportance, KeywordMatch, MatchResult, MatchResultWithJob, MissingKeyword,
    NewSkill, ProfessionMatchingProfile, RegionalMatchingProfile, RequirementMatchState,
    RequirementReview, Resume, ResumeAnalysisInput, ResumeCertification, ResumeEducation,
    ResumeEvidenceSnapshot, ResumeExperience, ResumeExporter, ResumeMatchFeedback,
    ResumeMatchFeedbackLabel, ResumeMatchingProfile, ResumePersonalInfo, ResumeProject,
    ResumeSkill, ResumeSkillCategory, SkillUpdate, StructuredResume, SuggestionCategory, Template,
    TemplateId, TemplateRenderer, UserSkill,
};

/// Main resume matcher service
pub struct ResumeMatcher {
    db: SqlitePool,
    parser: ResumeParser,
    skill_extractor: SkillExtractor,
    job_matcher: JobMatcher,
}

impl ResumeMatcher {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            parser: ResumeParser::new(),
            skill_extractor: SkillExtractor::new(),
            job_matcher: JobMatcher::new(db.clone()),
            db,
        }
    }

    /// Upload and parse a new resume
    pub async fn upload_resume(&self, name: &str, file_path: &str) -> Result<i64> {
        // Parse local resume file to extract readable text.
        let parsed_text = self.parser.parse_resume(Path::new(file_path))?;

        // Insert into database
        let result = sqlx::query(
            r#"
            INSERT INTO resumes (name, file_path, parsed_text, is_active)
            VALUES (?, ?, ?, 1)
            "#,
        )
        .bind(name)
        .bind(file_path)
        .bind(&parsed_text)
        .execute(&self.db)
        .await?;

        let resume_id = result.last_insert_rowid();

        // Extract skills automatically
        self.extract_skills(resume_id).await?;

        Ok(resume_id)
    }

    /// Get resume by ID
    pub async fn get_resume(&self, resume_id: i64) -> Result<Resume> {
        let row = sqlx::query(
            r#"
            SELECT id, name, file_path, parsed_text, is_active, created_at, updated_at
            FROM resumes
            WHERE id = ?
            "#,
        )
        .bind(resume_id)
        .fetch_one(&self.db)
        .await?;

        let created_str = row.try_get::<String, _>("created_at")?;
        let updated_str = row.try_get::<String, _>("updated_at")?;

        let created_at = parse_sqlite_datetime(&created_str)?;
        let updated_at = parse_sqlite_datetime(&updated_str)?;

        Ok(Resume {
            id: row.try_get::<i64, _>("id")?,
            name: row.try_get::<String, _>("name")?,
            file_path: row.try_get::<String, _>("file_path")?,
            parsed_text: row.try_get::<Option<String>, _>("parsed_text")?,
            is_active: row.try_get::<i64, _>("is_active")? != 0,
            created_at,
            updated_at,
        })
    }

    /// Read the reproducible identity of a saved resume without loading its contents.
    pub async fn get_resume_evidence_snapshot(
        &self,
        resume_id: i64,
    ) -> Result<Option<ResumeEvidenceSnapshot>> {
        let revision = sqlx::query_scalar::<_, String>(
            "SELECT updated_at
             FROM resumes
             WHERE id = ?",
        )
        .bind(resume_id)
        .fetch_optional(&self.db)
        .await?;

        revision
            .map(|revision| {
                Ok(ResumeEvidenceSnapshot {
                    source_id: format!("resume:{resume_id}"),
                    revision: parse_sqlite_datetime(&revision)?.to_rfc3339(),
                })
            })
            .transpose()
    }

    /// Get active resume (most recently created)
    pub async fn get_active_resume(&self) -> Result<Option<Resume>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, file_path, parsed_text, is_active, created_at, updated_at
            FROM resumes
            WHERE is_active = 1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let created_str = r.try_get::<String, _>("created_at")?;
                let updated_str = r.try_get::<String, _>("updated_at")?;

                let created_at = parse_sqlite_datetime(&created_str)?;
                let updated_at = parse_sqlite_datetime(&updated_str)?;

                Ok(Some(Resume {
                    id: r.try_get::<i64, _>("id")?,
                    name: r.try_get::<String, _>("name")?,
                    file_path: r.try_get::<String, _>("file_path")?,
                    parsed_text: r.try_get::<Option<String>, _>("parsed_text")?,
                    is_active: r.try_get::<i64, _>("is_active")? != 0,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Extract skills from resume
    pub async fn extract_skills(&self, resume_id: i64) -> Result<Vec<UserSkill>> {
        // Get resume text
        let resume = self.get_resume(resume_id).await?;
        let text = resume.parsed_text.unwrap_or_default();

        // Extract skills using keyword-based approach
        let extracted_skills = self.skill_extractor.extract_skills(&text);
        let mut transaction = self.db.begin().await?;

        // Insert skills into database
        for skill in &extracted_skills {
            sqlx::query(
                r#"
                INSERT INTO user_skills (resume_id, skill_name, skill_category, confidence_score, source)
                VALUES (?, ?, ?, ?, 'resume')
                ON CONFLICT(resume_id, skill_name) DO UPDATE SET
                    skill_category = excluded.skill_category,
                    confidence_score = excluded.confidence_score
                "#,
            )
            .bind(resume_id)
            .bind(&skill.skill_name)
            .bind(&skill.skill_category)
            .bind(skill.confidence_score)
            .execute(&mut *transaction)
            .await?;
        }
        skill_store::advance_resume_snapshot(&mut transaction, resume_id).await?;
        transaction.commit().await?;

        // Fetch inserted skills
        self.get_user_skills(resume_id).await
    }

    /// Get all skills for a resume
    pub async fn get_user_skills(&self, resume_id: i64) -> Result<Vec<UserSkill>> {
        skill_store::query_user_skills(&self.db, resume_id).await
    }

    /// Match resume against a job
    pub async fn match_resume_to_job(&self, resume_id: i64, job_hash: &str) -> Result<MatchResult> {
        // Extract job skills if not already done
        self.job_matcher.extract_job_skills(job_hash).await?;

        // Perform matching
        let match_result = self
            .job_matcher
            .calculate_match(resume_id, job_hash)
            .await?;

        // Store match result
        let result = sqlx::query(
            r#"
            INSERT INTO resume_job_matches (
                resume_id, job_hash, overall_match_score, skills_match_score,
                missing_skills, matching_skills, gap_analysis
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(resume_id, job_hash) DO UPDATE SET
                overall_match_score = excluded.overall_match_score,
                skills_match_score = excluded.skills_match_score,
                missing_skills = excluded.missing_skills,
                matching_skills = excluded.matching_skills,
                gap_analysis = excluded.gap_analysis
            "#,
        )
        .bind(resume_id)
        .bind(job_hash)
        .bind(match_result.overall_match_score)
        .bind(match_result.skills_match_score)
        .bind(serde_json::to_string(&match_result.missing_skills)?)
        .bind(serde_json::to_string(&match_result.matching_skills)?)
        .bind(&match_result.gap_analysis)
        .execute(&self.db)
        .await?;

        let match_id = result.last_insert_rowid();

        // Return with ID
        Ok(MatchResult {
            id: match_id,
            ..match_result
        })
    }

    /// Get match result for a resume-job pair
    pub async fn get_match_result(
        &self,
        resume_id: i64,
        job_hash: &str,
    ) -> Result<Option<MatchResult>> {
        let row = sqlx::query(
            r#"
            SELECT id, resume_id, job_hash, overall_match_score, skills_match_score,
                   experience_match_score, education_match_score, missing_skills,
                   matching_skills, gap_analysis, created_at
            FROM resume_job_matches
            WHERE resume_id = ? AND job_hash = ?
            "#,
        )
        .bind(resume_id)
        .bind(job_hash)
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let created_str = r.try_get::<String, _>("created_at")?;

                let created_at = parse_sqlite_datetime(&created_str)?;

                // Handle missing_skills and matching_skills JSON with proper NULL handling
                let missing_skills_str = r
                    .try_get::<Option<String>, _>("missing_skills")
                    .unwrap_or(None)
                    .unwrap_or_else(|| "[]".to_string());
                let matching_skills_str = r
                    .try_get::<Option<String>, _>("matching_skills")
                    .unwrap_or(None)
                    .unwrap_or_else(|| "[]".to_string());

                Ok(Some(MatchResult {
                    id: r.try_get::<i64, _>("id")?,
                    resume_id: r.try_get::<i64, _>("resume_id")?,
                    job_hash: r.try_get::<String, _>("job_hash")?,
                    overall_match_score: r.try_get::<f64, _>("overall_match_score")?,
                    skills_match_score: r.try_get::<Option<f64>, _>("skills_match_score")?,
                    experience_match_score: r
                        .try_get::<Option<f64>, _>("experience_match_score")?,
                    education_match_score: r.try_get::<Option<f64>, _>("education_match_score")?,
                    missing_skills: serde_json::from_str(&missing_skills_str)?,
                    matching_skills: serde_json::from_str(&matching_skills_str)?,
                    gap_analysis: r.try_get::<Option<String>, _>("gap_analysis")?,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests;
