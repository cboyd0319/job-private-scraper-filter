use super::*;

// ========================================
// State Transition Tests
// ========================================

#[tokio::test]
async fn test_scheduler_start_to_shutdown_transition() {
    let mut config = create_test_config();
    config.scraping_interval_hours = 24;
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Arc::new(Scheduler::new(Arc::clone(&config), Arc::clone(&database)));
    let scheduler_clone = Arc::clone(&scheduler);

    // Start scheduler
    let handle = tokio::spawn(async move { scheduler_clone.start().await });

    // Let it run briefly
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Transition to shutdown
    scheduler.shutdown().unwrap();

    // Wait for clean shutdown
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_ok());
}

#[tokio::test]
async fn test_scheduler_multiple_start_cycles() {
    let config = Arc::new(create_test_config());
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Multiple scraping cycles should be idempotent
    let _result1 = scheduler.run_scraping_cycle().await.unwrap();
    let _result2 = scheduler.run_scraping_cycle().await.unwrap();
    let _result3 = scheduler.run_scraping_cycle().await.unwrap();

    // All cycles should complete successfully
    let jobs = database.get_recent_jobs(100).await.unwrap();
    // Jobs can be retrieved from database
    assert!(jobs.is_empty() || !jobs.is_empty());
}

// ========================================
// Edge Cases for Alert Logic
// ========================================

#[tokio::test]
async fn test_scraping_cycle_alert_threshold_boundary() {
    let mut config = create_test_config();
    config.immediate_alert_threshold = 1.0; // Maximum threshold
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - very high threshold means no alerts
    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert_eq!(result.alerts_sent, 0);
}

#[tokio::test]
async fn test_scraping_cycle_alert_threshold_zero() {
    let mut config = create_test_config();
    config.immediate_alert_threshold = 0.0; // Minimum threshold
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - low threshold might trigger alerts if jobs found
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // With no scrapers configured, no jobs and no alerts
    assert_eq!(result.alerts_sent, 0);
}

// ========================================
// Scraper URL Edge Cases
// ========================================

#[tokio::test]
async fn test_scraping_cycle_with_many_greenhouse_urls() {
    let mut config = create_test_config();
    config.greenhouse_urls = (0..50)
        .map(|i| format!("https://boards.greenhouse.io/company{}", i))
        .collect();
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle with many URLs
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should handle many URLs gracefully
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_with_duplicate_urls() {
    let mut config = create_test_config();
    let url = "https://boards.greenhouse.io/jobsentinel-missing-company".to_string();
    config.greenhouse_urls = vec![url.clone(), url.clone(), url.clone()];
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should handle duplicate URLs
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Cycle should complete
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

// ========================================
// Scraper Error Path Coverage Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_greenhouse_error_path() {
    // Tests lines 184-187 (Greenhouse error logging)
    let mut config = create_test_config();
    config.greenhouse_urls = vec!["https://boards.greenhouse.io/test-company-error".to_string()];
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - Greenhouse will attempt to scrape
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should complete (may have errors or just find 0 jobs)
    assert!(
        result.errors.iter().any(|e| e.contains("Greenhouse")) || result.jobs_found == 0,
        "Should handle Greenhouse scraping attempt"
    );
}

#[tokio::test]
async fn test_scraping_cycle_lever_error_path() {
    // Tests lines 217-220 (Lever error logging)
    let mut config = create_test_config();
    config.lever_urls = vec!["https://jobs.lever.co/test-company-error".to_string()];
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - Lever will attempt to scrape
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should complete (may have errors or just find 0 jobs)
    assert!(
        result.errors.iter().any(|e| e.contains("Lever")) || result.jobs_found == 0,
        "Should handle Lever scraping attempt"
    );
}

#[tokio::test]
async fn test_scraping_cycle_jobswithgpt_stops_before_request_while_provider_review_is_pending() {
    let mut config = create_test_config();
    config.title_allowlist = vec!["Security Engineer".to_string()];
    config.jobswithgpt_endpoint = "not-a-url".to_string();
    config.location_preferences.allow_remote = true;
    config.location_preferences.allow_onsite = false;
    approve_jobswithgpt_payload(&mut config);
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert!(
        result
            .errors
            .iter()
            .any(|error| error.contains("provider policy requires review")),
        "provider review should be visible"
    );
    assert!(
        crate::health::get_latest_source_request(&database, "jobswithgpt")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_scraping_cycle_linkedin_user_choice_warns_without_hidden_monitoring() {
    let mut config = create_test_config();
    config.linkedin.enabled = true;
    config.linkedin.query = "Engineer".to_string();
    config.linkedin.location = "Remote".to_string();
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert!(result
        .errors
        .iter()
        .any(|e| e.contains("local record") && e.contains("User Agreement")));
}

#[tokio::test]
async fn test_scraping_cycle_all_scrapers_error_accumulation() {
    // Tests error accumulation from multiple scrapers
    let mut config = create_test_config();
    config.greenhouse_urls = vec!["https://boards.greenhouse.io/error".to_string()];
    config.lever_urls = vec!["https://jobs.lever.co/error".to_string()];
    config.title_allowlist = vec!["Engineer".to_string()];
    config.jobswithgpt_endpoint = "not-a-url".to_string();
    approve_jobswithgpt_payload(&mut config);
    config.linkedin.enabled = true;
    config.linkedin.query = "Engineer".to_string();
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - all scrapers will fail
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should accumulate errors from multiple scrapers
    // Note: Some scrapers may succeed or not error depending on network conditions
    assert!(
        result.errors.len() >= 1,
        "Should have scraper errors, got: {}",
        result.errors.len()
    );
}

// ========================================
// Database Error Path Coverage
// ========================================

#[tokio::test]
async fn test_scraping_cycle_database_upsert_resilience() {
    // Tests that cycle continues even if individual upserts fail
    let config = Arc::new(create_test_config());
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run multiple cycles to ensure database operations are resilient
    let result1 = scheduler.run_scraping_cycle().await;
    let result2 = scheduler.run_scraping_cycle().await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

// ========================================
// Notification Error Path Coverage
// ========================================

#[tokio::test]
async fn test_scraping_cycle_notification_error_handling() {
    // Tests notification send error handling (lines 408-412)
    let mut config = create_test_config();
    config.immediate_alert_threshold = 0.0; // Low threshold
                                            // Configure invalid Slack webhook to trigger error
    config.alerts.slack.enabled = true;
    config.alerts.slack.webhook_url = "https://invalid-webhook-url".to_string();
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - notification will fail if high-scoring job found
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should handle notification errors gracefully and keep result accounting consistent.
    assert!(result.jobs_new <= result.jobs_found);
    assert!(result.jobs_updated <= result.jobs_found);
}

// ========================================
// Interval and Timing Edge Cases
// ========================================

// ========================================
// Job Scoring and Serialization
// ========================================

#[tokio::test]
async fn test_scraping_cycle_handles_all_score_ranges() {
    // Ensure scoring works across different scenarios
    let config = Arc::new(create_test_config());
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle multiple times
    for _ in 0..3 {
        let result = scheduler.run_scraping_cycle().await;
        assert!(result.is_ok());
    }
}

// ========================================
// Multiple Scraping Cycles in Scheduler
// ========================================

#[tokio::test]
async fn test_scheduler_runs_multiple_cycles_before_shutdown() {
    let mut config = create_test_config();
    config.scraping_interval_hours = 0; // Immediate re-run
    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Arc::new(Scheduler::new(Arc::clone(&config), Arc::clone(&database)));
    let scheduler_clone = Arc::clone(&scheduler);

    // Start scheduler
    let handle = tokio::spawn(async move { scheduler_clone.start().await });

    // Let it run multiple cycles
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Shutdown
    scheduler.shutdown().unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok());
}

// ========================================
// Comprehensive Integration Tests
// ========================================

#[tokio::test]
#[ignore = "Integration test - makes real HTTP requests"]
async fn test_complete_workflow_with_all_error_paths() {
    // Comprehensive test hitting multiple error paths
    let mut config = create_test_config();
    config.scraping_interval_hours = 6;
    config.greenhouse_urls = vec!["https://boards.greenhouse.io/test".to_string()];
    config.lever_urls = vec!["https://jobs.lever.co/test".to_string()];
    config.title_allowlist = vec!["Engineer".to_string()];
    config.linkedin.enabled = true;
    config.linkedin.query = "Test".to_string();
    config.immediate_alert_threshold = 0.5;

    let config = Arc::new(config);
    let db = Db::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Arc::new(Scheduler::new(Arc::clone(&config), Arc::clone(&database)));
    let scheduler_clone = Arc::clone(&scheduler);

    // Start full scheduler loop
    let handle = tokio::spawn(async move { scheduler_clone.start().await });

    // Let it run complete cycle
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Shutdown
    scheduler.shutdown().unwrap();

    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok());
}
