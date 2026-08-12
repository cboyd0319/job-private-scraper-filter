//! Fetches and normalizes public job listings from configured Greenhouse boards.

use super::error::ScraperError;
use super::rate_limiter::RateLimiter;
#[cfg(test)]
use super::COMPANY_SCRAPE_FAILED;
use super::{
    collect_company_scrape_result, require_company_scrape_success, JobScraper, ScraperResult,
    JOBSENTINEL_USER_AGENT,
};
use crate::{is_safe_company_board_id, parse_greenhouse_company_url};
use async_trait::async_trait;
use chrono::Utc;
use jobsentinel_domain::{
    v3_source_manifest::{GREENHOUSE_API_ENDPOINT_PREFIX, GREENHOUSE_REQUEST_LIMIT_PER_HOUR},
    Job,
};
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use jobsentinel_security::sanitize_url_for_logging;
use std::num::NonZeroU16;

/// Greenhouse scraper configuration
#[derive(Debug, Clone)]
pub struct GreenhouseScraper {
    /// List of Greenhouse company URLs to scrape
    pub companies: Vec<GreenhouseCompany>,
    /// Rate limiter for respecting Greenhouse's request limits
    pub rate_limiter: RateLimiter,
    /// JobSentinel policy rate for this source
    pub request_limit_per_hour: u32,
}

#[derive(Debug, Clone)]
pub struct GreenhouseCompany {
    pub id: String,
    pub name: String,
    pub url: String,
}

impl GreenhouseScraper {
    pub fn new(companies: Vec<GreenhouseCompany>) -> Self {
        Self {
            companies,
            rate_limiter: RateLimiter::shared(),
            request_limit_per_hour: u32::from(GREENHOUSE_REQUEST_LIMIT_PER_HOUR),
        }
    }

    #[must_use]
    pub fn with_request_limit_per_hour(mut self, request_limit_per_hour: NonZeroU16) -> Self {
        self.request_limit_per_hour = u32::from(request_limit_per_hour.get());
        self
    }

    /// Scrape a single Greenhouse company
    #[tracing::instrument(skip(self), fields(company_name = %company.name))]
    async fn scrape_company(&self, company: &GreenhouseCompany) -> ScraperResult {
        tracing::info!("Starting Greenhouse scrape");
        let board = parse_greenhouse_company_url(&company.url).map_err(|reason| {
            ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason,
            }
        })?;

        if board.id != company.id {
            return Err(ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason: "Greenhouse company id does not match configured URL".to_string(),
            });
        }

        let jobs = self.scrape_greenhouse_api(company).await?;
        tracing::info!("Found {} jobs from {}", jobs.len(), company.name);
        Ok(jobs)
    }

    /// Scrape Greenhouse API (JSON format)
    async fn scrape_greenhouse_api(&self, company: &GreenhouseCompany) -> ScraperResult {
        if !is_safe_company_board_id(&company.id) {
            return Err(ScraperError::InvalidUrl {
                url: company.url.clone(),
                reason: "Greenhouse company id contains unsupported characters".to_string(),
            });
        }

        let company_id = company.id.as_str();

        let api_url = format!("{GREENHOUSE_API_ENDPOINT_PREFIX}{company_id}/jobs");

        tracing::debug!(url = %sanitize_url_for_logging(&api_url), "Fetching Greenhouse API");

        let response = send_external_http_text_with_retry(
            ExternalHttpRequest::get(&api_url)
                .without_retries()
                .user_agent(JOBSENTINEL_USER_AGENT),
        )
        .await
        .map_err(|error| ScraperError::from_external("greenhouse", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                &api_url,
                format!("Greenhouse API failed: {}", response.status),
            ));
        }

        let json: serde_json::Value = ScraperError::parse_json(&api_url, &response.body)?;

        Self::parse_api_jobs(&json, company, &api_url)
    }

    fn parse_api_jobs(
        json: &serde_json::Value,
        company: &GreenhouseCompany,
        url: &str,
    ) -> Result<Vec<Job>, ScraperError> {
        let jobs = json["jobs"]
            .as_array()
            .ok_or_else(|| ScraperError::ParseError {
                format: "JSON".to_string(),
                url: url.to_string(),
                source: Box::new(std::io::Error::other(
                    "Greenhouse jobs field is not an array",
                )),
            })?;
        Ok(jobs
            .iter()
            .filter_map(|job| {
                let title = job["title"].as_str()?.trim();
                let id = job["id"].as_i64().filter(|id| *id > 0)?;
                if title.is_empty() {
                    return None;
                }
                Some(Job::newly_discovered(
                    title,
                    company.name.clone(),
                    Self::api_job_url(&company.id, id, job),
                    job["location"]["name"].as_str().map(str::to_string),
                    "greenhouse",
                    Utc::now(),
                ))
            })
            .collect())
    }

    fn api_job_url(company_id: &str, job_id: i64, _job_data: &serde_json::Value) -> String {
        format!("https://job-boards.greenhouse.io/{company_id}/jobs/{job_id}")
    }
}

#[async_trait]
impl JobScraper for GreenhouseScraper {
    async fn scrape(&self) -> ScraperResult {
        let mut all_jobs = Vec::new();
        let mut failed_companies = 0usize;

        for company in &self.companies {
            self.rate_limiter
                .wait_paced("greenhouse", self.request_limit_per_hour)
                .await;

            collect_company_scrape_result(
                self.scrape_company(company).await,
                &mut all_jobs,
                &mut failed_companies,
                "greenhouse",
            );
        }

        require_company_scrape_success(self.companies.len(), failed_companies, "greenhouse")?;

        Ok(all_jobs)
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "greenhouse"
    }
}

#[cfg(test)]
#[path = "greenhouse_tests.rs"]
mod tests;
