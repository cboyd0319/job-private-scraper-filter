use super::*;
use serde_json::Value;
use std::path::Path;

pub(super) async fn database_with_private_export_data(path: &Path) -> Database {
    let database = Database::connect(path).await.unwrap();
    database.migrate().await.unwrap();
    sqlx::query(
        "INSERT INTO jobs(hash, title, company, url, source)
         VALUES (
            'export-job', 'Exportable Role', 'Example Co',
            'https://user:pass@example.test/job?job=42&utm_source=test&candidate_token=private#fragment',
            'test'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO resume_drafts(data) VALUES (
            '{
                \"contact\": {
                    \"linkedin\": \"https://draft-user:draft-pass@example.test/profile?session=draft-private#contact\"
                },
                \"projects\": [
                    {
                        \"url\": \"https://example.test/project?candidate_token=draft-project-private&view=public#details\"
                    }
                ],
                \"clearance\": \"draft-clearance-marker\",
                \"military_info\": \"draft-military-marker\",
                \"security_clearance\": \"draft-security-clearance-marker\",
                \"military_service\": \"draft-military-service-marker\",
                \"is_protected_veteran\": \"draft-protected-veteran-marker\",
                \"disability_accommodation\": \"draft-disability-marker\",
                \"api_key\": \"draft-api-key-marker\",
                \"file_path\": \"/private/draft-file-path-marker.pdf\",
                \"meeting_link\": \"https://example.test/draft-meeting-link-marker?session=private\"
            }'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO applications(job_hash, status, notes)
         VALUES ('export-job', 'to_apply', 'user-authored-pasted-token-marker')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO application_attempts(
            job_hash, application_id, status, ats_platform, error_message,
            screenshot_path, confirmation_screenshot_path
         ) VALUES (
            'export-job', (SELECT id FROM applications WHERE job_hash = 'export-job'),
            'failed', 'greenhouse', 'export-private-error-marker',
            '/private/export-screenshot-marker.png',
            '/private/export-confirmation-marker.png'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO captcha_challenges(
            application_attempt_id, challenge_type, challenge_url
         ) VALUES (
            (SELECT id FROM application_attempts WHERE job_hash = 'export-job'),
            'hcaptcha', 'https://example.test/export-challenge-marker?token=private'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO interviews(application_id, scheduled_at, location, meeting_link)
         VALUES (
            (SELECT id FROM applications WHERE job_hash = 'export-job'),
            '2026-07-20T12:00:00Z',
            'https://zoom.us/j/export-zoom-path-marker?pwd=export-zoom-pwd-marker',
            'https://example.test/export-meeting-marker?session=private'
         ), (
            (SELECT id FROM applications WHERE job_hash = 'export-job'),
            '2026-07-21T12:00:00Z',
            'https://meet.google.com/export-meet-path-marker',
            NULL
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO resumes(name, file_path, parsed_text)
         VALUES ('Primary resume', '/private/export-path-marker.pdf', 'portable resume evidence')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO application_profile(
            full_name, email, linkedin_url, website_url, resume_file_path,
            us_work_authorized, requires_sponsorship, require_manual_approval
         ) VALUES (
            'Review User', 'review@example.test',
            'https://example.test/profile?session=private&view=public#contact',
            'http://localhost/private?token=private',
            '/private/profile-path-marker.pdf', 1, 0, 1
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO screening_answers(question_pattern, answer, answer_type, notes)
         VALUES ('(?i)protected veteran', 'Prefer not to answer', 'select',
                 'export-protected-answer-marker')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO secret_vault(key, algorithm, key_version, nonce, ciphertext)
         VALUES ('export-secret-key-marker', 'xchacha20poly1305', 1, ?, ?)",
    )
    .bind(vec![1_u8; 24])
    .bind(b"export-secret-value-marker".to_vec())
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO credential_key_wrapping(
            id, mode, kdf, memory_kib, iterations, parallelism,
            salt, algorithm, nonce, ciphertext
         ) VALUES (
            1, 'passphrase', 'argon2id', 19456, 2, 1, ?,
            'xchacha20poly1305', ?, ?
         )",
    )
    .bind(vec![2_u8; 16])
    .bind(vec![3_u8; 24])
    .bind(b"export-wrapped-key-marker".to_vec())
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO credential_health(credential_key, notes)
         VALUES ('export-credential-name-marker', 'private credential health marker')",
    )
    .execute(database.pool())
    .await
    .unwrap();
    sqlx::query(
        "UPDATE scraper_config SET notes = 'export-scraper-notes-marker'
         WHERE scraper_name = (SELECT MIN(scraper_name) FROM scraper_config)",
    )
    .execute(database.pool())
    .await
    .unwrap();
    database
}

pub(super) fn records(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub(super) fn find_record<'a>(records: &'a [Value], table: &str) -> &'a Value {
    records
        .iter()
        .find(|record| record["kind"] == "record" && record["table"] == table)
        .unwrap()
}

#[tokio::test]
async fn reviewed_export_is_complete_and_excludes_secrets_paths_and_protected_records() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("primary/jobs.db");
    let export_path = temp_dir.path().join("JobSentinel reviewed export.jsonl");
    let database = database_with_private_export_data(&db_path).await;

    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();

    assert!(plan.user_review_required());
    assert!(!plan.contains_secrets());
    assert_eq!(plan.record_count("opportunities"), Some(1));
    assert_eq!(
        plan.total_record_count(),
        plan.section_counts().values().sum::<u64>()
    );
    assert!(!plan.protected_records_included());
    assert!(plan.protected_application_answer_count() > 0);
    assert!(plan.protected_resume_draft_count() > 0);
    assert!(plan
        .excluded_data()
        .iter()
        .any(|item| item.contains("credentials")));
    assert!(
        !export_path.exists(),
        "review unexpectedly wrote an artifact"
    );

    let info = database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap();
    assert_eq!(
        Database::inspect_reviewed_export(&export_path).unwrap(),
        info
    );
    let mut outcome = String::new();
    for _ in 0..100 {
        outcome = sqlx::query_scalar(
            "SELECT outcome FROM v3_recovery_operations
             WHERE operation_id = ? AND operation_kind = 'export'",
        )
        .bind(&info.export_id)
        .fetch_one(database.pool())
        .await
        .unwrap();
        if outcome == "succeeded" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(outcome, "succeeded");

    let exported = records(&export_path);
    assert_eq!(
        exported.first().unwrap()["schema"],
        "jobsentinel.v3.reviewed-export.v1"
    );
    assert_eq!(
        exported.first().unwrap()["secret_scope"],
        "jobsentinel_managed_credentials"
    );
    assert_eq!(
        exported.first().unwrap()["user_authored_text_is_verbatim"],
        true
    );
    assert_eq!(
        exported.first().unwrap()["integrity"],
        "structural_completion_only"
    );
    assert_eq!(exported.last().unwrap()["kind"], "complete");
    assert_eq!(exported.last().unwrap()["record_count"], info.record_count);
    let job = find_record(&exported, "jobs");
    assert_eq!(job["data"]["title"], "Exportable Role");
    assert_eq!(job["data"]["url"], "https://example.test/job?job=42");
    let resume = find_record(&exported, "resumes");
    assert_eq!(resume["data"]["parsed_text"], "portable resume evidence");
    assert!(resume["data"].get("file_path").is_none());
    let profile = find_record(&exported, "application_profile");
    assert_eq!(
        profile["data"]["linkedin_url"],
        "https://example.test/profile?view=public"
    );
    assert_eq!(profile["data"]["website_url"], Value::Null);
    assert!(profile["data"].get("resume_file_path").is_none());
    assert!(profile["data"].get("us_work_authorized").is_none());
    assert!(exported
        .iter()
        .filter(|record| record["kind"] == "record" && record["table"] == "interviews")
        .all(|record| record["data"]["location"] == Value::Null));

    let raw = std::fs::read_to_string(&export_path).unwrap();
    assert!(raw.contains("user-authored-pasted-token-marker"));
    for forbidden in [
        "export-secret-key-marker",
        "export-secret-value-marker",
        "export-wrapped-key-marker",
        "export-credential-name-marker",
        "/private/export-path-marker.pdf",
        "/private/profile-path-marker.pdf",
        "export-private-error-marker",
        "/private/export-screenshot-marker.png",
        "/private/export-confirmation-marker.png",
        "export-challenge-marker",
        "export-meeting-marker",
        "export-zoom-path-marker",
        "export-zoom-pwd-marker",
        "export-meet-path-marker",
        "export-scraper-notes-marker",
        "candidate_token",
        "draft-user",
        "draft-pass",
        "draft-private",
        "draft-project-private",
        "draft-clearance-marker",
        "draft-military-marker",
        "draft-security-clearance-marker",
        "draft-military-service-marker",
        "draft-protected-veteran-marker",
        "draft-disability-marker",
        "draft-api-key-marker",
        "/private/draft-file-path-marker.pdf",
        "draft-meeting-link-marker",
        "export-protected-answer-marker",
        "v3_source_consent_events",
        "v3_source_policy_ledger",
    ] {
        assert!(!raw.contains(forbidden), "export leaked {forbidden}");
    }
}

#[tokio::test]
async fn reviewed_export_never_overwrites_and_cleans_private_work_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("primary/jobs.db");
    let export_path = temp_dir.path().join("existing.jsonl");
    let database = database_with_private_export_data(&db_path).await;
    std::fs::write(&export_path, b"user-owned destination").unwrap();
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();

    let error = database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap_err();

    assert_eq!(
        std::fs::read(&export_path).unwrap(),
        b"user-owned destination"
    );
    assert!(error.as_database_error().is_none());
    let export_operations: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v3_recovery_operations WHERE operation_kind = 'export'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(export_operations, 0);
    assert!(
        std::fs::read_dir(temp_dir.path())
            .unwrap()
            .flatten()
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(".jobsentinel_export_")),
        "reviewed export left a private work file"
    );
}

#[tokio::test]
async fn reviewed_export_marks_changed_data_failed_without_publishing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        database_with_private_export_data(&temp_dir.path().join("primary/jobs.db")).await;
    let export_path = temp_dir.path().join("changed.jsonl");
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO jobs(hash, title, company, url, source)
         VALUES ('later-job', 'Changed after review', 'Example Co',
                 'https://example.test/later', 'test')",
    )
    .execute(database.pool())
    .await
    .unwrap();

    let error = database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("review it again"));
    assert!(!export_path.exists());
    let outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM v3_recovery_operations WHERE operation_kind = 'export'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(outcome, "failed");
    assert!(std::fs::read_dir(temp_dir.path())
        .unwrap()
        .flatten()
        .all(|entry| !entry
            .file_name()
            .to_string_lossy()
            .starts_with(".jobsentinel_export_")));
}

#[tokio::test]
async fn reviewed_export_rejects_malformed_nested_draft_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        database_with_private_export_data(&temp_dir.path().join("primary/jobs.db")).await;
    sqlx::query("UPDATE resume_drafts SET data = 'not-json'")
        .execute(database.pool())
        .await
        .unwrap();
    let export_path = temp_dir.path().join("malformed.jsonl");
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();

    let error = database
        .create_reviewed_export(&export_path, plan)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("nested data is invalid"));
    assert!(!export_path.exists());
    let outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM v3_recovery_operations WHERE operation_kind = 'export'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();
    assert_eq!(outcome, "failed");

    sqlx::query(
        r#"UPDATE resume_drafts
           SET data = '{"projects":[{"url":{"token":"nested-url-object-marker"}}]}'"#,
    )
    .execute(database.pool())
    .await
    .unwrap();
    let invalid_url_path = temp_dir.path().join("invalid-url.jsonl");
    let plan = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap();
    let error = database
        .create_reviewed_export(&invalid_url_path, plan)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("nested URL field is invalid"));
    assert!(!invalid_url_path.exists());
}

#[tokio::test]
async fn reviewed_export_fails_closed_for_unknown_tables_and_incomplete_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let database =
        database_with_private_export_data(&temp_dir.path().join("primary/jobs.db")).await;
    sqlx::query("CREATE TABLE future_private_records(secret TEXT)")
        .execute(database.pool())
        .await
        .unwrap();

    let error = database
        .review_plaintext_export(ReviewedExportSelection::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("unsupported table"));

    let incomplete = temp_dir.path().join("incomplete.jsonl");
    std::fs::write(
        &incomplete,
        "{\"kind\":\"export\",\"schema\":\"jobsentinel.v3.reviewed-export.v1\"}\n",
    )
    .unwrap();
    let error = Database::inspect_reviewed_export(&incomplete).unwrap_err();
    assert!(error.to_string().contains("could not be verified"));
}
