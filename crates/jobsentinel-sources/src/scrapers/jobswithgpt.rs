//! JobsWithGPT MCP Client
//!
//! Integrates with JobsWithGPT via Model Context Protocol for 500K+ job listings.
//! MCP is a JSON-RPC based protocol for querying structured data.

use super::error::ScraperError;
use super::rate_limiter::{limits, RateLimiter};
use super::{JobScraper, ScraperResult, BROWSER_USER_AGENT};
use jobsentinel_domain::{canonicalize_job_url, Job};

use async_trait::async_trait;
use chrono::Utc;
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use jobsentinel_security::sanitize_url_for_logging;
use std::fmt;

/// JobsWithGPT MCP scraper
#[derive(Clone)]
pub struct JobsWithGptScraper {
    /// MCP server endpoint
    pub endpoint: String,
    /// Search query parameters
    pub query: JobQuery,
    /// Rate limiter for respecting MCP server limits
    pub rate_limiter: RateLimiter,
}

#[derive(Clone, Default)]
pub struct JobQuery {
    /// Job titles to search for
    pub titles: Vec<String>,
    /// Location filter (optional)
    pub location: Option<String>,
    /// Remote filter
    pub remote_only: bool,
    /// Maximum results to return
    pub limit: usize,
}

impl fmt::Debug for JobsWithGptScraper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobsWithGptScraper")
            .field("endpoint_configured", &!self.endpoint.is_empty())
            .field("query", &self.query)
            .field("rate_limiter", &self.rate_limiter)
            .finish()
    }
}

impl fmt::Debug for JobQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobQuery")
            .field("title_count", &self.titles.len())
            .field("has_location", &self.location.is_some())
            .field("remote_only", &self.remote_only)
            .field("limit", &self.limit)
            .finish()
    }
}

impl JobsWithGptScraper {
    pub fn new(endpoint: impl Into<String>, query: JobQuery) -> Self {
        Self {
            endpoint: endpoint.into(),
            query,
            rate_limiter: RateLimiter::shared(),
        }
    }

    /// Query JobsWithGPT MCP server
    async fn query_mcp(&self) -> ScraperResult {
        jobsentinel_security::validate_external_https_url(&self.endpoint).map_err(|reason| {
            ScraperError::InvalidUrl {
                url: self.endpoint.clone(),
                reason,
            }
        })?;
        let endpoint_url = self.endpoint.clone();

        tracing::info!("Querying JobsWithGPT MCP server");

        // Use rate limiter (MCP server, high limit)
        self.rate_limiter
            .wait("jobswithgpt", limits::JOBSWITHGPT)
            .await;

        // Build MCP JSON-RPC request
        // MCP format: { "jsonrpc": "2.0", "method": "search", "params": {...}, "id": 1 }
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "jobs/search",
            "params": {
                "titles": self.query.titles,
                "location": self.query.location,
                "remote_only": self.query.remote_only,
                "limit": self.query.limit,
            },
            "id": 1
        });

        tracing::debug!(
            endpoint = %sanitize_url_for_logging(&self.endpoint),
            title_count = self.query.titles.len(),
            has_location = self.query.location.is_some(),
            remote_only = self.query.remote_only,
            limit = self.query.limit,
            "Sending JobsWithGPT MCP request"
        );

        let response =
            send_external_http_text_with_retry(jobswithgpt_request(&endpoint_url, request))
                .await
                .map_err(|error| ScraperError::from_external("jobswithgpt", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                &endpoint_url,
                format!("MCP server failed: {}", response.status),
            ));
        }

        let json: serde_json::Value = ScraperError::parse_json(&endpoint_url, &response.body)?;

        // Parse MCP response: { "jsonrpc": "2.0", "result": [...], "id": 1 }
        if let Some(error) = json.get("error") {
            return Err(ScraperError::Generic {
                scraper: "jobswithgpt".to_string(),
                message: Self::mcp_error_message(error),
            });
        }

        let mut jobs = Vec::new();

        if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
            for job_data in result {
                if let Some(job) = self.parse_mcp_job(job_data)? {
                    jobs.push(job);
                }
            }
        }

        tracing::info!("Found {} jobs from JobsWithGPT", jobs.len());
        Ok(jobs)
    }

    fn mcp_error_message(error: &serde_json::Value) -> String {
        let code = error
            .get("code")
            .and_then(serde_json::Value::as_i64)
            .map_or_else(|| "unknown".to_string(), |value| value.to_string());
        let has_message = error.get("message").is_some();
        let data_type = error
            .get("data")
            .map(Self::json_value_kind)
            .unwrap_or("none");

        format!(
            "MCP error response (code: {code}, has_message: {has_message}, data_type: {data_type})"
        )
    }

    fn json_value_kind(value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "bool",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }

    /// Parse a job from MCP response
    fn parse_mcp_job(&self, data: &serde_json::Value) -> Result<Option<Job>, ScraperError> {
        let title = data["title"].as_str().unwrap_or("").to_string();
        let company = data["company"].as_str().unwrap_or("Unknown").to_string();
        let raw_url = data["url"].as_str().unwrap_or("").trim();
        let location = data["location"].as_str().map(|s| s.to_string());
        let description = data["description"].as_str().map(|s| s.to_string());
        let remote = data["remote"].as_bool();

        // Parse salary if available
        let salary_min = data["salary_min"].as_i64();
        let salary_max = data["salary_max"].as_i64();
        let currency = data["currency"].as_str().map(|s| s.to_string());

        if title.is_empty() || raw_url.is_empty() {
            return Ok(None);
        }

        let url = match canonicalize_job_url(raw_url) {
            Ok(url) => url,
            Err(_) => {
                tracing::warn!(
                    source = "jobswithgpt",
                    "Dropped JobsWithGPT job with invalid public URL"
                );
                return Ok(None);
            }
        };

        Ok(Some(Job {
            description,
            remote,
            salary_min,
            salary_max,
            currency,
            ..Job::newly_discovered(title, company, url, location, "jobswithgpt", Utc::now())
        }))
    }
}

fn jobswithgpt_request(endpoint: &str, body: serde_json::Value) -> ExternalHttpRequest {
    ExternalHttpRequest::post(endpoint)
        .user_agent(BROWSER_USER_AGENT)
        .json(body)
        .without_retries()
}

#[async_trait]
impl JobScraper for JobsWithGptScraper {
    async fn scrape(&self) -> ScraperResult {
        self.query_mcp().await
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "jobswithgpt"
    }
}

#[cfg(test)]
mod tests;
