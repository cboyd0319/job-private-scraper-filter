//! Hacker News Who's Hiring Scraper
//!
//! Scrapes jobs from the monthly "Who is hiring?" threads on Hacker News.
//! These threads are posted on the first of each month and contain
//! high-quality tech job postings from the HN community.

use super::error::ScraperError;
use super::rate_limiter::RateLimiter;
use super::{
    decode_common_html_entities, strip_html_markup, JobScraper, ScraperResult,
    JOBSENTINEL_USER_AGENT,
};
use jobsentinel_domain::{
    normalization::infer_remote_status,
    v3_source_manifest::{
        HN_HIRING_ITEM_ENDPOINT_PREFIX, HN_HIRING_REQUEST_LIMIT_PER_HOUR, HN_HIRING_SEARCH_ENDPOINT,
    },
    Job,
};
use jobsentinel_network::{send_external_http_text_with_retry, ExternalHttpRequest};
use std::num::NonZeroU16;

use async_trait::async_trait;
use chrono::Utc;

/// Hacker News Who's Hiring scraper
#[derive(Debug, Clone)]
pub struct HnHiringScraper {
    /// Maximum results to return
    pub limit: usize,
    /// Filter for remote jobs only
    pub remote_only: bool,
    /// Rate limiter for respecting HN API limits
    pub rate_limiter: RateLimiter,
    /// JobSentinel policy rate for this source
    pub request_limit_per_hour: u32,
}

impl HnHiringScraper {
    pub fn new(limit: usize, remote_only: bool) -> Self {
        Self {
            limit,
            remote_only,
            rate_limiter: RateLimiter::shared(),
            request_limit_per_hour: u32::from(HN_HIRING_REQUEST_LIMIT_PER_HOUR),
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

    /// Fetch the latest "Who is hiring?" thread
    async fn fetch_jobs(&self) -> ScraperResult {
        tracing::info!("Fetching jobs from HN Who's Hiring");

        self.rate_limiter
            .wait_paced("hn_hiring", self.request_limit_per_hour)
            .await;

        let search_url = HN_HIRING_SEARCH_ENDPOINT;

        let response = send_external_http_text_with_retry(Self::request(search_url).query([
            ("tags".to_string(), "story,author_whoishiring".to_string()),
            ("hitsPerPage".to_string(), "10".to_string()),
        ]))
        .await
        .map_err(|error| ScraperError::from_external("hn_hiring", error))?;

        if !(200..300).contains(&response.status) {
            return Err(ScraperError::http_status(
                response.status,
                search_url,
                format!("HN search failed: {}", response.status),
            ));
        }

        let search_result: serde_json::Value =
            ScraperError::parse_json(search_url, &response.body)?;

        let thread_id = Self::canonical_thread_id(&search_result).ok_or_else(|| {
            ScraperError::SelectorNotFound {
                url: "HN API response".to_string(),
                selector: "current whoishiring thread".to_string(),
            }
        })?;

        self.rate_limiter
            .wait_paced("hn_hiring", self.request_limit_per_hour)
            .await;
        let thread_url = format!("{HN_HIRING_ITEM_ENDPOINT_PREFIX}{thread_id}");

        let comments_response = send_external_http_text_with_retry(Self::request(&thread_url))
            .await
            .map_err(|error| ScraperError::from_external("hn_hiring", error))?;

        if !(200..300).contains(&comments_response.status) {
            return Err(ScraperError::http_status(
                comments_response.status,
                &thread_url,
                format!("HN comments fetch failed: {}", comments_response.status),
            ));
        }

        let comments_result: serde_json::Value =
            ScraperError::parse_json(&thread_url, &comments_response.body)?;
        let jobs = self.parse_thread_item(&comments_result, thread_id)?;

        tracing::info!("Found {} jobs from HN Who's Hiring", jobs.len());
        Ok(jobs)
    }

    #[must_use]
    pub fn canonical_thread_id(data: &serde_json::Value) -> Option<u64> {
        data["hits"].as_array().and_then(|hits| {
            hits.iter().find_map(|hit| {
                Self::is_canonical_thread(hit["author"].as_str(), hit["title"].as_str())
                    .then(|| hit["objectID"].as_str()?.parse().ok())
                    .flatten()
                    .filter(|id| *id != 0)
            })
        })
    }

    fn is_canonical_thread(author: Option<&str>, title: Option<&str>) -> bool {
        author == Some("whoishiring")
            && title.is_some_and(|title| {
                title.starts_with("Ask HN: Who is hiring? (") && title.ends_with(')')
            })
    }

    fn parse_thread_item(
        &self,
        data: &serde_json::Value,
        expected_thread_id: u64,
    ) -> Result<Vec<Job>, ScraperError> {
        if !Self::is_canonical_thread_item(data, expected_thread_id) {
            return Err(ScraperError::SelectorNotFound {
                url: "HN item API response".to_string(),
                selector: "selected canonical whoishiring thread".to_string(),
            });
        }
        self.parse_thread_children(data)
    }

    #[must_use]
    pub fn is_canonical_thread_item(data: &serde_json::Value, expected_thread_id: u64) -> bool {
        data["id"].as_u64() == Some(expected_thread_id)
            && Self::is_canonical_thread(data["author"].as_str(), data["title"].as_str())
            && data["children"].is_array()
    }

    fn parse_thread_children(&self, data: &serde_json::Value) -> Result<Vec<Job>, ScraperError> {
        let children = data["children"].as_array().ok_or_else(|| {
            ScraperError::parse(
                "JSON",
                "HN item API response",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "No direct children in response",
                ),
            )
        })?;
        let mut jobs = Vec::new();

        for comment in children {
            if jobs.len() >= self.limit {
                break;
            }
            let text = comment["text"].as_str().unwrap_or("");
            let Some(comment_id) = comment["id"].as_u64().filter(|id| *id != 0) else {
                continue;
            };

            if text.len() < 50 {
                continue;
            }
            if let Some(job) = self.parse_job_text(text, comment_id) {
                if self.remote_only && job.remote != Some(true) {
                    continue;
                }
                jobs.push(job);
            }
        }

        Ok(jobs)
    }

    fn parse_job_text(&self, text: &str, comment_id: u64) -> Option<Job> {
        // Clean HTML from comment
        let clean_text = Self::strip_html(text);

        // Extract first line as potential company/title info
        let first_line = clean_text.lines().next().unwrap_or("");

        // Try to extract company name (usually at the start)
        let (company, rest) = Self::extract_company(first_line);

        if company.is_empty() || company.len() < 2 {
            return None;
        }

        // Extract job title (look for common patterns)
        let title = Self::extract_title(&clean_text).unwrap_or_else(|| {
            // Use first part after company as title
            rest.split('|')
                .next()
                .unwrap_or("Software Engineer")
                .trim()
                .to_string()
        });

        // Extract location
        let location = Self::extract_location(&clean_text);

        // Check if remote
        let is_remote = Self::is_remote(&clean_text);

        // Build URL to the HN comment
        let url = format!("https://news.ycombinator.com/item?id={}", comment_id);

        if title.is_empty() {
            return None;
        }

        let description = Some(clean_text.char_indices().nth(500).map_or_else(
            || clean_text.clone(),
            |(end, _)| format!("{}...", &clean_text[..end]),
        ));

        Some(Job {
            description,
            remote: Some(is_remote),
            ..Job::newly_discovered(title, company, url, location, "hn_hiring", Utc::now())
        })
    }

    /// Strip HTML tags from text
    fn strip_html(html: &str) -> String {
        // Replace common HTML entities and tags
        let processed = decode_common_html_entities(
            &html
                .replace("<p>", "\n\n")
                .replace("</p>", "")
                .replace("<br>", "\n")
                .replace("<br/>", "\n")
                .replace("<br />", "\n")
                .replace("&#x27;", "'"),
        );
        let result = strip_html_markup(&processed);

        // Clean up whitespace
        result
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract company name from first line
    fn extract_company(line: &str) -> (String, String) {
        // Common patterns:
        // "Company Name | Role | Location"
        // "Company Name (YC S20) | Role"
        // "Company Name - Role"

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            let company = parts[0]
                .trim()
                .trim_end_matches(['(', '-'])
                .trim()
                .to_string();
            let rest = parts[1..].join("|");
            return (company, rest);
        }

        let parts: Vec<&str> = line.split(" - ").collect();
        if parts.len() >= 2 {
            return (parts[0].trim().to_string(), parts[1..].join(" - "));
        }

        // Just return the whole line as company
        (line.trim().to_string(), String::new())
    }

    /// Extract job title from text
    fn extract_title(text: &str) -> Option<String> {
        let lower = text.to_ascii_lowercase();

        // Look for common title patterns
        let patterns = [
            "software engineer",
            "senior engineer",
            "backend engineer",
            "frontend engineer",
            "full stack",
            "fullstack",
            "data scientist",
            "machine learning",
            "devops",
            "sre",
            "engineering manager",
            "tech lead",
            "cto",
            "vp engineering",
            "product manager",
            "designer",
            "data engineer",
            "platform engineer",
            "infrastructure",
        ];

        for pattern in patterns {
            if let Some(pos) = lower.find(pattern) {
                // Extract the title with surrounding context
                let start = text[..pos].rfind(['|', '\n']).map(|p| p + 1).unwrap_or(0);
                let end = pos + pattern.len();
                let end = text[end..]
                    .find(['|', '\n', ','])
                    .map(|p| end + p)
                    .unwrap_or(end);

                let title = text[start..end].trim();
                if title.len() > 5 && title.len() < 100 {
                    return Some(title.to_string());
                }
            }
        }

        None
    }

    /// Extract location from text
    fn extract_location(text: &str) -> Option<String> {
        let lower = text.to_lowercase();

        // Check for specific city mentions
        let cities = [
            ("san francisco", "San Francisco, CA"),
            ("sf", "San Francisco, CA"),
            ("new york", "New York, NY"),
            ("nyc", "New York, NY"),
            ("los angeles", "Los Angeles, CA"),
            ("seattle", "Seattle, WA"),
            ("austin", "Austin, TX"),
            ("boston", "Boston, MA"),
            ("denver", "Denver, CO"),
            ("chicago", "Chicago, IL"),
            ("london", "London, UK"),
            ("berlin", "Berlin, Germany"),
            ("toronto", "Toronto, Canada"),
            ("vancouver", "Vancouver, Canada"),
        ];

        for (pattern, location) in cities {
            if lower.contains(pattern) {
                return Some(location.to_string());
            }
        }

        // Check for generic remote
        if lower.contains("remote") {
            if lower.contains("us only") || lower.contains("usa only") {
                return Some("Remote (US)".to_string());
            }
            if lower.contains("europe") || lower.contains("eu only") {
                return Some("Remote (EU)".to_string());
            }
            return Some("Remote".to_string());
        }

        None
    }

    /// Check if job is remote
    fn is_remote(text: &str) -> bool {
        infer_remote_status(&[text]).is_remote()
    }
}

#[async_trait]
impl JobScraper for HnHiringScraper {
    async fn scrape(&self) -> ScraperResult {
        self.fetch_jobs().await
    }

    #[cfg(test)]
    fn name(&self) -> &'static str {
        "hn_hiring"
    }
}

#[cfg(test)]
mod tests;
