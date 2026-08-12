use super::*;

fn test_company() -> LeverCompany {
    LeverCompany {
        id: "test".to_string(),
        name: "Test Company".to_string(),
        url: "https://jobs.lever.co/test".to_string(),
    }
}

fn parse_test_postings(json: &serde_json::Value) -> Vec<Job> {
    LeverScraper::parse_postings(json, &test_company())
}

fn default_lever_job() -> Job {
    Job::newly_discovered(
        "Test".to_string(),
        "Test".to_string(),
        "https://test.com".to_string(),
        None,
        "lever",
        Utc::now(),
    )
}

#[path = "tests/basic_tests.rs"]
mod basic_tests;

#[path = "tests/api_contract_tests.rs"]
mod api_contract_tests;

#[path = "tests/json_edge_tests.rs"]
mod json_edge_tests;

#[path = "tests/remote_inference_tests.rs"]
mod remote_inference_tests;

#[path = "tests/scrape_company_flow_tests.rs"]
mod scrape_company_flow_tests;

// ========================================
// Integration Tests for scrape_company
// ========================================

#[tokio::test]
async fn test_scrape_company_creates_jobs_from_api_response() {
    // This test validates the full scrape_company flow with mock data
    let company = LeverCompany {
        id: "test-company".to_string(),
        name: "Test Company".to_string(),
        url: "https://jobs.lever.co/test-company".to_string(),
    };

    let _scraper = LeverScraper::new(vec![company.clone()]);

    // We can't directly test scrape_company without a real API or mock server
    // but we can test the JSON processing logic that it uses
    let json_response = serde_json::json!([
        {
            "text": "Senior Public Health Analyst",
            "hostedUrl": "https://jobs.lever.co/test-company/job1",
            "categories": {
                "location": "Remote",
                "team": "Community Care"
            },
            "description": "<p>Join our care coordination team</p>",
            "descriptionPlain": "Join our care coordination team"
        },
        {
            "text": "Customer Support Manager",
            "hostedUrl": "https://jobs.lever.co/test-company/job2",
            "categories": {
                "location": "San Francisco, CA"
            },
            "descriptionPlain": "Support neighbors across multiple channels"
        },
        {
            "text": "",
            "hostedUrl": "https://jobs.lever.co/test-company/job3"
        }
    ]);

    let jobs = LeverScraper::parse_postings(&json_response, &company);

    // Validate results
    assert_eq!(
        jobs.len(),
        2,
        "Should create 2 jobs (third has empty title)"
    );

    // Validate first job
    assert_eq!(jobs[0].title, "Senior Public Health Analyst");
    assert_eq!(jobs[0].company, "Test Company");
    assert_eq!(jobs[0].url, "https://jobs.lever.co/test-company/job1");
    assert_eq!(jobs[0].location, Some("Remote".to_string()));
    assert_eq!(
        jobs[0].description,
        Some("<p>Join our care coordination team</p>".to_string())
    );
    assert_eq!(jobs[0].source, "lever");
    assert_eq!(jobs[0].remote, Some(true));
    assert_eq!(jobs[0].times_seen, 1);
    assert_eq!(jobs[0].immediate_alert_sent, false);
    assert_eq!(jobs[0].hidden, false);
    assert_eq!(jobs[0].bookmarked, false);
    assert_eq!(jobs[0].notes, None);
    assert_eq!(jobs[0].hash.len(), 64);

    // Validate second job
    assert_eq!(jobs[1].title, "Customer Support Manager");
    assert_eq!(jobs[1].company, "Test Company");
    assert_eq!(jobs[1].remote, Some(false));
    assert_eq!(jobs[1].location, Some("San Francisco, CA".to_string()));
}

#[tokio::test]
async fn test_scrape_company_handles_empty_response() {
    let company = LeverCompany {
        id: "empty-company".to_string(),
        name: "Empty Company".to_string(),
        url: "https://jobs.lever.co/empty-company".to_string(),
    };

    let json_response = serde_json::json!([]);

    let jobs = LeverScraper::parse_postings(&json_response, &company);

    assert_eq!(jobs.len(), 0);
}

#[tokio::test]
async fn test_scrape_reports_error_when_all_companies_fail() {
    let scraper = LeverScraper::new(vec![LeverCompany {
        id: "broken".to_string(),
        name: "Broken Company".to_string(),
        url: "not a valid url".to_string(),
    }]);

    let error = scraper
        .scrape()
        .await
        .expect_err("all failed companies should not look like an empty success");

    assert!(matches!(
        error,
        ScraperError::Generic {
            scraper,
            message
        } if scraper == "lever" && message.contains("All configured company boards failed")
    ));
}

#[tokio::test]
async fn test_scrape_with_empty_companies() {
    let scraper = LeverScraper::new(vec![]);
    let result = scraper.scrape().await;

    assert!(result.is_ok());
    let jobs = result.unwrap();
    assert_eq!(jobs.len(), 0);
}

#[test]
fn test_job_struct_fields_are_populated_correctly() {
    // Test that Job struct is created with all required fields
    let company_name = "Test Co";
    let title = "Care Coordinator (Remote)";
    let location = Some("Remote - US");
    let url = "https://jobs.lever.co/test/abc";
    let description = Some("<p>Description</p>".to_string());

    let remote = LeverScraper::infer_remote(title, location);

    let job = Job {
        description,
        remote: Some(remote),
        ..Job::newly_discovered(
            title.to_string(),
            company_name.to_string(),
            url.to_string(),
            location.map(str::to_string),
            "lever",
            Utc::now(),
        )
    };

    assert_eq!(job.title, "Care Coordinator (Remote)");
    assert_eq!(job.company, "Test Co");
    assert_eq!(job.source, "lever");
    assert_eq!(job.remote, Some(true));
    assert_eq!(job.times_seen, 1);
    assert!(!job.immediate_alert_sent);
    assert!(!job.hidden);
    assert!(!job.bookmarked);
    assert!(job.notes.is_none());
    assert!(!job.included_in_digest);
    assert_eq!(job.hash.len(), 64);
}

#[test]
fn test_job_struct_with_missing_optional_fields() {
    let job = Job {
        hash: "test-hash".to_string(),
        ..default_lever_job()
    };

    assert!(job.location.is_none());
    assert!(job.description.is_none());
    assert!(job.remote.is_none());
    assert!(job.salary_min.is_none());
    assert!(job.salary_max.is_none());
}

#[test]
fn test_description_extraction_priority() {
    // Test description field has priority over descriptionPlain
    let json = serde_json::json!({
        "text": "Care Coordinator",
        "hostedUrl": "https://test.com/job",
        "description": "<p>HTML description</p>",
        "descriptionPlain": "Plain description"
    });

    let description = LeverScraper::posting_description(&json);

    assert_eq!(description, Some("<p>HTML description</p>".to_string()));

    // Test descriptionPlain fallback
    let json2 = serde_json::json!({
        "text": "Care Coordinator",
        "hostedUrl": "https://test.com/job",
        "descriptionPlain": "Plain description"
    });

    let description2 = LeverScraper::posting_description(&json2);

    assert_eq!(description2, Some("Plain description".to_string()));
}

#[test]
fn test_location_extraction_with_team_fallback() {
    // Test location field exists
    let json = serde_json::json!({
        "categories": {
            "location": "Remote",
            "team": "Community Care"
        }
    });

    let location = LeverScraper::posting_location(&json);

    assert_eq!(location, Some("Remote".to_string()));

    // Test team fallback when location is missing
    let json2 = serde_json::json!({
        "categories": {
            "team": "Care Operations"
        }
    });

    let location2 = LeverScraper::posting_location(&json2);

    assert_eq!(location2, Some("Care Operations".to_string()));

    // Test neither field exists
    let json3 = serde_json::json!({
        "categories": {}
    });

    let location3 = LeverScraper::posting_location(&json3);

    assert_eq!(location3, None);
}

#[path = "tests/property_tests.rs"]
mod property_tests;
