//! Job Board Scrapers
//!
//! Implementations for scraping various job boards.

use async_trait::async_trait;
use jobsentinel_domain::Job;
use jobsentinel_network::FULL_BROWSER_USER_AGENT as BROWSER_USER_AGENT;

const JOBSENTINEL_USER_AGENT: &str = "JobSentinel/1.0";
const COMPANY_SCRAPE_FAILED: &str =
    "Company board scrape failed; continuing with other company boards";
fn decode_common_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn strip_html_markup(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(character),
            _ => {}
        }
    }

    result
}

fn collect_company_scrape_result(
    result: ScraperResult,
    jobs: &mut Vec<Job>,
    failed_companies: &mut usize,
    source: &'static str,
) {
    match result {
        Ok(company_jobs) => jobs.extend(company_jobs),
        Err(_) => {
            *failed_companies += 1;
            tracing::warn!(
                source,
                message = COMPANY_SCRAPE_FAILED,
                "Company board scrape failed"
            );
        }
    }
}

fn require_company_scrape_success(
    company_count: usize,
    failed_companies: usize,
    source: &'static str,
) -> Result<(), ScraperError> {
    if company_count > 0 && failed_companies == company_count {
        return Err(ScraperError::Generic {
            scraper: source.to_string(),
            message: "All configured company boards failed".to_string(),
        });
    }
    Ok(())
}

mod error;
mod greenhouse;
mod hn_hiring;
mod jobswithgpt;
mod lever;
mod linkedin;
mod rate_limiter;
mod remoteok;
mod rss;
#[cfg(test)]
mod source_adapters;
mod usajobs;
mod weworkremotely;

pub use error::ScraperError;
pub use greenhouse::{GreenhouseCompany, GreenhouseScraper};
pub use hn_hiring::HnHiringScraper;
pub use jobswithgpt::{JobQuery, JobsWithGptScraper};
pub use lever::{LeverCompany, LeverScraper};
pub use linkedin::LINKEDIN_AUTOMATION_DISABLED_MESSAGE;
pub use rate_limiter::{limits, RateLimiter};
pub use remoteok::RemoteOkScraper;
pub use usajobs::UsaJobsScraper;
pub use weworkremotely::WeWorkRemotelyScraper;

// NOTE: GovernmentJobs.com and ClearanceJobs.com explicitly prohibit scraping in their ToS.
// Use Deep Link Generator and Bookmarklet features instead; see docs/user/DEEP_LINKS.md and
// docs/BOOKMARKLET.md for supported alternatives.
//
// Y Combinator's Terms of Use also prohibit scraping and similar extraction.
// Keep YC access user-opened or manual.

/// Scraper result type using ScraperError for better error context
///
/// This provides structured errors with helpful context for debugging and user messages.
/// All scrapers now use this typed error approach instead of anyhow::Error.
pub(crate) type ScraperResult = Result<Vec<Job>, ScraperError>;

/// Job scraper trait
#[async_trait]
pub trait JobScraper: Send + Sync {
    /// Scrape jobs from this source
    async fn scrape(&self) -> Result<Vec<Job>, ScraperError>;

    /// Get scraper name
    #[cfg(test)]
    fn name(&self) -> &'static str;
}

#[cfg(test)]
#[path = "tests/integration.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "tests/live.rs"]
mod live_tests;
