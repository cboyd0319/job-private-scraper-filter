use super::*;
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};

struct SavedMatchFixture {
    database: crate::Database,
    job_hash: String,
    resume_id: i64,
    saved_match_id: i64,
    job_revision: String,
    snapshot: ResumeEvidenceSnapshot,
    skills: Vec<String>,
    citation: ResumeEvidenceCitation,
}

async fn saved_match_fixture() -> SavedMatchFixture {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    crate::test_support::insert_test_job(
        database.pool(),
        "job-saved-match",
        "Office Assistant",
        Some("Example"),
        None,
        "2026-07-19T12:00:00Z",
    )
    .await;
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, updated_at)
         VALUES ('Resume', 'resume.txt', 'Reviewed local resume', '2026-07-19T12:00:00Z')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    sqlx::query(
        "INSERT INTO user_skills (resume_id, skill_name, confidence_score, source)
         VALUES (?, 'Scheduling', 1.0, 'user_input')",
    )
    .bind(resume_id)
    .execute(database.pool())
    .await
    .unwrap();
    let saved_match_id = sqlx::query(
        "INSERT INTO resume_job_matches (
            resume_id, job_hash, overall_match_score, skills_match_score
         ) VALUES (?, 'job-saved-match', 0.8, 0.8)",
    )
    .bind(resume_id)
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    let snapshot = ResumeEvidenceSnapshot {
        source_id: format!("resume:{resume_id}"),
        revision: "2026-07-19T12:00:00+00:00".to_string(),
    };
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "summary").unwrap();
    SavedMatchFixture {
        database,
        job_hash: "job-saved-match".to_string(),
        resume_id,
        saved_match_id,
        job_revision: "2026-07-19T12:00:00+00:00".to_string(),
        snapshot,
        skills: vec!["Scheduling".to_string()],
        citation,
    }
}

fn confirmation(fixture: &SavedMatchFixture) -> SavedMatchEvidenceConfirmation {
    SavedMatchEvidenceConfirmation {
        job_hash: fixture.job_hash.clone(),
        resume_id: fixture.resume_id,
        saved_match_id: fixture.saved_match_id,
        expected_job_revision: fixture.job_revision.clone(),
        expected_resume_snapshot: fixture.snapshot.clone(),
        expected_skills: fixture.skills.clone(),
        citation: fixture.citation.clone(),
    }
}

async fn case_count(database: &crate::Database) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM opportunity_case_files")
        .fetch_one(database.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn saved_match_confirmation_is_atomic_current_and_idempotent() {
    let fixture = saved_match_fixture().await;
    let input = confirmation(&fixture);

    assert!(fixture
        .database
        .confirm_saved_match_evidence(&input)
        .await
        .unwrap());
    assert!(!fixture
        .database
        .confirm_saved_match_evidence(&input)
        .await
        .unwrap());
    assert_eq!(case_count(&fixture.database).await, 1);
    let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM career_graph_links")
        .fetch_one(fixture.database.pool())
        .await
        .unwrap();
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_job_events")
        .fetch_one(fixture.database.pool())
        .await
        .unwrap();
    assert_eq!((links, events), (1, 1));
}

#[tokio::test]
async fn saved_match_confirmation_rejects_stale_job_resume_and_skills_without_case() {
    let stale_job = saved_match_fixture().await;
    sqlx::query("UPDATE jobs SET updated_at = '2026-07-19T12:00:01Z'")
        .execute(stale_job.database.pool())
        .await
        .unwrap();
    assert!(stale_job
        .database
        .confirm_saved_match_evidence(&confirmation(&stale_job))
        .await
        .is_err());
    assert_eq!(case_count(&stale_job.database).await, 0);

    let stale_resume = saved_match_fixture().await;
    sqlx::query("UPDATE resumes SET updated_at = '2026-07-19T12:00:01Z'")
        .execute(stale_resume.database.pool())
        .await
        .unwrap();
    assert!(stale_resume
        .database
        .confirm_saved_match_evidence(&confirmation(&stale_resume))
        .await
        .is_err());
    assert_eq!(case_count(&stale_resume.database).await, 0);

    let stale_skills = saved_match_fixture().await;
    sqlx::query(
        "INSERT INTO user_skills (resume_id, skill_name, confidence_score, source)
         VALUES (?, 'Reports', 1.0, 'user_input')",
    )
    .bind(stale_skills.resume_id)
    .execute(stale_skills.database.pool())
    .await
    .unwrap();
    assert!(stale_skills
        .database
        .confirm_saved_match_evidence(&confirmation(&stale_skills))
        .await
        .is_err());
    assert_eq!(case_count(&stale_skills.database).await, 0);
}

#[tokio::test]
async fn saved_match_confirmation_rejects_missing_match_and_forged_citation_without_case() {
    let missing = saved_match_fixture().await;
    sqlx::query("DELETE FROM resume_job_matches WHERE id = ?")
        .bind(missing.saved_match_id)
        .execute(missing.database.pool())
        .await
        .unwrap();
    assert!(missing
        .database
        .confirm_saved_match_evidence(&confirmation(&missing))
        .await
        .is_err());
    assert_eq!(case_count(&missing.database).await, 0);

    let forged = saved_match_fixture().await;
    let mut forged_input = confirmation(&forged);
    forged_input.citation = ResumeEvidenceCitation::for_field(
        &ResumeEvidenceSnapshot {
            source_id: forged.snapshot.source_id.clone(),
            revision: "2026-07-19T12:00:01+00:00".to_string(),
        },
        "summary",
    )
    .unwrap();
    assert!(forged
        .database
        .confirm_saved_match_evidence(&forged_input)
        .await
        .is_err());
    assert_eq!(case_count(&forged.database).await, 0);
}

#[tokio::test]
async fn saved_match_confirmation_rolls_back_the_new_case_when_event_write_fails() {
    let fixture = saved_match_fixture().await;
    sqlx::query(
        "CREATE TRIGGER block_saved_match_event
         BEFORE INSERT ON v3_job_events
         WHEN NEW.event_kind = 'evidence_linked'
         BEGIN SELECT RAISE(ABORT, 'blocked'); END",
    )
    .execute(fixture.database.pool())
    .await
    .unwrap();

    assert!(fixture
        .database
        .confirm_saved_match_evidence(&confirmation(&fixture))
        .await
        .is_err());
    assert_eq!(case_count(&fixture.database).await, 0);
}

#[tokio::test]
async fn saved_match_packet_reload_resolves_the_hidden_case_and_rejects_stale_context() {
    let fixture = saved_match_fixture().await;
    assert_eq!(
        fixture
            .database
            .list_saved_match_evidence_packet_claims(&fixture.job_hash, fixture.resume_id)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Current(Vec::new())
    );
    fixture
        .database
        .confirm_saved_match_evidence(&confirmation(&fixture))
        .await
        .unwrap();
    let case_file = fixture
        .database
        .ensure_case_file(&fixture.job_hash)
        .await
        .unwrap();
    fixture
        .database
        .persist_evidence_bound_packet_claim(&NewEvidenceBoundPacketClaim {
            case_file_id: case_file.case_file_id,
            claim_id: Uuid::new_v4().to_string(),
            reviewed_text: "Reviewed packet claim.".to_string(),
            resume_snapshot: fixture.snapshot.clone(),
            job_revision: fixture.job_revision.clone(),
            evidence_ids: vec![fixture.citation.evidence_id.clone()],
            citations: vec![fixture.citation.clone()],
            boundaries: Vec::new(),
        })
        .await
        .unwrap();
    assert!(matches!(
        fixture
            .database
            .list_saved_match_evidence_packet_claims(&fixture.job_hash, fixture.resume_id)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Current(records) if records.len() == 1
    ));
    sqlx::query("UPDATE jobs SET updated_at = '2026-07-19T12:00:01Z'")
        .execute(fixture.database.pool())
        .await
        .unwrap();
    assert_eq!(
        fixture
            .database
            .list_saved_match_evidence_packet_claims(&fixture.job_hash, fixture.resume_id)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Invalid
    );
    sqlx::query("DELETE FROM resume_job_matches WHERE id = ?")
        .bind(fixture.saved_match_id)
        .execute(fixture.database.pool())
        .await
        .unwrap();
    assert!(fixture
        .database
        .list_saved_match_evidence_packet_claims(&fixture.job_hash, fixture.resume_id)
        .await
        .is_err());
}

#[tokio::test]
async fn saved_match_confirmed_evidence_reload_returns_only_paired_local_ids() {
    let fixture = saved_match_fixture().await;
    let before = fixture
        .database
        .read_saved_match_confirmed_evidence(&fixture.job_hash, fixture.resume_id)
        .await
        .unwrap();
    assert_eq!(before.case_file_id(), None);
    assert!(before.evidence_ids().is_empty());
    assert!(fixture
        .database
        .confirm_saved_match_evidence(&confirmation(&fixture))
        .await
        .unwrap());
    assert!(!fixture
        .database
        .confirm_saved_match_evidence(&confirmation(&fixture))
        .await
        .unwrap());
    let case_file = fixture
        .database
        .ensure_case_file(&fixture.job_hash)
        .await
        .unwrap();
    let forged_id = "a".repeat(64);
    sqlx::query(
        "INSERT INTO career_graph_links (
            link_id, subject_id, relation, object_id, provenance, provenance_ref, created_at
         ) VALUES (?, ?, 'evidence', ?, 'user_confirmed', NULL, '2026-07-19T12:00:00Z')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&case_file.case_file_id)
    .bind(&forged_id)
    .execute(fixture.database.pool())
    .await
    .unwrap();
    let event_only_id = "b".repeat(64);
    sqlx::query(
        "INSERT INTO v3_job_events (
            event_id, case_file_id, event_kind, origin, user_action,
            local_only, sensitive, metadata_json, created_at
         ) VALUES (?, ?, 'evidence_linked', 'user', 1, 1, 1, ?, '2026-07-19T12:00:00Z')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&case_file.case_file_id)
    .bind(serde_json::json!({"kind": "local_reference", "reference_id": event_only_id}).to_string())
    .execute(fixture.database.pool())
    .await
    .unwrap();
    let after = fixture
        .database
        .read_saved_match_confirmed_evidence(&fixture.job_hash, fixture.resume_id)
        .await
        .unwrap();
    assert_eq!(after.case_file_id(), Some(case_file.case_file_id.as_str()));
    assert_eq!(after.evidence_ids(), [fixture.citation.evidence_id.clone()]);
    sqlx::query("DELETE FROM resume_job_matches WHERE id = ?")
        .bind(fixture.saved_match_id)
        .execute(fixture.database.pool())
        .await
        .unwrap();
    assert!(fixture
        .database
        .read_saved_match_confirmed_evidence(&fixture.job_hash, fixture.resume_id)
        .await
        .is_err());
}
