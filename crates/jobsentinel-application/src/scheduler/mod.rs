//! Job Scraping Scheduler
//!
//! Manages periodic job scraping based on user configuration.

use anyhow::Result;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::{sync::RwLock, time};

// Module declarations
mod pipeline;
mod types;
mod workers;

// Re-exports
pub use types::{ScheduleConfig, Scheduler, ScrapingResult};

impl Scheduler {
    pub fn new(
        config: Arc<crate::config::Config>,
        database: Arc<jobsentinel_storage::Database>,
    ) -> Self {
        Self::new_shared(Arc::new(RwLock::new((*config).clone())), database)
    }

    pub fn new_shared(
        config: Arc<RwLock<crate::config::Config>>,
        database: Arc<jobsentinel_storage::Database>,
    ) -> Self {
        Self::new_shared_with_credentials(
            config,
            database,
            Arc::new(crate::credentials::CredentialService::compatibility_keyring()),
        )
    }

    pub fn new_shared_with_credentials(
        config: Arc<RwLock<crate::config::Config>>,
        database: Arc<jobsentinel_storage::Database>,
        credentials: Arc<crate::credentials::CredentialService>,
    ) -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        Self {
            config,
            database,
            credentials,
            shutdown_tx,
            shutdown_requested: AtomicBool::new(false),
            scrape_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Get a shutdown signal receiver
    ///
    /// This can be used to gracefully stop the scheduler
    pub fn subscribe_shutdown(&self) -> tokio::sync::broadcast::Receiver<()> {
        let receiver = self.shutdown_tx.subscribe();
        if self.is_shutdown_requested() {
            self.shutdown_tx.send(()).ok();
        }
        receiver
    }

    /// Shutdown the scheduler gracefully
    pub fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down scheduler");
        self.shutdown_requested.store(true, Ordering::Release);
        self.shutdown_tx.send(()).ok();
        Ok(())
    }

    /// Reconcile source checks interrupted before their terminal audit write.
    pub async fn recover_interrupted_runs(&self) -> Result<u64> {
        let scraper_runs = crate::health::interrupt_running_runs(&self.database)
            .await
            .map_err(|_| anyhow::anyhow!("Could not recover interrupted source checks"))?;
        let request_attempts = crate::health::interrupt_started_source_requests(&self.database)
            .await
            .map_err(|_| anyhow::anyhow!("Could not recover interrupted source requests"))?;
        Ok(scraper_runs.saturating_add(request_attempts))
    }

    /// Start the scheduler
    ///
    /// This runs in the background and triggers job scraping at regular intervals.
    /// The scheduler will continue running until a shutdown signal is received.
    ///
    /// # Cancellation
    ///
    /// The scheduler can be stopped gracefully by calling `shutdown()` on the Scheduler instance.
    pub async fn start(&self) -> Result<()> {
        let mut shutdown_rx = self.subscribe_shutdown();
        let interrupted_runs = self.recover_interrupted_runs().await?;
        if interrupted_runs > 0 {
            tracing::warn!(
                interrupted_runs,
                "Recovered interrupted source checks before scheduler start"
            );
        }
        if self.is_shutdown_requested() {
            return Ok(());
        }

        loop {
            if self.is_shutdown_requested() {
                break;
            }

            let schedule = {
                let config = self.config.read().await;
                ScheduleConfig::from(&*config)
            };

            if !schedule.enabled {
                tracing::info!("Scheduler: auto-refresh disabled; waiting before rechecking");

                tokio::select! {
                    _ = time::sleep(Duration::from_mins(1)) => {
                        continue;
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Scheduler received shutdown signal, stopping gracefully");
                        break;
                    }
                }
            }

            let interval = Duration::from_secs(schedule.interval_hours.saturating_mul(3600));

            tracing::info!("Scheduler: Running job scraping cycle");

            match self.run_scraping_cycle().await {
                Ok(result) => {
                    tracing::info!(
                        "Scraping cycle complete: {} jobs found, {} new, {} high matches, {} alerts sent",
                        result.jobs_found,
                        result.jobs_new,
                        result.high_matches,
                        result.alerts_sent
                    );

                    if !result.errors.is_empty() {
                        tracing::warn!(
                            error_count = result.errors.len(),
                            "Errors occurred during scraping"
                        );
                    }
                }
                Err(_e) => {
                    tracing::error!("Scraping cycle failed");
                }
            }

            if self.is_shutdown_requested() {
                tracing::info!("Scheduler reached a safe shutdown checkpoint after active cycle");
                break;
            }

            // Wait for next run or shutdown signal
            tracing::info!("Next scraping cycle in {} hours", schedule.interval_hours);

            tokio::select! {
                _ = time::sleep(interval) => {
                    // Continue to next iteration
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Scheduler received shutdown signal, stopping gracefully");
                    break;
                }
            }
        }

        tracing::info!("Scheduler stopped");
        Ok(())
    }
}

// Tests
#[cfg(test)]
mod tests;
