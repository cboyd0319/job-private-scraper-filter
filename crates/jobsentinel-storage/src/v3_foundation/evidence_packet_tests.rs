use super::*;
use jobsentinel_domain::{
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata,
        EventOrigin, GraphProvenance,
    },
    v3_manifests::PrivacyLabel,
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};

struct PacketFixture {
    database: crate::Database,
    case_file_id: String,
    resume_id: i64,
    snapshot: ResumeEvidenceSnapshot,
    citation: ResumeEvidenceCitation,
    job_revision: String,
}

async fn packet_fixture() -> PacketFixture {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    crate::test_support::insert_test_job(
        database.pool(),
        "job-packet",
        "Office Assistant",
        Some("Example"),
        None,
        "2026-07-19T12:00:00Z",
    )
    .await;
    let case_file = database.ensure_case_file("job-packet").await.unwrap();
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text, updated_at)
         VALUES ('Resume', 'resume.txt', 'Reviewed local resume', '2026-07-19T12:00:00Z')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    let snapshot = ResumeEvidenceSnapshot {
        source_id: format!("resume:{resume_id}"),
        revision: "2026-07-19T12:00:00+00:00".to_string(),
    };
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "summary").unwrap();
    let link = CareerGraphLink {
        link_id: Uuid::new_v4().to_string(),
        subject_id: case_file.case_file_id.clone(),
        relation: CareerRelation::Evidence,
        object_id: citation.evidence_id.clone(),
        provenance: GraphProvenance::UserConfirmed,
        provenance_ref: None,
    };
    let confirmation = CaseFileEventInput {
        case_file_id: case_file.case_file_id.clone(),
        kind: CaseFileEventKind::EvidenceLinked,
        origin: EventOrigin::User,
        user_action: true,
        privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        metadata: EventMetadata::LocalReference {
            reference_id: citation.evidence_id.clone(),
        },
    };
    database
        .insert_case_file_evidence(&link, &confirmation)
        .await
        .unwrap();
    PacketFixture {
        database,
        case_file_id: case_file.case_file_id,
        resume_id,
        snapshot,
        citation,
        job_revision: "2026-07-19T12:00:00+00:00".to_string(),
    }
}

fn packet_input(fixture: &PacketFixture) -> NewEvidenceBoundPacketClaim {
    NewEvidenceBoundPacketClaim {
        case_file_id: fixture.case_file_id.clone(),
        claim_id: Uuid::new_v4().to_string(),
        reviewed_text: "Led a twelve-person operations team.".to_string(),
        resume_snapshot: fixture.snapshot.clone(),
        job_revision: fixture.job_revision.clone(),
        evidence_ids: vec![fixture.citation.evidence_id.clone()],
        citations: vec![fixture.citation.clone()],
        boundaries: vec![EvidencePacketBoundary::ClearanceCurrentnessUnverified],
    }
}

#[tokio::test]
async fn packet_claim_accepts_normal_sqlite_default_timestamp_rows() {
    let database = crate::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    sqlx::query(
        "INSERT INTO jobs (hash, title, company, url, source)
         VALUES ('job-default-time', 'Office Assistant', 'Example', 'https://example.test/job', 'test')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    let case_file = database.ensure_case_file("job-default-time").await.unwrap();
    let resume_id = sqlx::query(
        "INSERT INTO resumes (name, file_path, parsed_text)
         VALUES ('Resume', 'resume.txt', 'Reviewed local resume')",
    )
    .execute(database.pool())
    .await
    .unwrap()
    .last_insert_rowid();
    let (snapshot, _, _) = database
        .read_case_file_resume_evidence_context(&case_file.case_file_id, resume_id)
        .await
        .unwrap()
        .unwrap();
    let citation = ResumeEvidenceCitation::for_field(&snapshot, "summary").unwrap();
    database
        .insert_case_file_evidence(
            &CareerGraphLink {
                link_id: Uuid::new_v4().to_string(),
                subject_id: case_file.case_file_id.clone(),
                relation: CareerRelation::Evidence,
                object_id: citation.evidence_id.clone(),
                provenance: GraphProvenance::UserConfirmed,
                provenance_ref: None,
            },
            &CaseFileEventInput {
                case_file_id: case_file.case_file_id.clone(),
                kind: CaseFileEventKind::EvidenceLinked,
                origin: EventOrigin::User,
                user_action: true,
                privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
                metadata: EventMetadata::LocalReference {
                    reference_id: citation.evidence_id.clone(),
                },
            },
        )
        .await
        .unwrap();
    let job_revision: String =
        sqlx::query_scalar("SELECT updated_at FROM jobs WHERE hash = 'job-default-time'")
            .fetch_one(database.pool())
            .await
            .unwrap();
    let job_revision = crate::sqlite_time::parse_sqlite_datetime(&job_revision)
        .unwrap()
        .to_rfc3339();
    let input = NewEvidenceBoundPacketClaim {
        case_file_id: case_file.case_file_id.clone(),
        claim_id: Uuid::new_v4().to_string(),
        reviewed_text: "Reviewed packet claim.".to_string(),
        resume_snapshot: snapshot,
        job_revision,
        evidence_ids: vec![citation.evidence_id.clone()],
        citations: vec![citation],
        boundaries: Vec::new(),
    };
    database
        .persist_evidence_bound_packet_claim(&input)
        .await
        .unwrap();
    assert!(matches!(
        database
            .list_current_evidence_bound_packet_claims(&case_file.case_file_id, resume_id)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Current(records) if records.len() == 1
    ));
}

#[tokio::test]
async fn packet_claim_persists_only_reviewed_text_and_ordered_opaque_evidence() {
    let fixture = packet_fixture().await;
    let input = packet_input(&fixture);

    let stored = fixture
        .database
        .persist_evidence_bound_packet_claim(&input)
        .await
        .unwrap();
    assert_eq!(stored.case_file_id(), fixture.case_file_id);
    assert_eq!(stored.claim_id(), input.claim_id);
    assert_eq!(stored.reviewed_text(), input.reviewed_text);
    assert_eq!(
        stored.evidence_ids(),
        [fixture.citation.evidence_id.clone()]
    );
    assert_eq!(
        stored.boundaries(),
        [EvidencePacketBoundary::ClearanceCurrentnessUnverified]
    );
    assert_eq!(
        fixture
            .database
            .list_current_evidence_bound_packet_claims(&fixture.case_file_id, fixture.resume_id,)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Current(vec![stored])
    );
    let persisted: (String, String, String, bool, bool, String) = sqlx::query_as(
        "SELECT resume_revision, job_revision, reviewed_text, local_only, sensitive,
                (SELECT evidence_id FROM v3_evidence_packet_evidence
                 WHERE packet_id = v3_evidence_packets.packet_id)
         FROM v3_evidence_packets",
    )
    .fetch_one(fixture.database.pool())
    .await
    .unwrap();
    assert_eq!(persisted.0, fixture.snapshot.revision);
    assert_eq!(persisted.1, fixture.job_revision);
    assert_eq!(persisted.2, input.reviewed_text);
    assert!(persisted.3 && persisted.4);
    assert_eq!(persisted.5, fixture.citation.evidence_id);
}

async fn packet_count(database: &crate::Database) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM v3_evidence_packets")
        .fetch_one(database.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn packet_claim_rejects_each_invalid_input_without_writes() {
    let stale_resume = packet_fixture().await;
    let stale_resume_input = packet_input(&stale_resume);
    sqlx::query("UPDATE resumes SET updated_at = '2026-07-19T12:00:01Z'")
        .execute(stale_resume.database.pool())
        .await
        .unwrap();
    assert!(stale_resume
        .database
        .persist_evidence_bound_packet_claim(&stale_resume_input)
        .await
        .is_err());
    assert_eq!(packet_count(&stale_resume.database).await, 0);

    let stale_job = packet_fixture().await;
    let stale_job_input = packet_input(&stale_job);
    sqlx::query("UPDATE jobs SET updated_at = '2026-07-19T12:00:01Z' WHERE hash = 'job-packet'")
        .execute(stale_job.database.pool())
        .await
        .unwrap();
    assert!(stale_job
        .database
        .persist_evidence_bound_packet_claim(&stale_job_input)
        .await
        .is_err());
    assert_eq!(packet_count(&stale_job.database).await, 0);

    let unconfirmed = packet_fixture().await;
    let mut unconfirmed_input = packet_input(&unconfirmed);
    let unconfirmed_citation =
        ResumeEvidenceCitation::for_field(&unconfirmed.snapshot, "clearance").unwrap();
    unconfirmed_input.evidence_ids = vec![unconfirmed_citation.evidence_id.clone()];
    unconfirmed_input.citations = vec![unconfirmed_citation];
    assert!(unconfirmed
        .database
        .persist_evidence_bound_packet_claim(&unconfirmed_input)
        .await
        .is_err());
    assert_eq!(packet_count(&unconfirmed.database).await, 0);

    let duplicate = packet_fixture().await;
    let mut duplicate_input = packet_input(&duplicate);
    duplicate_input.citations = vec![duplicate.citation.clone(), duplicate.citation.clone()];
    duplicate_input.evidence_ids = vec![
        duplicate.citation.evidence_id.clone(),
        duplicate.citation.evidence_id.clone(),
    ];
    assert!(duplicate
        .database
        .persist_evidence_bound_packet_claim(&duplicate_input)
        .await
        .is_err());
    assert_eq!(packet_count(&duplicate.database).await, 0);

    let oversize = packet_fixture().await;
    let mut oversize_input = packet_input(&oversize);
    oversize_input.reviewed_text = "x".repeat(8_193);
    assert!(oversize
        .database
        .persist_evidence_bound_packet_claim(&oversize_input)
        .await
        .is_err());
    assert_eq!(packet_count(&oversize.database).await, 0);
}

#[tokio::test]
async fn duplicate_packet_claim_id_has_no_partial_write() {
    let fixture = packet_fixture().await;
    let first = packet_input(&fixture);
    fixture
        .database
        .persist_evidence_bound_packet_claim(&first)
        .await
        .unwrap();
    let mut duplicate = packet_input(&fixture);
    duplicate.claim_id = first.claim_id;

    assert!(fixture
        .database
        .persist_evidence_bound_packet_claim(&duplicate)
        .await
        .is_err());
    assert_eq!(packet_count(&fixture.database).await, 1);
    let evidence: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_evidence_packet_evidence")
        .fetch_one(fixture.database.pool())
        .await
        .unwrap();
    assert_eq!(evidence, 1);
}

#[tokio::test]
async fn packet_claim_rolls_back_on_child_write_failure_and_reads_stale_packets_as_invalid() {
    let fixture = packet_fixture().await;
    let input = packet_input(&fixture);
    sqlx::query(
        "CREATE TRIGGER block_packet_evidence
         BEFORE INSERT ON v3_evidence_packet_evidence
         BEGIN SELECT RAISE(ABORT, 'blocked'); END",
    )
    .execute(fixture.database.pool())
    .await
    .unwrap();
    assert!(fixture
        .database
        .persist_evidence_bound_packet_claim(&input)
        .await
        .is_err());
    let packets: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_evidence_packets")
        .fetch_one(fixture.database.pool())
        .await
        .unwrap();
    assert_eq!(packets, 0);
    sqlx::query("DROP TRIGGER block_packet_evidence")
        .execute(fixture.database.pool())
        .await
        .unwrap();
    fixture
        .database
        .persist_evidence_bound_packet_claim(&input)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET updated_at = '2026-07-19T12:00:01Z' WHERE hash = 'job-packet'")
        .execute(fixture.database.pool())
        .await
        .unwrap();
    assert_eq!(
        fixture
            .database
            .list_current_evidence_bound_packet_claims(&fixture.case_file_id, fixture.resume_id,)
            .await
            .unwrap(),
        EvidenceBoundPacketClaimsRead::Invalid
    );
}
