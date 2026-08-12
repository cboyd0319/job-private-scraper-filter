//! Interview scheduling and management

use super::tracker::ApplicationTracker;
use super::types::*;
use anyhow::Result;
use chrono::Utc;
use sqlx::{sqlite::SqliteRow, Row};

macro_rules! interview_with_job_query {
    ($criteria:literal) => {
        concat!(
            r#"
                SELECT
                    i.id,
                    i.application_id,
                    a.job_hash,
                    i.interview_type,
                    i.scheduled_at,
                    i.duration_minutes,
                    i.location,
                    i.interviewer_name,
                    i.interviewer_title,
                    i.notes,
                    i.completed,
                    i.outcome,
                    i.post_interview_notes,
                    j.title as job_title,
                    j.company
                FROM interviews i
                JOIN applications a ON i.application_id = a.id
                JOIN jobs j ON a.job_hash = j.hash
            "#,
            $criteria
        )
    };
}

fn interview_with_job_from_row(row: SqliteRow) -> Result<InterviewWithJob> {
    Ok(InterviewWithJob {
        id: row.try_get("id")?,
        application_id: row.try_get("application_id")?,
        job_hash: row.try_get("job_hash")?,
        interview_type: row
            .try_get::<Option<String>, _>("interview_type")?
            .unwrap_or_else(|| "other".to_string()),
        scheduled_at: row.try_get("scheduled_at")?,
        duration_minutes: row
            .try_get::<Option<i32>, _>("duration_minutes")?
            .unwrap_or(60),
        location: row.try_get("location")?,
        interviewer_name: row.try_get("interviewer_name")?,
        interviewer_title: row.try_get("interviewer_title")?,
        notes: row.try_get("notes")?,
        completed: row.try_get::<i64, _>("completed")? != 0,
        outcome: row.try_get("outcome")?,
        post_interview_notes: row.try_get("post_interview_notes")?,
        job_title: row.try_get("job_title")?,
        company: row.try_get("company")?,
    })
}

impl ApplicationTracker {
    /// Schedule a new interview
    pub async fn schedule_interview(
        &self,
        application_id: i64,
        interview_type: &str,
        scheduled_at: &str,
        duration_minutes: i32,
        location: Option<&str>,
        interviewer_name: Option<&str>,
        interviewer_title: Option<&str>,
        notes: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO interviews (
                application_id, interview_type, scheduled_at, duration_minutes,
                location, interviewer_name, interviewer_title, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            application_id,
            interview_type,
            scheduled_at,
            duration_minutes,
            location,
            interviewer_name,
            interviewer_title,
            notes
        )
        .execute(&self.db)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get incomplete interviews, including overdue debriefs and the next 30 days.
    pub async fn get_upcoming_interviews(&self) -> Result<Vec<InterviewWithJob>> {
        let interviews = sqlx::query(interview_with_job_query!(
            r#"
                WHERE i.completed = 0
                  AND datetime(i.scheduled_at) <= datetime('now', '+30 days')
                ORDER BY ABS(julianday(i.scheduled_at) - julianday('now')) ASC
            "#
        ))
        .fetch_all(&self.db)
        .await?;

        interviews
            .into_iter()
            .map(interview_with_job_from_row)
            .collect()
    }

    /// Get completed interviews.
    pub async fn get_past_interviews(&self) -> Result<Vec<InterviewWithJob>> {
        let interviews = sqlx::query(interview_with_job_query!(
            r#"
                WHERE i.completed = 1
                ORDER BY i.scheduled_at DESC
            "#
        ))
        .fetch_all(&self.db)
        .await?;

        interviews
            .into_iter()
            .map(interview_with_job_from_row)
            .collect()
    }

    /// Update interview outcome with optional post-interview notes
    ///
    /// Note: post_notes is stored in the post_interview_notes column added via migration.
    /// The column exists in the DB but SQLx compile-time checking requires schema regeneration.
    pub async fn complete_interview(
        &self,
        interview_id: i64,
        outcome: &str,
        post_notes: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE interviews
            SET completed = 1,
                outcome = ?,
                post_interview_notes = COALESCE(?, post_interview_notes),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(outcome)
        .bind(post_notes)
        .bind(now)
        .bind(interview_id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Delete an interview
    pub async fn delete_interview(&self, interview_id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM interviews WHERE id = ?", interview_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod row_mapper_tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn interview_mapper_applies_nullable_column_defaults() {
        let database = crate::Database::connect_memory().await.unwrap();
        let row = sqlx::query(
            r#"
            SELECT 3 AS id, 9 AS application_id, 'job-9' AS job_hash,
                   NULL AS interview_type,
                   '2026-02-01 10:00:00' AS scheduled_at,
                   NULL AS duration_minutes, NULL AS location,
                   NULL AS interviewer_name, NULL AS interviewer_title,
                   NULL AS notes, 0 AS completed, NULL AS outcome,
                   NULL AS post_interview_notes, 'Engineer' AS job_title,
                   'Example Co' AS company
            "#,
        )
        .fetch_one(database.pool())
        .await
        .unwrap();

        let interview = interview_with_job_from_row(row).unwrap();

        assert_eq!(interview.job_hash, "job-9");
        assert_eq!(interview.interview_type, "other");
        assert_eq!(interview.duration_minutes, 60);
        assert!(!interview.completed);
    }

    #[tokio::test]
    async fn interview_queries_keep_debriefs_reachable_and_ordered() {
        let database = crate::Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        sqlx::query(
            "INSERT INTO jobs (hash, title, company, url, source) VALUES ('debrief-job', 'Office Assistant', 'CareBridge', 'https://example.com/job', 'test')",
        )
        .execute(database.pool())
        .await
        .unwrap();
        let tracker = ApplicationTracker::new(database.pool().clone());
        let application_id = tracker.create_application("debrief-job").await.unwrap();
        let interview_id = tracker
            .schedule_interview(
                application_id,
                "screening_call",
                &(Utc::now() - Duration::days(120)).to_rfc3339(),
                30,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let upcoming_id = tracker
            .schedule_interview(
                application_id,
                "screening_call",
                &(Utc::now() + Duration::days(3)).to_rfc3339(),
                30,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        tracker
            .schedule_interview(
                application_id,
                "screening_call",
                &(Utc::now() + Duration::days(31)).to_rfc3339(),
                30,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let recent_id = tracker
            .schedule_interview(
                application_id,
                "screening_call",
                &(Utc::now() - Duration::days(2)).to_rfc3339(),
                30,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        tracker
            .complete_interview(recent_id, "pending", None)
            .await
            .unwrap();

        let open_interviews = tracker.get_upcoming_interviews().await.unwrap();

        assert_eq!(
            open_interviews
                .iter()
                .map(|interview| interview.id)
                .collect::<Vec<_>>(),
            vec![upcoming_id, interview_id]
        );
        assert!(open_interviews
            .iter()
            .all(|interview| interview.job_hash == "debrief-job"));

        let debrief = "Questions asked: How would you prioritize urgent requests?\nFollow-up deadline: 2026-08-01";
        tracker
            .complete_interview(interview_id, "passed", Some(debrief))
            .await
            .unwrap();

        assert_eq!(
            tracker
                .get_upcoming_interviews()
                .await
                .unwrap()
                .iter()
                .map(|interview| interview.id)
                .collect::<Vec<_>>(),
            vec![upcoming_id]
        );
        let past_interviews = tracker.get_past_interviews().await.unwrap();
        assert_eq!(past_interviews.len(), 2);
        assert_eq!(past_interviews[0].id, recent_id);
        assert_eq!(past_interviews[1].outcome.as_deref(), Some("passed"));
        assert_eq!(
            past_interviews[1].post_interview_notes.as_deref(),
            Some(debrief)
        );
    }
}
