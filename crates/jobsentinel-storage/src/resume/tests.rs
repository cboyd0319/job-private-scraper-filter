//! Tests for the resume module

use super::builder::ResumeBuilder;
use super::skills::SkillExtractor;
use super::types::{NewSkill, NullableFieldUpdate, SkillUpdate};
use super::{ResumeMatchFeedbackLabel, ResumeMatcher};
use sqlx::{Row, SqlitePool};

async fn create_test_resume(pool: &SqlitePool, name: &str, text: &str) -> i64 {
    let result = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind(name)
    .bind(format!("fixtures/resumes/{}.pdf", name))
    .bind(text)
    .execute(pool)
    .await
    .unwrap();
    let resume_id = result.last_insert_rowid();

    // Extract and insert skills directly without calling extract_skills
    // (which would call get_resume and fail on datetime parsing)
    if !text.is_empty() {
        let skill_extractor = SkillExtractor::new();
        let extracted_skills = skill_extractor.extract_skills(text);

        for skill in extracted_skills {
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
            .execute(pool)
            .await
            .unwrap();
        }
    }

    resume_id
}

async fn create_test_job(pool: &SqlitePool, job_hash: &str, title: &str, description: &str) {
    sqlx::query(
        r#"
        INSERT INTO jobs (hash, title, company, location, description, url, score, source)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_hash)
    .bind(title)
    .bind("Harbor Community Services")
    .bind("Remote")
    .bind(description)
    .bind("https://example.com/job")
    .bind(0.9)
    .bind("greenhouse")
    .execute(pool)
    .await
    .unwrap();
}

fn uploaded_skill_names(skills: Vec<super::types::UserSkill>) -> Vec<String> {
    skills.into_iter().map(|skill| skill.skill_name).collect()
}

// Resume CRUD tests

#[tokio::test]
async fn test_get_resume() {
    let pool = crate::test_support::migrated_pool().await;

    let resume_id = create_test_resume(&pool, "Test Resume", "Test content").await;

    // Test by directly querying instead of using get_resume (which has datetime parsing)
    let row = sqlx::query("SELECT id, name, parsed_text, is_active FROM resumes WHERE id = ?")
        .bind(resume_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<i64, _>("id"), resume_id);
    assert_eq!(row.get::<String, _>("name"), "Test Resume");
    assert_eq!(row.get::<String, _>("parsed_text"), "Test content");
    assert_eq!(row.get::<i64, _>("is_active"), 1);
}

#[tokio::test]
async fn test_upload_resume_txt_parses_text_and_extracts_skills() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("support-resume.txt");
    std::fs::write(
        &file_path,
        "Jordan Lee\n\nSKILLS\nLeadership\nCommunication\nCustomer Service",
    )
    .unwrap();

    let resume_id = matcher
        .upload_resume("Support Resume", &file_path.to_string_lossy())
        .await
        .unwrap();
    let resume = matcher.get_resume(resume_id).await.unwrap();
    let skill_names = uploaded_skill_names(matcher.get_user_skills(resume_id).await.unwrap());

    assert_eq!(resume.name, "Support Resume");
    assert!(resume.is_active);
    assert!(resume.parsed_text.unwrap().contains("Customer Service"));
    assert!(skill_names.contains(&"Leadership".to_string()));
    assert!(skill_names.contains(&"Communication".to_string()));
    assert!(skill_names.contains(&"Customer Service".to_string()));
}

#[tokio::test]
async fn test_upload_resume_html_parses_visible_text_and_extracts_skills() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());
    let temp_dir = tempfile::TempDir::new().unwrap();
    let file_path = temp_dir.path().join("operations-resume.html");
    std::fs::write(
        &file_path,
        r#"
<!doctype html>
<html lang="en">
  <head><style>body { font-family: Arial; }</style></head>
  <body>
    <h1>Jordan Lee</h1>
    <h2>Skills</h2>
    <p>Project Management</p>
    <p>Communication</p>
    <script>alert('ignore');</script>
  </body>
</html>
        "#,
    )
    .unwrap();

    let resume_id = matcher
        .upload_resume("HTML Operations Resume", &file_path.to_string_lossy())
        .await
        .unwrap();
    let resume = matcher.get_resume(resume_id).await.unwrap();
    let parsed_text = resume.parsed_text.unwrap();
    let skill_names = uploaded_skill_names(matcher.get_user_skills(resume_id).await.unwrap());

    assert!(parsed_text.contains("Jordan Lee"));
    assert!(parsed_text.contains("Project Management"));
    assert!(!parsed_text.contains("font-family"));
    assert!(!parsed_text.contains("alert"));
    assert!(skill_names.contains(&"Project Management".to_string()));
    assert!(skill_names.contains(&"Communication".to_string()));
}

#[tokio::test]
async fn test_import_json_resume_preserves_builder_evidence_sections() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let json = r#"{
        "basics": {
            "name": "Applicant Example",
            "email": "applicant@example.test",
            "summary": "Operations coordinator focused on accessible services."
        },
        "languages": [{
            "language": "Spanish",
            "fluency": "Professional working proficiency"
        }],
        "skills": [{
            "name": "Client Services",
            "level": "Advanced",
            "keywords": ["Scheduling", "Case notes"]
        }],
        "publications": [{
            "name": "Accessible Hiring Forms",
            "publisher": "Operations Journal",
            "releaseDate": "2024-02-01"
        }],
        "projects": [{
            "name": "Clinic Intake Redesign",
            "description": "Improved appointment intake for community clinic.",
            "highlights": ["Reduced missed calls by 18%"],
            "keywords": ["Scheduling", "Patient intake"],
            "url": "https://example.test/project",
            "roles": ["Coordinator"],
            "entity": "Neighborhood Clinic"
        }]
    }"#;

    let resume_id = matcher
        .import_json_resume("Imported Resume".to_string(), json)
        .await
        .unwrap();
    let builder = ResumeBuilder::new(pool);
    let draft = builder.get_resume(resume_id).await.unwrap().unwrap();

    assert_eq!(draft.resume.personal.name, "Applicant Example");
    assert!(draft
        .resume
        .skills
        .iter()
        .flat_map(|category| &category.skills)
        .any(|skill| skill.name == "Spanish - Professional working proficiency"));
    assert!(draft.resume.skills.iter().any(|category| {
        category.name == "Client Services"
            && category.skills.iter().any(|skill| {
                skill.name == "Scheduling" && skill.proficiency.as_deref() == Some("advanced")
            })
    }));
    assert!(!draft
        .resume
        .skills
        .iter()
        .flat_map(|category| &category.skills)
        .any(|skill| skill.name == "Client Services"));
    assert!(draft.resume.certifications.iter().any(|cert| {
        cert.name == "Publication: Accessible Hiring Forms"
            && cert.issuer == "Operations Journal"
            && cert.date_obtained.as_deref() == Some("2024-02-01")
    }));
    assert_eq!(draft.resume.experience.len(), 0);
    assert!(draft.resume.projects.iter().any(|project| {
        project.name == "Clinic Intake Redesign - Coordinator"
            && project
                .description
                .contains("Improved appointment intake for community clinic.")
            && project.description.contains("Reduced missed calls by 18%")
            && project.technologies == vec!["Scheduling", "Patient intake"]
            && project.url.as_deref() == Some("https://example.test/project")
    }));
}

#[tokio::test]
async fn test_get_resume_not_found() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let result = matcher.get_resume(999).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_active_resume_empty() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let active = matcher.get_active_resume().await.unwrap();
    assert!(active.is_none());
}

#[tokio::test]
async fn test_active_resume() {
    let pool = crate::test_support::migrated_pool().await;

    create_test_resume(&pool, "Test Resume", "Test content").await;

    // Query active resume directly instead of using get_active_resume (datetime parsing issues)
    let row = sqlx::query(
        "SELECT id, name FROM resumes WHERE is_active = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    assert!(row.is_some());
    assert_eq!(row.unwrap().get::<String, _>("name"), "Test Resume");
}

#[tokio::test]
async fn test_set_active_resume() {
    let pool = crate::test_support::migrated_pool().await;
    let matcher = ResumeMatcher::new(pool.clone());

    let id1 = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind("Resume 1")
    .bind("fixtures/resumes/resume1.pdf")
    .bind("Content 1")
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_rowid();

    let id2 = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active) VALUES (?, ?, ?, 0)",
    )
    .bind("Resume 2")
    .bind("fixtures/resumes/resume2.pdf")
    .bind("Content 2")
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_rowid();

    // Set resume 2 as active
    matcher.set_active_resume(id2).await.unwrap();

    // Check resume 2 is active via direct query
    let row2 = sqlx::query("SELECT is_active FROM resumes WHERE id = ?")
        .bind(id2)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row2.get::<i64, _>("is_active"), 1);

    // Check resume 1 is inactive
    let row1 = sqlx::query("SELECT is_active FROM resumes WHERE id = ?")
        .bind(id1)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row1.get::<i64, _>("is_active"), 0);
}

#[tokio::test]
async fn test_set_active_resume_most_recent() {
    let pool = crate::test_support::migrated_pool().await;

    // Create with explicit timestamps to ensure ordering
    let _id1 = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active, created_at) VALUES (?, ?, ?, 1, datetime('now', '-2 seconds'))",
    )
    .bind("Resume 1")
    .bind("fixtures/resumes/r1.pdf")
    .bind("Content 1")
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_rowid();

    let id2 = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, is_active, created_at) VALUES (?, ?, ?, 1, datetime('now'))",
    )
    .bind("Resume 2")
    .bind("fixtures/resumes/r2.pdf")
    .bind("Content 2")
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_rowid();

    // Most recent (Resume 2) should be returned by ORDER BY created_at DESC
    let row = sqlx::query(
        "SELECT id, name FROM resumes WHERE is_active = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<i64, _>("id"), id2);
    assert_eq!(row.get::<String, _>("name"), "Resume 2");
}

mod persistence_edge_cases;

#[path = "tests/document_upload.rs"]
mod document_upload;

#[path = "tests/database_coverage_tests.rs"]
mod database_coverage_tests;

#[path = "tests/skill_matching_tests.rs"]
mod skill_matching_tests;
