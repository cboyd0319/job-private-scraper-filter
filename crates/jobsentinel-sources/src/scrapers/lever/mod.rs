//! Lever ATS Scraper
//!
//! Scrapes jobs from Lever-powered career pages.
//! Lever is used by companies like Netflix, Shopify, IDEO, etc.

use super::error::ScraperError;
use super::rate_limiter::RateLimiter;
#[cfg(test)]
use super::COMPANY_SCRAPE_FAILED;
use super::{
    collect_company_scrape_result, require_company_scrape_success, JobScraper, ScraperResult,
    JOBSENTINEL_USER_AGENT,
};
use crate::{is_safe_company_board_id, parse_lever_company_url};
use async_trait::async_trait;
use chrono::Utc;
use jobsentinel_domain::normalization::infer_remote_status;
use jobsentinel_domain::{
    canonicalize_job_url,
    v3_source_manifest::{LEVER_API_ENDPOINT_PREFIX, LEVER_REQUEST_LIMIT_PER_HOUR},
    Job,
};
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use jobsentinel_security::sanitize_url_for_logging;
use std::num::NonZeroU16;

/// Lever scraper configuration
#[derive(Debug, Clone)]
pub struct LeverScraper {
    /// List of Lever company URLs to scrape
    pub companies: Vec<LeverCompany>,
    /// Rate limiter for respecting Lever's request limits
    pub rate_limiter: RateLimiter,
    /// JobSentinel policy rate for this source
    pub request_limit_per_hour: u32,
}

#[derive(Debug, Clone)]
pub struct LeverCompany {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl LeverScraper {
    pub fn new(companies: Vec<LeverCompany>) -> Self {
        Self {
            companies,
            rate_limiter: RateLimiter::shared(),
            request_limit_per_hour: u32::from(LEVER_REQUEST_LIMIT_PER_HOUR),
        }
    }

    #[must_use]
    pub fn with_request_limit_per_hour(mut self, request_limit_per_hour: NonZeroU16) -> Self {
        self.request_limit_per_hour = u32::from(request_limit_per_hour.get());
        self
    }

    /// Scrape a single Lever company via API
    async fn scrape_company(&self, company: &LeverCompany) -> ScraperResult {
        tracing::info!("Scraping Lever: {}", company.name);
        if !is_safe_company_board_id(&company.id) {
            return Err(ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason: "Lever company id contains unsupported characters".to_string(),
            });
        }
        let board =
            parse_lever_company_url(&company.url).map_err(|reason| ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason,
            })?;
        if board.id != company.id {
            return Err(ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason: "Lever company id does not match configured URL".to_string(),
            });
        }

        let api_url = format!("{LEVER_API_ENDPOINT_PREFIX}{}", company.id);

        tracing::debug!(url = %sanitize_url_for_logging(&api_url), "Fetching Lever API");

        let response = send_external_http_text_with_retry(
            ExternalHttpRequest::get(&api_url)
                .without_retries()
                .user_agent(JOBSENTINEL_USER_AGENT),
        )
        .await
        .map_err(|error| ScraperError::from_external("lever", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                &api_url,
                format!("Lever API failed: {}", response.status),
            ));
        }

        let json: serde_json::Value = ScraperError::parse_json(&api_url, &response.body)?;

        let jobs = Self::parse_api_response(&json, company, &api_url)?;

        tracing::info!("Found {} jobs from {}", jobs.len(), company.name);
        Ok(jobs)
    }

    fn parse_api_response(
        json: &serde_json::Value,
        company: &LeverCompany,
        url: &str,
    ) -> Result<Vec<Job>, ScraperError> {
        if !json.is_array() {
            return Err(ScraperError::ParseError {
                format: "JSON".to_string(),
                url: url.to_string(),
                source: Box::new(std::io::Error::other("Lever response root is not an array")),
            });
        }
        Ok(Self::parse_postings(json, company))
    }

    /// Infer if job is remote from title or location
    fn infer_remote(title: &str, location: Option<&str>) -> bool {
        infer_remote_status(&[title, location.unwrap_or("")]).is_remote()
    }

    fn parse_postings(json: &serde_json::Value, company: &LeverCompany) -> Vec<Job> {
        let Some(postings) = json.as_array() else {
            return Vec::new();
        };

        postings
            .iter()
            .filter_map(|posting| {
                let title = posting["text"].as_str().unwrap_or("").to_string();
                let url = canonicalize_job_url(posting["hostedUrl"].as_str()?).ok()?;
                if title.is_empty() {
                    return None;
                }

                let location = Self::posting_location(posting);
                Some(Job {
                    description: Self::posting_description(posting),
                    remote: Some(Self::infer_remote(&title, location.as_deref())),
                    ..Job::newly_discovered(
                        title,
                        company.name.clone(),
                        url,
                        location,
                        "lever",
                        Utc::now(),
                    )
                })
            })
            .collect()
    }

    fn posting_location(posting: &serde_json::Value) -> Option<String> {
        posting["categories"]["location"]
            .as_str()
            .or_else(|| posting["categories"]["team"].as_str())
            .map(str::to_string)
    }

    fn posting_description(posting: &serde_json::Value) -> Option<String> {
        posting["description"]
            .as_str()
            .or_else(|| posting["descriptionPlain"].as_str())
            .map(str::to_string)
    }
}

#[async_trait]
impl JobScraper for LeverScraper {
    async fn scrape(&self) -> ScraperResult {
        let mut all_jobs = Vec::new();
        let mut failed_companies = 0usize;

        for company in &self.companies {
            self.rate_limiter
                .wait_paced("lever", self.request_limit_per_hour)
                .await;

            collect_company_scrape_result(
                self.scrape_company(company).await,
                &mut all_jobs,
                &mut failed_companies,
                "lever",
            );
        }

        require_company_scrape_success(self.companies.len(), failed_companies, "lever")?;

        Ok(all_jobs)
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "lever"
    }
}

#[cfg(test)]
mod tests;
