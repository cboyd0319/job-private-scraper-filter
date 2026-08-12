use super::*;

async fn set_match_subscores(
    pool: &SqlitePool,
    resume_id: i64,
    job_hash: &str,
    experience: f64,
    education: f64,
) {
    sqlx::query(
        r#"
        UPDATE resume_job_matches
        SET experience_match_score = ?, education_match_score = ?
        WHERE resume_id = ? AND job_hash = ?
        "#,
    )
    .bind(experience)
    .bind(education)
    .bind(resume_id)
    .bind(job_hash)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_match_resume_to_job_with_null_skills_json() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let resume_id = create_test_resume(&pool, "Resume", "Python").await;
    let job_hash = "job_null_json";
    create_test_job(&pool, job_hash, "Care Coordinator", "Python").await;

    // Create a match
    matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();

    // Manually corrupt the JSON to test error handling
    sqlx::query(
        "UPDATE resume_job_matches SET missing_skills = NULL WHERE resume_id = ? AND job_hash = ?",
    )
    .bind(resume_id)
    .bind(job_hash)
    .execute(&pool)
    .await
    .unwrap();

    // get_match_result should handle NULL JSON gracefully
    let result = matcher.get_match_result(resume_id, job_hash).await.unwrap();
    assert!(result.is_some());
    let match_data = result.unwrap();
    // Should default to empty array
    assert!(match_data.missing_skills.is_empty());
}

#[tokio::test]
async fn test_match_resume_to_job_idempotent() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let resume_id = create_test_resume(&pool, "Resume", "Python JavaScript").await;
    let job_hash = "job_idempotent";
    create_test_job(&pool, job_hash, "Care Coordinator", "Python JavaScript").await;

    // Create match multiple times
    let match1 = matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();
    let match2 = matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();
    let match3 = matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();

    // Should have same scores (deterministic matching)
    assert_eq!(match1.overall_match_score, match2.overall_match_score);
    assert_eq!(match2.overall_match_score, match3.overall_match_score);

    // Should only have one record in DB
    let count = sqlx::query(
        "SELECT COUNT(*) as count FROM resume_job_matches WHERE resume_id = ? AND job_hash = ?",
    )
    .bind(resume_id)
    .bind(job_hash)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 1);
}

#[tokio::test]
async fn test_extract_skills_with_empty_text() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let result = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind("Empty Text Resume")
    .bind("fixtures/resumes/empty.pdf")
    .bind("")
    .execute(&pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // extract_skills should handle empty text
    let skills = matcher.extract_skills(resume_id).await.unwrap();
    assert!(skills.is_empty());
}

#[tokio::test]
async fn test_extract_skills_overwrites_existing() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    // Create resume with initial skills
    let resume_id = create_test_resume(&pool, "Resume", "Python JavaScript").await;

    // Verify initial skills
    let skills = matcher.get_user_skills(resume_id).await.unwrap();
    let initial_count = skills.len();
    assert!(initial_count > 0);

    // Update resume text
    sqlx::query("UPDATE resumes SET parsed_text = ? WHERE id = ?")
        .bind("Rust Go")
        .bind(resume_id)
        .execute(&pool)
        .await
        .unwrap();

    // Re-extract skills
    let new_skills = matcher.extract_skills(resume_id).await.unwrap();

    // Should have new skills (Rust, Go) and old skills updated via UPSERT
    assert!(!new_skills.is_empty());
    let skill_names: Vec<String> = new_skills.iter().map(|s| s.skill_name.clone()).collect();
    assert!(skill_names.contains(&"Rust".to_string()));
    assert!(skill_names.contains(&"Go".to_string()));
}

#[tokio::test]
async fn test_get_match_result_with_all_scores() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let resume_id = create_test_resume(&pool, "Resume", "Python").await;
    let job_hash = "job_all_scores";
    create_test_job(&pool, job_hash, "Care Coordinator", "Python").await;

    // Create match and manually set all score fields
    matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();

    set_match_subscores(&pool, resume_id, job_hash, 0.8, 0.75).await;

    let result = matcher.get_match_result(resume_id, job_hash).await.unwrap();
    assert!(result.is_some());
    let match_data = result.unwrap();
    assert_eq!(match_data.experience_match_score, Some(0.8));
    assert_eq!(match_data.education_match_score, Some(0.75));
}

#[tokio::test]
async fn test_recent_matches_include_all_sub_scores() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let resume_id = create_test_resume(&pool, "Resume", "Python").await;
    let job_hash = "job_recent_scores";
    create_test_job(&pool, job_hash, "Care Coordinator", "Python").await;

    matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();

    set_match_subscores(&pool, resume_id, job_hash, 0.8, 0.75).await;

    let recent_matches = matcher.get_recent_matches(resume_id, 10).await.unwrap();
    assert_eq!(recent_matches.len(), 1);
    assert_eq!(recent_matches[0].experience_match_score, Some(0.8));
    assert_eq!(recent_matches[0].education_match_score, Some(0.75));
}

#[tokio::test]
async fn saved_match_feedback_is_closed_replaceable_clearable_and_local() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());
    let resume_id = create_test_resume(&pool, "Feedback Resume", "Python").await;
    let job_hash = "job_feedback";
    create_test_job(&pool, job_hash, "Care Coordinator", "Python").await;
    let saved_match = matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();

    let feedback = matcher
        .set_match_feedback(saved_match.id, Some(ResumeMatchFeedbackLabel::NotRelevant))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(feedback.match_id, saved_match.id);
    assert_eq!(feedback.label, ResumeMatchFeedbackLabel::NotRelevant);

    matcher
        .set_match_feedback(saved_match.id, Some(ResumeMatchFeedbackLabel::Useful))
        .await
        .unwrap();
    let recent = matcher.get_recent_matches(resume_id, 10).await.unwrap();
    assert_eq!(
        recent[0].feedback.as_ref().map(|value| value.label),
        Some(ResumeMatchFeedbackLabel::Useful)
    );
    matcher
        .match_resume_to_job(resume_id, job_hash)
        .await
        .unwrap();
    assert_eq!(
        matcher.get_recent_matches(resume_id, 10).await.unwrap()[0]
            .feedback
            .as_ref()
            .map(|value| value.label),
        Some(ResumeMatchFeedbackLabel::Useful)
    );

    assert!(matcher
        .set_match_feedback(saved_match.id, None)
        .await
        .unwrap()
        .is_none());
    assert!(matcher.get_recent_matches(resume_id, 10).await.unwrap()[0]
        .feedback
        .is_none());
    assert!(matcher
        .set_match_feedback(i64::MAX, Some(ResumeMatchFeedbackLabel::Useful))
        .await
        .is_err());
}

#[tokio::test]
async fn test_boundary_values_for_scores() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let result = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind("Boundary Resume")
    .bind("fixtures/resumes/boundary.pdf")
    .bind("")
    .execute(&pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Insert skill with boundary confidence scores
    for (skill, score) in &[("MinScore", 0.0), ("MaxScore", 1.0)] {
        sqlx::query(
            "INSERT INTO user_skills (resume_id, skill_name, confidence_score, source) VALUES (?, ?, ?, ?)",
        )
        .bind(resume_id)
        .bind(skill)
        .bind(score)
        .bind("user_input")
        .execute(&pool)
        .await
        .unwrap();
    }

    let skills = matcher.get_user_skills(resume_id).await.unwrap();
    assert_eq!(skills.len(), 2);

    // Verify boundary values are preserved
    let max_skill = skills.iter().find(|s| s.skill_name == "MaxScore").unwrap();
    assert_eq!(max_skill.confidence_score, 1.0);

    let min_skill = skills.iter().find(|s| s.skill_name == "MinScore").unwrap();
    assert_eq!(min_skill.confidence_score, 0.0);
}

#[tokio::test]
async fn test_concurrent_resume_creation() {
    let pool = crate::test_support::migrated_pool().await;

    // Create multiple resumes concurrently
    let mut tasks = vec![];
    for i in 0..5 {
        let pool_clone = pool.clone();
        let task = tokio::spawn(async move {
            create_test_resume(&pool_clone, &format!("Concurrent Resume {}", i), "Python").await
        });
        tasks.push(task);
    }

    // All should succeed
    for task in tasks {
        let result = task.await;
        assert!(result.is_ok());
    }

    // Verify all resumes were created
    let count = sqlx::query("SELECT COUNT(*) as count FROM resumes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 5);
}

#[tokio::test]
async fn test_unicode_in_skill_names() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let result = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind("Unicode Resume")
    .bind("fixtures/resumes/unicode.pdf")
    .bind("")
    .execute(&pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Insert skills with unicode characters
    sqlx::query(
        "INSERT INTO user_skills (resume_id, skill_name, confidence_score, source) VALUES (?, ?, ?, ?)",
    )
    .bind(resume_id)
    .bind("日本語スキル")
    .bind(0.9)
    .bind("user_input")
    .execute(&pool)
    .await
    .unwrap();

    let skills = matcher.get_user_skills(resume_id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].skill_name, "日本語スキル");
}

#[tokio::test]
async fn test_very_long_skill_names() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let result = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind("Long Skill Resume")
    .bind("fixtures/resumes/long.pdf")
    .bind("")
    .execute(&pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Insert skill with very long name
    let long_name = "A".repeat(1000);
    sqlx::query(
        "INSERT INTO user_skills (resume_id, skill_name, confidence_score, source) VALUES (?, ?, ?, ?)",
    )
    .bind(resume_id)
    .bind(&long_name)
    .bind(0.8)
    .bind("user_input")
    .execute(&pool)
    .await
    .unwrap();

    let skills = matcher.get_user_skills(resume_id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].skill_name.len(), 1000);
}
