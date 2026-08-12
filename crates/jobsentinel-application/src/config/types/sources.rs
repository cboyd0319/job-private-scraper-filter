use jobsentinel_security::redacted_secret_for_debug;
use serde::{Deserialize, Serialize};
use std::fmt;

/// LinkedIn search-link configuration.
///
/// Hidden background monitoring is not run. LinkedIn can still be opened
/// through user-controlled job-site search links, manual entry, and Browser
/// Import for individual pages the user chooses.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LinkedInConfig {
    /// Preserved for imported or legacy settings. The scheduler treats true as
    /// an advisory request and emits a warning instead of running hidden checks.
    #[serde(default)]
    pub enabled: bool,

    /// Search query (job title, keywords)
    /// Example: "registered nurse", "program coordinator"
    #[serde(default)]
    pub query: String,

    /// Location filter
    /// Example: "San Francisco Bay Area", "Remote", "United States"
    #[serde(default)]
    pub location: String,

    /// Search only for remote jobs
    #[serde(default)]
    pub remote_only: bool,

    /// Maximum results to return (default: 50, max: 100)
    #[serde(default = "super::super::defaults::default_linkedin_limit")]
    pub limit: usize,
}

impl fmt::Debug for LinkedInConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LinkedInConfig")
            .field("enabled", &self.enabled)
            .field("query", &self.query)
            .field("location", &self.location)
            .field("remote_only", &self.remote_only)
            .field("limit", &self.limit)
            .finish()
    }
}

/// RemoteOK scraper configuration
///
/// RemoteOK provides a public JSON API for remote job listings.
/// No authentication required.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteOkConfig {
    /// Enable RemoteOK job scraping
    #[serde(default)]
    pub enabled: bool,

    /// Search tags to filter jobs (e.g., ["rust", "python", "engineer"])
    #[serde(default)]
    pub tags: Vec<String>,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// WeWorkRemotely scraper configuration
///
/// WeWorkRemotely provides an RSS feed for remote-only jobs.
/// No authentication required.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeWorkRemotelyConfig {
    /// Enable WeWorkRemotely job scraping
    #[serde(default)]
    pub enabled: bool,

    /// Category to search (e.g., "remote-programming-jobs", "remote-design-jobs")
    #[serde(default)]
    pub category: Option<String>,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Legacy Built In configuration.
///
/// Retained for config compatibility. Automated access is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuiltInConfig {
    /// Legacy automation flag; retained but ignored.
    #[serde(default)]
    pub enabled: bool,

    /// Only fetch remote jobs (uses /jobs/remote endpoint)
    #[serde(default)]
    pub remote_only: bool,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Hacker News Who's Hiring scraper configuration
///
/// Scrapes jobs from the monthly "Who is hiring?" threads.
/// No authentication required.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HnHiringConfig {
    /// Enable HN Who's Hiring job scraping
    #[serde(default)]
    pub enabled: bool,

    /// Filter for remote jobs only
    #[serde(default)]
    pub remote_only: bool,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Legacy Dice configuration.
///
/// Retained for config compatibility. The legacy HTML adapter is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiceConfig {
    /// Legacy automation flag; retained but ignored.
    #[serde(default)]
    pub enabled: bool,

    /// Search query for this tech-focused source (e.g., "IT support", "security analyst")
    #[serde(default)]
    pub query: String,

    /// Location filter (e.g., "Remote", "New York, NY")
    #[serde(default)]
    pub location: Option<String>,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Legacy Y Combinator Work at a Startup configuration.
///
/// Retained for config compatibility. Automated access is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YcStartupConfig {
    /// Legacy automation flag; retained but ignored.
    #[serde(default)]
    pub enabled: bool,

    /// Optional keyword filter
    #[serde(default)]
    pub query: Option<String>,

    /// Filter for remote jobs only
    #[serde(default)]
    pub remote_only: bool,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Legacy SimplyHired configuration.
///
/// Retained for config compatibility. Automated access is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimplyHiredConfig {
    /// Legacy automation flag; retained but ignored.
    #[serde(default)]
    pub enabled: bool,

    /// Search query (e.g., "marketing manager", "care coordinator")
    #[serde(default)]
    pub query: String,

    /// Location filter (e.g., "Remote", "San Francisco, CA")
    #[serde(default)]
    pub location: Option<String>,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// Legacy Glassdoor configuration.
///
/// Retained for config compatibility. Automated access is unavailable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlassdoorConfig {
    /// Legacy automation flag; retained but ignored.
    #[serde(default)]
    pub enabled: bool,

    /// Search query (e.g., "office manager", "data analyst")
    #[serde(default)]
    pub query: String,

    /// Location filter (e.g., "San Francisco, CA", "Remote")
    #[serde(default)]
    pub location: Option<String>,

    /// Maximum results to return (default: 50)
    #[serde(default = "super::super::defaults::default_scraper_limit")]
    pub limit: usize,
}

/// USAJobs.gov scraper configuration
///
/// Official federal government job API. Requires a free API key from:
/// https://developer.usajobs.gov/
///
/// # Security
/// The API key is stored through `CredentialService`, not config.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UsaJobsConfig {
    /// Enable USAJobs scraping
    #[serde(default)]
    pub enabled: bool,

    /// API key from developer.usajobs.gov - stored through `CredentialService`.
    #[serde(skip)]
    pub api_key: String,

    /// Email used for User-Agent header (required by API)
    #[serde(default)]
    pub email: String,

    /// Search keywords (e.g., "program analyst", "data analyst")
    #[serde(default)]
    pub keywords: Option<String>,

    /// Location filter (city, state, or zip)
    #[serde(default)]
    pub location: Option<String>,

    /// Search radius in miles from location
    #[serde(default)]
    pub radius: Option<u32>,

    /// Only show remote positions
    #[serde(default)]
    pub remote_only: bool,

    /// Minimum GS pay grade (1-15)
    #[serde(default)]
    pub pay_grade_min: Option<u8>,

    /// Maximum GS pay grade (1-15)
    #[serde(default)]
    pub pay_grade_max: Option<u8>,

    /// Only show jobs posted within N days (1-60, default: 30)
    #[serde(default = "super::super::defaults::default_usajobs_date_posted")]
    pub date_posted_days: u8,

    /// Maximum results to return (default: 100)
    #[serde(default = "super::super::defaults::default_usajobs_limit")]
    pub limit: usize,
}

impl fmt::Debug for UsaJobsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UsaJobsConfig")
            .field("enabled", &self.enabled)
            .field("api_key", &redacted_secret_for_debug(&self.api_key))
            .field("email", &self.email)
            .field("keywords", &self.keywords)
            .field("location", &self.location)
            .field("radius", &self.radius)
            .field("remote_only", &self.remote_only)
            .field("pay_grade_min", &self.pay_grade_min)
            .field("pay_grade_max", &self.pay_grade_max)
            .field("date_posted_days", &self.date_posted_days)
            .field("limit", &self.limit)
            .finish()
    }
}
