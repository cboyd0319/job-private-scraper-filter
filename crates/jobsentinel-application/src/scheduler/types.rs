//! Type definitions for the scheduler module

use crate::config::Config;
use crate::credentials::CredentialService;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::{broadcast, Mutex, RwLock};

/// Schedule configuration
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    /// Interval between scraping runs (in hours)
    pub interval_hours: u64,

    /// Whether auto-scheduling is enabled
    pub enabled: bool,
}

impl From<&Config> for ScheduleConfig {
    fn from(config: &Config) -> Self {
        ScheduleConfig {
            interval_hours: config.scraping_interval_hours,
            enabled: config.auto_refresh.enabled,
        }
    }
}

/// Scheduler handle
#[derive(Debug)]
pub struct Scheduler {
    pub(crate) config: Arc<RwLock<Config>>,
    pub(crate) database: Arc<jobsentinel_storage::Database>,
    pub(crate) credentials: Arc<CredentialService>,
    pub(crate) shutdown_tx: broadcast::Sender<()>,
    pub(crate) shutdown_requested: AtomicBool,
    pub(crate) scrape_lock: Arc<Mutex<()>>,
}

impl Scheduler {
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Acquire)
    }
}

/// Scraping result statistics
#[derive(Debug, Clone, Default)]
pub struct ScrapingResult {
    pub jobs_found: usize,
    pub jobs_new: usize,
    pub jobs_updated: usize,
    pub high_matches: usize,
    pub alerts_sent: usize,
    pub errors: Vec<String>,
}
