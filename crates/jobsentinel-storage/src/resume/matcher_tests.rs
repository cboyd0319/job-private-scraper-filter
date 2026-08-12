use super::*;

#[path = "matcher_tests/edge_case_tests.rs"]
mod edge_case_tests;

async fn create_test_job(pool: &SqlitePool, job_hash: &str) {
    sqlx::query(
        r#"
            INSERT INTO jobs (hash, title, company, location, description, url, score, source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
    )
    .bind(job_hash)
    .bind("Client Support Coordinator")
    .bind("Harbor Community Services")
    .bind("Remote")
    .bind("Looking for Case Management and CRM coordinator")
    .bind("https://example.com/job")
    .bind(0.9)
    .bind("greenhouse")
    .execute(pool)
    .await
    .unwrap();
}

async fn create_test_resume_with_skills(pool: &SqlitePool) -> i64 {
    let result = sqlx::query(
        r#"
            INSERT INTO resumes (name, file_path, parsed_text, is_active)
            VALUES (?, ?, ?, 1)
            "#,
    )
    .bind("Test Resume")
    .bind("fixtures/resumes/test.pdf")
    .bind("Case Management, Care Coordination, CRM experience")
    .execute(pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Add user skills
    for skill in &["Case Management", "Care Coordination", "CRM"] {
        sqlx::query(
                r#"
                INSERT INTO user_skills (resume_id, skill_name, skill_category, confidence_score, source)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(resume_id)
            .bind(*skill)
            .bind("client_services")
            .bind(0.9)
            .bind("resume")
            .execute(pool)
            .await
            .unwrap();
    }

    resume_id
}

#[tokio::test]
async fn test_extract_job_skills() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_123";
    create_test_job(&pool, job_hash).await;

    let skills = matcher.extract_job_skills(job_hash).await.unwrap();

    assert!(skills.contains(&"Case Management".to_string()));
    assert!(skills.contains(&"CRM".to_string()));
}

#[tokio::test]
async fn test_calculate_match() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_456";
    create_test_job(&pool, job_hash).await;
    let resume_id = create_test_resume_with_skills(&pool).await;

    // Extract job skills first
    matcher.extract_job_skills(job_hash).await.unwrap();

    // Calculate match
    let match_result = matcher.calculate_match(resume_id, job_hash).await.unwrap();

    assert!(match_result.overall_match_score > 0.0);
    assert!(!match_result.matching_skills.is_empty());
}

#[tokio::test]
async fn test_gap_analysis() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let matching = vec![
        "Case Management".to_string(),
        "Care Coordination".to_string(),
    ];
    let missing = vec!["Budgeting".to_string(), "Reporting".to_string()];

    let analysis = matcher.generate_gap_analysis(&matching, &missing, 0.5);

    assert!(analysis.contains("50%"));
    assert!(analysis.contains("Case Management"));
    assert!(analysis.contains("Care Coordination"));
    assert!(analysis.contains("Budgeting"));
    assert!(analysis.contains("Reporting"));
    assert!(analysis.contains("Matching Skills (2)"));
    assert!(analysis.contains("Missing Skills (2)"));
}

#[tokio::test]
async fn test_match_score_calculation() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_789";
    create_test_job(&pool, job_hash).await;
    let resume_id = create_test_resume_with_skills(&pool).await;

    // Add job skills manually for precise testing
    for skill in &[
        "Case Management",
        "Care Coordination",
        "Budgeting",
        "Reporting",
    ] {
        sqlx::query(
                "INSERT INTO job_skills (job_hash, skill_name, is_required, skill_category) VALUES (?, ?, 1, ?)",
            )
            .bind(job_hash)
            .bind(*skill)
            .bind("client_services")
            .execute(&pool)
            .await
            .unwrap();
    }

    let match_result = matcher.calculate_match(resume_id, job_hash).await.unwrap();

    // User has Case Management and Care Coordination (2/4 = 50%)
    assert_eq!(match_result.skills_match_score, Some(0.5));
    assert_eq!(match_result.matching_skills.len(), 2);
    assert_eq!(match_result.missing_skills.len(), 2);
}

#[tokio::test]
async fn test_calculate_match_no_job_skills() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_no_skills";
    create_test_job(&pool, job_hash).await;
    let resume_id = create_test_resume_with_skills(&pool).await;

    // Don't add any job skills
    let match_result = matcher.calculate_match(resume_id, job_hash).await.unwrap();

    // No recognized job skills means insufficient evidence, not a perfect match.
    assert_eq!(match_result.skills_match_score, Some(0.0));
    assert!(match_result.overall_match_score < 1.0);
    assert_eq!(match_result.matching_skills.len(), 0);
    assert_eq!(match_result.missing_skills.len(), 0);
    assert!(match_result
        .gap_analysis
        .as_deref()
        .unwrap_or_default()
        .contains("not enough job-post skill detail recognized"));
}

#[tokio::test]
async fn test_calculate_match_no_user_skills() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_user_no_skills";
    create_test_job(&pool, job_hash).await;

    // Create resume without skills
    let result = sqlx::query(
        r#"
            INSERT INTO resumes (name, file_path, parsed_text, is_active)
            VALUES (?, ?, ?, 1)
            "#,
    )
    .bind("Empty Resume")
    .bind("fixtures/resumes/empty.pdf")
    .bind("No skills here")
    .execute(&pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Add job skills
    sqlx::query("INSERT INTO job_skills (job_hash, skill_name, is_required) VALUES (?, ?, 1)")
        .bind(job_hash)
        .bind("Case Management")
        .execute(&pool)
        .await
        .unwrap();

    let match_result = matcher.calculate_match(resume_id, job_hash).await.unwrap();

    // User has no skills = 0% match
    assert_eq!(match_result.skills_match_score, Some(0.0));
    assert_eq!(match_result.matching_skills.len(), 0);
    assert_eq!(match_result.missing_skills.len(), 1);
    let analysis = match_result.gap_analysis.as_deref().unwrap();
    assert!(analysis.contains("Scoring sources:"));
    assert!(analysis.contains("skills") || analysis.contains("skill coverage"));
    assert!(!analysis.contains("local semantic similarity"));
    assert!(!analysis.contains("local reranker"));
    if analysis.contains("skill coverage") {
        assert!(analysis.contains(
            "Why not: Score limited because skill evidence was not found: Case Management"
        ));
    } else {
        assert!(!analysis.contains("Why not:"));
    }
}

#[tokio::test]
async fn test_gap_analysis_strong_match() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let matching = vec![
        "Case Management".to_string(),
        "Care Coordination".to_string(),
        "CRM".to_string(),
    ];
    let missing = vec!["Reporting".to_string()];

    let analysis = matcher.generate_gap_analysis(&matching, &missing, 0.85);

    assert!(analysis.contains("85%"));
    assert!(analysis.contains("review the missing items"));
    assert!(analysis.contains("decide whether to apply"));
}

#[tokio::test]
async fn test_gap_analysis_good_match() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let matching = vec![
        "Case Management".to_string(),
        "Care Coordination".to_string(),
    ];
    let missing = vec!["Reporting".to_string()];

    let analysis = matcher.generate_gap_analysis(&matching, &missing, 0.67);

    assert!(analysis.contains("67%"));
    assert!(analysis.contains("Review transferable skills"));
    assert!(analysis.contains("support truthfully"));
}

#[tokio::test]
async fn test_gap_analysis_moderate_match() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let matching = vec!["Case Management".to_string()];
    let missing = vec!["Care Coordination".to_string(), "Reporting".to_string()];

    let analysis = matcher.generate_gap_analysis(&matching, &missing, 0.5);

    assert!(analysis.contains("50%"));
    assert!(analysis.contains("missing items are required"));
    assert!(analysis.contains("add it truthfully"));
}

#[tokio::test]
async fn test_gap_analysis_low_match() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let matching = vec![];
    let missing = vec![
        "Case Management".to_string(),
        "Care Coordination".to_string(),
        "Reporting".to_string(),
    ];

    let analysis = matcher.generate_gap_analysis(&matching, &missing, 0.2);

    assert!(analysis.contains("20%"));
    assert!(analysis.contains("goals and constraints"));
}

#[tokio::test]
async fn test_gap_analysis_empty_skills() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let analysis = matcher.generate_gap_analysis(&[], &[], 1.0);

    assert!(analysis.contains("100%"));
    assert!(analysis.contains("decide whether to apply"));
}

#[tokio::test]
async fn test_get_job_missing() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let result = matcher.get_job("nonexistent_job").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_user_skills_empty() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    // Create resume without skills
    let result = sqlx::query("INSERT INTO resumes (name, file_path, is_active) VALUES (?, ?, 1)")
        .bind("Empty Resume")
        .bind("fixtures/resumes/empty.pdf")
        .execute(&pool)
        .await
        .unwrap();
    let resume_id = result.last_insert_rowid();

    let skills = matcher.get_user_skills(resume_id).await.unwrap();
    assert_eq!(skills.len(), 0);
}

#[tokio::test]
async fn test_get_job_skills_empty() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "job_no_skills";
    create_test_job(&pool, job_hash).await;

    let skills = matcher.get_job_skills(job_hash).await.unwrap();
    assert_eq!(skills.len(), 0);
}

#[tokio::test]
async fn test_education_detection_ignores_management_words() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let requirement = matcher.extract_education_requirements(
        "Looking for Case Management and CRM coordination experience.",
    );

    assert!(requirement.is_none());
}

#[tokio::test]
async fn test_education_detection_keeps_degree_requirement() {
    let matcher = JobMatcher::new(SqlitePool::connect("sqlite::memory:").await.unwrap());

    let requirement = matcher
        .extract_education_requirements("Bachelor's degree required for this role.")
        .unwrap();

    assert_eq!(requirement.degree_level, DegreeLevel::Bachelor);
    assert!(requirement.is_required);
}

#[tokio::test]
async fn test_calculate_match_errors_when_education_lookup_fails() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "job_requires_degree";
    sqlx::query(
        "INSERT INTO jobs (hash, title, company, description, url, source) VALUES (?, 'Client Support Coordinator', 'Harbor Community Services', ?, 'https://example.com/job', 'greenhouse')",
    )
        .bind(job_hash)
        .bind("Bachelor's degree required. Looking for client-services coordinator.")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("ALTER TABLE resumes RENAME TO resumes_unavailable")
        .execute(&pool)
        .await
        .unwrap();

    let result = matcher.calculate_match(42, job_hash).await;

    assert!(
        result.is_err(),
        "education lookup database failures must not be scored as missing education"
    );
}

#[tokio::test]
async fn test_extract_job_skills_duplicate_prevention() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = JobMatcher::new(pool.clone());

    let job_hash = "test_job_duplicates";
    create_test_job(&pool, job_hash).await;

    // Extract skills twice
    let skills1 = matcher.extract_job_skills(job_hash).await.unwrap();
    let skills2 = matcher.extract_job_skills(job_hash).await.unwrap();

    // Should not create duplicates due to UNIQUE constraint
    assert_eq!(skills1.len(), skills2.len());

    // Verify in database
    let rows = sqlx::query("SELECT COUNT(*) as count FROM job_skills WHERE job_hash = ?")
        .bind(job_hash)
        .fetch_one(&pool)
        .await
        .unwrap();
    let count: i64 = rows.try_get("count").unwrap();
    assert_eq!(count as usize, skills1.len());
}
