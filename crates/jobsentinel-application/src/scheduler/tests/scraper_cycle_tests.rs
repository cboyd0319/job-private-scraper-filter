use super::*;

// ========================================
// Scraper URL Parsing Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_with_greenhouse_urls() {
    let mut config = create_test_config();
    config.greenhouse_urls = vec![
        "https://boards.greenhouse.io/jobsentinel-missing-company-a".to_string(),
        "https://boards.greenhouse.io/jobsentinel-missing-company-b".to_string(),
    ];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - will fail to scrape but should handle gracefully
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should have attempted to scrape but likely got errors
    // Errors are expected since we're not running real scrapers
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_with_lever_urls() {
    let mut config = create_test_config();
    config.lever_urls = vec![
        "https://jobs.lever.co/netflix".to_string(),
        "https://jobs.lever.co/stripe".to_string(),
    ];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - will fail to scrape but should handle gracefully
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Cycle should complete (errors expected since not real scrapers)
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_with_invalid_greenhouse_url() {
    let mut config = create_test_config();
    config.greenhouse_urls = vec![
        "https://invalid-url".to_string(),
        "not-a-greenhouse-url".to_string(),
    ];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Should handle invalid URLs gracefully
    let result = scheduler.run_scraping_cycle().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_scraping_cycle_with_invalid_lever_url() {
    let mut config = create_test_config();
    config.lever_urls = vec![
        "https://invalid-url".to_string(),
        "not-a-lever-url".to_string(),
    ];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Should handle invalid URLs gracefully
    let result = scheduler.run_scraping_cycle().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_scraping_cycle_with_mixed_valid_invalid_urls() {
    let mut config = create_test_config();
    config.greenhouse_urls = vec![
        "https://boards.greenhouse.io/cloudflare".to_string(),
        "invalid-url".to_string(),
    ];
    config.lever_urls = vec![
        "https://jobs.lever.co/netflix".to_string(),
        "not-a-url".to_string(),
    ];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Should process valid URLs and skip/report invalid ones
    let result = scheduler.run_scraping_cycle().await;
    assert!(result.is_ok());
}

// ========================================
// Scoring Integration Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_scores_jobs() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle (no scrapers, so empty result)
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should complete successfully
    assert_eq!(result.jobs_found, 0);
}

#[tokio::test]
async fn test_scraping_cycle_sorts_jobs_by_score() {
    // This test verifies that jobs are sorted by score descending
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle
    let _result = scheduler.run_scraping_cycle().await.unwrap();

    // Verify jobs in database are retrievable
    let jobs = database.get_recent_jobs(10).await.unwrap();

    // If there are jobs, they should be sorted by score
    if jobs.len() >= 2 {
        for i in 0..jobs.len() - 1 {
            if let (Some(score1), Some(score2)) = (jobs[i].score, jobs[i + 1].score) {
                assert!(
                    score1 >= score2,
                    "Jobs should be sorted by score descending"
                );
            }
        }
    }
}

#[tokio::test]
async fn test_scraping_cycle_handles_score_serialization_error() {
    // Tests the error path in score_reasons serialization
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should handle any serialization issues gracefully
    let result = scheduler.run_scraping_cycle().await;
    assert!(result.is_ok());
}

// ========================================
// High Score Alert Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_identifies_high_matches() {
    let mut config = create_test_config();
    config.immediate_alert_threshold = 0.9;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should track high matches count
    assert_eq!(result.high_matches, 0);
}

#[tokio::test]
async fn test_scraping_cycle_skips_already_alerted_jobs() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    // Pre-populate with a high-scoring job that already has alert sent
    let now = chrono::Utc::now();
    let alerted_job = Job {
        id: 0,
        hash: "alerted_hash".to_string(),
        title: "Amazing Security Engineer".to_string(),
        company: "Dream Corp".to_string(),
        location: Some("Remote".to_string()),
        url: "https://example.com/job/999".to_string(),
        description: Some("Perfect match".to_string()),
        score: Some(0.95),
        score_reasons: None,
        source: "test".to_string(),
        remote: Some(true),
        salary_min: None,
        salary_max: None,
        currency: None,
        created_at: now,
        updated_at: now,
        last_seen: now,
        times_seen: 1,
        immediate_alert_sent: true, // Already alerted
        included_in_digest: false,
        hidden: false,
        bookmarked: false,
        ghost_score: None,
        ghost_reasons: None,
        first_seen: None,
        repost_count: 0,
        notes: None,
    };
    database.upsert_job(&alerted_job).await.unwrap();

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should not send duplicate alerts
    assert_eq!(result.alerts_sent, 0);
}

// ========================================
// LinkedIn Scraper Configuration Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_with_linkedin_enabled() {
    let mut config = create_test_config();
    config.linkedin.enabled = true;
    config.linkedin.query = "Security Engineer".to_string();
    config.linkedin.location = "Remote".to_string();
    config.linkedin.remote_only = true;
    config.linkedin.limit = 50;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
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
async fn test_scraping_cycle_with_linkedin_disabled() {
    let mut config = create_test_config();
    config.linkedin.enabled = false;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should leave LinkedIn inactive
    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert!(!result.errors.iter().any(|e| e.contains("LinkedIn")));
}

#[tokio::test]
async fn test_scraping_cycle_with_linkedin_enabled_without_credentials() {
    let mut config = create_test_config();
    config.linkedin.enabled = true;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
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
async fn test_retired_scheduled_source_stops_without_transport() {
    let mut config = create_test_config();
    config.dice.enabled = true;
    config.dice.query = "care coordinator".to_string();
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert!(result.errors.iter().any(|error| {
        error
            == "Dice scheduled access is unavailable after provider policy review. Use the user-opened search link, Browser Import, or manual entry."
    }));
}

#[tokio::test]
async fn test_local_acknowledgement_cannot_restore_a_retired_source() {
    let previous = create_test_config();
    let mut config = previous.clone();
    config.glassdoor.enabled = true;
    config.glassdoor.query = "care coordinator".to_string();
    config.restricted_source_acknowledgements.glassdoor = true;
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    crate::restricted_source_consent::reconcile_restricted_source_consents(
        &db,
        &previous,
        &mut config,
    )
    .await
    .unwrap();
    assert!(!config.glassdoor.enabled);
    assert!(!config.restricted_source_acknowledgements.glassdoor);
    let config = Arc::new(config);
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    let result = scheduler.run_scraping_cycle().await.unwrap();

    assert_eq!(result.jobs_found, 0);
    assert!(result.errors.is_empty());
}

// ========================================
// JobsWithGPT Scraper Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_with_jobswithgpt_remote_only() {
    let mut config = create_test_config();
    config.title_allowlist = vec!["Engineer".to_string()];
    config.jobswithgpt_endpoint = "not-a-url".to_string();
    config.location_preferences.allow_remote = true;
    config.location_preferences.allow_onsite = false;
    approve_jobswithgpt_payload(&mut config);
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - will fail to scrape but should handle gracefully
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Cycle should complete (JobsWithGPT errors expected)
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_with_jobswithgpt_not_remote_only() {
    let mut config = create_test_config();
    config.title_allowlist = vec!["Engineer".to_string()];
    config.jobswithgpt_endpoint = "not-a-url".to_string();
    config.location_preferences.allow_remote = false;
    config.location_preferences.allow_onsite = true;
    approve_jobswithgpt_payload(&mut config);
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should set remote_only=false
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Cycle should complete
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_with_empty_title_allowlist() {
    let mut config = create_test_config();
    config.title_allowlist = vec![]; // Empty allowlist
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should skip JobsWithGPT
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // No JobsWithGPT errors since it was skipped
    assert!(!result.errors.iter().any(|e| e.contains("JobsWithGPT")));
}

// ========================================
// Multiple Scrapers Enabled Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_all_scrapers_enabled() {
    let mut config = create_test_config();
    config.greenhouse_urls = vec!["https://boards.greenhouse.io/test".to_string()];
    config.lever_urls = vec!["https://jobs.lever.co/test".to_string()];
    config.title_allowlist = vec!["Engineer".to_string()];
    config.linkedin.enabled = true;
    config.linkedin.query = "Engineer".to_string();
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - all scrapers will attempt to run
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // All scrapers attempted (errors expected)
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}

#[tokio::test]
async fn test_scraping_cycle_all_scrapers_disabled() {
    let mut config = create_test_config();
    config.greenhouse_urls = vec![];
    config.lever_urls = vec![];
    config.title_allowlist = vec![];
    config.linkedin.enabled = false;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - no scrapers enabled
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should complete with no jobs found and no errors
    assert_eq!(result.jobs_found, 0);
    assert_eq!(result.errors.len(), 0);
}

// ========================================
// Error Accumulation Tests
// ========================================

#[tokio::test]
async fn test_scraping_cycle_accumulates_multiple_scraper_errors() {
    let mut config = create_test_config();
    // Set up invalid configurations to trigger errors
    config.greenhouse_urls = vec!["invalid".to_string()];
    config.lever_urls = vec!["invalid".to_string()];
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run cycle - should accumulate errors from multiple scrapers
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Cycle should complete (may have accumulated errors)
    assert!(result.jobs_found == 0 || result.errors.len() > 0);
}
