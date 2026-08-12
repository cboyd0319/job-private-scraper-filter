//! Runs configured scrapers with source-governance audit and failure handling.

mod federal;
mod hn_hiring_worker;
mod jobswithgpt_worker;
mod public_ats_worker;
mod remoteok_worker;
mod weworkremotely_worker;

use crate::{config::Config, credentials::CredentialService};
use jobsentinel_domain::Job;
use jobsentinel_sources::{JobScraper, ScraperError, LINKEDIN_AUTOMATION_DISABLED_MESSAGE};
use jobsentinel_storage::Database;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn scraper_failure_kind(error: &ScraperError) -> &'static str {
    match error {
        ScraperError::Network => "network",
        ScraperError::HttpStatus { .. } => "http_status",
        ScraperError::ParseError { .. } => "parse_error",
        ScraperError::SelectorNotFound { .. } => "selector_not_found",
        ScraperError::MissingField { .. } => "missing_field",
        ScraperError::InvalidUrl { .. } => "invalid_url",
        ScraperError::BotProtection { .. } => "bot_protection",
        ScraperError::Timeout { .. } => "timeout",
        ScraperError::InvalidConfiguration { .. } => "invalid_configuration",
        ScraperError::Generic { .. } => "generic",
    }
}

fn source_failure_message(source_label: &'static str, failure_kind: &'static str) -> String {
    format!("{source_label} source check failed ({failure_kind})")
}

fn record_audit_failure(errors: &mut Vec<String>, source_label: &'static str) {
    tracing::error!(
        source = source_label,
        failure_kind = "audit_unavailable",
        "Source check audit unavailable"
    );
    errors.push(source_failure_message(source_label, "audit_unavailable"));
}

#[cfg(test)]
fn assert_scheduled_simulation_allowed(
    manifest: &jobsentinel_domain::v3_source_manifest::SourceManifest,
    policy: &jobsentinel_domain::v3_foundation::SourcePolicy,
    fixtures: &[(&str, &[u8])],
) {
    use jobsentinel_domain::{
        v3_source_authorization::{SourceActionDecision, SourceGrantState},
        v3_source_manifest::SourceOperation,
    };

    assert_eq!(
        manifest
            .simulate(
                policy,
                SourceOperation::ScheduledCheck,
                chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
                SourceGrantState::NotRequired,
                fixtures,
            )
            .unwrap()
            .decision,
        SourceActionDecision::Allowed {
            request_limit_per_hour: 500,
            connectivity_required: true,
        }
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScraperRunOutcome {
    Success { jobs_found: usize },
    Timeout,
    Failure,
    Cancelled,
}

pub(super) async fn run_scraper<S: JobScraper + ?Sized>(
    db: &Arc<Database>,
    scraper: &S,
    source_id: &'static str,
    source_label: &'static str,
    shutdown_requested: &AtomicBool,
    all_jobs: &mut Vec<Job>,
    errors: &mut Vec<String>,
) -> ScraperRunOutcome {
    if shutdown_requested.load(Ordering::Acquire) {
        return ScraperRunOutcome::Cancelled;
    }
    let run_id = match crate::health::start_run(db, source_id).await {
        Ok(run_id) => run_id,
        Err(_) => {
            record_audit_failure(errors, source_label);
            return ScraperRunOutcome::Failure;
        }
    };
    let started_at = std::time::Instant::now();

    match scraper.scrape().await {
        Ok(jobs) => {
            let jobs_found = jobs.len();
            if crate::health::complete_run(
                db,
                run_id,
                started_at.elapsed().as_millis() as i64,
                jobs_found,
                0,
            )
            .await
            .is_err()
            {
                record_audit_failure(errors, source_label);
                return ScraperRunOutcome::Failure;
            }
            tracing::info!(
                source = source_label,
                jobs_found,
                "Scraper source check completed"
            );
            all_jobs.extend(jobs);
            ScraperRunOutcome::Success { jobs_found }
        }
        Err(error) => {
            let outcome = if matches!(error, ScraperError::Timeout { .. }) {
                ScraperRunOutcome::Timeout
            } else {
                ScraperRunOutcome::Failure
            };
            record_scraper_failure(
                db,
                run_id,
                started_at.elapsed().as_millis() as i64,
                source_label,
                &error,
                errors,
            )
            .await;
            outcome
        }
    }
}

pub(super) async fn record_scraper_failure(
    db: &Arc<Database>,
    run_id: i64,
    duration_ms: i64,
    source_label: &'static str,
    error: &ScraperError,
    errors: &mut Vec<String>,
) {
    let failure_kind = scraper_failure_kind(error);
    let error_message = source_failure_message(source_label, failure_kind);

    let audit_result = if matches!(error, ScraperError::Timeout { .. }) {
        crate::health::timeout_run(db, run_id, duration_ms).await
    } else {
        crate::health::fail_run(db, run_id, duration_ms, &error_message, Some(failure_kind)).await
    };
    tracing::error!(
        source = source_label,
        failure_kind,
        "Scraper source check failed"
    );
    errors.push(error_message);
    if audit_result.is_err() {
        record_audit_failure(errors, source_label);
    }
}

fn record_source_credential_failure(errors: &mut Vec<String>, source_label: &'static str) {
    let failure_kind = "credential_unavailable";
    tracing::error!(
        source = source_label,
        failure_kind,
        "Source credential unavailable"
    );
    errors.push(source_failure_message(source_label, failure_kind));
}

fn retired_restricted_source_message(source_label: &'static str) -> String {
    format!(
        "{source_label} scheduled access is unavailable after provider policy review. Use the user-opened search link, Browser Import, or manual entry."
    )
}

fn record_retired_restricted_source(errors: &mut Vec<String>, source_label: &'static str) {
    tracing::warn!(
        source = source_label,
        "Restricted scheduled source disabled by provider policy"
    );
    errors.push(retired_restricted_source_message(source_label));
}

/// Run all configured scrapers and return jobs and errors
#[tracing::instrument(skip_all)]
pub(crate) async fn run_scrapers(
    config: &Arc<Config>,
    db: &Arc<Database>,
    credentials: &CredentialService,
    shutdown_requested: &AtomicBool,
) -> (Vec<Job>, Vec<String>) {
    tracing::info!("Starting scraper execution across all enabled sources");
    let mut all_jobs = Vec::new();
    let mut errors = Vec::new();

    public_ats_worker::run_greenhouse(config, db, shutdown_requested, &mut all_jobs, &mut errors)
        .await;
    public_ats_worker::run_lever(config, db, shutdown_requested, &mut all_jobs, &mut errors).await;

    jobswithgpt_worker::run_jobswithgpt_scraper(
        config.as_ref(),
        db,
        shutdown_requested,
        &mut all_jobs,
        &mut errors,
    )
    .await;

    // 4. LinkedIn stays user-directed. Warn without running hidden monitoring.
    if config.linkedin.enabled {
        tracing::warn!("{}", LINKEDIN_AUTOMATION_DISABLED_MESSAGE);
        errors.push(LINKEDIN_AUTOMATION_DISABLED_MESSAGE.to_string());
    }

    remoteok_worker::run_remoteok(
        config.as_ref(),
        db,
        shutdown_requested,
        &mut all_jobs,
        &mut errors,
    )
    .await;

    weworkremotely_worker::run_weworkremotely(
        config.as_ref(),
        db,
        shutdown_requested,
        &mut all_jobs,
        &mut errors,
    )
    .await;

    if config.builtin.enabled {
        record_retired_restricted_source(&mut errors, "BuiltIn");
    }

    hn_hiring_worker::run_hn_hiring(
        config.as_ref(),
        db,
        shutdown_requested,
        &mut all_jobs,
        &mut errors,
    )
    .await;

    if config.dice.enabled {
        record_retired_restricted_source(&mut errors, "Dice");
    }

    federal::run_usajobs(
        config,
        db,
        credentials,
        shutdown_requested,
        &mut all_jobs,
        &mut errors,
    )
    .await;
    if config.simplyhired.enabled {
        record_retired_restricted_source(&mut errors, "SimplyHired");
    }
    if config.glassdoor.enabled {
        record_retired_restricted_source(&mut errors, "Glassdoor");
    }

    tracing::info!(
        "Scraper execution complete: {} total jobs, {} errors",
        all_jobs.len(),
        errors.len()
    );

    if !errors.is_empty() {
        tracing::warn!("Scraping errors encountered: {:?}", errors);
    }

    (all_jobs, errors)
}

#[cfg(test)]
mod tests;
