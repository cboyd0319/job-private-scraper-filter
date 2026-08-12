use super::*;
use crate::test_support::minimal_test_config;

fn jobswithgpt_smoke_config(endpoint: &str) -> Config {
    let mut config = minimal_test_config();
    config.jobswithgpt_endpoint = endpoint.to_string();
    config.jobswithgpt_approval.enabled = true;
    config.jobswithgpt_approval.payload = config.jobswithgpt_payload_preview();
    config
}

#[tokio::test]
async fn jobswithgpt_smoke_reports_provider_review_before_endpoint_checks() {
    let db = Database::connect_memory()
        .await
        .expect("test database should connect");
    db.migrate().await.expect("test database should migrate");
    let config = jobswithgpt_smoke_config("http://example.com/mcp");

    let result = run_smoke_test(&db, &config, "jobswithgpt")
        .await
        .expect("smoke test should record a failed result");

    assert!(result.passed);
    assert_eq!(
        result
            .details
            .and_then(|details| details["reason"].as_str().map(str::to_string)),
        Some("JobsWithGPT provider endpoint and usage policy require review".to_string())
    );
}

#[tokio::test]
async fn jobswithgpt_smoke_uses_local_request_history_without_provider_contact() {
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let config = jobswithgpt_smoke_config("https://example.test/jobs");

    let result = run_smoke_test(&db, &config, "jobswithgpt").await.unwrap();

    assert_eq!(
        result
            .details
            .and_then(|details| details["status"].as_str().map(str::to_string)),
        Some("skipped".to_string())
    );
}

#[tokio::test]
async fn restricted_smoke_test_is_unavailable_without_distinct_reviewed_scope() {
    let db = Database::connect_memory()
        .await
        .expect("test database should connect");
    db.migrate().await.expect("test database should migrate");
    let config = jobswithgpt_smoke_config("https://example.test/jobs");

    let result = run_smoke_test(&db, &config, "indeed")
        .await
        .expect("smoke test should record a skipped result");

    assert!(result.passed);
    assert_eq!(
        result
            .details
            .as_ref()
            .and_then(|details| details.get("status")),
        Some(&serde_json::json!("skipped"))
    );
    assert_eq!(
        result
            .details
            .as_ref()
            .and_then(|details| details.get("reason")),
        Some(&serde_json::json!(RESTRICTED_SOURCE_CHECK_UNAVAILABLE))
    );
}

#[tokio::test]
async fn retired_yc_startup_smoke_stops_locally() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let config = minimal_test_config();

    let result = run_smoke_test(&database, &config, "yc_startup")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result
            .details
            .and_then(|details| details["reason"].as_str().map(str::to_string)),
        Some(YC_STARTUP_AUTOMATION_UNAVAILABLE.to_string())
    );
}

#[tokio::test]
async fn public_ats_smoke_stops_before_network_without_current_governance() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let credential_database = Database::connect_memory().await.unwrap();
    credential_database.migrate().await.unwrap();
    let credentials =
        CredentialService::with_fixed_master_key(credential_database.credentials(), [7; 32], false);
    let config = minimal_test_config();

    for (source, reason) in [
        ("greenhouse", GREENHOUSE_SOURCE_CHECK_UNAVAILABLE),
        ("lever", LEVER_SOURCE_CHECK_UNAVAILABLE),
    ] {
        let result = run_smoke_test_with_credentials(&database, &config, source, &credentials)
            .await
            .unwrap();

        assert!(result.passed);
        assert_eq!(
            result.details.and_then(|details| {
                details["reason"]
                    .as_str()
                    .map(std::string::ToString::to_string)
            }),
            Some(reason.to_string())
        );
    }
}

#[tokio::test]
async fn renderer_and_legacy_booleans_cannot_authorize_restricted_health_checks() {
    let mut config = jobswithgpt_smoke_config("https://example.test/jobs");
    config.restricted_source_acknowledgements.dice = true;
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();

    let result = run_smoke_test(&db, &config, "dice").await.unwrap();

    assert_eq!(
        result
            .details
            .and_then(|details| details["status"].as_str().map(str::to_string)),
        Some("skipped".to_string())
    );
    assert!(!is_restricted_source_check("greenhouse"));
}

#[tokio::test]
async fn missing_usajobs_governance_stops_before_smoke_credential_access() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let credential_database = Database::connect_memory().await.unwrap();
    credential_database.migrate().await.unwrap();
    let credentials =
        CredentialService::with_fixed_master_key(credential_database.credentials(), [7; 32], false);
    credential_database.close().await;
    let mut config = minimal_test_config();
    config.usajobs.enabled = true;
    config.usajobs.email = "test@example.com".to_string();

    let result = run_smoke_test_with_credentials(&database, &config, "usajobs", &credentials)
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result
            .details
            .and_then(|details| details["status"].as_str().map(str::to_string)),
        Some("skipped".to_string())
    );
}

#[tokio::test]
async fn disabled_or_email_less_usajobs_smoke_skips_before_governance_and_credentials() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let credential_database = Database::connect_memory().await.unwrap();
    credential_database.migrate().await.unwrap();
    let credentials =
        CredentialService::with_fixed_master_key(credential_database.credentials(), [7; 32], false);
    credential_database.close().await;
    let mut disabled = minimal_test_config();
    disabled.usajobs.email = "test@example.com".to_string();
    let mut email_less = minimal_test_config();
    email_less.usajobs.enabled = true;

    for (config, reason) in [
        (disabled, USAJOBS_DISABLED),
        (email_less, USAJOBS_EMAIL_MISSING),
    ] {
        let result = run_smoke_test_with_credentials(&database, &config, "usajobs", &credentials)
            .await
            .unwrap();
        assert!(result.passed);
        assert_eq!(
            result.details.and_then(|details| {
                details["reason"]
                    .as_str()
                    .map(std::string::ToString::to_string)
            }),
            Some(reason.to_string())
        );
    }
}

#[tokio::test]
async fn remoteok_smoke_stops_before_network_without_current_governance() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut config = minimal_test_config();
    config.remoteok.enabled = true;

    let result = run_smoke_test(&database, &config, "remoteok")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(REMOTEOK_SOURCE_CHECK_UNAVAILABLE.to_string())
    );
}

#[tokio::test]
async fn disabled_remoteok_smoke_skips_before_governance_and_network() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let config = minimal_test_config();

    let result = run_smoke_test(&database, &config, "remoteok")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(REMOTEOK_DISABLED.to_string())
    );
}

#[tokio::test]
async fn weworkremotely_smoke_stops_before_network_without_current_governance() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut config = minimal_test_config();
    config.weworkremotely.enabled = true;

    let result = run_smoke_test(&database, &config, "weworkremotely")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(WEWORKREMOTELY_SOURCE_CHECK_UNAVAILABLE.to_string())
    );
}

#[tokio::test]
async fn disabled_weworkremotely_smoke_skips_before_governance_and_network() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let config = minimal_test_config();

    let result = run_smoke_test(&database, &config, "weworkremotely")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(WEWORKREMOTELY_DISABLED.to_string())
    );
}

#[tokio::test]
async fn hn_hiring_smoke_stops_before_network_without_current_governance() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut config = minimal_test_config();
    config.hn_hiring.enabled = true;

    let result = run_smoke_test(&database, &config, "hn_hiring")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(HN_HIRING_SOURCE_CHECK_UNAVAILABLE.to_string())
    );
}

#[tokio::test]
async fn disabled_hn_hiring_smoke_skips_before_governance_and_network() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let config = minimal_test_config();

    let result = run_smoke_test(&database, &config, "hn_hiring")
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.details.and_then(|details| {
            details["reason"]
                .as_str()
                .map(std::string::ToString::to_string)
        }),
        Some(HN_HIRING_DISABLED.to_string())
    );
}

#[test]
fn hn_hiring_health_contract_requires_the_selected_item_schema() {
    let list: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../jobsentinel-sources/src/fixtures/hn_hiring_list_v1.json"
    ))
    .unwrap();
    let mut detail: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../jobsentinel-sources/src/fixtures/hn_hiring_detail_v1.json"
    ))
    .unwrap();

    let thread_id = jobsentinel_sources::HnHiringScraper::canonical_thread_id(&list).unwrap();
    assert!(jobsentinel_sources::HnHiringScraper::is_canonical_thread_item(&detail, thread_id));

    detail["id"] = serde_json::json!(0);
    assert!(!jobsentinel_sources::HnHiringScraper::is_canonical_thread_item(&detail, thread_id));
}

#[test]
fn validate_smoke_details_allows_skipped_sources() {
    let details = serde_json::json!({
        "status": "skipped",
        "reason": "source disabled"
    });

    validate_smoke_details("usajobs", &details).expect("skipped checks are neutral");
}

#[test]
fn validate_smoke_details_rejects_missing_selectors() {
    let details = serde_json::json!({
        "status": 200,
        "selectors_found": false,
        "html_size_kb": 12
    });

    let error = validate_smoke_details("glassdoor", &details).unwrap_err();
    assert!(error.to_string().contains("expected job selectors"));
}

#[test]
fn validate_smoke_details_rejects_empty_job_counts() {
    let details = serde_json::json!({
        "status": 200,
        "jobs_found": 0,
        "format": "rss"
    });

    let error = validate_smoke_details("simplyhired", &details).unwrap_err();
    assert!(error.to_string().contains("did not find any jobs"));
}

#[test]
fn source_check_error_message_hides_raw_urls_and_tokens() {
    let error = anyhow::anyhow!(
        "JobsWithGPT smoke test request failed: error sending request for url https://example.test/jobs?token=secret"
    );
    let message = source_check_error_message(&error);

    assert_eq!(message, SOURCE_CHECK_NETWORK_ERROR);
    assert!(!message.contains("https://"));
    assert!(!message.contains("token"));
    assert!(!message.contains("secret"));
    assert!(!message.contains("JobsWithGPT"));
}

#[test]
fn source_check_error_message_keeps_status_actionable_without_provider_detail() {
    let cases = [
        (
            anyhow::anyhow!("HTTP 429 Too Many Requests"),
            SOURCE_CHECK_RATE_LIMITED,
        ),
        (anyhow::anyhow!("HTTP 403 Forbidden"), SOURCE_CHECK_REJECTED),
        (
            anyhow::anyhow!("HTTP 500 Internal Server Error"),
            SOURCE_CHECK_BAD_RESPONSE,
        ),
        (
            anyhow::anyhow!("Failed to parse JSON response from https://example.test/private"),
            SOURCE_CHECK_READ_ERROR,
        ),
    ];

    for (error, expected) in cases {
        let message = source_check_error_message(&error);
        assert_eq!(message, expected);
        assert!(!message.contains("https://"));
        assert!(!message.contains("Forbidden"));
        assert!(!message.contains("Internal Server Error"));
    }
}
