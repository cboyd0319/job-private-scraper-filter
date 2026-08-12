use anyhow::Result;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use super::{types, ResumeMatcher, UserSkill};
use crate::v3_vectors::resume_vector_subject;
use types::NullableFieldUpdate;

pub(super) async fn query_user_skills(db: &SqlitePool, resume_id: i64) -> Result<Vec<UserSkill>> {
    let rows = sqlx::query(
        r#"
        SELECT id, resume_id, skill_name, skill_category, confidence_score,
               years_experience, proficiency_level, source
        FROM user_skills
        WHERE resume_id = ?
        ORDER BY confidence_score DESC, skill_name ASC
        "#,
    )
    .bind(resume_id)
    .fetch_all(db)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(UserSkill {
                id: row.try_get("id")?,
                resume_id: row.try_get("resume_id")?,
                skill_name: row.try_get("skill_name")?,
                skill_category: row.try_get("skill_category")?,
                confidence_score: row.try_get("confidence_score")?,
                years_experience: row.try_get("years_experience")?,
                proficiency_level: row.try_get("proficiency_level")?,
                source: row.try_get("source")?,
            })
        })
        .collect()
}

impl ResumeMatcher {
    /// Return the normalized skill names stored for a job.
    pub async fn get_job_skill_names(&self, job_hash: &str) -> Result<Vec<String>> {
        self.job_matcher.get_job_skills(job_hash).await
    }

    /// Update an existing user skill
    pub async fn update_user_skill(
        &self,
        skill_id: i64,
        updates: types::SkillUpdate,
    ) -> Result<()> {
        if updates.skill_name.is_none()
            && updates.skill_category.is_unset()
            && updates.proficiency_level.is_unset()
            && updates.years_experience.is_unset()
        {
            return Ok(());
        }

        let mut transaction = self.db.begin().await?;
        let resume_id = require_user_skill_resume_id(&mut transaction, skill_id).await?;

        if let Some(name) = updates.skill_name {
            let name = normalize_skill_name(&name)?;
            sqlx::query("UPDATE user_skills SET skill_name = ? WHERE id = ?")
                .bind(name)
                .bind(skill_id)
                .execute(&mut *transaction)
                .await?;
        }
        if !updates.skill_category.is_unset() {
            let category = match updates.skill_category {
                NullableFieldUpdate::Unset => unreachable!(),
                NullableFieldUpdate::Clear => None,
                NullableFieldUpdate::Set(category) => normalize_optional_skill_text(Some(category)),
            };
            sqlx::query("UPDATE user_skills SET skill_category = ? WHERE id = ?")
                .bind(category)
                .bind(skill_id)
                .execute(&mut *transaction)
                .await?;
        }
        if !updates.proficiency_level.is_unset() {
            let level = match updates.proficiency_level {
                NullableFieldUpdate::Unset => unreachable!(),
                NullableFieldUpdate::Clear => None,
                NullableFieldUpdate::Set(level) => normalize_optional_skill_text(Some(level)),
            };
            sqlx::query("UPDATE user_skills SET proficiency_level = ? WHERE id = ?")
                .bind(level)
                .bind(skill_id)
                .execute(&mut *transaction)
                .await?;
        }
        if !updates.years_experience.is_unset() {
            let years = match updates.years_experience {
                NullableFieldUpdate::Unset => unreachable!(),
                NullableFieldUpdate::Clear => None,
                NullableFieldUpdate::Set(years) => validate_skill_years(Some(years))?,
            };
            sqlx::query("UPDATE user_skills SET years_experience = ? WHERE id = ?")
                .bind(years)
                .bind(skill_id)
                .execute(&mut *transaction)
                .await?;
        }

        advance_resume_snapshot(&mut transaction, resume_id).await?;
        transaction.commit().await?;
        tracing::info!("Updated skill {}", skill_id);
        Ok(())
    }

    /// Delete a user skill
    pub async fn delete_user_skill(&self, skill_id: i64) -> Result<()> {
        let mut transaction = self.db.begin().await?;
        let resume_id = require_user_skill_resume_id(&mut transaction, skill_id).await?;
        let result = sqlx::query("DELETE FROM user_skills WHERE id = ?")
            .bind(skill_id)
            .execute(&mut *transaction)
            .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!("Skill with id {} not found", skill_id);
        }

        advance_resume_snapshot(&mut transaction, resume_id).await?;
        transaction.commit().await?;
        tracing::info!("Deleted skill {}", skill_id);
        Ok(())
    }

    /// Add a new skill manually
    pub async fn add_user_skill(&self, resume_id: i64, skill: types::NewSkill) -> Result<i64> {
        let skill_name = normalize_skill_name(&skill.skill_name)?;
        let skill_category = normalize_optional_skill_text(skill.skill_category);
        let proficiency_level = normalize_optional_skill_text(skill.proficiency_level);
        let years_experience = validate_skill_years(skill.years_experience)?;
        let mut transaction = self.db.begin().await?;

        let result = sqlx::query(
            r#"
            INSERT INTO user_skills (
                resume_id, skill_name, skill_category, confidence_score,
                proficiency_level, years_experience, source
            )
            VALUES (?, ?, ?, 1.0, ?, ?, 'user_input')
            "#,
        )
        .bind(resume_id)
        .bind(&skill_name)
        .bind(&skill_category)
        .bind(&proficiency_level)
        .bind(years_experience)
        .execute(&mut *transaction)
        .await?;

        let skill_id = result.last_insert_rowid();
        advance_resume_snapshot(&mut transaction, resume_id).await?;
        transaction.commit().await?;
        tracing::info!(
            resume_id,
            skill_id,
            skill_name_chars = skill_name.chars().count(),
            "Added manual skill"
        );

        Ok(skill_id)
    }
}

fn normalize_skill_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Skill name is required");
    }
    Ok(trimmed.to_string())
}

async fn require_user_skill_resume_id(
    transaction: &mut Transaction<'_, Sqlite>,
    skill_id: i64,
) -> Result<i64> {
    let row = sqlx::query_scalar("SELECT resume_id FROM user_skills WHERE id = ?")
        .bind(skill_id)
        .fetch_optional(&mut **transaction)
        .await?;

    row.ok_or_else(|| anyhow::anyhow!("Skill with id {} not found", skill_id))
}

pub(super) async fn advance_resume_snapshot(
    transaction: &mut Transaction<'_, Sqlite>,
    resume_id: i64,
) -> Result<()> {
    let result = sqlx::query(
        "UPDATE resumes
         SET updated_at = CASE
             WHEN updated_at >= strftime('%Y-%m-%d %H:%M:%f', 'now')
             THEN strftime('%Y-%m-%d %H:%M:%f', updated_at, '+0.001 seconds')
             ELSE strftime('%Y-%m-%d %H:%M:%f', 'now')
         END
         WHERE id = ?",
    )
    .bind(resume_id)
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() == 0 {
        anyhow::bail!("Resume with id {} not found", resume_id);
    }
    sqlx::query("DELETE FROM v3_local_vectors WHERE subject_id = ?")
        .bind(resume_vector_subject(resume_id)?)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn normalize_optional_skill_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn validate_skill_years(value: Option<f64>) -> Result<Option<f64>> {
    match value {
        Some(years) if !years.is_finite() => anyhow::bail!("Skill years must be a number"),
        Some(years) if !(0.0..=50.0).contains(&years) => {
            anyhow::bail!("Skill years must be between 0 and 50")
        }
        other => Ok(other),
    }
}

#[cfg(test)]
#[path = "skill_store_tests.rs"]
mod mutation_tests;

#[cfg(test)]
mod query_tests {
    use super::*;

    #[tokio::test]
    async fn user_skill_query_preserves_nullable_fields_and_tie_order() {
        let database = crate::Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let resume_id =
            sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
                .execute(database.pool())
                .await
                .unwrap()
                .last_insert_rowid();
        for skill_name in ["Zulu", "Alpha"] {
            sqlx::query(
                "INSERT INTO user_skills
                 (resume_id, skill_name, confidence_score, source)
                 VALUES (?, ?, 0.8, 'resume')",
            )
            .bind(resume_id)
            .bind(skill_name)
            .execute(database.pool())
            .await
            .unwrap();
        }

        let skills = query_user_skills(database.pool(), resume_id).await.unwrap();

        assert_eq!(skills[0].skill_name, "Alpha");
        assert!(skills[0].skill_category.is_none());
        assert!(skills[0].years_experience.is_none());
        assert!(skills[0].proficiency_level.is_none());
    }
}
