//! RemoteOK Job Scraper
//!
//! Scrapes remote jobs from RemoteOK's public JSON API.
//! RemoteOK is a popular remote job board with tech-focused listings.

use super::error::ScraperError;
use super::rate_limiter::RateLimiter;
use super::{JobScraper, ScraperResult, JOBSENTINEL_USER_AGENT};
use async_trait::async_trait;
use chrono::Utc;
use jobsentinel_domain::{v3_source_manifest::REMOTEOK_REQUEST_LIMIT_PER_HOUR, Job};
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use std::num::NonZeroU16;

/// RemoteOK job scraper
#[derive(Debug, Clone)]
pub struct RemoteOkScraper {
    /// Search tags to filter jobs (e.g., "rust", "python", "engineer")
    pub tags: Vec<String>,
    /// Maximum results to return
    pub limit: usize,
    /// Rate limiter for respecting RemoteOK's request limits
    pub rate_limiter: RateLimiter,
    /// JobSentinel policy rate for this source
    pub request_limit_per_hour: u32,
}

impl RemoteOkScraper {
    pub fn new(tags: Vec<String>, limit: usize) -> Self {
        Self {
            tags,
            limit,
            rate_limiter: RateLimiter::shared(),
            request_limit_per_hour: u32::from(REMOTEOK_REQUEST_LIMIT_PER_HOUR),
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

    /// Fetch jobs from RemoteOK API
    async fn fetch_jobs(&self) -> ScraperResult {
        tracing::info!("Fetching jobs from RemoteOK");

        self.rate_limiter
            .wait_paced("remoteok", self.request_limit_per_hour)
            .await;

        let url = "https://remoteok.com/api";

        let response = send_external_http_text_with_retry(Self::request(url))
            .await
            .map_err(|error| ScraperError::from_external("remoteok", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                url,
                format!("RemoteOK API failed: {}", response.status),
            ));
        }

        let json: serde_json::Value = ScraperError::parse_json(url, &response.body)?;

        // RemoteOK returns an array where first element is a legal notice
        let jobs_array = json.as_array().ok_or_else(|| {
            ScraperError::parse(
                "JSON",
                url,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Expected array from RemoteOK API",
                ),
            )
        })?;

        let mut jobs = Vec::new();
        let tags_lower: Vec<String> = self.tags.iter().map(|t| t.to_lowercase()).collect();

        for job_data in jobs_array.iter().skip(1) {
            // Skip the first element (legal notice)
            if let Some(job) = self.parse_job(job_data)? {
                // Filter by tags if specified
                if tags_lower.is_empty() || self.job_matches_tags(&job, &tags_lower) {
                    jobs.push(job);
                }

                if jobs.len() >= self.limit {
                    break;
                }
            }
        }

        tracing::info!("Found {} jobs from RemoteOK", jobs.len());
        Ok(jobs)
    }

    /// Check if job matches any of the filter tags
    fn job_matches_tags(&self, job: &Job, tags: &[String]) -> bool {
        let title_lower = job.title.to_lowercase();
        let description_lower = job
            .description
            .as_ref()
            .map(|d| d.to_lowercase())
            .unwrap_or_default();

        tags.iter()
            .any(|tag| title_lower.contains(tag) || description_lower.contains(tag))
    }

    /// Parse a job from RemoteOK JSON response
    fn parse_job(&self, data: &serde_json::Value) -> Result<Option<Job>, ScraperError> {
        // Skip if no position field (might be a non-job entry)
        let title = match data["position"].as_str() {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => return Ok(None),
        };

        let company = data["company"].as_str().unwrap_or("Unknown").to_string();
        let url = data["url"]
            .as_str()
            .map(|u| {
                if u.starts_with("http") {
                    u.to_string()
                } else {
                    format!("https://remoteok.com{}", u)
                }
            })
            .unwrap_or_default();

        if url.is_empty() {
            return Ok(None);
        }

        let location = data["location"].as_str().map(|s| s.to_string());
        let description = data["description"].as_str().map(|s| s.to_string());

        // RemoteOK has salary_min and salary_max
        let salary_min = data["salary_min"].as_i64();
        let salary_max = data["salary_max"].as_i64();

        Ok(Some(Job {
            description,
            remote: Some(true), // All RemoteOK jobs are remote
            salary_min,
            salary_max,
            currency: Some("USD".to_string()), // RemoteOK typically uses USD
            ..Job::newly_discovered(title, company, url, location, "remoteok", Utc::now())
        }))
    }
}

#[async_trait]
impl JobScraper for RemoteOkScraper {
    async fn scrape(&self) -> ScraperResult {
        self.fetch_jobs().await
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "remoteok"
    }
}

#[cfg(test)]
mod tests;
