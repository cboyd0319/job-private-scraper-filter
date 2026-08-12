use super::*;

#[test]
fn jobswithgpt_post_disables_automatic_retries() {
    let request = jobswithgpt_request(
        "https://api.jobswithgpt.example/mcp",
        serde_json::json!({"jsonrpc": "2.0"}),
    );

    assert!(format!("{request:?}").contains("max_retries: 0"));
}

// MCP job parsing tests
#[test]
fn test_parse_mcp_job_complete() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec!["Care Coordinator".to_string()],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "Care Coordinator",
        "company": "Community Care Network",
        "url": "https://example.com/job/123",
        "location": "Remote",
        "description": "Coordinate services and follow-up for community members",
        "remote": true,
        "salary_min": 55000,
        "salary_max": 72000,
        "currency": "USD"
    });

    let job = scraper.parse_mcp_job(&job_data).unwrap().unwrap();

    assert_eq!(job.title, "Care Coordinator");
    assert_eq!(job.company, "Community Care Network");
    assert_eq!(job.url, "https://example.com/job/123");
    assert_eq!(job.location, Some("Remote".to_string()));
    assert_eq!(
        job.description,
        Some("Coordinate services and follow-up for community members".to_string())
    );
    assert_eq!(job.remote, Some(true));
    assert_eq!(job.salary_min, Some(55000));
    assert_eq!(job.salary_max, Some(72000));
    assert_eq!(job.currency, Some("USD".to_string()));
    assert_eq!(job.source, "jobswithgpt");
}

#[test]
fn manifest_fixture_remains_parseable_by_the_jobswithgpt_adapter() {
    let scraper = JobsWithGptScraper::new(
        "https://api.jobswithgpt.example/mcp".to_string(),
        JobQuery {
            titles: vec!["Case Manager".to_string()],
            location: None,
            remote_only: true,
            limit: 100,
        },
    );
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../fixtures/jobswithgpt_list_v1.json")).unwrap();

    let job = scraper.parse_mcp_job(&fixture).unwrap().unwrap();

    assert_eq!(job.title, "Case Manager");
    assert_eq!(job.company, "Community Care Network");
    assert_eq!(job.url, "https://example.com/jobs/case-manager");
    assert_eq!(job.source, "jobswithgpt");
}

#[test]
fn test_parse_mcp_job_minimal() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "Program Coordinator",
        "company": "Community Services",
        "url": "https://example.com/job"
    });

    let job = scraper.parse_mcp_job(&job_data).unwrap().unwrap();

    assert_eq!(job.title, "Program Coordinator");
    assert_eq!(job.company, "Community Services");
    assert_eq!(job.url, "https://example.com/job");
    assert_eq!(job.location, None);
    assert_eq!(job.description, None);
    assert_eq!(job.remote, None);
    assert_eq!(job.salary_min, None);
    assert_eq!(job.salary_max, None);
    assert_eq!(job.currency, None);
}

#[test]
fn test_parse_mcp_job_minimizes_returned_url() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "Program Coordinator",
        "company": "Community Services",
        "url": "https://user:pass@example.com/job?utm_source=mcp&jobId=123&token=private#resume"
    });

    let job = scraper.parse_mcp_job(&job_data).unwrap().unwrap();

    assert_eq!(job.url, "https://example.com/job?jobId=123");
    assert!(!job.url.contains("user"));
    assert!(!job.url.contains("pass"));
    assert!(!job.url.contains("utm_source"));
    assert!(!job.url.contains("token"));
    assert!(!job.url.contains("resume"));
}

#[test]
fn test_parse_mcp_job_rejects_unsafe_returned_url() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    for url in [
        "javascript:alert(1)",
        "file:///private/resume.pdf",
        "http://127.0.0.1:4321/api/bookmarklet/import?token=private",
        "https://intranet.corp/jobs/1",
    ] {
        let job_data = serde_json::json!({
            "title": "Program Coordinator",
            "company": "Community Services",
            "url": url
        });

        let result = scraper.parse_mcp_job(&job_data).unwrap();
        assert!(result.is_none(), "unsafe URL should be dropped: {url}");
    }
}

#[test]
fn test_parse_mcp_job_empty_title_returns_none() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "",
        "company": "Company",
        "url": "https://example.com/job"
    });

    let result = scraper.parse_mcp_job(&job_data).unwrap();
    assert!(result.is_none(), "Empty title should return None");
}

#[test]
fn test_parse_mcp_job_empty_url_returns_none() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "Program Coordinator",
        "company": "Company",
        "url": ""
    });

    let result = scraper.parse_mcp_job(&job_data).unwrap();
    assert!(result.is_none(), "Empty URL should return None");
}

#[test]
fn test_parse_mcp_job_missing_company_defaults_to_unknown() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    let job_data = serde_json::json!({
        "title": "Program Coordinator",
        "url": "https://example.com/job"
    });

    let job = scraper.parse_mcp_job(&job_data).unwrap().unwrap();
    assert_eq!(job.company, "Unknown");
}

// JobQuery tests
#[test]
fn test_job_query_creation() {
    let query = JobQuery {
        titles: vec![
            "Care Coordinator".to_string(),
            "Inventory Planner".to_string(),
        ],
        location: Some("Denver".to_string()),
        remote_only: true,
        limit: 50,
    };

    assert_eq!(query.titles.len(), 2);
    assert_eq!(query.titles[0], "Care Coordinator");
    assert_eq!(query.location, Some("Denver".to_string()));
    assert!(query.remote_only);
    assert_eq!(query.limit, 50);
}

#[test]
fn test_job_query_remote_only() {
    let query = JobQuery {
        titles: vec!["Care Coordinator".to_string()],
        location: None,
        remote_only: true,
        limit: 100,
    };

    assert!(query.remote_only);
    assert_eq!(query.location, None);
}

// Scraper initialization tests
#[test]
fn test_scraper_name() {
    let scraper = JobsWithGptScraper::new(
        "http://localhost:3000/mcp".to_string(),
        JobQuery {
            titles: vec![],
            location: None,
            remote_only: false,
            limit: 10,
        },
    );

    assert_eq!(scraper.name(), "jobswithgpt");
}

#[test]
fn test_new_scraper_with_endpoint() {
    let endpoint = "https://api.jobswithgpt.com/mcp".to_string();
    let query = JobQuery {
        titles: vec!["Care Coordinator".to_string()],
        location: Some("Remote".to_string()),
        remote_only: true,
        limit: 25,
    };

    let scraper = JobsWithGptScraper::new(endpoint.clone(), query.clone());

    assert_eq!(scraper.endpoint, endpoint);
    assert_eq!(scraper.query.titles, query.titles);
    assert_eq!(scraper.query.location, query.location);
    assert_eq!(scraper.query.remote_only, query.remote_only);
    assert_eq!(scraper.query.limit, query.limit);
}

mod property_and_edge_tests;

mod response_edge_cases;
