use super::*;
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};

fn citation(resume: &jobsentinel_documents::Resume) -> ResumeEvidenceCitation {
    ResumeEvidenceCitation::for_field(
        &ResumeEvidenceSnapshot {
            source_id: format!("resume:{}", resume.id),
            revision: resume.updated_at.to_rfc3339(),
        },
        "skills.0",
    )
    .unwrap()
}

#[tokio::test]
async fn every_skill_mutation_advances_the_resume_snapshot() {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let matcher = database.resume_matcher();
    let resume_id =
        sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
            .execute(database.pool())
            .await
            .unwrap()
            .last_insert_rowid();
    let before = matcher.get_resume(resume_id).await.unwrap();

    let skill_id = matcher
        .add_user_skill(
            resume_id,
            types::NewSkill {
                skill_name: "Scheduling".to_string(),
                ..types::NewSkill::default()
            },
        )
        .await
        .unwrap();
    let added = matcher.get_resume(resume_id).await.unwrap();
    assert!(added.updated_at > before.updated_at);
    assert_ne!(citation(&added).evidence_id, citation(&before).evidence_id);

    matcher
        .update_user_skill(
            skill_id,
            types::SkillUpdate {
                skill_name: Some("Case management".to_string()),
                ..types::SkillUpdate::default()
            },
        )
        .await
        .unwrap();
    let updated = matcher.get_resume(resume_id).await.unwrap();
    assert!(updated.updated_at > added.updated_at);
    assert_ne!(citation(&updated).evidence_id, citation(&added).evidence_id);

    matcher.delete_user_skill(skill_id).await.unwrap();
    let deleted = matcher.get_resume(resume_id).await.unwrap();
    assert!(deleted.updated_at > updated.updated_at);
    assert_ne!(
        citation(&deleted).evidence_id,
        citation(&updated).evidence_id
    );
}

#[tokio::test]
async fn activating_a_resume_preserves_its_evidence_snapshot() {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let matcher = database.resume_matcher();
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, updated_at)
         VALUES ('Resume', 'resume.txt', '2099-01-01 12:34:56.500')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();

    matcher
        .add_user_skill(
            resume_id,
            types::NewSkill {
                skill_name: "Scheduling".to_string(),
                ..types::NewSkill::default()
            },
        )
        .await
        .unwrap();
    let before_activation = matcher.get_resume(resume_id).await.unwrap();

    matcher.set_active_resume(resume_id).await.unwrap();
    let after_activation = matcher.get_resume(resume_id).await.unwrap();

    assert_eq!(after_activation.updated_at, before_activation.updated_at);
    assert_eq!(
        citation(&after_activation).evidence_id,
        citation(&before_activation).evidence_id
    );
}

#[tokio::test]
async fn extracted_skills_and_their_snapshot_revision_commit_together() {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let matcher = database.resume_matcher();
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text)
         VALUES ('Resume', 'resume.txt', 'Rust and PostgreSQL')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    let before = matcher.get_resume(resume_id).await.unwrap();

    let skills = matcher.extract_skills(resume_id).await.unwrap();
    let after = matcher.get_resume(resume_id).await.unwrap();

    assert!(skills.iter().any(|skill| skill.skill_name == "Rust"));
    assert!(skills.iter().any(|skill| skill.skill_name == "PostgreSQL"));
    assert!(after.updated_at > before.updated_at);
    assert_ne!(citation(&after).evidence_id, citation(&before).evidence_id);
}

#[tokio::test]
async fn resume_evidence_snapshot_reads_current_revision_without_resume_contents() {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let matcher = database.resume_matcher();
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, updated_at)
         VALUES ('Private resume', '/private/path', 'private resume text',
                 '2026-07-19 12:00:00.000')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();

    let first = matcher
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.source_id, format!("resume:{resume_id}"));
    assert_eq!(first.revision, "2026-07-19T12:00:00+00:00");

    sqlx::query(
        "UPDATE resumes
         SET updated_at = '2026-07-19 12:00:01.000'
         WHERE id = ?",
    )
    .bind(resume_id)
    .execute(database.pool())
    .await
    .unwrap();

    let second = matcher
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(first, second);
    assert!(matcher
        .get_resume_evidence_snapshot(resume_id + 1)
        .await
        .unwrap()
        .is_none());
}
