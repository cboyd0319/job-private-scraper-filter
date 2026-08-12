use super::*;

impl AnswerLearningManager {
    /// Record that an answer was used
    ///
    /// Tracks usage and user modifications for learning.
    pub async fn record_answer_usage(
        &self,
        screening_answer_id: Option<i64>,
        question_text: &str,
        answer_filled: &str,
        was_modified: bool,
        modified_to: Option<&str>,
        job_hash: Option<&str>,
        application_attempt_id: Option<i64>,
    ) -> Result<()> {
        if requires_user_answer(question_text) {
            return Ok(());
        }

        if let Some(answer_id) = screening_answer_id {
            let pattern: Option<String> =
                sqlx::query_scalar("SELECT question_pattern FROM screening_answers WHERE id = ?")
                    .bind(answer_id)
                    .fetch_optional(&self.db)
                    .await?;

            if pattern.is_some_and(|pattern| requires_user_answer(&pattern)) {
                return Ok(());
            }
        }

        let normalized = Self::normalize_question(question_text);

        // Insert history record
        sqlx::query(
            r#"
            INSERT INTO screening_answer_history (
                screening_answer_id,
                question_text,
                question_normalized,
                answer_filled,
                was_modified,
                modified_to,
                job_hash,
                application_attempt_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(screening_answer_id)
        .bind(question_text)
        .bind(&normalized)
        .bind(answer_filled)
        .bind(was_modified as i32)
        .bind(modified_to)
        .bind(job_hash)
        .bind(application_attempt_id)
        .execute(&self.db)
        .await
        .context("Failed to insert screening answer history")?;

        // Update screening_answers stats if matched
        if let Some(answer_id) = screening_answer_id {
            sqlx::query(
                r#"
                UPDATE screening_answers
                SET times_used = times_used + 1,
                    times_modified = times_modified + ?,
                    last_used_at = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(was_modified as i32)
            .bind(answer_id)
            .execute(&self.db)
            .await?;

            // Recalculate confidence score
            self.update_answer_confidence(answer_id).await?;
        }

        Ok(())
    }

    /// Update confidence score for a screening answer
    async fn update_answer_confidence(&self, answer_id: i64) -> Result<()> {
        let row = sqlx::query(
            "SELECT times_used, times_modified, last_used_at FROM screening_answers WHERE id = ?",
        )
        .bind(answer_id)
        .fetch_one(&self.db)
        .await?;

        let times_used: i32 = row.get("times_used");
        let times_modified: i32 = row.get("times_modified");
        let last_used: Option<String> = row.get("last_used_at");

        let new_confidence = self.calculate_confidence(1.0, times_used, times_modified, &last_used);

        sqlx::query("UPDATE screening_answers SET confidence_score = ? WHERE id = ?")
            .bind(new_confidence)
            .bind(answer_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get statistics for an answer pattern
    pub async fn get_answer_statistics(&self, pattern: &str) -> Result<Option<AnswerStatistics>> {
        let row = sqlx::query(
            r#"
            SELECT id, question_pattern, answer, times_used, times_modified,
                   confidence_score, last_used_at, created_at
            FROM screening_answers
            WHERE question_pattern = ?
            "#,
        )
        .bind(pattern)
        .fetch_optional(&self.db)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let answer_id: i64 = row.get("id");
        let created_at: String = row.get("created_at");
        let last_used: Option<String> = row.get("last_used_at");

        let modification_examples = self.get_recent_modifications(answer_id, 5).await?;

        let times_used: i32 = row.get("times_used");
        let times_modified: i32 = row.get("times_modified");
        let modification_rate = if times_used > 0 {
            times_modified as f64 / times_used as f64
        } else {
            0.0
        };

        Ok(Some(AnswerStatistics {
            pattern: row.get("question_pattern"),
            answer: row.get("answer"),
            times_used,
            times_modified,
            modification_rate,
            confidence_score: row.get("confidence_score"),
            last_used_at: parse_optional_answer_datetime(last_used.as_deref()),
            created_at: parse_sqlite_datetime(&created_at)?,
            recent_modifications: modification_examples,
        }))
    }

    /// Get recent modification examples
    async fn get_recent_modifications(
        &self,
        answer_id: i64,
        limit: usize,
    ) -> Result<Vec<ModificationExample>> {
        let rows = sqlx::query(
            r#"
            SELECT question_text, answer_filled, modified_to, created_at
            FROM screening_answer_history
            WHERE screening_answer_id = ?
              AND was_modified = 1
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(answer_id)
        .bind(limit as i32)
        .fetch_all(&self.db)
        .await?;

        let examples = rows
            .into_iter()
            .filter_map(|row| {
                let modified_to: Option<String> = row.get("modified_to");
                let created_at: String = row.get("created_at");

                Some(ModificationExample {
                    original_answer: row.get("answer_filled"),
                    modified_to: modified_to?,
                    question_text: row.get("question_text"),
                    modified_at: parse_optional_answer_datetime(Some(&created_at))?,
                })
            })
            .collect();

        Ok(examples)
    }

    /// Clear answer history (optionally for a specific pattern)
    pub async fn clear_answer_history(&self, pattern: Option<&str>) -> Result<usize> {
        let count = if let Some(p) = pattern {
            // Clear history for specific pattern
            let answer_id: Option<i64> =
                sqlx::query_scalar("SELECT id FROM screening_answers WHERE question_pattern = ?")
                    .bind(p)
                    .fetch_optional(&self.db)
                    .await?;

            if let Some(id) = answer_id {
                let result = sqlx::query(
                    "DELETE FROM screening_answer_history WHERE screening_answer_id = ?",
                )
                .bind(id)
                .execute(&self.db)
                .await?;

                // Reset stats
                sqlx::query(
                    r#"
                    UPDATE screening_answers
                    SET times_used = 0, times_modified = 0, last_used_at = NULL, confidence_score = 1.0
                    WHERE id = ?
                    "#,
                )
                .bind(id)
                .execute(&self.db)
                .await?;

                result.rows_affected() as usize
            } else {
                0
            }
        } else {
            // Clear all history
            let result = sqlx::query("DELETE FROM screening_answer_history")
                .execute(&self.db)
                .await?;

            // Reset all stats
            sqlx::query(
                r#"
                UPDATE screening_answers
                SET times_used = 0, times_modified = 0, last_used_at = NULL, confidence_score = 1.0
                "#,
            )
            .execute(&self.db)
            .await?;

            result.rows_affected() as usize
        };

        Ok(count)
    }
}
