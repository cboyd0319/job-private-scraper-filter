// Verifies shared scraper execution, audit, cancellation, and failure behavior.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static RUNNING: AtomicBool = AtomicBool::new(false);

async fn run_closing_scraper<S: JobScraper>(
    database: &Arc<Database>,
    scraper: &S,
) -> (ScraperRunOutcome, Vec<Job>, Vec<String>) {
    let mut jobs = Vec::new();
    let mut errors = Vec::new();
    let outcome = run_scraper(
        database,
        scraper,
        "stub",
        "Stub",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;
    (outcome, jobs, errors)
}

enum StubOutcome {
    Success(Vec<Job>),
    Timeout,
    Failure,
}

struct StubScraper {
    outcome: StubOutcome,
}

#[async_trait::async_trait]
impl JobScraper for StubScraper {
    async fn scrape(&self) -> Result<Vec<Job>, ScraperError> {
        match &self.outcome {
            StubOutcome::Success(jobs) => Ok(jobs.clone()),
            StubOutcome::Timeout => Err(ScraperError::Timeout { timeout_secs: 10 }),
            StubOutcome::Failure => Err(ScraperError::Network),
        }
    }
}

struct ObservedScraper {
    called: AtomicBool,
}

#[async_trait::async_trait]
impl JobScraper for ObservedScraper {
    async fn scrape(&self) -> Result<Vec<Job>, ScraperError> {
        self.called.store(true, Ordering::Release);
        Ok(vec![test_job()])
    }
}

struct ClosingScraper {
    database: Arc<Database>,
}

#[async_trait::async_trait]
impl JobScraper for ClosingScraper {
    async fn scrape(&self) -> Result<Vec<Job>, ScraperError> {
        self.database.close().await;
        Ok(vec![test_job()])
    }
}

struct ClosingFailureScraper {
    database: Arc<Database>,
}

#[async_trait::async_trait]
impl JobScraper for ClosingFailureScraper {
    async fn scrape(&self) -> Result<Vec<Job>, ScraperError> {
        self.database.close().await;
        Err(ScraperError::Network)
    }
}

async fn test_database() -> Arc<Database> {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    Arc::new(database)
}

fn test_job() -> Job {
    Job::newly_discovered(
        "Care Coordinator",
        "Community Care",
        "https://example.com/jobs/1",
        Some("Remote".to_string()),
        "stub",
        chrono::Utc::now(),
    )
}

#[tokio::test]
async fn scraper_runner_records_and_accumulates_success() {
    let database = test_database().await;
    let scraper = StubScraper {
        outcome: StubOutcome::Success(vec![test_job()]),
    };
    let mut jobs = Vec::new();
    let mut errors = Vec::new();

    let outcome = run_scraper(
        &database,
        &scraper,
        "stub",
        "Stub",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;

    assert_eq!(outcome, ScraperRunOutcome::Success { jobs_found: 1 });
    assert_eq!(jobs.len(), 1);
    assert!(errors.is_empty());
}

#[tokio::test]
async fn scraper_runner_preserves_partial_results_after_a_later_failure() {
    let database = test_database().await;
    let successful = StubScraper {
        outcome: StubOutcome::Success(vec![test_job()]),
    };
    let failed = StubScraper {
        outcome: StubOutcome::Failure,
    };
    let mut jobs = Vec::new();
    let mut errors = Vec::new();

    run_scraper(
        &database,
        &successful,
        "first",
        "First",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;
    let outcome = run_scraper(
        &database,
        &failed,
        "second",
        "Second",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;

    assert_eq!(outcome, ScraperRunOutcome::Failure);
    assert_eq!(jobs.len(), 1);
    assert_eq!(errors, ["Second source check failed (network)"]);
}

#[tokio::test]
async fn scraper_runner_preserves_timeout_outcome() {
    let database = test_database().await;
    let scraper = StubScraper {
        outcome: StubOutcome::Timeout,
    };
    let mut jobs = Vec::new();
    let mut errors = Vec::new();

    let outcome = run_scraper(
        &database,
        &scraper,
        "stub",
        "Stub",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;

    assert_eq!(outcome, ScraperRunOutcome::Timeout);
    assert!(jobs.is_empty());
    assert_eq!(errors, ["Stub source check failed (timeout)"]);
}

#[tokio::test]
async fn scraper_runner_does_not_call_external_source_without_audit_row() {
    let database = Arc::new(Database::connect_memory().await.unwrap());
    let scraper = ObservedScraper {
        called: AtomicBool::new(false),
    };
    let mut jobs = Vec::new();
    let mut errors = Vec::new();

    let outcome = run_scraper(
        &database,
        &scraper,
        "stub",
        "Stub",
        &RUNNING,
        &mut jobs,
        &mut errors,
    )
    .await;

    assert_eq!(outcome, ScraperRunOutcome::Failure);
    assert!(!scraper.called.load(Ordering::Acquire));
    assert!(jobs.is_empty());
    assert_eq!(errors, ["Stub source check failed (audit_unavailable)"]);
}

#[tokio::test]
async fn scraper_runner_does_not_report_success_when_terminal_audit_write_fails() {
    let database = test_database().await;
    let scraper = ClosingScraper {
        database: Arc::clone(&database),
    };
    let (outcome, jobs, errors) = run_closing_scraper(&database, &scraper).await;

    assert_eq!(outcome, ScraperRunOutcome::Failure);
    assert!(jobs.is_empty());
    assert_eq!(errors, ["Stub source check failed (audit_unavailable)"]);
}

#[tokio::test]
async fn scraper_runner_reports_failed_terminal_error_audit_write() {
    let database = test_database().await;
    let scraper = ClosingFailureScraper {
        database: Arc::clone(&database),
    };
    let (outcome, jobs, errors) = run_closing_scraper(&database, &scraper).await;

    assert_eq!(outcome, ScraperRunOutcome::Failure);
    assert!(jobs.is_empty());
    assert_eq!(
        errors,
        [
            "Stub source check failed (network)",
            "Stub source check failed (audit_unavailable)"
        ]
    );
}

#[tokio::test]
async fn scraper_runner_stops_before_the_next_external_source() {
    let database = test_database().await;
    let scraper = ObservedScraper {
        called: AtomicBool::new(false),
    };
    let shutdown_requested = AtomicBool::new(true);
    let mut jobs = Vec::new();
    let mut errors = Vec::new();

    let outcome = run_scraper(
        &database,
        &scraper,
        "stub",
        "Stub",
        &shutdown_requested,
        &mut jobs,
        &mut errors,
    )
    .await;

    assert_eq!(outcome, ScraperRunOutcome::Cancelled);
    assert!(!scraper.called.load(Ordering::Acquire));
    assert!(jobs.is_empty());
    assert!(errors.is_empty());
    assert!(crate::health::get_scraper_runs(&database, "stub", 1)
        .await
        .unwrap()
        .is_empty());
}

#[test]
fn source_failure_message_omits_raw_scraper_error_details() {
    let error = ScraperError::Generic {
        scraper: "Dice".to_string(),
        message: "query='private search' token=secret location=home".to_string(),
    };

    let message = source_failure_message("Dice", scraper_failure_kind(&error));

    assert_eq!(message, "Dice source check failed (generic)");
    assert!(!message.contains("private search"));
    assert!(!message.contains("token"));
    assert!(!message.contains("home"));
}

#[test]
fn scraper_failure_kind_keeps_coarse_timeout_category() {
    let error = ScraperError::Timeout { timeout_secs: 10 };

    assert_eq!(scraper_failure_kind(&error), "timeout");
}

#[test]
fn retired_restricted_source_message_does_not_offer_local_override() {
    assert_eq!(
        retired_restricted_source_message("Dice"),
        "Dice scheduled access is unavailable after provider policy review. Use the user-opened search link, Browser Import, or manual entry."
    );
}

#[tokio::test]
async fn restored_retired_source_flags_stop_before_transport() {
    let database = test_database().await;
    let credentials =
        CredentialService::with_fixed_master_key(database.credentials(), [7_u8; 32], false);
    let mut config = Config::first_run();
    config.builtin.enabled = true;
    config.dice.enabled = true;
    config.simplyhired.enabled = true;
    config.glassdoor.enabled = true;

    let (jobs, errors) = run_scrapers(&Arc::new(config), &database, &credentials, &RUNNING).await;

    assert!(jobs.is_empty());
    assert_eq!(
        errors,
        [
            retired_restricted_source_message("BuiltIn"),
            retired_restricted_source_message("Dice"),
            retired_restricted_source_message("SimplyHired"),
            retired_restricted_source_message("Glassdoor"),
        ]
    );
    for source_id in ["builtin", "dice", "simplyhired", "glassdoor"] {
        assert!(crate::health::get_scraper_runs(&database, source_id, 1)
            .await
            .unwrap()
            .is_empty());
    }
}
