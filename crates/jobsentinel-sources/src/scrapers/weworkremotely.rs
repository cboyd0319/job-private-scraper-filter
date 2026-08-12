//! WeWorkRemotely Job Scraper
//!
//! Scrapes remote jobs from WeWorkRemotely's RSS feed.
//! WeWorkRemotely is a popular remote-only job board.

use super::error::ScraperError;
use super::rate_limiter::RateLimiter;
#[cfg(test)]
use super::rss::extract_xml_tag;
use super::rss::parse_rss_items;
use super::{
    decode_common_html_entities, strip_html_markup, JobScraper, ScraperResult,
    JOBSENTINEL_USER_AGENT,
};
use jobsentinel_domain::{v3_source_manifest::WEWORKREMOTELY_REQUEST_LIMIT_PER_HOUR, Job};
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use std::num::NonZeroU16;

use async_trait::async_trait;
use chrono::Utc;
use url::Url;

/// WeWorkRemotely job scraper
#[derive(Debug, Clone)]
pub struct WeWorkRemotelyScraper {
    /// Category to search (e.g., "programming", "design", "devops")
    pub category: Option<String>,
    /// Maximum results to return
    pub limit: usize,
    /// Rate limiter for respecting WeWorkRemotely's request limits
    pub rate_limiter: RateLimiter,
    /// JobSentinel policy rate for this source
    pub request_limit_per_hour: u32,
}

impl WeWorkRemotelyScraper {
    pub fn new(category: Option<String>, limit: usize) -> Self {
        Self {
            category,
            limit,
            rate_limiter: RateLimiter::shared(),
            request_limit_per_hour: u32::from(WEWORKREMOTELY_REQUEST_LIMIT_PER_HOUR),
        }
    }

    #[must_use]
    pub fn with_request_limit_per_hour(mut self, request_limit_per_hour: NonZeroU16) -> Self {
        self.request_limit_per_hour = u32::from(request_limit_per_hour.get());
        self
    }

    fn request(url: &str) -> ExternalHttpRequest {
        ExternalHttpRequest::get(url)
            .without_retries()
            .user_agent(JOBSENTINEL_USER_AGENT)
    }

    fn is_canonical_listing_url(value: &str) -> bool {
        let Ok(url) = Url::parse(value) else {
            return false;
        };
        url.scheme() == "https"
            && url.host_str() == Some("weworkremotely.com")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && (url.path().starts_with("/remote-jobs/") || url.path().starts_with("/jobs/"))
    }

    /// Build the RSS feed URL
    fn build_url(&self) -> Result<&'static str, ScraperError> {
        match self.category.as_deref() {
            None => Ok("https://weworkremotely.com/remote-jobs.rss"),
            Some("remote-customer-support-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-customer-support-jobs.rss")
            }
            Some("remote-product-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-product-jobs.rss")
            }
            Some("remote-full-stack-programming-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-full-stack-programming-jobs.rss")
            }
            Some("remote-back-end-programming-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-back-end-programming-jobs.rss")
            }
            Some("remote-front-end-programming-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-front-end-programming-jobs.rss")
            }
            Some("programming" | "remote-programming-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-programming-jobs.rss")
            }
            Some("remote-sales-and-marketing-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-sales-and-marketing-jobs.rss")
            }
            Some("remote-management-and-finance-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-management-and-finance-jobs.rss")
            }
            Some("design" | "remote-design-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-design-jobs.rss")
            }
            Some("devops" | "remote-devops-sysadmin-jobs") => {
                Ok("https://weworkremotely.com/categories/remote-devops-sysadmin-jobs.rss")
            }
            Some("all-other-remote-jobs") => {
                Ok("https://weworkremotely.com/categories/all-other-remote-jobs.rss")
            }
            Some(_) => Err(ScraperError::InvalidConfiguration {
                scraper: "weworkremotely".to_string(),
                message: "category is not a reviewed public RSS feed".to_string(),
            }),
        }
    }

    /// Fetch jobs from WeWorkRemotely RSS feed
    async fn fetch_jobs(&self) -> ScraperResult {
        tracing::info!("Fetching jobs from WeWorkRemotely");

        self.rate_limiter
            .wait_paced("weworkremotely", self.request_limit_per_hour)
            .await;

        let url = self.build_url()?;

        let response = send_external_http_text_with_retry(Self::request(url))
            .await
            .map_err(|error| ScraperError::from_external("weworkremotely", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                url,
                format!("WeWorkRemotely request failed: {}", response.status),
            ));
        }

        let xml = response.body;
        let jobs = self.parse_rss(&xml)?;

        tracing::info!("Found {} jobs from WeWorkRemotely", jobs.len());
        Ok(jobs)
    }

    /// Parse RSS XML to extract job listings
    fn parse_rss(&self, xml: &str) -> Result<Vec<Job>, ScraperError> {
        let mut jobs = Vec::with_capacity(self.limit);
        let url = self.build_url()?.to_string();

        let items =
            parse_rss_items(xml, self.limit).map_err(|source| ScraperError::ParseError {
                format: "RSS".to_string(),
                url,
                source,
            })?;

        for item in items {
            let title = item
                .get("title")
                .map(Self::decode_html_entities)
                .unwrap_or_default();

            let url = item
                .get("link")
                .map(str::trim)
                .filter(|value| Self::is_canonical_listing_url(value))
                .unwrap_or_default()
                .to_string();

            // WeWorkRemotely titles often include company: "Company: Job Title"
            let (company, job_title) = if let Some(pos) = title.find(':') {
                let (c, t) = title.split_at(pos);
                (c.trim().to_string(), t[1..].trim().to_string())
            } else {
                ("Unknown Company".to_string(), title)
            };

            if job_title.is_empty() || url.is_empty() {
                continue;
            }

            // Extract description and clean HTML
            let description = item
                .get("description")
                .map(Self::decode_html_entities)
                .map(|d| Self::strip_html_tags(&d));

            // Try to extract location from description
            let location = description.as_ref().and_then(|d| Self::extract_location(d));

            jobs.push(Job {
                description,
                remote: Some(true), // All WeWorkRemotely jobs are remote
                ..Job::newly_discovered(
                    job_title,
                    company,
                    url,
                    location,
                    "weworkremotely",
                    Utc::now(),
                )
            });
        }

        Ok(jobs)
    }

    /// Extract content between XML tags
    #[cfg(test)]
    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        extract_xml_tag(xml, tag)
    }

    /// Decode HTML entities
    fn decode_html_entities(text: &str) -> String {
        decode_common_html_entities(text)
    }

    /// Strip HTML tags from text
    fn strip_html_tags(html: &str) -> String {
        strip_html_markup(html)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Try to extract location from description
    fn extract_location(description: &str) -> Option<String> {
        let desc_lower = description.to_lowercase();

        // Common location patterns in WWR descriptions
        if desc_lower.contains("worldwide") || desc_lower.contains("anywhere") {
            return Some("Worldwide".to_string());
        }
        if desc_lower.contains("usa only") || desc_lower.contains("us only") {
            return Some("USA".to_string());
        }
        if desc_lower.contains("europe") || desc_lower.contains("eu only") {
            return Some("Europe".to_string());
        }
        if desc_lower.contains("north america") {
            return Some("North America".to_string());
        }

        None
    }
}

#[async_trait]
impl JobScraper for WeWorkRemotelyScraper {
    async fn scrape(&self) -> ScraperResult {
        self.fetch_jobs().await
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "weworkremotely"
    }
}

#[cfg(test)]
#[path = "weworkremotely_tests.rs"]
mod tests;
