//! Application Profile Management
//!
//! Manages user profile information for candidate-controlled application help.

use crate::sqlite_time::parse_sqlite_datetime;
use anyhow::Result;
use jobsentinel_domain::{
    requires_user_answer, screening_question_matches, ApplicationProfile, ApplicationProfileInput,
    ScreeningAnswer,
};
use sqlx::SqlitePool;

/// Profile manager
#[derive(Debug)]
pub struct ProfileManager {
    db: SqlitePool,
}

fn normalize_screening_answer_type(answer_type: &str) -> Result<&'static str> {
    match answer_type.trim().to_ascii_lowercase().as_str() {
        "text" | "number" => Ok("text"),
        "yes_no" | "boolean" => Ok("yes_no"),
        "textarea" => Ok("textarea"),
        "select" | "multiple_choice" => Ok("select"),
        other => anyhow::bail!("invalid screening answer type: {other}"),
    }
}

impl ProfileManager {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Create or update application profile
    pub async fn upsert_profile(&self, profile: &ApplicationProfileInput) -> Result<i64> {
        // Check if profile exists
        let existing = sqlx::query_scalar::<_, i64>("SELECT id FROM application_profile LIMIT 1")
            .fetch_optional(&self.db)
            .await?;

        if let Some(id) = existing {
            // Update existing
            sqlx::query(
                r#"
                UPDATE application_profile
                SET full_name = ?, email = ?, phone = ?, linkedin_url = ?,
                    github_url = ?, portfolio_url = ?, website_url = ?,
                    default_resume_id = ?,
                    resume_file_path = CASE
                        WHEN ? != 0 THEN NULL
                        WHEN ? != 0 THEN ?
                        ELSE resume_file_path
                    END,
                    default_cover_letter_template = ?,
                    us_work_authorized = ?, requires_sponsorship = ?,
                    max_applications_per_day = ?, require_manual_approval = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(&profile.full_name)
            .bind(&profile.email)
            .bind(&profile.phone)
            .bind(&profile.linkedin_url)
            .bind(&profile.github_url)
            .bind(&profile.portfolio_url)
            .bind(&profile.website_url)
            .bind(profile.default_resume_id)
            .bind(profile.clear_resume_file.unwrap_or(false) as i32)
            .bind(profile.resume_file_path.is_some() as i32)
            .bind(&profile.resume_file_path)
            .bind(&profile.default_cover_letter_template)
            .bind(profile.us_work_authorized as i32)
            .bind(profile.requires_sponsorship as i32)
            .bind(profile.max_applications_per_day)
            .bind(profile.require_manual_approval as i32)
            .bind(id)
            .execute(&self.db)
            .await?;

            Ok(id)
        } else {
            // Insert new
            let result = sqlx::query(
                r#"
                INSERT INTO application_profile (
                    full_name, email, phone, linkedin_url, github_url,
                    portfolio_url, website_url, default_resume_id,
                    resume_file_path, default_cover_letter_template, us_work_authorized,
                    requires_sponsorship, max_applications_per_day,
                    require_manual_approval
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&profile.full_name)
            .bind(&profile.email)
            .bind(&profile.phone)
            .bind(&profile.linkedin_url)
            .bind(&profile.github_url)
            .bind(&profile.portfolio_url)
            .bind(&profile.website_url)
            .bind(profile.default_resume_id)
            .bind(&profile.resume_file_path)
            .bind(&profile.default_cover_letter_template)
            .bind(profile.us_work_authorized as i32)
            .bind(profile.requires_sponsorship as i32)
            .bind(profile.max_applications_per_day)
            .bind(profile.require_manual_approval as i32)
            .execute(&self.db)
            .await?;

            Ok(result.last_insert_rowid())
        }
    }

    /// Get application profile
    pub async fn get_profile(&self) -> Result<Option<ApplicationProfile>> {
        let row = sqlx::query(
            r#"
            SELECT id, full_name, email, phone, linkedin_url, github_url,
                   portfolio_url, website_url, default_resume_id,
                   resume_file_path, default_cover_letter_template, us_work_authorized,
                   requires_sponsorship, max_applications_per_day,
                   require_manual_approval, created_at, updated_at
            FROM application_profile
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.db)
        .await?;

        use sqlx::Row;
        match row {
            Some(r) => {
                let created_at: String = r.get("created_at");
                let updated_at: String = r.get("updated_at");
                Ok(Some(ApplicationProfile {
                    id: r.get("id"),
                    full_name: r.get("full_name"),
                    email: r.get("email"),
                    phone: r.get("phone"),
                    linkedin_url: r.get("linkedin_url"),
                    github_url: r.get("github_url"),
                    portfolio_url: r.get("portfolio_url"),
                    website_url: r.get("website_url"),
                    default_resume_id: r.get("default_resume_id"),
                    resume_file_path: r.get("resume_file_path"),
                    default_cover_letter_template: r.get("default_cover_letter_template"),
                    us_work_authorized: r.get::<i32, _>("us_work_authorized") != 0,
                    requires_sponsorship: r.get::<i32, _>("requires_sponsorship") != 0,
                    max_applications_per_day: r.get("max_applications_per_day"),
                    require_manual_approval: r.get::<i32, _>("require_manual_approval") != 0,
                    created_at: parse_sqlite_datetime(&created_at)?,
                    updated_at: parse_sqlite_datetime(&updated_at)?,
                }))
            }
            None => Ok(None),
        }
    }

    /// Check whether an application profile exists without returning private profile data.
    pub async fn has_profile(&self) -> Result<bool> {
        let profile_id = sqlx::query_scalar::<_, i64>("SELECT id FROM application_profile LIMIT 1")
            .fetch_optional(&self.db)
            .await?;

        Ok(profile_id.is_some())
    }

    /// Add or update screening answer
    pub async fn upsert_screening_answer(
        &self,
        question_pattern: &str,
        answer: &str,
        answer_type: &str,
        notes: Option<&str>,
    ) -> Result<()> {
        let answer_type = normalize_screening_answer_type(answer_type)?;

        sqlx::query(
            r#"
            INSERT INTO screening_answers (question_pattern, answer, answer_type, notes)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(question_pattern) DO UPDATE SET
                answer = excluded.answer,
                answer_type = excluded.answer_type,
                notes = excluded.notes,
                times_modified = times_modified + 1,
                confidence_score = 1.0,
                updated_at = datetime('now')
            "#,
        )
        .bind(question_pattern)
        .bind(answer)
        .bind(answer_type)
        .bind(notes)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get all screening answers
    ///
    /// OPTIMIZATION: Added reasonable LIMIT (1000) to prevent unbounded result sets.
    /// Most users will have <100 screening patterns; 1000 is a safe upper bound.
    pub async fn get_screening_answers(&self) -> Result<Vec<ScreeningAnswer>> {
        let rows = sqlx::query(
            r#"
            SELECT id, question_pattern, answer, answer_type, notes,
                   times_used, times_modified, confidence_score, last_used_at,
                   created_at, updated_at
            FROM screening_answers
            ORDER BY created_at DESC
            LIMIT 1000
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        use sqlx::Row;
        rows.into_iter()
            .map(|r| {
                let created_at: String = r.get("created_at");
                let updated_at: String = r.get("updated_at");
                Ok(ScreeningAnswer {
                    id: r.get("id"),
                    question_pattern: r.get("question_pattern"),
                    answer: r.get("answer"),
                    answer_type: r.get("answer_type"),
                    notes: r.get("notes"),
                    times_used: r.get("times_used"),
                    times_modified: r.get("times_modified"),
                    confidence_score: r.get("confidence_score"),
                    last_used_at: r
                        .get::<Option<String>, _>("last_used_at")
                        .and_then(|date| parse_sqlite_datetime(&date).ok()),
                    created_at: parse_sqlite_datetime(&created_at)?,
                    updated_at: parse_sqlite_datetime(&updated_at)?,
                })
            })
            .collect()
    }

    /// Find matching screening answer for a question
    ///
    /// OPTIMIZATION: Fetch only pattern+answer columns instead of full rows.
    /// Reduces data transfer and memory allocation for pattern matching.
    pub async fn find_answer_for_question(&self, question: &str) -> Result<Option<String>> {
        if requires_user_answer(question) {
            return Ok(None);
        }

        // Fetch only needed columns for pattern matching
        let rows = sqlx::query(
            "SELECT question_pattern, answer FROM screening_answers ORDER BY created_at DESC",
        )
        .fetch_all(&self.db)
        .await?;

        use sqlx::Row;
        for row in rows {
            let pattern: String = row.try_get("question_pattern")?;
            let answer: String = row.try_get("answer")?;

            if requires_user_answer(&pattern) {
                continue;
            }

            if screening_question_matches(&pattern, question) {
                return Ok(Some(answer));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests;
