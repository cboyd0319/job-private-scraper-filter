use std::{
    num::NonZeroU16,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::{
    config::Config,
    credentials::{CredentialKey, CredentialService},
};
use chrono::Utc;
use jobsentinel_domain::{
    v3_source_authorization::SourceActionDecision, v3_source_manifest::SourceOperation, Job,
};
use jobsentinel_sources::UsaJobsScraper;
use jobsentinel_storage::Database;

use super::{record_source_credential_failure, run_scraper};

pub(super) async fn run_usajobs(
    config: &Arc<Config>,
    db: &Arc<Database>,
    credentials: &CredentialService,
    shutdown_requested: &AtomicBool,
    all_jobs: &mut Vec<Job>,
    errors: &mut Vec<String>,
) {
    // 11. USAJobs federal government scraper - requires API key from keyring
    if config.usajobs.enabled && !config.usajobs.email.is_empty() {
        if shutdown_requested.load(Ordering::Acquire) {
            return;
        }
        let request_limit_per_hour = match crate::v3_source_governance::authorize_usajobs(
            db,
            SourceOperation::ScheduledCheck,
            Utc::now().date_naive(),
        )
        .await
        {
            Ok(SourceActionDecision::Allowed {
                request_limit_per_hour,
                connectivity_required: true,
            }) => NonZeroU16::new(request_limit_per_hour),
            Ok(_) | Err(_) => {
                tracing::warn!(
                    source = "usajobs",
                    "USAJobs source check skipped because current source governance did not authorize it"
                );
                errors.push(
                    "USAJobs source check skipped because its reviewed source policy is unavailable or stale"
                        .to_string(),
                );
                return;
            }
        };
        let Some(request_limit_per_hour) = request_limit_per_hour else {
            errors.push(
                "USAJobs source check skipped because its reviewed source policy is invalid"
                    .to_string(),
            );
            return;
        };
        match credentials.retrieve(CredentialKey::UsaJobsApiKey).await {
            Ok(Some(api_key)) => {
                tracing::info!("Running USAJobs scraper");
                let mut scraper = UsaJobsScraper::new(api_key, config.usajobs.email.clone());

                // Apply configuration
                if let Some(ref kw) = config.usajobs.keywords {
                    scraper = scraper.with_keywords(kw.clone());
                }
                if let Some(ref loc) = config.usajobs.location {
                    scraper = scraper.with_location(loc.clone(), config.usajobs.radius);
                }
                if config.usajobs.remote_only {
                    scraper = scraper.remote_only();
                }
                if config.usajobs.pay_grade_min.is_some() || config.usajobs.pay_grade_max.is_some()
                {
                    scraper = scraper
                        .with_pay_grade(config.usajobs.pay_grade_min, config.usajobs.pay_grade_max);
                }
                scraper = scraper
                    .posted_within_days(config.usajobs.date_posted_days)
                    .with_limit(config.usajobs.limit)
                    .with_request_limit_per_hour(request_limit_per_hour);

                run_scraper(
                    db,
                    &scraper,
                    "usajobs",
                    "USAJobs",
                    shutdown_requested,
                    all_jobs,
                    errors,
                )
                .await;
            }
            Ok(None) => {
                tracing::warn!("USAJobs enabled but API key not configured in keyring");
            }
            Err(_e) => {
                record_source_credential_failure(errors, "USAJobs");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::minimal_test_config;
    use jobsentinel_domain::v3_source_authorization::SourceGrantState;

    #[tokio::test]
    async fn shutdown_skips_usajobs_before_credential_access() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let credentials =
            CredentialService::with_fixed_master_key(database.credentials(), [7; 32], false);
        database.close().await;
        let mut config = minimal_test_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        let shutdown_requested = AtomicBool::new(true);
        let mut jobs = Vec::new();
        let mut errors = Vec::new();

        run_usajobs(
            &Arc::new(config),
            &database,
            &credentials,
            &shutdown_requested,
            &mut jobs,
            &mut errors,
        )
        .await;

        assert!(jobs.is_empty());
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn installed_usajobs_governance_requires_approved_use_review() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

        crate::v3_source_governance::install_usajobs(&database)
            .await
            .unwrap();

        assert_eq!(
            crate::v3_source_governance::authorize_usajobs(
                &database,
                SourceOperation::ScheduledCheck,
                today,
            )
            .await
            .unwrap(),
            SourceActionDecision::ReviewRequired
        );
    }

    #[test]
    fn usajobs_source_simulator_binds_reviewed_parser_and_policy_fixtures() {
        let policy = crate::v3_source_governance::usajobs_policy().unwrap();
        let manifest = jobsentinel_domain::v3_source_manifest::parse_source_manifest(
            jobsentinel_domain::v3_source_manifest::USAJOBS_SOURCE_MANIFEST_V1,
            &policy,
        )
        .unwrap();
        let fixtures = [
            (
                "crates/jobsentinel-sources/src/fixtures/usajobs_list_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-sources/src/fixtures/usajobs_list_v1.json"
                )
                .as_slice(),
            ),
            (
                "crates/jobsentinel-sources/src/fixtures/usajobs_detail_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-sources/src/fixtures/usajobs_detail_v1.json"
                )
                .as_slice(),
            ),
            (
                "crates/jobsentinel-domain/src/fixtures/source_reviews/usajobs_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-domain/src/fixtures/source_reviews/usajobs_v1.json"
                )
                .as_slice(),
            ),
        ];

        assert_eq!(
            manifest
                .simulate(
                    &policy,
                    SourceOperation::ScheduledCheck,
                    chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
                    SourceGrantState::NotRequired,
                    &fixtures,
                )
                .unwrap()
                .decision,
            SourceActionDecision::ReviewRequired
        );
    }

    #[tokio::test]
    async fn missing_governance_stops_before_credential_repository_access() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let credentials =
            CredentialService::with_fixed_master_key(database.credentials(), [7; 32], false);
        database.close().await;
        let mut config = minimal_test_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        let mut jobs = Vec::new();
        let mut errors = Vec::new();

        run_usajobs(
            &Arc::new(config),
            &database,
            &credentials,
            &AtomicBool::new(false),
            &mut jobs,
            &mut errors,
        )
        .await;

        assert!(jobs.is_empty());
        assert_eq!(
            errors,
            vec![
                "USAJobs source check skipped because its reviewed source policy is unavailable or stale"
            ]
        );
    }

    async fn unavailable_credentials() -> CredentialService {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let credentials =
            CredentialService::with_fixed_master_key(database.credentials(), [7; 32], false);
        database.close().await;
        credentials
    }

    async fn run_enabled_usajobs(
        database: Arc<Database>,
        credentials: &CredentialService,
    ) -> Vec<String> {
        let mut config = minimal_test_config();
        config.usajobs.enabled = true;
        config.usajobs.email = "test@example.com".to_string();
        let mut jobs = Vec::new();
        let mut errors = Vec::new();
        run_usajobs(
            &Arc::new(config),
            &database,
            credentials,
            &AtomicBool::new(false),
            &mut jobs,
            &mut errors,
        )
        .await;
        assert!(jobs.is_empty());
        errors
    }

    #[tokio::test]
    async fn same_revision_policy_drift_stops_before_credential_access() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let mut policy = crate::v3_source_governance::usajobs_policy().unwrap();
        policy.request_limit_per_hour = 24;
        database.upsert_source_policy(&policy).await.unwrap();
        let manifest = jobsentinel_domain::v3_source_manifest::parse_source_manifest(
            jobsentinel_domain::v3_source_manifest::USAJOBS_SOURCE_MANIFEST_V1,
            &policy,
        )
        .unwrap();
        database.store_source_manifest(&manifest).await.unwrap();

        assert_eq!(
            run_enabled_usajobs(database, &unavailable_credentials().await).await,
            vec![
                "USAJobs source check skipped because its reviewed source policy is unavailable or stale"
            ]
        );
    }

    #[tokio::test]
    async fn same_revision_manifest_drift_stops_before_credential_access() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let policy = crate::v3_source_governance::usajobs_policy().unwrap();
        database.upsert_source_policy(&policy).await.unwrap();
        let mut manifest = jobsentinel_domain::v3_source_manifest::parse_source_manifest(
            jobsentinel_domain::v3_source_manifest::USAJOBS_SOURCE_MANIFEST_V1,
            &policy,
        )
        .unwrap();
        manifest.confidence_percent -= 1;
        database.store_source_manifest(&manifest).await.unwrap();

        assert_eq!(
            run_enabled_usajobs(database, &unavailable_credentials().await).await,
            vec![
                "USAJobs source check skipped because its reviewed source policy is unavailable or stale"
            ]
        );
    }
}
