//! Tests for health monitoring module

use super::*;

#[test]
fn test_run_status_serialization() {
    assert_eq!(RunStatus::Running.as_str(), "running");
    assert_eq!(RunStatus::Success.as_str(), "success");
    assert_eq!(RunStatus::Failure.as_str(), "failure");
    assert_eq!(RunStatus::Timeout.as_str(), "timeout");
}

#[test]
fn test_run_status_deserialization() {
    assert_eq!(RunStatus::from_str("running"), RunStatus::Running);
    assert_eq!(RunStatus::from_str("success"), RunStatus::Success);
    assert_eq!(RunStatus::from_str("failure"), RunStatus::Failure);
    assert_eq!(RunStatus::from_str("timeout"), RunStatus::Timeout);
    assert_eq!(RunStatus::from_str("unknown"), RunStatus::Failure); // fallback
}

#[tokio::test]
async fn test_source_request_summary_tracks_minimized_metadata() {
    let db = jobsentinel_storage::Database::connect_memory()
        .await
        .unwrap();
    db.migrate().await.unwrap();

    let request_id = record_source_request_started(
        &db,
        "jobswithgpt",
        Some("api.jobswithgpt.example"),
        2,
        false,
        true,
        100,
    )
    .await
    .unwrap();
    finish_source_request(&db, request_id, SourceRequestOutcome::Success)
        .await
        .unwrap();

    let summary = get_latest_source_request(&db, "jobswithgpt")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(summary.id, request_id);
    assert_eq!(summary.source, "jobswithgpt");
    assert_eq!(
        summary.endpoint_host.as_deref(),
        Some("api.jobswithgpt.example")
    );
    assert_eq!(summary.title_count, 2);
    assert!(!summary.has_location);
    assert!(summary.remote_only);
    assert_eq!(summary.result_limit, 100);
    assert_eq!(summary.outcome, SourceRequestOutcome::Success);
}

#[tokio::test]
async fn test_linkedin_scraper_config_has_no_cookie_setup() {
    let db = jobsentinel_storage::Database::connect_memory()
        .await
        .unwrap();
    db.migrate().await.unwrap();

    let configs = get_scraper_configs(&db).await.unwrap();
    let linkedin = configs
        .iter()
        .find(|config| config.scraper_name == "linkedin")
        .expect("LinkedIn source metadata should exist");

    assert!(!linkedin.requires_auth);
    assert!(linkedin.auth_type.is_none());
}

#[test]
fn test_health_status_deserialization() {
    assert_eq!(HealthStatus::from_str("healthy"), HealthStatus::Healthy);
    assert_eq!(HealthStatus::from_str("degraded"), HealthStatus::Degraded);
    assert_eq!(HealthStatus::from_str("down"), HealthStatus::Down);
    assert_eq!(HealthStatus::from_str("disabled"), HealthStatus::Disabled);
    assert_eq!(HealthStatus::from_str("unknown"), HealthStatus::Unknown);
}

#[test]
fn test_selector_health_deserialization() {
    assert_eq!(SelectorHealth::from_str("healthy"), SelectorHealth::Healthy);
    assert_eq!(
        SelectorHealth::from_str("degraded"),
        SelectorHealth::Degraded
    );
    assert_eq!(SelectorHealth::from_str("broken"), SelectorHealth::Broken);
    assert_eq!(SelectorHealth::from_str("unknown"), SelectorHealth::Unknown);
}

#[test]
fn test_scraper_type_deserialization() {
    assert_eq!(ScraperType::from_str("api"), ScraperType::Api);
    assert_eq!(ScraperType::from_str("html"), ScraperType::Html);
    assert_eq!(ScraperType::from_str("rss"), ScraperType::Rss);
    assert_eq!(ScraperType::from_str("graphql"), ScraperType::Graphql);
}

#[test]
fn test_smoke_test_type_serialization() {
    assert_eq!(SmokeTestType::Connectivity.as_str(), "connectivity");
    assert_eq!(SmokeTestType::Selector.as_str(), "selector");
    assert_eq!(SmokeTestType::Auth.as_str(), "auth");
    assert_eq!(SmokeTestType::RateLimit.as_str(), "rate_limit");
}

#[test]
fn test_smoke_test_scraper_list_matches_configured_sources() {
    let scrapers = smoke_test_scrapers();

    assert_eq!(scrapers.len(), 14);
    assert!(scrapers.contains(&"greenhouse"));
    assert!(!scrapers.contains(&"yc_startup"));
    assert!(scrapers.contains(&"usajobs"));
    assert!(scrapers.contains(&"simplyhired"));
    assert!(scrapers.contains(&"glassdoor"));
    assert!(!scrapers.contains(&"linkedin"));
    assert!(!scrapers.contains(&"usa_jobs"));
}

#[test]
fn test_known_scraper_validation() {
    assert!(is_known_scraper_name("usajobs"));
    assert!(!is_known_scraper_name("usa_jobs"));
    assert!(!is_known_scraper_name("unknown"));
}
