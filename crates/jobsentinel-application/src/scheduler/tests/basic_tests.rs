use super::*;

#[test]
fn test_schedule_config_from_config() {
    let config = create_test_config();
    let schedule_config = ScheduleConfig::from(&config);

    assert_eq!(schedule_config.interval_hours, 2);
    assert!(schedule_config.enabled);
}

// ========================================
// Scheduler Lifecycle Tests
// ========================================

#[tokio::test]
async fn test_scheduler_creation() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Verify scheduler was created with correct config
    let scheduler_config = scheduler.config.read().await;
    assert_eq!(
        scheduler_config.scraping_interval_hours,
        config.scraping_interval_hours
    );
}

#[tokio::test]
async fn test_scheduler_shared_config_updates_without_restart() {
    let config = Arc::new(RwLock::new(create_test_config()));
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new_shared(Arc::clone(&config), Arc::clone(&database));

    {
        let mut config = config.write().await;
        config.scraping_interval_hours = 6;
        config.use_resume_matching = true;
    }

    let scheduler_config = scheduler.config.read().await;
    assert_eq!(scheduler_config.scraping_interval_hours, 6);
    assert!(scheduler_config.use_resume_matching);
}

#[tokio::test]
async fn test_scheduler_shutdown_signal() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Subscribe to shutdown signal before calling shutdown
    let mut rx = scheduler.subscribe_shutdown();

    // Trigger shutdown
    scheduler.shutdown().unwrap();

    // Verify shutdown signal was received
    assert!(
        rx.recv().await.is_ok(),
        "Shutdown signal should be received"
    );
}

#[tokio::test]
async fn test_scheduler_multiple_shutdown_subscribers() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Create multiple subscribers
    let mut rx1 = scheduler.subscribe_shutdown();
    let mut rx2 = scheduler.subscribe_shutdown();
    let mut rx3 = scheduler.subscribe_shutdown();

    // Trigger shutdown
    scheduler.shutdown().unwrap();

    // All subscribers should receive the signal
    assert!(
        rx1.recv().await.is_ok(),
        "Subscriber 1 should receive signal"
    );
    assert!(
        rx2.recv().await.is_ok(),
        "Subscriber 2 should receive signal"
    );
    assert!(
        rx3.recv().await.is_ok(),
        "Subscriber 3 should receive signal"
    );
}

#[tokio::test]
async fn test_scheduler_graceful_stop_with_timeout() {
    let mut config = create_test_config();
    config.scraping_interval_hours = 24; // Long interval to ensure we can stop it
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Arc::new(Scheduler::new(Arc::clone(&config), Arc::clone(&database)));
    let scheduler_clone = Arc::clone(&scheduler);

    // Start scheduler in background task
    let handle = tokio::spawn(async move { scheduler_clone.start().await });

    // Give scheduler time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown the scheduler
    scheduler.shutdown().unwrap();

    // Wait for scheduler to stop (with timeout)
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;

    assert!(result.is_ok(), "Scheduler should stop within timeout");
    assert!(
        result.unwrap().is_ok(),
        "Scheduler should stop without errors"
    );
}

#[test]
fn test_schedule_config_creation() {
    let config = ScheduleConfig {
        interval_hours: 4,
        enabled: true,
    };

    assert_eq!(config.interval_hours, 4);
    assert!(config.enabled);
}

#[test]
fn test_schedule_config_disabled() {
    let config = ScheduleConfig {
        interval_hours: 2,
        enabled: false,
    };

    assert!(!config.enabled);
}

#[test]
fn test_scraping_result_creation() {
    let result = ScrapingResult {
        jobs_found: 100,
        jobs_new: 25,
        jobs_updated: 75,
        high_matches: 10,
        alerts_sent: 5,
        errors: vec!["Test error".to_string()],
    };

    assert_eq!(result.jobs_found, 100);
    assert_eq!(result.jobs_new, 25);
    assert_eq!(result.jobs_updated, 75);
    assert_eq!(result.high_matches, 10);
    assert_eq!(result.alerts_sent, 5);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_scraping_result_no_errors() {
    let result = ScrapingResult {
        jobs_found: 50,
        jobs_new: 50,
        jobs_updated: 0,
        high_matches: 5,
        alerts_sent: 5,
        errors: vec![],
    };

    assert!(result.errors.is_empty(), "Should have no errors");
    assert_eq!(result.jobs_new, result.jobs_found);
}

#[tokio::test]
async fn test_run_scraping_cycle_with_empty_config() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Run scraping cycle with empty config (no scraper URLs)
    let result = scheduler.run_scraping_cycle().await.unwrap();

    // Should complete without errors but find no jobs
    assert_eq!(result.jobs_found, 0);
    assert_eq!(result.jobs_new, 0);
    assert_eq!(result.jobs_updated, 0);
}

#[tokio::test]
async fn test_scheduler_config_interval_mapping() {
    let mut config = create_test_config();
    config.scraping_interval_hours = 6;
    let schedule_config = ScheduleConfig::from(&config);

    assert_eq!(schedule_config.interval_hours, 6);
}

#[test]
fn test_scraping_result_clone() {
    let result = ScrapingResult {
        jobs_found: 10,
        jobs_new: 5,
        jobs_updated: 5,
        high_matches: 2,
        alerts_sent: 1,
        errors: vec!["Error 1".to_string()],
    };

    let cloned = result.clone();

    assert_eq!(cloned.jobs_found, result.jobs_found);
    assert_eq!(cloned.jobs_new, result.jobs_new);
    assert_eq!(cloned.high_matches, result.high_matches);
    assert_eq!(cloned.errors, result.errors);
}

#[test]
fn test_schedule_config_clone() {
    let config = ScheduleConfig {
        interval_hours: 3,
        enabled: true,
    };

    let cloned = config.clone();

    assert_eq!(cloned.interval_hours, config.interval_hours);
    assert_eq!(cloned.enabled, config.enabled);
}

#[tokio::test]
async fn test_scheduler_concurrent_shutdowns() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Arc::new(Scheduler::new(Arc::clone(&config), Arc::clone(&database)));

    // Multiple concurrent shutdown calls should not panic
    let scheduler1 = Arc::clone(&scheduler);
    let scheduler2 = Arc::clone(&scheduler);
    let scheduler3 = Arc::clone(&scheduler);

    let handle1 = tokio::spawn(async move { scheduler1.shutdown() });
    let handle2 = tokio::spawn(async move { scheduler2.shutdown() });
    let handle3 = tokio::spawn(async move { scheduler3.shutdown() });

    // All should complete without error
    assert!(handle1.await.is_ok());
    assert!(handle2.await.is_ok());
    assert!(handle3.await.is_ok());
}

#[tokio::test]
async fn test_scheduler_shutdown_is_sticky_for_late_subscribers() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);

    let scheduler = Scheduler::new(Arc::clone(&config), Arc::clone(&database));

    // Trigger shutdown first
    scheduler.shutdown().unwrap();

    // Subscribe after shutdown
    let mut rx = scheduler.subscribe_shutdown();

    // Shutdown remains observable when the receiver is created after the request.
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;

    assert_eq!(result.unwrap().unwrap(), ());
}

#[tokio::test]
async fn test_shutdown_does_not_drop_an_active_cycle() {
    let config = Arc::new(RwLock::new(create_test_config()));
    let config_guard = config.write().await;
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let database = Arc::new(db);
    let scheduler = Arc::new(Scheduler::new_shared(
        Arc::clone(&config),
        Arc::clone(&database),
    ));
    let scheduler_clone = Arc::clone(&scheduler);
    let mut handle = tokio::spawn(async move { scheduler_clone.run_scraping_cycle().await });

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match scheduler.scrape_lock.try_lock() {
                Ok(guard) => drop(guard),
                Err(_) => break,
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    scheduler.shutdown().unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut handle)
            .await
            .is_err(),
        "an active cycle must reach a safe completion boundary before shutdown"
    );

    drop(config_guard);
    let error = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scraping cycle stopped after source audit completion"
    );
}

#[tokio::test]
async fn test_scheduler_does_not_start_a_cycle_after_shutdown() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let scheduler = Scheduler::new(config, Arc::new(db));
    let _scrape_guard = scheduler.scrape_lock.lock().await;

    scheduler.shutdown().unwrap();

    assert!(
        tokio::time::timeout(Duration::from_millis(100), scheduler.start())
            .await
            .unwrap()
            .is_ok()
    );
}

#[tokio::test]
async fn test_cycle_waiting_for_admission_stops_after_shutdown() {
    let config = Arc::new(create_test_config());
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let scheduler = Arc::new(Scheduler::new(config, Arc::new(db)));
    let scrape_guard = scheduler.scrape_lock.lock().await;
    let scheduler_clone = Arc::clone(&scheduler);
    let handle = tokio::spawn(async move { scheduler_clone.run_scraping_cycle().await });

    tokio::task::yield_now().await;
    scheduler.shutdown().unwrap();
    drop(scrape_guard);

    let error = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scraping cycle stopped before external work"
    );
}

#[tokio::test]
async fn test_scheduler_start_recovers_runs_for_disabled_sources() {
    let mut config = create_test_config();
    config.auto_refresh.enabled = false;
    let config = Arc::new(config);
    let db = Database::connect_memory().await.unwrap();
    db.migrate().await.unwrap();
    let interrupted_run_id = crate::health::start_run(&db, "disabled-source")
        .await
        .unwrap();
    let database = Arc::new(db);
    let scheduler = Arc::new(Scheduler::new(config, Arc::clone(&database)));
    let scheduler_clone = Arc::clone(&scheduler);
    let handle = tokio::spawn(async move { scheduler_clone.start().await });

    tokio::task::yield_now().await;
    scheduler.shutdown().unwrap();
    handle.await.unwrap().unwrap();

    let run = crate::health::get_scraper_runs(&database, "disabled-source", 1)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(run.id, interrupted_run_id);
    assert_eq!(run.status, crate::health::RunStatus::Failure);
    assert_eq!(run.error_code.as_deref(), Some("interrupted"));
}

#[test]
fn test_schedule_config_debug() {
    let config = ScheduleConfig {
        interval_hours: 2,
        enabled: true,
    };

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("interval_hours"));
    assert!(debug_str.contains("enabled"));
}

#[test]
fn test_scraping_result_debug() {
    let result = ScrapingResult {
        jobs_found: 10,
        jobs_new: 5,
        jobs_updated: 5,
        high_matches: 2,
        alerts_sent: 1,
        errors: vec![],
    };

    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("jobs_found"));
    assert!(debug_str.contains("high_matches"));
}
