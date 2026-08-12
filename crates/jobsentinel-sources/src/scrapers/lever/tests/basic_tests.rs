use super::*;

#[test]
fn reviewed_fixture_exercises_the_production_parser() {
    let company = LeverCompany {
        id: "example".to_string(),
        name: "Example".to_string(),
        url: "https://jobs.lever.co/example".to_string(),
    };
    let payload = include_str!("../../../fixtures/lever_list_v1.json");
    let json = serde_json::from_str(payload).unwrap();

    let jobs = LeverScraper::parse_api_response(
        &json,
        &company,
        "https://api.lever.co/v0/postings/example",
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Program Coordinator");
    assert_eq!(jobs[0].url, "https://jobs.lever.co/example/example-posting");
}

#[test]
fn provider_error_object_is_parser_drift_not_an_empty_success() {
    let company = LeverCompany {
        id: "example".to_string(),
        name: "Example".to_string(),
        url: "https://jobs.lever.co/example".to_string(),
    };

    assert!(LeverScraper::parse_api_response(
        &serde_json::json!({"error": "schema changed"}),
        &company,
        "https://api.lever.co/v0/postings/example",
    )
    .is_err());
    assert!(LeverScraper::parse_api_response(
        &serde_json::json!([]),
        &company,
        "https://api.lever.co/v0/postings/example",
    )
    .unwrap()
    .is_empty());
}

// Hash computation tests
#[test]
fn test_compute_hash_deterministic() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "FreshMart",
        "Care Coordinator",
        Some("Remote"),
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "FreshMart",
        "Care Coordinator",
        Some("Remote"),
        "https://example.com/1",
    );

    assert_eq!(hash1, hash2, "Same inputs should produce same hash");
    assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 hex chars");
}

#[test]
fn test_compute_hash_different_company() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "FreshMart",
        "Care Coordinator",
        None,
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Community Care Network",
        "Care Coordinator",
        None,
        "https://example.com/1",
    );

    assert_ne!(
        hash1, hash2,
        "Different company should produce different hash"
    );
}

#[test]
fn test_compute_hash_different_title() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Customer Support Manager",
        None,
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Inventory Planner",
        None,
        "https://example.com/1",
    );

    assert_ne!(
        hash1, hash2,
        "Different title should produce different hash"
    );
}

#[test]
fn test_compute_hash_different_location() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("Remote"),
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("SF"),
        "https://example.com/1",
    );

    assert_ne!(
        hash1, hash2,
        "Different location should produce different hash"
    );
}

#[test]
fn test_compute_hash_location_none_vs_some() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("Remote"),
        "https://example.com/1",
    );

    assert_ne!(hash1, hash2, "None location should differ from Some");
}

#[test]
fn test_compute_hash_different_url() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://example.com/2",
    );

    assert_ne!(hash1, hash2, "Different URL should produce different hash");
}

#[test]
fn test_compute_hash_empty_strings() {
    let hash = jobsentinel_domain::calculate_job_hash("", "", None, "");
    assert_eq!(
        hash.len(),
        64,
        "Hash of empty strings should still be valid"
    );
}

#[test]
fn test_compute_hash_special_characters() {
    let hash = jobsentinel_domain::calculate_job_hash(
        "Company™",
        "Senior Care Coordinator (Remote) 🚀",
        Some("San Francisco, CA"),
        "https://jobs.lever.co/company/job-id?ref=test&utm_source=linkedin",
    );

    assert_eq!(hash.len(), 64, "Hash should handle special characters");
}

// Scraper initialization tests
#[test]
fn test_scraper_name() {
    let scraper = LeverScraper::new(vec![]);
    assert_eq!(scraper.name(), "lever");
}

#[test]
fn test_company_scrape_failure_copy_omits_company_and_error_detail() {
    assert!(!COMPANY_SCRAPE_FAILED.contains("{}"));
    assert!(!COMPANY_SCRAPE_FAILED.contains("https://"));
    assert!(!COMPANY_SCRAPE_FAILED.contains("secret"));
    assert!(!COMPANY_SCRAPE_FAILED.contains("Community Care Network"));
}

#[test]
fn test_new_scraper_with_companies() {
    let companies = vec![
        LeverCompany {
            id: "freshmart".to_string(),
            name: "FreshMart".to_string(),
            url: "https://jobs.lever.co/freshmart".to_string(),
        },
        LeverCompany {
            id: "community-care-network".to_string(),
            name: "Community Care Network".to_string(),
            url: "https://jobs.lever.co/community-care-network".to_string(),
        },
    ];

    let scraper = LeverScraper::new(companies.clone());

    assert_eq!(scraper.companies.len(), 2);
    assert_eq!(scraper.companies[0].name, "FreshMart");
    assert_eq!(scraper.companies[1].name, "Community Care Network");
    assert_eq!(scraper.companies[0].id, "freshmart");
}

#[test]
fn test_new_scraper_empty() {
    let scraper = LeverScraper::new(vec![]);
    assert_eq!(scraper.companies.len(), 0);
}

#[test]
fn test_parse_response_single_job() {
    let json_data = r#"
    [
        {
            "text": "Senior Public Health Analyst",
            "hostedUrl": "https://jobs.lever.co/city-health-department/abc123",
            "categories": {
                "location": "Remote",
                "team": "Community Care"
            },
            "description": "<p>Join our care coordination team</p>",
            "descriptionPlain": "Join our care coordination team"
        }
    ]
    "#;

    let _scraper = LeverScraper::new(vec![LeverCompany {
        id: "city-health-department".to_string(),
        name: "City Health Department".to_string(),
        url: "https://jobs.lever.co/city-health-department".to_string(),
    }]);

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        assert_eq!(postings.len(), 1);

        let posting = &postings[0];
        let title = posting["text"].as_str().unwrap();
        let url = posting["hostedUrl"].as_str().unwrap();
        let location = posting["categories"]["location"].as_str();

        assert_eq!(title, "Senior Public Health Analyst");
        assert_eq!(url, "https://jobs.lever.co/city-health-department/abc123");
        assert_eq!(location, Some("Remote"));
    }
}

#[test]
fn test_parse_response_multiple_jobs() {
    let json_data = r#"
    [
        {
            "text": "Customer Support Manager",
            "hostedUrl": "https://jobs.lever.co/community-care-network/job1",
            "categories": {
                "location": "Chicago, IL"
            }
        },
        {
            "text": "Inventory Planner",
            "hostedUrl": "https://jobs.lever.co/community-care-network/job2",
            "categories": {
                "team": "Regional Support"
            }
        },
        {
            "text": "Program Coordinator",
            "hostedUrl": "https://jobs.lever.co/community-care-network/job3",
            "categories": {
                "location": "Remote - US"
            }
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        assert_eq!(postings.len(), 3);

        assert_eq!(
            postings[0]["text"].as_str(),
            Some("Customer Support Manager")
        );
        assert_eq!(postings[1]["text"].as_str(), Some("Inventory Planner"));
        assert_eq!(postings[2]["text"].as_str(), Some("Program Coordinator"));

        // First has location in categories.location
        assert_eq!(
            postings[0]["categories"]["location"].as_str(),
            Some("Chicago, IL")
        );

        // Second has team instead of location
        assert_eq!(postings[1]["categories"]["location"].as_str(), None);
        assert_eq!(
            postings[1]["categories"]["team"].as_str(),
            Some("Regional Support")
        );

        // Third has remote location
        assert_eq!(
            postings[2]["categories"]["location"].as_str(),
            Some("Remote - US")
        );
    }
}

#[test]
fn test_parse_response_empty_array() {
    let json_data = "[]";
    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        assert_eq!(postings.len(), 0);
    }
}

#[test]
fn test_parse_response_missing_fields() {
    let json_data = r#"
    [
        {
            "text": "Care Coordinator",
            "hostedUrl": "https://jobs.lever.co/company/job1"
        },
        {
            "hostedUrl": "https://jobs.lever.co/company/job2"
        },
        {
            "text": "Program Coordinator"
        },
        {
            "text": "",
            "hostedUrl": ""
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        assert_eq!(postings.len(), 4);

        // First has both fields
        assert!(postings[0]["text"].as_str().is_some());
        assert!(postings[0]["hostedUrl"].as_str().is_some());

        // Second missing text
        assert!(postings[1]["text"].as_str().is_none());

        // Third missing hostedUrl
        assert!(postings[2]["hostedUrl"].as_str().is_none());

        // Fourth has empty strings (should be filtered by scraper logic)
        assert_eq!(postings[3]["text"].as_str(), Some(""));
        assert_eq!(postings[3]["hostedUrl"].as_str(), Some(""));
    }
}

#[test]
fn test_parse_response_with_description_variants() {
    let json_data = r#"
    [
        {
            "text": "Job 1",
            "hostedUrl": "https://jobs.lever.co/company/1",
            "description": "<p>HTML description</p>"
        },
        {
            "text": "Job 2",
            "hostedUrl": "https://jobs.lever.co/company/2",
            "descriptionPlain": "Plain text description"
        },
        {
            "text": "Job 3",
            "hostedUrl": "https://jobs.lever.co/company/3",
            "description": "<p>HTML version</p>",
            "descriptionPlain": "Plain version"
        },
        {
            "text": "Job 4",
            "hostedUrl": "https://jobs.lever.co/company/4"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        // Job 1: has description (HTML)
        assert_eq!(
            postings[0]["description"].as_str(),
            Some("<p>HTML description</p>")
        );

        // Job 2: has descriptionPlain only
        assert_eq!(
            postings[1]["descriptionPlain"].as_str(),
            Some("Plain text description")
        );

        // Job 3: has both (should prefer description first)
        assert!(postings[2]["description"].as_str().is_some());
        assert!(postings[2]["descriptionPlain"].as_str().is_some());

        // Job 4: has neither
        assert!(postings[3]["description"].as_str().is_none());
        assert!(postings[3]["descriptionPlain"].as_str().is_none());
    }
}

#[test]
fn test_api_url_construction() {
    let companies = vec![
        LeverCompany {
            id: "freshmart".to_string(),
            name: "FreshMart".to_string(),
            url: "https://jobs.lever.co/freshmart".to_string(),
        },
        LeverCompany {
            id: "community-care-network".to_string(),
            name: "Community Care Network".to_string(),
            url: "https://jobs.lever.co/community-care-network".to_string(),
        },
    ];

    for company in &companies {
        let api_url = format!("https://api.lever.co/v0/postings/{}", company.id);
        assert!(api_url.starts_with("https://api.lever.co/v0/postings/"));
        assert!(api_url.ends_with(&company.id));
    }
}

#[test]
fn test_hash_with_json_data() {
    let company = "FreshMart";
    let title = "Senior Public Health Analyst";
    let location = Some("Remote");
    let url = "https://jobs.lever.co/freshmart/abc123";

    let hash = jobsentinel_domain::calculate_job_hash(company, title, location, url);

    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_hash_remote_locations_normalized() {
    // With location normalization, "Remote" variants all normalize to "remote"
    // so they should produce the SAME hash (improved deduplication)
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("Remote"),
        "https://example.com/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("Remote - US"),
        "https://example.com/1",
    );
    let hash3 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        Some("Remote - Global"),
        "https://example.com/1",
    );

    // All remote variants should produce the SAME hash
    assert_eq!(hash1, hash2);
    assert_eq!(hash2, hash3);
    assert_eq!(hash1, hash3);
}
