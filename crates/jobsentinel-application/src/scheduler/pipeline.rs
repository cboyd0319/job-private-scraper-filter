//! Main scraping pipeline orchestration

use anyhow::Result;
use std::sync::Arc;

use super::types::{Scheduler, ScrapingResult};
use super::workers::{persist_and_notify, run_scrapers, score_jobs};

impl Scheduler {
    /// Run a single scraping cycle
    ///
    /// This is the main pipeline:
    /// 1. Run all scrapers (Greenhouse, Lever, JobsWithGPT)
    /// 2. Score each job
    /// 3. Store in database (with deduplication)
    /// 4. Send notifications for high-scoring jobs
    #[tracing::instrument(skip(self), level = "info")]
    pub async fn run_scraping_cycle(&self) -> Result<ScrapingResult> {
        use std::time::Instant;

        let _scrape_guard = self.scrape_lock.lock().await;
        anyhow::ensure!(
            !self.is_shutdown_requested(),
            "Scraping cycle stopped before external work"
        );
        let cycle_start = Instant::now();
        tracing::info!("Starting full scraping cycle");

        let config = {
            let config = self.config.read().await;
            Arc::new(config.clone())
        };

        // 1. Run all scrapers
        let stage1_start = Instant::now();
        tracing::info!("Pipeline stage 1/3: Running scrapers");
        let (all_jobs, mut errors) = run_scrapers(
            &config,
            &self.database,
            &self.credentials,
            &self.shutdown_requested,
        )
        .await;
        let stage1_duration = stage1_start.elapsed();
        tracing::info!(
            job_count = all_jobs.len(),
            elapsed_ms = stage1_duration.as_millis(),
            "Stage 1 complete: Scrapers finished"
        );
        anyhow::ensure!(
            !self.is_shutdown_requested(),
            "Scraping cycle stopped after source audit completion"
        );

        // 2. Score all jobs and run ghost detection
        let stage2_start = Instant::now();
        tracing::info!("Pipeline stage 2/3: Scoring jobs and detecting ghost postings");
        let scored_jobs = score_jobs(all_jobs, &config, &self.database).await;
        let stage2_duration = stage2_start.elapsed();
        tracing::info!(
            job_count = scored_jobs.len(),
            elapsed_ms = stage2_duration.as_millis(),
            "Stage 2 complete: Scoring and ghost detection finished"
        );
        anyhow::ensure!(
            !self.is_shutdown_requested(),
            "Scraping cycle stopped before persistence and notifications"
        );

        // 3. Store in database and send notifications
        let stage3_start = Instant::now();
        tracing::info!("Pipeline stage 3/3: Persisting jobs and sending notifications");
        let stats =
            persist_and_notify(&scored_jobs, &config, &self.database, &self.credentials).await;
        let stage3_duration = stage3_start.elapsed();
        tracing::info!(
            elapsed_ms = stage3_duration.as_millis(),
            "Stage 3 complete: Persistence and notifications finished"
        );

        // Combine errors from all stages
        errors.extend(stats.errors);

        let total_duration = cycle_start.elapsed();
        tracing::info!(
            jobs_new = stats.jobs_new,
            jobs_updated = stats.jobs_updated,
            high_matches = stats.high_matches,
            alerts_sent = stats.alerts_sent,
            error_count = errors.len(),
            total_elapsed_ms = total_duration.as_millis(),
            stage1_ms = stage1_duration.as_millis(),
            stage2_ms = stage2_duration.as_millis(),
            stage3_ms = stage3_duration.as_millis(),
            "Scraping cycle complete"
        );

        Ok(ScrapingResult {
            jobs_found: scored_jobs.len(),
            jobs_new: stats.jobs_new,
            jobs_updated: stats.jobs_updated,
            high_matches: stats.high_matches,
            alerts_sent: stats.alerts_sent,
            errors,
        })
    }
}
