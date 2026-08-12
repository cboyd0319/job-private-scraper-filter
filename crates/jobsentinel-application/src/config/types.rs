//! Configuration type definitions

pub(super) mod sources;

use super::ExternalAiConfig;
pub use jobsentinel_notifications::{
    AlertConfig, DesktopConfig, DiscordConfig, EmailConfig, SlackConfig, TeamsConfig,
    TelegramConfig,
};
use serde::{Deserialize, Serialize};
use sources::{
    BuiltInConfig, DiceConfig, GlassdoorConfig, HnHiringConfig, LinkedInConfig, RemoteOkConfig,
    SimplyHiredConfig, UsaJobsConfig, WeWorkRemotelyConfig, YcStartupConfig,
};

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Job titles to match (e.g., "Security Engineer")
    pub title_allowlist: Vec<String>,

    /// Job titles to exclude (e.g., "Manager", "Intern")
    #[serde(default)]
    pub title_blocklist: Vec<String>,

    /// Keywords to boost score
    #[serde(default)]
    pub keywords_boost: Vec<String>,

    /// Keywords to auto-reject
    #[serde(default)]
    pub keywords_exclude: Vec<String>,

    /// Location preferences
    pub location_preferences: LocationPreferences,

    /// Minimum salary in USD (hard floor - jobs below this get low scores)
    pub salary_floor_usd: i64,

    /// Target salary in USD (ideal salary - used for graduated scoring)
    /// If not set, defaults to salary_floor_usd
    #[serde(default)]
    pub salary_target_usd: Option<i64>,

    /// Penalize jobs with missing salary information
    /// If true, jobs without salary get 0.3 score; if false, get 0.5 (neutral)
    #[serde(default)]
    pub penalize_missing_salary: bool,

    /// Auto-refresh configuration
    #[serde(default)]
    pub auto_refresh: AutoRefreshConfig,

    /// Browser Import local helper port.
    #[serde(default = "super::defaults::default_bookmarklet_port")]
    pub bookmarklet_port: u16,

    /// Immediate alert threshold (0.0 - 1.0)
    #[serde(default = "super::defaults::default_immediate_threshold")]
    pub immediate_alert_threshold: f64,

    /// Scraping interval in hours
    #[serde(default = "super::defaults::default_scraping_interval")]
    pub scraping_interval_hours: u64,

    /// Alert configuration
    pub alerts: AlertConfig,

    /// Greenhouse company URLs to scrape (e.g., "https://boards.greenhouse.io/cloudflare")
    #[serde(default)]
    pub greenhouse_urls: Vec<String>,

    /// Lever company URLs to scrape (e.g., "https://jobs.lever.co/netflix")
    #[serde(default)]
    pub lever_urls: Vec<String>,

    /// LinkedIn source configuration
    #[serde(default)]
    pub linkedin: LinkedInConfig,

    /// Legacy restricted-source review flags retained for config compatibility.
    ///
    /// These booleans never authorize a source request and are always cleared.
    #[serde(default)]
    pub restricted_source_acknowledgements: RestrictedSourceAcknowledgements,

    /// RemoteOK scraper configuration
    #[serde(default)]
    pub remoteok: RemoteOkConfig,

    /// WeWorkRemotely scraper configuration
    #[serde(default)]
    pub weworkremotely: WeWorkRemotelyConfig,

    /// Legacy Built In configuration retained for deserialization compatibility.
    #[serde(default)]
    pub builtin: BuiltInConfig,

    /// Hacker News Who's Hiring scraper configuration
    #[serde(default)]
    pub hn_hiring: HnHiringConfig,

    /// Legacy Dice configuration retained for deserialization compatibility.
    #[serde(default)]
    pub dice: DiceConfig,

    /// Y Combinator Work at a Startup scraper configuration
    #[serde(default)]
    pub yc_startup: YcStartupConfig,

    /// USAJobs.gov federal government job scraper configuration
    #[serde(default)]
    pub usajobs: UsaJobsConfig,

    /// Legacy SimplyHired configuration retained for deserialization compatibility.
    #[serde(default)]
    pub simplyhired: SimplyHiredConfig,

    /// Legacy Glassdoor configuration retained for deserialization compatibility.
    #[serde(default)]
    pub glassdoor: GlassdoorConfig,

    /// Optional JobsWithGPT MCP endpoint URL.
    ///
    /// Empty by default. A configured endpoint is not enough to send data:
    /// scheduled source checks also require a matching reviewed payload in
    /// `jobswithgpt_approval`.
    #[serde(default = "super::defaults::default_jobswithgpt_endpoint")]
    pub jobswithgpt_endpoint: String,

    /// Local approval record for the exact JobsWithGPT payload.
    ///
    /// This duplicates the reviewed public-source payload locally so the
    /// scheduler can block silent sends after endpoint, title, or remote-setting
    /// changes.
    #[serde(default)]
    pub jobswithgpt_approval: JobsWithGptApproval,

    /// Optional outside-AI configuration. Provider credentials are stored
    /// through `CredentialService`, not serialized in config.
    #[serde(default)]
    pub external_ai: ExternalAiConfig,

    /// Ghost job detection configuration
    #[serde(default)]
    pub ghost_config: Option<jobsentinel_intelligence::GhostConfig>,

    /// Use resume matching for skills scoring (requires uploaded resume)
    /// When enabled and a resume is available, scores are calculated based on actual resume skills
    /// Falls back to keyword matching when no resume is present
    #[serde(default)]
    pub use_resume_matching: bool,

    /// Preferred companies for scoring bonuses (case-insensitive fuzzy matching).
    /// Companies in this list receive scoring bonus
    #[serde(default, alias = "company_\u{77}hitelist")]
    pub preferred_companies: Vec<String>,

    /// Blocked companies for scoring penalties (case-insensitive fuzzy matching).
    /// Jobs from companies in this list receive very low scores
    #[serde(default, alias = "company_\u{62}lacklist")]
    pub blocked_companies: Vec<String>,
}

pub(super) const JOBSWITHGPT_DEFAULT_LIMIT: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobsWithGptPayload {
    pub endpoint: String,
    pub titles: Vec<String>,
    #[serde(default)]
    pub location: Option<String>,
    pub remote_only: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobsWithGptApproval {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub payload: Option<JobsWithGptPayload>,
    #[serde(default)]
    pub approved_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RestrictedSourceAcknowledgements {
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub dice: bool,
    #[serde(default)]
    pub simplyhired: bool,
    #[serde(default)]
    pub glassdoor: bool,
}

impl Config {
    #[must_use]
    pub fn first_run() -> Self {
        Self {
            title_allowlist: vec![],
            title_blocklist: vec![],
            keywords_boost: vec![],
            keywords_exclude: vec![],
            location_preferences: LocationPreferences {
                allow_remote: true,
                allow_hybrid: false,
                allow_onsite: false,
                cities: vec![],
                states: vec![],
                country: "US".to_string(),
            },
            salary_floor_usd: 0,
            salary_target_usd: None,
            penalize_missing_salary: false,
            auto_refresh: AutoRefreshConfig::default(),
            bookmarklet_port: 4321,
            immediate_alert_threshold: 0.9,
            scraping_interval_hours: 2,
            alerts: AlertConfig::default(),
            greenhouse_urls: vec![],
            lever_urls: vec![],
            linkedin: LinkedInConfig::default(),
            restricted_source_acknowledgements: RestrictedSourceAcknowledgements::default(),
            remoteok: RemoteOkConfig::default(),
            weworkremotely: WeWorkRemotelyConfig::default(),
            builtin: BuiltInConfig::default(),
            hn_hiring: HnHiringConfig::default(),
            dice: DiceConfig::default(),
            yc_startup: YcStartupConfig::default(),
            usajobs: UsaJobsConfig::default(),
            simplyhired: SimplyHiredConfig::default(),
            glassdoor: GlassdoorConfig::default(),
            jobswithgpt_endpoint: String::new(),
            jobswithgpt_approval: JobsWithGptApproval::default(),
            external_ai: ExternalAiConfig::default(),
            preferred_companies: vec![],
            blocked_companies: vec![],
            use_resume_matching: false,
            ghost_config: None,
        }
    }

    pub fn jobswithgpt_payload_preview(&self) -> Option<JobsWithGptPayload> {
        let endpoint = self.jobswithgpt_endpoint.trim().to_string();
        if endpoint.is_empty() {
            return None;
        }

        let titles: Vec<String> = self
            .title_allowlist
            .iter()
            .map(|title| title.trim())
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if titles.is_empty() {
            return None;
        }

        Some(JobsWithGptPayload {
            endpoint,
            titles,
            location: None,
            remote_only: self.location_preferences.allow_remote
                && !self.location_preferences.allow_onsite,
            limit: JOBSWITHGPT_DEFAULT_LIMIT,
        })
    }

    pub fn jobswithgpt_payload_approved(&self) -> bool {
        if !self.jobswithgpt_approval.enabled {
            return false;
        }

        self.jobswithgpt_payload_preview()
            .is_some_and(|payload| self.jobswithgpt_approval.payload.as_ref() == Some(&payload))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationPreferences {
    pub allow_remote: bool,

    #[serde(default)]
    pub allow_hybrid: bool,

    #[serde(default)]
    pub allow_onsite: bool,

    #[serde(default)]
    pub cities: Vec<String>,

    #[serde(default)]
    pub states: Vec<String>,

    #[serde(default = "super::defaults::default_country")]
    pub country: String,
}

/// Auto-refresh configuration for the frontend
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoRefreshConfig {
    /// Enable automatic job refresh
    #[serde(default)]
    pub enabled: bool,

    /// Refresh interval in minutes (default: 30)
    #[serde(default = "super::defaults::default_auto_refresh_interval")]
    pub interval_minutes: u32,
}

#[cfg(test)]
mod tests;
