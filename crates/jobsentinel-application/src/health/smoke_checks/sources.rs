//! Runs bounded source health checks behind each source's authorization contract.

use crate::{
    credentials::{CredentialKey, CredentialService},
    Config,
};
use anyhow::Result;
use jobsentinel_domain::v3_source_manifest::{
    GREENHOUSE_API_ENDPOINT_PREFIX, HN_HIRING_ITEM_ENDPOINT_PREFIX, HN_HIRING_SEARCH_ENDPOINT,
    LEVER_API_ENDPOINT_PREFIX,
};
use jobsentinel_network::{
    send_external_http_text_with_retry, ExternalHttpRequest, ExternalTextResponse,
    MINIMAL_BROWSER_USER_AGENT,
};
use jobsentinel_sources::{HnHiringScraper, RateLimiter};

async fn smoke_request(
    request: ExternalHttpRequest,
    context: &'static str,
) -> Result<ExternalTextResponse> {
    send_external_http_text_with_retry(request)
        .await
        .map_err(|_| anyhow::anyhow!(context))
}

fn require_success(response: ExternalTextResponse) -> Result<ExternalTextResponse> {
    if (200..300).contains(&response.status) {
        Ok(response)
    } else {
        Err(anyhow::anyhow!("HTTP {}", response.status))
    }
}

fn parse_json(response: &ExternalTextResponse) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(&response.body)?)
}

pub(super) async fn test_greenhouse() -> Result<serde_json::Value> {
    let url = format!("{GREENHOUSE_API_ENDPOINT_PREFIX}cloudflare/jobs");
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(&url).without_retries(),
            "Greenhouse smoke test request failed",
        )
        .await?,
    )?;
    let json = parse_json(&response)?;
    let job_count = json
        .get("jobs")
        .and_then(|j| j.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": job_count,
        "api_version": "v1"
    }))
}

pub(super) async fn test_lever() -> Result<serde_json::Value> {
    let url = format!("{LEVER_API_ENDPOINT_PREFIX}netflix");
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(&url).without_retries(),
            "Lever smoke test request failed",
        )
        .await?,
    )?;
    let json = parse_json(&response)?;
    let job_count = json.as_array().map(|a| a.len()).unwrap_or(0);

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": job_count,
        "api_version": "v0"
    }))
}

pub(super) async fn test_indeed() -> Result<serde_json::Value> {
    let url = "https://www.indeed.com/jobs?q=customer+support&l=remote";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url).user_agent(MINIMAL_BROWSER_USER_AGENT),
            "Indeed smoke test request failed",
        )
        .await?,
    )?;
    let html = response.body;
    let has_jobs = html.contains("job_seen_beacon") || html.contains("jobsearch-SerpJobCard");

    Ok(serde_json::json!({
        "status": response.status,
        "selectors_found": has_jobs,
        "html_size_kb": html.len() / 1024
    }))
}

pub(super) async fn test_remoteok() -> Result<serde_json::Value> {
    let url = "https://remoteok.com/api";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url).without_retries(),
            "RemoteOK smoke test request failed",
        )
        .await?,
    )?;
    let json = parse_json(&response)?;
    // First element is legal notice, rest are jobs
    let job_count = json
        .as_array()
        .map(|a| a.len().saturating_sub(1))
        .unwrap_or(0);

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": job_count
    }))
}

pub(super) async fn test_wellfound() -> Result<serde_json::Value> {
    let url = "https://wellfound.com/role/software-engineer";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url).user_agent(MINIMAL_BROWSER_USER_AGENT),
            "Wellfound smoke test request failed",
        )
        .await?,
    )?;
    let html = response.body;
    let has_jobs = html.contains("StartupResult") || html.contains("startupResult");

    Ok(serde_json::json!({
        "status": response.status,
        "selectors_found": has_jobs,
        "html_size_kb": html.len() / 1024
    }))
}

pub(super) async fn test_weworkremotely() -> Result<serde_json::Value> {
    let url = "https://weworkremotely.com/remote-jobs.rss";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url).without_retries(),
            "We Work Remotely smoke test request failed",
        )
        .await?,
    )?;
    let rss = response.body;
    let item_count = rss.matches("<item>").count();

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": item_count,
        "format": "rss"
    }))
}

pub(super) async fn test_hn_hiring(request_limit_per_hour: u32) -> Result<serde_json::Value> {
    let url = HN_HIRING_SEARCH_ENDPOINT;
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url).without_retries().query([
                ("tags".to_string(), "story,author_whoishiring".to_string()),
                ("hitsPerPage".to_string(), "10".to_string()),
            ]),
            "HN Hiring smoke test request failed",
        )
        .await?,
    )?;
    let json = parse_json(&response)?;
    let thread_id = HnHiringScraper::canonical_thread_id(&json)
        .ok_or_else(|| anyhow::anyhow!("HN hiring thread not found"))?;

    RateLimiter::shared()
        .wait_paced("hn_hiring", request_limit_per_hour)
        .await;
    let thread_url = format!("{HN_HIRING_ITEM_ENDPOINT_PREFIX}{thread_id}");
    let thread_response = require_success(
        smoke_request(
            ExternalHttpRequest::get(&thread_url).without_retries(),
            "HN Hiring item smoke test request failed",
        )
        .await?,
    )?;
    let thread = parse_json(&thread_response)?;

    Ok(serde_json::json!({
        "status": thread_response.status,
        "selectors_found": HnHiringScraper::is_canonical_thread_item(&thread, thread_id),
        "api": "algolia"
    }))
}

pub(super) async fn test_jobswithgpt(config: &Config) -> Result<serde_json::Value> {
    let reason = if config.jobswithgpt_payload_approved() {
        "JobsWithGPT provider endpoint and usage policy require review"
    } else {
        "JobsWithGPT exact request details are not approved"
    };

    Ok(serde_json::json!({
        "status": "skipped",
        "reason": reason
    }))
}

pub(super) async fn test_ziprecruiter() -> Result<serde_json::Value> {
    let url = "https://www.ziprecruiter.com/jobs-rss?search=customer+support&location=Remote";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url),
            "ZipRecruiter smoke test request failed",
        )
        .await?,
    )?;
    let rss = response.body;
    let item_count = rss.matches("<item>").count();

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": item_count,
        "format": "rss"
    }))
}

pub(super) async fn test_usajobs(
    config: &Config,
    credentials: &CredentialService,
) -> Result<serde_json::Value> {
    let api_key = match credentials
        .retrieve(CredentialKey::UsaJobsApiKey)
        .await
        .map_err(|_| anyhow::anyhow!("USAJobs API key could not be read from secure storage"))?
    {
        Some(api_key) if !api_key.trim().is_empty() => api_key,
        _ => {
            return Ok(serde_json::json!({
                "status": "skipped",
                "reason": "USAJobs API key not saved"
            }));
        }
    };

    let url = "https://data.usajobs.gov/api/Search?Keyword=software&ResultsPerPage=1";
    let response = require_success(
        smoke_request(
            ExternalHttpRequest::get(url)
                .without_retries()
                .header("Host", "data.usajobs.gov")
                .user_agent(&config.usajobs.email)
                .header("Authorization-Key", api_key.trim()),
            "USAJobs smoke test request failed",
        )
        .await?,
    )?;
    let json = parse_json(&response)?;
    let job_count = json
        .pointer("/SearchResult/SearchResultCount")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);

    Ok(serde_json::json!({
        "status": response.status,
        "jobs_found": job_count,
        "api": "usajobs"
    }))
}

#[cfg(test)]
mod tests {
    use super::test_jobswithgpt;
    use crate::Config;

    #[tokio::test]
    async fn jobswithgpt_smoke_check_requires_exact_payload_approval() {
        let mut config = Config::first_run();
        config.jobswithgpt_endpoint = "https://jobs.example.test/mcp".to_string();
        config.title_allowlist = vec!["Case Manager".to_string()];

        let result = test_jobswithgpt(&config).await.unwrap();
        assert_eq!(
            result["reason"],
            "JobsWithGPT exact request details are not approved"
        );

        config.jobswithgpt_approval.enabled = true;
        config.jobswithgpt_approval.payload = config.jobswithgpt_payload_preview();
        let approved = test_jobswithgpt(&config).await.unwrap();
        assert_eq!(
            approved["reason"],
            "JobsWithGPT provider endpoint and usage policy require review"
        );
    }
}
