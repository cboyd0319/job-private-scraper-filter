use super::*;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Create resume_drafts table
    sqlx::query(
        r#"
            CREATE TABLE resume_drafts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

#[tokio::test]
async fn test_create_resume() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();
    assert!(resume_id > 0);

    let resume = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(resume.id, resume_id);
    assert!(resume.resume.personal.name.is_empty());
    assert!(resume
        .resume
        .summary
        .as_deref()
        .unwrap_or_default()
        .is_empty());
}

#[tokio::test]
async fn explicitly_supplied_military_fields_survive_storage_round_trip() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);
    let resume_id = builder.create_resume().await.unwrap();
    let resume = StructuredResume {
        clearance: Some("User-confirmed current clearance".to_string()),
        military_info: Some("User-entered service evidence".to_string()),
        ..StructuredResume::default()
    };

    builder.replace_content(resume_id, resume).await.unwrap();

    let stored = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(
        stored.resume.clearance.as_deref(),
        Some("User-confirmed current clearance")
    );
    assert_eq!(
        stored.resume.military_info.as_deref(),
        Some("User-entered service evidence")
    );
}

#[tokio::test]
async fn test_update_contact() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();

    let contact = ResumePersonalInfo {
        name: "Jordan Lee".to_string(),
        email: "jordan@example.com".to_string(),
        phone: Some("+1-555-0100".to_string()),
        linkedin: Some("linkedin.com/in/jordan-lee".to_string()),
        github: None,
        location: Some("Portland, OR".to_string()),
        website: Some("jordanlee.example.com".to_string()),
    };

    builder
        .update_contact(resume_id, contact.clone())
        .await
        .unwrap();

    let resume = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(resume.resume.personal.name, "Jordan Lee");
    assert_eq!(resume.resume.personal.email, "jordan@example.com");
    assert_eq!(resume.resume.personal.phone.unwrap(), "+1-555-0100");
}

#[tokio::test]
async fn test_add_experience() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();

    let exp = DraftExperience {
        id: 0, // Will be assigned
        experience: ResumeExperience {
            company: "Harbor Community Services".to_string(),
            title: "Program Operations Lead".to_string(),
            location: Some("Remote".to_string()),
            start_date: "2020-01".to_string(),
            end_date: None,
            is_current: true,
            achievements: vec![
                "Coordinated a 5-person intake team".to_string(),
                "Reduced client intake turnaround by 30%".to_string(),
            ],
        },
    };

    let exp_id = builder.add_experience(resume_id, exp).await.unwrap();
    assert_eq!(exp_id, 1);

    let resume = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(resume.resume.experience.len(), 1);
    assert_eq!(
        resume.resume.experience[0].company,
        "Harbor Community Services"
    );
    assert_eq!(resume.resume.experience[0].achievements.len(), 2);
}

#[tokio::test]
async fn test_delete_experience() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();

    let exp = DraftExperience {
        id: 0,
        experience: ResumeExperience {
            company: "Harbor Community Services".to_string(),
            title: "Program Operations Lead".to_string(),
            location: None,
            start_date: "2020-01".to_string(),
            end_date: None,
            is_current: true,
            achievements: vec![],
        },
    };

    let exp_id = builder.add_experience(resume_id, exp).await.unwrap();
    builder.delete_experience(resume_id, exp_id).await.unwrap();

    let resume = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(resume.resume.experience.len(), 0);
}

#[tokio::test]
async fn test_delete_missing_experience_returns_error() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();
    let result = builder.delete_experience(resume_id, 999).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_skills() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();

    let skills = vec![
        DraftSkill {
            category: "Programming Language".to_string(),
            skill: ResumeSkill {
                name: "Rust".to_string(),
                proficiency: Some("expert".to_string()),
                years_experience: Some(5.0),
            },
        },
        DraftSkill {
            category: "Framework".to_string(),
            skill: ResumeSkill {
                name: "Tokio".to_string(),
                proficiency: Some("advanced".to_string()),
                years_experience: Some(3.0),
            },
        },
    ];

    builder.set_skills(resume_id, skills).await.unwrap();

    let resume = builder.get_resume(resume_id).await.unwrap().unwrap();
    assert_eq!(resume.resume.skills.len(), 2);
    assert_eq!(resume.resume.skills[0].skills[0].name, "Rust");
}

#[test]
fn test_builder_deserializes_frontend_resume_payload_shapes() {
    let experience: DraftExperience = serde_json::from_value(serde_json::json!({
        "id": 0,
        "title": "Program Coordinator",
        "company": "Community Clinic",
        "location": "Portland, OR",
        "start_date": "2022-01",
        "end_date": null,
        "achievements": ["Improved intake scheduling"]
    }))
    .expect("frontend experience payload should deserialize");

    assert_eq!(
        experience.experience.achievements,
        vec!["Improved intake scheduling"]
    );

    let education: DraftEducation = serde_json::from_value(serde_json::json!({
        "id": 0,
        "degree": "BA",
        "institution": "Metro College",
        "location": "Portland, OR",
        "graduation_date": "2020",
        "gpa": "3.8",
        "honors": ["Dean's List"]
    }))
    .expect("frontend education payload should deserialize");

    assert_eq!(education.education.graduation_date.as_deref(), Some("2020"));
    assert_eq!(education.education.honors, vec!["Dean's List"]);

    let skill: DraftSkill = serde_json::from_value(serde_json::json!({
        "name": "Patient Intake",
        "category": "Operations",
        "proficiency": "advanced"
    }))
    .expect("frontend skill payload should deserialize");

    assert_eq!(skill.category, "Operations");
    assert_eq!(skill.skill.proficiency.as_deref(), Some("advanced"));
}

#[test]
fn resume_draft_preserves_the_flat_persistence_contract() {
    let payload = serde_json::json!({
        "id": 7,
        "contact": {
            "name": "Jordan Lee",
            "email": "jordan@example.com",
            "phone": null,
            "location": "Portland, OR",
            "linkedin": null,
            "github": null,
            "website": null
        },
        "summary": "Program operations lead",
        "experience": [{
            "id": 11,
            "title": "Operations Lead",
            "company": "Community Services",
            "location": "Portland, OR",
            "start_date": "2022-01",
            "end_date": null,
            "is_current": true,
            "achievements": ["Reduced intake time by 30%"]
        }],
        "education": [{
            "id": 13,
            "institution": "Metro College",
            "degree": "BA",
            "field_of_study": "Public Administration",
            "location": "Portland, OR",
            "graduation_date": "2020",
            "gpa": "3.8",
            "honors": ["Dean's List"]
        }],
        "skills": [{
            "category": "Operations",
            "name": "Scheduling",
            "proficiency": "advanced",
            "years_experience": 4.0
        }],
        "certifications": [{
            "name": "Project Management",
            "issuer": "Professional Institute",
            "date_obtained": "2024-01",
            "expiration_date": null,
            "credential_id": "PM-123"
        }],
        "projects": [{
            "name": "Intake Redesign",
            "description": "Simplified client intake",
            "technologies": ["Excel"],
            "url": null,
            "start_date": "2023-01",
            "end_date": "2023-06"
        }],
        "created_at": "2026-07-16T12:00:00Z",
        "updated_at": "2026-07-16T13:00:00Z"
    });

    let draft: ResumeDraft = serde_json::from_value(payload.clone()).unwrap();

    assert_eq!(serde_json::to_value(draft).unwrap(), payload);
}

#[tokio::test]
async fn test_delete_resume() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();
    builder.delete_resume(resume_id).await.unwrap();

    let result = builder.get_resume(resume_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_missing_education_returns_error() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let resume_id = builder.create_resume().await.unwrap();
    let result = builder.delete_education(resume_id, 999).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_missing_resume_returns_error() {
    let pool = setup_test_db().await;
    let builder = ResumeBuilder::new(pool);

    let result = builder.delete_resume(999).await;

    assert!(result.is_err());
}
