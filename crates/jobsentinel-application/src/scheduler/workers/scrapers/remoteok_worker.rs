// Runs the governed RemoteOK source worker.

use std::{
    num::NonZeroU16,
    sync::{atomic::AtomicBool, Arc},
};

use chrono::Utc;
use jobsentinel_domain::{
    v3_source_authorization::SourceActionDecision, v3_source_manifest::SourceOperation, Job,
};
use jobsentinel_sources::RemoteOkScraper;
use jobsentinel_storage::Database;

use crate::config::Config;

use super::run_scraper;

pub(super) async fn run_remoteok(
    config: &Config,
    database: &Arc<Database>,
    shutdown_requested: &AtomicBool,
    all_jobs: &mut Vec<Job>,
    errors: &mut Vec<String>,
) {
    if !config.remoteok.enabled {
        return;
    }
    let request_limit_per_hour = match crate::v3_source_governance::authorize_remoteok(
        database,
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
            errors.push(
                "RemoteOK source check skipped because its reviewed source policy is unavailable or stale"
                    .to_string(),
            );
            return;
        }
    };
    let Some(request_limit_per_hour) = request_limit_per_hour else {
        errors.push(
            "RemoteOK source check skipped because its reviewed source policy is invalid"
                .to_string(),
        );
        return;
    };
    tracing::info!("Running RemoteOK scraper");
    let scraper = RemoteOkScraper::new(config.remoteok.tags.clone(), config.remoteok.limit)
        .with_request_limit_per_hour(request_limit_per_hour);
    run_scraper(
        database,
        &scraper,
        "remoteok",
        "RemoteOK",
        shutdown_requested,
        all_jobs,
        errors,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobsentinel_domain::v3_source_manifest::{
        parse_source_manifest, REMOTEOK_SOURCE_MANIFEST_V1,
    };

    use crate::test_support::minimal_test_config;

    #[tokio::test]
    async fn missing_governance_stops_remoteok_before_source_access() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let mut config = minimal_test_config();
        config.remoteok.enabled = true;
        let mut jobs = Vec::new();
        let mut errors = Vec::new();

        super::run_remoteok(
            &config,
            &database,
            &AtomicBool::new(false),
            &mut jobs,
            &mut errors,
        )
        .await;

        assert!(jobs.is_empty());
        assert_eq!(
            errors,
            ["RemoteOK source check skipped because its reviewed source policy is unavailable or stale"]
        );
    }

    #[tokio::test]
    async fn installed_governance_authorizes_the_remoteok_policy_rate() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        crate::v3_source_governance::install_remoteok(&database)
            .await
            .unwrap();

        assert_eq!(
            crate::v3_source_governance::authorize_remoteok(
                &database,
                SourceOperation::ScheduledCheck,
                chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
            )
            .await
            .unwrap(),
            SourceActionDecision::Allowed {
                request_limit_per_hour: 500,
                connectivity_required: true,
            }
        );
    }

    #[test]
    fn remoteok_source_simulator_binds_reviewed_parser_and_policy_fixtures() {
        let policy = crate::v3_source_governance::remoteok_policy().unwrap();
        let manifest = parse_source_manifest(REMOTEOK_SOURCE_MANIFEST_V1, &policy).unwrap();
        let fixtures = [
            (
                "crates/jobsentinel-sources/src/fixtures/remoteok_list_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-sources/src/fixtures/remoteok_list_v1.json"
                )
                .as_slice(),
            ),
            (
                "crates/jobsentinel-sources/src/fixtures/remoteok_detail_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-sources/src/fixtures/remoteok_detail_v1.json"
                )
                .as_slice(),
            ),
            (
                "crates/jobsentinel-domain/src/fixtures/source_reviews/remoteok_v1.json",
                include_bytes!(
                    "../../../../../jobsentinel-domain/src/fixtures/source_reviews/remoteok_v1.json"
                )
                .as_slice(),
            ),
        ];

        super::super::assert_scheduled_simulation_allowed(&manifest, &policy, &fixtures);
    }
}
