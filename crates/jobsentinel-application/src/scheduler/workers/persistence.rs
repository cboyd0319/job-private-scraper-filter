//! Database persistence and notification logic

use crate::{
    config::Config,
    credentials::CredentialService,
    notify::{Notification, NotificationService},
    scoring::{JobScore, ScoringEngine},
};
use jobsentinel_storage::Database;
use std::sync::Arc;

use jobsentinel_storage::database_error_kind;

/// Statistics from persistence and notification operations
#[derive(Debug)]
pub(crate) struct PersistenceStats {
    pub jobs_new: usize,
    pub jobs_updated: usize,
    pub high_matches: usize,
    pub alerts_sent: usize,
    pub errors: Vec<String>,
}

/// Persist jobs to database and send notifications for high-scoring jobs
#[tracing::instrument(skip_all, fields(job_count = scored_jobs.len()), level = "info")]
pub(crate) async fn persist_and_notify(
    scored_jobs: &[(jobsentinel_domain::Job, JobScore)],
    config: &Arc<Config>,
    database: &Arc<Database>,
    credentials: &Arc<CredentialService>,
) -> PersistenceStats {
    use std::time::Instant;

    let start = Instant::now();
    let mut jobs_new = 0;
    let mut jobs_updated = 0;
    let mut high_matches = 0;
    let mut alerts_sent = 0;
    let mut errors = Vec::new();

    // Store in database
    let job_count = scored_jobs.len();
    tracing::debug!(job_count, "Starting database persistence");

    for (job, _score) in scored_jobs {
        // Check if job exists before upserting
        let was_existing = database
            .get_job_by_hash(&job.hash)
            .await
            .ok()
            .flatten()
            .is_some();

        if was_existing {
            jobs_updated += 1;
        } else {
            jobs_new += 1;
        }

        if let Err(e) = database.upsert_job(job).await {
            tracing::error!(
                job_hash = %job.hash,
                error_kind = database_error_kind(&e),
                "Failed to upsert job"
            );
            errors.push(format!(
                "Database error while saving one job ({})",
                database_error_kind(&e)
            ));
        }

        // Track reposts for ghost detection
        if let Err(e) = database
            .track_repost(&job.hash, &job.company, &job.title, &job.source)
            .await
        {
            tracing::debug!(
                job_hash = %job.hash,
                error_kind = database_error_kind(&e),
                "Failed to track repost"
            );
        }
    }

    let persist_duration = start.elapsed();
    tracing::info!(
        jobs_new,
        jobs_updated,
        elapsed_ms = persist_duration.as_millis(),
        "Database persistence complete"
    );

    // Send notifications for high-scoring jobs
    let notify_start = Instant::now();
    tracing::debug!("Processing notifications");
    let notification_service =
        NotificationService::with_credentials(Arc::clone(config), Arc::clone(credentials));
    let scoring_engine = ScoringEngine::new(Arc::clone(config));

    for (job, score) in scored_jobs {
        if scoring_engine.should_alert_immediately(score) {
            high_matches += 1;

            // Claim before delivery to preserve at-most-once external alert attempts.
            // Ambiguous or failed attempts are visible but never retried automatically.
            match database.claim_immediate_alert(&job.hash).await {
                Ok(true) => {}
                Ok(false) => continue,
                Err(e) => {
                    tracing::error!(
                        job_hash = %job.hash,
                        error_kind = database_error_kind(&e),
                        "Failed to claim alert delivery"
                    );
                    errors.push(format!(
                        "Database error while claiming one alert ({})",
                        database_error_kind(&e)
                    ));
                    continue;
                }
            }

            let notification = Notification {
                job: job.clone(),
                score: score.clone(),
            };

            match notification_service
                .send_immediate_alert(&notification)
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        job_hash = %job.hash,
                        job_score = score.total,
                        "Notification alert sent"
                    );
                    alerts_sent += 1;
                }
                Err(_e) => {
                    tracing::error!(
                        job_hash = %job.hash,
                        error_kind = "notification_delivery",
                        "Failed to send notification alert"
                    );
                    errors.push("Notification delivery error for one job".to_string());
                }
            }
        }
    }

    let notify_duration = notify_start.elapsed();
    tracing::info!(
        high_matches,
        alerts_sent,
        elapsed_ms = notify_duration.as_millis(),
        "Notifications complete"
    );

    PersistenceStats {
        jobs_new,
        jobs_updated,
        high_matches,
        alerts_sent,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        scoring::{JobScore, ScoreBreakdown},
        test_support::{minimal_test_config, test_job},
    };

    #[tokio::test]
    async fn failed_notification_is_not_retried_automatically() {
        let mut config = minimal_test_config();
        config.alerts.slack.enabled = true;
        let config = Arc::new(config);
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let credentials = Arc::new(CredentialService::with_fixed_master_key(
            database.credentials(),
            [7; 32],
            false,
        ));
        let job = test_job("notification-failure", "Care Coordinator", "Community Care");
        let score = JobScore {
            total: 1.0,
            breakdown: ScoreBreakdown {
                skills: 0.4,
                salary: 0.25,
                location: 0.2,
                company: 0.1,
                recency: 0.05,
            },
            reasons: vec![],
        };

        let first = persist_and_notify(
            &[(job.clone(), score.clone())],
            &config,
            &database,
            &credentials,
        )
        .await;
        let second = persist_and_notify(&[(job, score)], &config, &database, &credentials).await;

        assert_eq!(first.alerts_sent, 0);
        assert_eq!(first.errors, ["Notification delivery error for one job"]);
        assert!(second.errors.is_empty());
        assert!(
            database
                .get_job_by_hash("notification-failure")
                .await
                .unwrap()
                .unwrap()
                .immediate_alert_sent
        );
    }
}
