//! Smart Screening Answer Learning
//!
//! Learns from user behavior to improve screening question answers over time.
//! Tracks usage, modifications, and suggests answers with confidence scores.

use crate::sqlite_time::parse_sqlite_datetime;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jobsentinel_domain::{
    requires_user_answer, screening_question_matches, AnswerSource, AnswerStatistics,
    AnswerSuggestion, ModificationExample,
};
use sqlx::{sqlite::SqliteRow, Row, SqlitePool};
use std::collections::HashMap;

/// Manages screening answer learning
pub struct AnswerLearningManager {
    db: SqlitePool,
}

fn parse_optional_answer_datetime(value: Option<&str>) -> Option<DateTime<Utc>> {
    value.and_then(|date| parse_sqlite_datetime(date).ok())
}

fn days_since_answer_datetime(value: Option<&str>) -> Option<i32> {
    let parsed = parse_optional_answer_datetime(value)?;
    Some((Utc::now() - parsed).num_days() as i32)
}

#[derive(Clone, Copy)]
enum PatternAnswerSource {
    Manual,
    Learned,
}

impl AnswerLearningManager {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    fn pattern_suggestion_from_row(
        &self,
        row: &SqliteRow,
        source_kind: PatternAnswerSource,
    ) -> Result<(String, AnswerSuggestion)> {
        let pattern: String = row.try_get("question_pattern")?;
        let id: i64 = row.try_get("id")?;
        let times_used: i32 = row.try_get("times_used")?;
        let times_modified: i32 = row.try_get("times_modified")?;
        let base_confidence: f64 = row.try_get("confidence_score")?;
        let last_used: Option<String> = row.try_get("last_used_at")?;
        let answer = row.try_get(match source_kind {
            PatternAnswerSource::Manual => "answer",
            PatternAnswerSource::Learned => "learned_answer",
        })?;
        let source = match source_kind {
            PatternAnswerSource::Manual => AnswerSource::Manual {
                pattern: pattern.clone(),
                answer_id: id,
            },
            PatternAnswerSource::Learned => AnswerSource::Learned {
                pattern: pattern.clone(),
                learned_id: id,
            },
        };
        let modification_rate = if times_used > 0 {
            times_modified as f64 / times_used as f64
        } else {
            0.0
        };

        Ok((
            pattern,
            AnswerSuggestion {
                answer,
                confidence: self.calculate_confidence(
                    base_confidence,
                    times_used,
                    times_modified,
                    &last_used,
                ),
                source,
                times_used,
                times_modified,
                last_used_days_ago: days_since_answer_datetime(last_used.as_deref()),
                modification_rate,
            },
        ))
    }

    /// Normalize question text for fuzzy matching
    ///
    /// - Lowercase
    /// - Trim whitespace
    /// - Collapse multiple spaces to one
    /// - Remove punctuation except spaces
    pub fn normalize_question(text: &str) -> String {
        let lower = text.to_lowercase();
        let no_punct = lower
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>();

        no_punct.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Calculate similarity between two normalized question texts
    ///
    /// Uses Jaro-Winkler distance (0.0 = completely different, 1.0 = identical)
    fn calculate_similarity(q1: &str, q2: &str) -> f64 {
        if q1 == q2 {
            return 1.0;
        }
        if q1.is_empty() || q2.is_empty() {
            return 0.0;
        }

        // Simple word overlap similarity (good enough for MVP)
        let words1: std::collections::HashSet<&str> = q1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = q2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Get suggested answers for a question
    ///
    /// Returns suggestions ranked by confidence score.
    /// Includes manual patterns, learned patterns, and historical answers.
    pub async fn get_suggested_answers(
        &self,
        question: &str,
        limit: usize,
    ) -> Result<Vec<AnswerSuggestion>> {
        if requires_user_answer(question) {
            return Ok(Vec::new());
        }

        let normalized = Self::normalize_question(question);
        let mut suggestions = Vec::new();

        // 1. Check manual patterns (screening_answers table)
        let manual = self.get_manual_pattern_matches(question).await?;
        suggestions.extend(manual);

        // 2. Check learned patterns (screening_learned_answers table)
        let learned = self.get_learned_pattern_matches(question).await?;
        suggestions.extend(learned);

        // 3. Check historical answers with similar questions
        let historical = self.get_historical_matches(&normalized, 0.6).await?;
        suggestions.extend(historical);

        // Sort by confidence descending
        suggestions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate by answer text (keep highest confidence)
        let mut seen = HashMap::new();
        suggestions.retain(|s| {
            if seen.contains_key(&s.answer) {
                false
            } else {
                seen.insert(s.answer.clone(), true);
                true
            }
        });

        // Take top N
        Ok(suggestions.into_iter().take(limit).collect())
    }

    /// Get matches from manual screening_answers patterns
    async fn get_manual_pattern_matches(&self, question: &str) -> Result<Vec<AnswerSuggestion>> {
        let rows = sqlx::query(
            r#"
            SELECT id, question_pattern, answer, times_used, times_modified,
                   confidence_score, last_used_at
            FROM screening_answers
            ORDER BY confidence_score DESC, times_used DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut suggestions = Vec::new();

        for row in rows {
            let pattern: String = row.try_get("question_pattern")?;
            if requires_user_answer(&pattern) {
                continue;
            }
            let (pattern, suggestion) =
                self.pattern_suggestion_from_row(&row, PatternAnswerSource::Manual)?;

            if screening_question_matches(&pattern, question) {
                suggestions.push(suggestion);
            }
        }

        Ok(suggestions)
    }

    /// Get matches from learned patterns
    async fn get_learned_pattern_matches(&self, question: &str) -> Result<Vec<AnswerSuggestion>> {
        let rows = sqlx::query(
            r#"
            SELECT id, question_pattern, learned_answer, times_used, times_modified,
                   confidence_score, last_used_at
            FROM screening_learned_answers
            WHERE confidence_score > 0.3
            ORDER BY confidence_score DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut suggestions = Vec::new();

        for row in rows {
            let pattern: String = row.try_get("question_pattern")?;
            if requires_user_answer(&pattern) {
                continue;
            }
            let (pattern, suggestion) =
                self.pattern_suggestion_from_row(&row, PatternAnswerSource::Learned)?;

            if screening_question_matches(&pattern, question) {
                suggestions.push(suggestion);
            }
        }

        Ok(suggestions)
    }

    /// Get historical answers with similar questions
    async fn get_historical_matches(
        &self,
        normalized_question: &str,
        min_similarity: f64,
    ) -> Result<Vec<AnswerSuggestion>> {
        // Fetch recent history (last 6 months, not modified)
        let rows = sqlx::query(
            r#"
            SELECT question_text, question_normalized, answer_filled, was_modified
            FROM screening_answer_history
            WHERE was_modified = 0
              AND created_at > datetime('now', '-6 months')
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut matches: HashMap<String, (String, usize)> = HashMap::new();

        for row in rows {
            let q_normalized: String = row.get("question_normalized");
            let q_original: String = row.get("question_text");
            let answer: String = row.get("answer_filled");

            if requires_user_answer(&q_original) {
                continue;
            }

            let similarity = Self::calculate_similarity(normalized_question, &q_normalized);

            if similarity >= min_similarity {
                matches
                    .entry(answer.clone())
                    .and_modify(|(_, count)| *count += 1)
                    .or_insert((q_original, 1));
            }
        }

        let suggestions: Vec<AnswerSuggestion> = matches
            .into_iter()
            .map(|(answer, (original_q, count))| {
                let confidence = min_similarity * 0.8; // Historical matches have lower confidence
                AnswerSuggestion {
                    answer,
                    confidence,
                    source: AnswerSource::Historical {
                        original_question: original_q,
                    },
                    times_used: count as i32,
                    times_modified: 0,
                    last_used_days_ago: None,
                    modification_rate: 0.0,
                }
            })
            .collect();

        Ok(suggestions)
    }

    /// Calculate confidence score based on usage metrics
    ///
    /// Formula:
    /// confidence = base * usage_weight * recency_weight * modification_penalty
    fn calculate_confidence(
        &self,
        base_confidence: f64,
        times_used: i32,
        times_modified: i32,
        last_used_at: &Option<String>,
    ) -> f64 {
        // Usage weight: increases with usage up to 10 times (then capped at 1.0)
        let usage_weight = (times_used as f64 / 10.0).min(1.0);

        // Recency weight: decays linearly over 1 year
        let recency_weight =
            if let Some(dt) = parse_optional_answer_datetime(last_used_at.as_deref()) {
                let days_ago = (Utc::now() - dt).num_days();
                (1.0 - (days_ago as f64 / 365.0)).max(0.3) // Min 30% confidence
            } else {
                0.8 // Never used yet, or unparseable legacy data
            };

        // Modification penalty: reduces confidence based on modification rate
        let modification_penalty = if times_used > 0 {
            1.0 - (times_modified as f64 / times_used as f64)
        } else {
            1.0 // No usage yet, no penalty
        };

        let final_confidence =
            base_confidence * usage_weight * recency_weight * modification_penalty;
        final_confidence.clamp(0.0, 1.0)
    }
}

mod history;

#[cfg(test)]
mod row_mapper_tests {
    use super::*;

    #[tokio::test]
    async fn pattern_suggestion_mapper_preserves_usage_fields() {
        let database = crate::Database::connect_memory().await.unwrap();
        let manager = AnswerLearningManager::new(database.pool().clone());
        let row = sqlx::query(
            r#"
            SELECT 4 AS id, 'authorized to work' AS question_pattern,
                   'Yes' AS answer, 8 AS times_used, 2 AS times_modified,
                   0.9 AS confidence_score, NULL AS last_used_at
            "#,
        )
        .fetch_one(database.pool())
        .await
        .unwrap();

        let (pattern, suggestion) = manager
            .pattern_suggestion_from_row(&row, PatternAnswerSource::Manual)
            .unwrap();

        assert_eq!(pattern, "authorized to work");
        assert_eq!(suggestion.answer, "Yes");
        assert_eq!(suggestion.modification_rate, 0.25);
        assert_eq!(
            suggestion.source,
            AnswerSource::Manual {
                pattern,
                answer_id: 4
            }
        );
    }
}

#[cfg(test)]
mod tests;
