use super::{reviewed_export_tests::*, *};
use serde_json::json;

#[tokio::test]
async fn reviewed_export_includes_durable_evidence_packets_without_citation_paths() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        database_with_private_export_data(&temp_dir.path().join("primary/jobs.db")).await;
    let export_path = temp_dir.path().join("evidence-packets.jsonl");
    let case_file_id = "11111111-1111-1111-1111-111111111111";
    let packet_id = "22222222-2222-2222-2222-222222222222";
    let claim_id = "33333333-3333-3333-3333-333333333333";
    let evidence_id = "a".repeat(64);

    sqlx::query(
        "INSERT INTO opportunity_case_files(case_file_id, job_hash, created_at)
         VALUES (?, 'export-job', '2026-07-21T12:00:00Z')",
    )
    .bind(case_file_id)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_evidence_packets(
            packet_id, case_file_id, resume_id, resume_revision, job_revision,
            claim_id, reviewed_text, local_only, sensitive, created_at
         ) VALUES (?, ?, 7, '2026-07-20T12:00:00Z', '2026-07-21T12:00:00Z',
                   ?, 'Led a twelve-person operations team.', 1, 1, '2026-07-21T12:01:00Z')",
    )
    .bind(packet_id)
    .bind(case_file_id)
    .bind(claim_id)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_evidence_packet_evidence(packet_id, ordinal, evidence_id)
         VALUES (?, 0, ?)",
    )
    .bind(packet_id)
    .bind(&evidence_id)
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v3_evidence_packet_boundaries(packet_id, ordinal, boundary)
         VALUES (?, 0, 'clearance_currentness_unverified')",
    )
    .bind(packet_id)
    .execute(database.pool())
    .await
    .unwrap();

    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();
    assert_eq!(plan.record_count("campaign"), Some(4));
    assert_eq!(
        plan.total_record_count(),
        plan.section_counts().values().sum::<u64>()
    );
    let info = database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap();
    assert_eq!(
        Database::inspect_reviewed_export(&export_path).unwrap(),
        info
    );

    let exported = records(&export_path);
    for table in [
        "v3_evidence_packets",
        "v3_evidence_packet_evidence",
        "v3_evidence_packet_boundaries",
    ] {
        assert_eq!(find_record(&exported, table)["section"], "campaign");
    }
    assert_eq!(
        find_record(&exported, "v3_evidence_packets")["data"],
        json!({
            "packet_id": packet_id,
            "case_file_id": case_file_id,
            "resume_id": 7,
            "resume_revision": "2026-07-20T12:00:00Z",
            "job_revision": "2026-07-21T12:00:00Z",
            "claim_id": claim_id,
            "reviewed_text": "Led a twelve-person operations team.",
            "local_only": 1,
            "sensitive": 1,
            "created_at": "2026-07-21T12:01:00Z",
        })
    );
    assert_eq!(
        find_record(&exported, "v3_evidence_packet_evidence")["data"],
        json!({"packet_id": packet_id, "ordinal": 0, "evidence_id": evidence_id})
    );
    assert_eq!(
        find_record(&exported, "v3_evidence_packet_boundaries")["data"],
        json!({
            "packet_id": packet_id,
            "ordinal": 0,
            "boundary": "clearance_currentness_unverified",
        })
    );

    let raw = std::fs::read_to_string(export_path).unwrap();
    for forbidden in [
        "field_path",
        "packet-private-path",
        "packet-raw-resume-text",
    ] {
        assert!(!raw.contains(forbidden), "export leaked {forbidden}");
    }
}

#[tokio::test]
async fn protected_records_require_a_separate_review_selection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        database_with_private_export_data(&temp_dir.path().join("primary/jobs.db")).await;
    let export_path = temp_dir.path().join("protected.jsonl");
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::including_protected_records())
        .await
        .unwrap();

    assert!(plan.protected_records_included());
    assert!(plan.record_count("protected_answers").unwrap() > 0);
    database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap();

    let raw = std::fs::read_to_string(export_path).unwrap();
    assert!(raw.contains("export-protected-answer-marker"));
    assert!(raw.contains("\"us_work_authorized\":1"));
    assert!(raw.contains("draft-clearance-marker"));
    assert!(raw.contains("draft-military-marker"));
    assert!(raw.contains("draft-security-clearance-marker"));
    assert!(raw.contains("draft-military-service-marker"));
    assert!(raw.contains("draft-protected-veteran-marker"));
    assert!(raw.contains("draft-disability-marker"));
}
