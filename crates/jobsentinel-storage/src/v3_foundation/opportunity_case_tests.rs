//! Proves opportunity-case reads aggregate only bounded, privacy-safe local facts.

use crate::test_support::{insert_test_job, migrated_database};

#[tokio::test]
async fn read_aggregates_safe_case_state_without_private_payloads() {
    let database = migrated_database().await;
    let pool = database.pool();
    let case_time = "2026-07-19T00:00:00Z";
    let revision = "2026-07-20T00:00:00+00:00";
    insert_test_job(
        pool,
        "case-office-assistant",
        "Office Assistant",
        Some("Example"),
        Some("Denver, CO"),
        case_time,
    )
    .await;
    sqlx::query(
        "UPDATE jobs SET remote = 1, times_seen = 3, source = 'greenhouse',
                last_seen = '2026-07-20T00:00:00Z', updated_at = '2026-07-20T00:00:00Z',
                ghost_score = 0.7,
                ghost_reasons = '[\" Reposted frequently \",\"\",\"line\\nbreak\"]'
         WHERE hash = 'case-office-assistant'",
    )
    .execute(pool)
    .await
    .unwrap();
    let case_file = database
        .ensure_case_file("case-office-assistant")
        .await
        .unwrap();
    sqlx::query("UPDATE opportunity_case_files SET created_at = ? WHERE case_file_id = ?")
        .bind(case_time)
        .bind(&case_file.case_file_id)
        .execute(pool)
        .await
        .unwrap();

    let application_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO applications
             (job_hash, status, notes, recruiter_email, created_at, updated_at)
         VALUES
             ('case-office-assistant', 'offer_accepted', 'private-note-marker',
              'private-contact@example.test', ?, ?)
         RETURNING id",
    )
    .bind(case_time)
    .bind(case_time)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO interviews (application_id, scheduled_at, completed)
         VALUES (?, '2026-07-21T12:00:00Z', 0),
                (?, '2026-07-18T12:00:00Z', 1)",
    )
    .bind(application_id)
    .bind(application_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO career_graph_links
             (link_id, subject_id, relation, object_id, provenance, created_at)
         VALUES ('case-evidence-link', ?, 'evidence', 'evidence-1',
                 'user_confirmed', ?)",
    )
    .bind(&case_file.case_file_id)
    .bind(case_time)
    .execute(pool)
    .await
    .unwrap();

    let resume_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO resumes (name, file_path, parsed_text, created_at, updated_at)
         VALUES ('Private resume', '/private/resume-marker',
                 'private-resume-text-marker', ?, ?)
         RETURNING id",
    )
    .bind(case_time)
    .bind(revision)
    .fetch_one(pool)
    .await
    .unwrap();
    for (packet_id, claim_id, job_revision) in [
        (
            "00000000-0000-4000-8000-000000000001",
            "10000000-0000-4000-8000-000000000001",
            revision,
        ),
        (
            "00000000-0000-4000-8000-000000000002",
            "10000000-0000-4000-8000-000000000002",
            "2026-07-19T00:00:00+00:00",
        ),
    ] {
        sqlx::query(
            "INSERT INTO v3_evidence_packets
                 (packet_id, case_file_id, resume_id, resume_revision,
                  job_revision, claim_id, reviewed_text, created_at)
             VALUES (?, ?, ?, ?, ?, ?, 'private-reviewed-claim-marker', ?)",
        )
        .bind(packet_id)
        .bind(&case_file.case_file_id)
        .bind(resume_id)
        .bind(revision)
        .bind(job_revision)
        .bind(claim_id)
        .bind(case_time)
        .execute(pool)
        .await
        .unwrap();
    }

    for (event_id, event_kind, metadata_json) in [
        (
            "case-event-1-source",
            "source_checked",
            r#"{"kind":"source_outcome","source_id":"private-source-marker","outcome":"failure","item_count":0,"connectivity_required":true}"#,
        ),
        (
            "case-event-2-recovery",
            "recovery_recorded",
            r#"{"kind":"recovery_outcome","outcome":"restored"}"#,
        ),
    ] {
        sqlx::query(
            "INSERT INTO v3_job_events
                 (event_id, case_file_id, event_kind, origin, user_action,
                  local_only, sensitive, metadata_json, created_at)
             VALUES (?, ?, ?, 'system', 0, 1, 1, ?, ?)",
        )
        .bind(event_id)
        .bind(&case_file.case_file_id)
        .bind(event_kind)
        .bind(metadata_json)
        .bind(case_time)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO application_events
             (application_id, event_type, event_data, created_at)
         VALUES (?, 'note_added', 'private-event-payload-marker', ?)",
    )
    .bind(application_id)
    .bind(case_time)
    .execute(pool)
    .await
    .unwrap();

    let case = database
        .read_opportunity_case("case-office-assistant")
        .await
        .unwrap();

    assert_eq!(case.title, "Office Assistant");
    assert_eq!(case.company, "Example");
    assert_eq!(case.location.as_deref(), Some("Denver, CO"));
    assert_eq!(case.remote, Some(true));
    assert_eq!(case.times_seen, 3);
    assert_eq!(case.source_name, "greenhouse");
    assert_eq!(case.posting_risk_score, Some(0.7));
    assert_eq!(case.posting_risk_reasons, ["Reposted frequently"]);
    assert_eq!(case.application_status.as_deref(), Some("offer_accepted"));
    assert!(case.has_contact);
    assert_eq!(case.upcoming_interview_count, 1);
    assert_eq!(case.completed_interview_count, 1);
    assert_eq!(case.offer_status.as_deref(), Some("accepted"));
    assert_eq!(case.outcome_status.as_deref(), Some("offer_accepted"));
    assert_eq!(case.confirmed_evidence_count, 1);
    assert_eq!(case.current_packet_count, 1);
    assert_eq!(case.stale_packet_count, 1);
    let mut timeline_kinds = case
        .timeline
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    timeline_kinds.sort_unstable();
    assert_eq!(
        timeline_kinds,
        [
            "application_note_added",
            "case_created",
            "recovery_restored",
            "source_checked_failed",
        ]
    );
    let safe_read = format!("{case:?}");
    for private_marker in [
        "private-note-marker",
        "private-contact@example.test",
        "private-resume-text-marker",
        "private-reviewed-claim-marker",
        "private-source-marker",
        "private-event-payload-marker",
    ] {
        assert!(
            !safe_read.contains(private_marker),
            "leaked {private_marker}"
        );
    }
}

#[tokio::test]
async fn read_includes_dossier_pay_posting_and_exact_name_history_without_private_text() {
    let database = migrated_database().await;
    let pool = database.pool();
    let at = "2026-07-19T00:00:00Z";
    for (hash, company) in [
        ("dossier-anchor", "Example"),
        ("dossier-history", "Example"),
        ("dossier-unmerged", "example"),
    ] {
        insert_test_job(pool, hash, "Analyst", Some(company), None, at).await;
    }
    sqlx::query(
        "UPDATE jobs SET source = 'greenhouse', url = 'https://job-boards.greenhouse.io/example/jobs/1',
                first_seen = '2026-07-01T00:00:00Z', last_seen = '2026-07-20T00:00:00Z',
                times_seen = 4, repost_count = 2, salary_min = 100000,
                salary_max = 140000, currency = 'USD', notes = 'private-job-note-marker'
         WHERE hash = 'dossier-anchor'",
    )
    .execute(pool)
    .await
    .unwrap();
    database.ensure_case_file("dossier-anchor").await.unwrap();

    for (job_hash, status) in [
        ("dossier-anchor", "offer_accepted"),
        ("dossier-history", "applied"),
        ("dossier-unmerged", "rejected"),
    ] {
        let application_id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO applications
                 (job_hash, status, notes, recruiter_email, created_at, updated_at)
             VALUES (?, ?, 'private-application-note-marker',
                     'private-contact@example.test', ?, ?)
             RETURNING id",
        )
        .bind(job_hash)
        .bind(status)
        .bind(at)
        .bind(at)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO interviews
                 (application_id, scheduled_at, completed, notes)
             VALUES (?, ?, 1, 'private-interview-note-marker')",
        )
        .bind(application_id)
        .bind(at)
        .execute(pool)
        .await
        .unwrap();
        if job_hash == "dossier-anchor" {
            sqlx::query(
                "INSERT INTO offers
                     (application_id, base_salary, benefits_summary, accepted)
                 VALUES (?, 150000, 'private-offer-marker', 1)",
            )
            .bind(application_id)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    let read = database
        .read_opportunity_case("dossier-anchor")
        .await
        .unwrap();

    assert_eq!(
        read.job_url,
        "https://job-boards.greenhouse.io/example/jobs/1"
    );
    assert_eq!(read.first_seen_at.to_rfc3339(), "2026-07-01T00:00:00+00:00");
    assert_eq!(read.repost_count, 2);
    assert_eq!(read.salary_min, Some(100_000));
    assert_eq!(read.salary_max, Some(140_000));
    assert_eq!(read.currency.as_deref(), Some("USD"));
    assert_eq!(read.employer_history.saved_job_count, 2);
    assert_eq!(read.employer_history.application_count, 2);
    assert_eq!(read.employer_history.interview_count, 2);
    assert_eq!(read.employer_history.offer_count, 1);
    assert_eq!(read.employer_history.terminal_outcome_count, 1);

    let safe_read = format!("{read:?}");
    for private_marker in [
        "private-job-note-marker",
        "private-application-note-marker",
        "private-contact@example.test",
        "private-interview-note-marker",
        "private-offer-marker",
        "150000",
    ] {
        assert!(
            !safe_read.contains(private_marker),
            "leaked {private_marker}"
        );
    }
}

#[tokio::test]
async fn case_creation_precedes_same_time_application_events() {
    let database = migrated_database().await;
    let pool = database.pool();
    let at = "2026-07-19T00:00:00Z";
    insert_test_job(
        pool,
        "case-same-time",
        "Office Assistant",
        Some("Example"),
        None,
        at,
    )
    .await;
    let case_file = database.ensure_case_file("case-same-time").await.unwrap();
    sqlx::query("UPDATE opportunity_case_files SET created_at = ? WHERE case_file_id = ?")
        .bind(at)
        .bind(&case_file.case_file_id)
        .execute(pool)
        .await
        .unwrap();
    let application_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO applications (job_hash, status, created_at, updated_at)
         VALUES ('case-same-time', 'to_apply', ?, ?) RETURNING id",
    )
    .bind(at)
    .bind(at)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO application_events (application_id, event_type, created_at)
         VALUES (?, 'note_added', ?)",
    )
    .bind(application_id)
    .bind(at)
    .execute(pool)
    .await
    .unwrap();

    let case = database
        .read_opportunity_case("case-same-time")
        .await
        .unwrap();

    assert_eq!(case.timeline[0].kind, "case_created");
    assert_eq!(case.timeline[1].kind, "application_note_added");
}
