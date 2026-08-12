use super::*;

#[test]
fn test_normalize_question() {
    assert_eq!(
        AnswerLearningManager::normalize_question("How many years of experience do you have?"),
        "how many years of experience do you have"
    );

    assert_eq!(
        AnswerLearningManager::normalize_question("  Multiple   spaces   "),
        "multiple spaces"
    );

    assert_eq!(
        AnswerLearningManager::normalize_question("What's your salary? (USD)"),
        "what s your salary usd"
    );
}

#[test]
fn test_calculate_similarity() {
    // Identical
    assert_eq!(
        AnswerLearningManager::calculate_similarity("hello world", "hello world"),
        1.0
    );

    // Completely different
    assert!(AnswerLearningManager::calculate_similarity("hello", "goodbye") < 0.5);

    // Partial overlap
    let sim = AnswerLearningManager::calculate_similarity(
        "how many years experience",
        "how many years of programming experience",
    );
    assert!(sim > 0.6);
    assert!(sim < 1.0);

    // Empty strings
    assert_eq!(AnswerLearningManager::calculate_similarity("", "test"), 0.0);
}

#[test]
fn test_parse_optional_answer_datetime_accepts_sqlite_format() {
    let parsed = parse_optional_answer_datetime(Some("2026-05-20 12:34:56"))
        .expect("SQLite datetime should parse");

    assert_eq!(parsed.to_rfc3339(), "2026-05-20T12:34:56+00:00");
}

#[test]
fn test_parse_optional_answer_datetime_accepts_rfc3339() {
    let parsed = parse_optional_answer_datetime(Some("2026-05-20T12:34:56Z"))
        .expect("RFC3339 datetime should parse");

    assert_eq!(parsed.to_rfc3339(), "2026-05-20T12:34:56+00:00");
}

#[test]
fn test_parse_optional_answer_datetime_rejects_invalid() {
    assert!(parse_optional_answer_datetime(Some("not a date")).is_none());
    assert!(parse_optional_answer_datetime(None).is_none());
}

#[tokio::test]
async fn protected_questions_are_neither_suggested_nor_learned() {
    let pool = crate::test_support::migrated_pool().await;
    for (pattern, answer, answer_type) in [
        ("veteran status", "Yes", "yes_no"),
        ("Hispanic or Latino", "Decline to answer", "select"),
        ("pronouns", "they/them", "text"),
    ] {
        sqlx::query(
            "INSERT INTO screening_answers (question_pattern, answer, answer_type) VALUES (?, ?, ?)",
        )
        .bind(pattern)
        .bind(answer)
        .bind(answer_type)
        .execute(&pool)
        .await
        .unwrap();
    }
    let answer_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM screening_answers WHERE question_pattern = 'veteran status'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let manager = AnswerLearningManager::new(pool.clone());
    for question in [
        "Do you identify as a protected veteran?",
        "Are you Hispanic or Latino?",
        "What are your pronouns?",
    ] {
        assert!(manager
            .get_suggested_answers(question, 5)
            .await
            .unwrap()
            .is_empty());
    }
    let question = "Do you identify as a protected veteran?";
    manager
        .record_answer_usage(Some(answer_id), question, "Yes", false, None, None, None)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM screening_answer_history")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn protected_saved_patterns_and_history_are_not_learned_or_suggested() {
    let pool = crate::test_support::migrated_pool().await;
    sqlx::query(
        "INSERT INTO screening_answers (question_pattern, answer, answer_type) VALUES (?, ?, ?)",
    )
    .bind("protected veteran")
    .bind("Yes")
    .bind("yes_no")
    .execute(&pool)
    .await
    .unwrap();
    let answer_id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM screening_answers WHERE question_pattern = 'protected veteran'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO screening_answer_history (
            question_text, question_normalized, answer_filled, was_modified
        ) VALUES (?, ?, ?, 0)
        "#,
    )
    .bind("What is your race?")
    .bind("work history")
    .bind("Sensitive answer")
    .execute(&pool)
    .await
    .unwrap();
    let manager = AnswerLearningManager::new(pool.clone());

    manager
        .record_answer_usage(
            Some(answer_id),
            "Describe your work history.",
            "Yes",
            false,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM screening_answer_history")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(manager
        .get_historical_matches("work history", 0.6)
        .await
        .unwrap()
        .is_empty());
}
