use jobsentinel_domain::{
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata,
        EventOrigin, GraphProvenance,
    },
    v3_manifests::PrivacyLabel,
};

async fn case_evidence_database() -> (tempfile::TempDir, crate::Database, String) {
    let temp_dir = tempfile::tempdir().unwrap();
    let database = crate::Database::connect(&temp_dir.path().join("jobs.db"))
        .await
        .unwrap();
    database.migrate().await.unwrap();
    crate::test_support::insert_test_job(
        database.pool(),
        "job-1",
        "Office Assistant",
        Some("Example"),
        None,
        "2026-07-19T00:00:00Z",
    )
    .await;
    let case_file = database.ensure_case_file("job-1").await.unwrap();
    (temp_dir, database, case_file.case_file_id)
}

fn reviewed_evidence(
    case_file_id: &str,
    evidence_id: &str,
) -> (CareerGraphLink, CaseFileEventInput) {
    (
        CareerGraphLink {
            link_id: uuid::Uuid::new_v4().to_string(),
            subject_id: case_file_id.to_string(),
            relation: CareerRelation::Evidence,
            object_id: evidence_id.to_string(),
            provenance: GraphProvenance::UserConfirmed,
            provenance_ref: None,
        },
        CaseFileEventInput {
            case_file_id: case_file_id.to_string(),
            kind: CaseFileEventKind::EvidenceLinked,
            origin: EventOrigin::User,
            user_action: true,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::LocalReference {
                reference_id: evidence_id.to_string(),
            },
        },
    )
}

#[tokio::test]
async fn reviewed_case_evidence_is_concurrent_idempotent_typed_and_opaque() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    assert!(database.pool().options().get_max_connections() > 1);
    let evidence_id = "a".repeat(64);
    let (first_link, first_event) = reviewed_evidence(&case_file_id, &evidence_id);
    let (duplicate_link, duplicate_event) = reviewed_evidence(&case_file_id, &evidence_id);

    let (first, duplicate) = tokio::join!(
        database.insert_case_file_evidence(&first_link, &first_event),
        database.insert_case_file_evidence(&duplicate_link, &duplicate_event),
    );
    assert_eq!(
        [first.unwrap(), duplicate.unwrap()]
            .into_iter()
            .filter(|created| *created)
            .count(),
        1
    );
    let links = database
        .list_case_file_evidence_links(&case_file_id)
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert!(links[0].link_id == first_link.link_id || links[0].link_id == duplicate_link.link_id);
    assert_eq!(links[0].subject_id, case_file_id);
    assert_eq!(links[0].object_id, evidence_id);
    assert_eq!(links[0].relation, CareerRelation::Evidence);
    assert_eq!(links[0].provenance, GraphProvenance::UserConfirmed);
    let events = database.list_case_file_events(&case_file_id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].metadata,
        EventMetadata::LocalReference {
            reference_id: evidence_id
        }
    );
    assert!(database
        .list_case_file_evidence_links("missing-case")
        .await
        .is_err());
}

#[tokio::test]
async fn generic_graph_insert_cannot_bypass_case_evidence_confirmation() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    let evidence_id = "b".repeat(64);
    let (link, _) = reviewed_evidence(&case_file_id, &evidence_id);

    assert!(database.insert_career_graph_link(&link).await.is_err());
    assert!(database
        .list_case_file_evidence_links(&case_file_id)
        .await
        .unwrap()
        .is_empty());
    assert!(database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn generic_event_insert_cannot_bypass_case_evidence_confirmation() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    let (_, event) = reviewed_evidence(&case_file_id, &"b".repeat(64));

    assert!(database.append_case_file_event(&event).await.is_err());
    assert!(database
        .list_case_file_evidence_links(&case_file_id)
        .await
        .unwrap()
        .is_empty());
    assert!(database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn case_evidence_rejects_semantic_identifiers_without_writes() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    let (semantic_evidence, semantic_event) = reviewed_evidence(&case_file_id, "military_info");
    assert!(database
        .insert_case_file_evidence(&semantic_evidence, &semantic_event)
        .await
        .is_err());

    let (mut semantic_link, event) = reviewed_evidence(&case_file_id, &"c".repeat(64));
    semantic_link.link_id = "veteran-clearance".to_string();
    assert!(database
        .insert_case_file_evidence(&semantic_link, &event)
        .await
        .is_err());

    assert!(database
        .list_case_file_evidence_links(&case_file_id)
        .await
        .unwrap()
        .is_empty());
    assert!(database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn missing_case_rolls_back_its_evidence_link() {
    let (_temp_dir, database, _) = case_evidence_database().await;
    let (link, confirmation) = reviewed_evidence("missing-case", &"d".repeat(64));

    assert!(database
        .insert_case_file_evidence(&link, &confirmation)
        .await
        .is_err());
    let stored: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM career_graph_links WHERE link_id = ?")
            .bind(&link.link_id)
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn case_evidence_rolls_back_when_its_confirmation_event_fails() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    sqlx::query(
        "CREATE TRIGGER block_evidence_event
         BEFORE INSERT ON v3_job_events
         WHEN NEW.event_kind = 'evidence_linked'
         BEGIN SELECT RAISE(ABORT, 'blocked'); END",
    )
    .execute(database.pool())
    .await
    .unwrap();
    let (link, event) = reviewed_evidence(&case_file_id, &"e".repeat(64));

    assert!(database
        .insert_case_file_evidence(&link, &event)
        .await
        .is_err());
    assert!(database
        .list_case_file_evidence_links(&case_file_id)
        .await
        .unwrap()
        .is_empty());
    assert!(database
        .list_case_file_events(&case_file_id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn packet_context_reads_resume_revision_and_case_evidence_consistently() {
    let (_temp_dir, database, case_file_id) = case_evidence_database().await;
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, updated_at)
         VALUES ('Resume', 'resume.txt', 'Resume text', '2026-07-19 12:00:00.000')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    let evidence_id = "f".repeat(64);
    let (link, event) = reviewed_evidence(&case_file_id, &evidence_id);
    database
        .insert_case_file_evidence(&link, &event)
        .await
        .unwrap();

    let mut writer = database.pool().begin().await.unwrap();
    sqlx::query(
        "UPDATE resumes
         SET updated_at = '2026-07-19 12:00:01.000'
         WHERE id = ?",
    )
    .bind(resume_id)
    .execute(&mut *writer)
    .await
    .unwrap();
    let (before_commit, resume_text, links) = database
        .read_case_file_resume_evidence_context(&case_file_id, resume_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(before_commit.revision, "2026-07-19T12:00:00+00:00");
    assert_eq!(resume_text.as_deref(), Some("Resume text"));
    assert_eq!(links, vec![link]);

    writer.commit().await.unwrap();
    let (after_commit, _, _) = database
        .read_case_file_resume_evidence_context(&case_file_id, resume_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after_commit.revision, "2026-07-19T12:00:01+00:00");
    sqlx::query("UPDATE resumes SET parsed_text = 'Changed text' WHERE id = ?")
        .bind(resume_id)
        .execute(database.pool())
        .await
        .unwrap();
    let (same_revision, changed_text, _) = database
        .read_case_file_resume_evidence_context(&case_file_id, resume_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(same_revision, after_commit);
    assert_eq!(changed_text.as_deref(), Some("Changed text"));
    assert!(database
        .read_case_file_resume_evidence_context(&case_file_id, resume_id + 1)
        .await
        .unwrap()
        .is_none());
    assert!(database
        .read_case_file_resume_evidence_context("missing-case", resume_id)
        .await
        .is_err());
}
