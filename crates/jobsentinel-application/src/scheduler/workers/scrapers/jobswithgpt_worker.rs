use super::{record_audit_failure, run_scraper, ScraperRunOutcome};
use crate::{config::Config, health::SourceRequestOutcome};
use chrono::Utc;
use jobsentinel_domain::{
    v3_source_authorization::SourceActionDecision, v3_source_manifest::SourceOperation, Job,
};
use jobsentinel_sources::{JobQuery, JobsWithGptScraper};
use jobsentinel_storage::Database;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

fn endpoint_host_for_source_request(endpoint: &str) -> Option<String> {
    url::Url::parse(endpoint)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
}

fn required_source_request_id(
    result: anyhow::Result<i64>,
    errors: &mut Vec<String>,
) -> Option<i64> {
    match result {
        Ok(request_id) => Some(request_id),
        Err(_) => {
            record_audit_failure(errors, "JobsWithGPT");
            None
        }
    }
}

fn require_terminal_source_request_audit(
    result: anyhow::Result<()>,
    outcome: ScraperRunOutcome,
    all_jobs: &mut Vec<Job>,
    errors: &mut Vec<String>,
) {
    if result.is_err() {
        if let ScraperRunOutcome::Success { jobs_found } = outcome {
            all_jobs.truncate(all_jobs.len().saturating_sub(jobs_found));
        }
        record_audit_failure(errors, "JobsWithGPT");
    }
}

pub(super) async fn run_jobswithgpt_scraper(
    config: &Config,
    db: &Arc<Database>,
    shutdown_requested: &AtomicBool,
    all_jobs: &mut Vec<Job>,
    errors: &mut Vec<String>,
) {
    if shutdown_requested.load(Ordering::Acquire) {
        return;
    }
    let Some(jobswithgpt_payload) = config.jobswithgpt_payload_preview() else {
        return;
    };

    if !config.jobswithgpt_payload_approved() {
        tracing::warn!(
            source = "JobsWithGPT",
            title_count = jobswithgpt_payload.titles.len(),
            has_location = jobswithgpt_payload.location.is_some(),
            remote_only = jobswithgpt_payload.remote_only,
            limit = jobswithgpt_payload.limit,
            "JobsWithGPT source check skipped because the exact payload has not been reviewed and approved"
        );
        return;
    }

    if !matches!(
        crate::v3_source_governance::authorize_jobswithgpt(
            db,
            SourceOperation::ScheduledCheck,
            Utc::now().date_naive(),
        )
        .await,
        Ok(SourceActionDecision::Allowed {
            connectivity_required: true,
            ..
        })
    ) {
        tracing::warn!(
            source = "JobsWithGPT",
            "JobsWithGPT source check skipped because its provider policy requires review"
        );
        errors.push(
            "JobsWithGPT source check skipped because its provider policy requires review"
                .to_string(),
        );
        return;
    }

    let jobswithgpt_query = JobQuery {
        titles: jobswithgpt_payload.titles.clone(),
        location: jobswithgpt_payload.location.clone(),
        remote_only: jobswithgpt_payload.remote_only,
        limit: jobswithgpt_payload.limit,
    };

    let jobswithgpt =
        JobsWithGptScraper::new(jobswithgpt_payload.endpoint.clone(), jobswithgpt_query);

    tracing::info!(
        source = "JobsWithGPT",
        title_count = jobswithgpt_payload.titles.len(),
        has_location = jobswithgpt_payload.location.is_some(),
        remote_only = jobswithgpt_payload.remote_only,
        limit = jobswithgpt_payload.limit,
        "JobsWithGPT source check approved; sending minimized payload"
    );

    let endpoint_host = endpoint_host_for_source_request(&jobswithgpt_payload.endpoint);
    let Some(source_request_id) = required_source_request_id(
        crate::health::record_source_request_started(
            db,
            "jobswithgpt",
            endpoint_host.as_deref(),
            jobswithgpt_payload.titles.len(),
            jobswithgpt_payload.location.is_some(),
            jobswithgpt_payload.remote_only,
            jobswithgpt_payload.limit,
        )
        .await,
        errors,
    ) else {
        return;
    };

    let outcome = run_scraper(
        db,
        &jobswithgpt,
        "jobswithgpt",
        "JobsWithGPT",
        shutdown_requested,
        all_jobs,
        errors,
    )
    .await;
    let source_request_outcome = match outcome {
        ScraperRunOutcome::Success { .. } => SourceRequestOutcome::Success,
        ScraperRunOutcome::Timeout => SourceRequestOutcome::Timeout,
        ScraperRunOutcome::Failure | ScraperRunOutcome::Cancelled => SourceRequestOutcome::Failure,
    };
    require_terminal_source_request_audit(
        crate::health::finish_source_request(db, source_request_id, source_request_outcome).await,
        outcome,
        all_jobs,
        errors,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::minimal_test_config;

    #[test]
    fn unavailable_request_audit_stops_the_external_source() {
        let mut errors = Vec::new();

        assert!(
            required_source_request_id(Err(anyhow::anyhow!("audit unavailable")), &mut errors)
                .is_none()
        );
        assert_eq!(
            errors,
            ["JobsWithGPT source check failed (audit_unavailable)"]
        );
    }

    #[test]
    fn terminal_audit_failure_discards_results_and_surfaces_the_gap() {
        let mut errors = Vec::new();
        let mut jobs = vec![crate::test_support::test_job(
            "external-job",
            "Public Job",
            "Example",
        )];

        require_terminal_source_request_audit(
            Err(anyhow::anyhow!("audit unavailable")),
            ScraperRunOutcome::Success { jobs_found: 1 },
            &mut jobs,
            &mut errors,
        );

        assert!(jobs.is_empty());
        assert_eq!(
            errors,
            ["JobsWithGPT source check failed (audit_unavailable)"]
        );
    }

    #[tokio::test]
    async fn exact_payload_approval_does_not_bypass_missing_source_governance() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let mut config = minimal_test_config();
        config.title_allowlist = vec!["Case Manager".to_string()];
        config.jobswithgpt_endpoint = "https://api.jobswithgpt.example/mcp".to_string();
        config.jobswithgpt_approval.enabled = true;
        config.jobswithgpt_approval.payload = config.jobswithgpt_payload_preview();
        let mut jobs = Vec::new();
        let mut errors = Vec::new();

        run_jobswithgpt_scraper(
            &config,
            &database,
            &AtomicBool::new(false),
            &mut jobs,
            &mut errors,
        )
        .await;

        assert!(jobs.is_empty());
        assert!(
            crate::health::get_latest_source_request(&database, "jobswithgpt")
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            errors,
            ["JobsWithGPT source check skipped because its provider policy requires review"]
        );
    }
}
