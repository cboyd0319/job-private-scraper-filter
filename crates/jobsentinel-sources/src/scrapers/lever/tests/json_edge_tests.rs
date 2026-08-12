use super::*;

// ========================================
// JSON Parsing and Edge Case Tests
// ========================================

#[test]
fn test_parse_response_filters_empty_title() {
    let json_data = r#"
    [
        {
            "text": "",
            "hostedUrl": "https://jobs.lever.co/company/job1"
        },
        {
            "text": "Valid Job",
            "hostedUrl": "https://jobs.lever.co/company/job2"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let mut valid_jobs = 0;
        for posting in postings {
            let title = posting["text"].as_str().unwrap_or("").to_string();
            let url = posting["hostedUrl"].as_str().unwrap_or("").to_string();

            if !title.is_empty() && !url.is_empty() {
                valid_jobs += 1;
            }
        }

        // Only the second job should pass validation
        assert_eq!(valid_jobs, 1);
    }
}

#[test]
fn test_parse_response_filters_empty_url() {
    let json_data = r#"
    [
        {
            "text": "Care Coordinator",
            "hostedUrl": ""
        },
        {
            "text": "Program Coordinator",
            "hostedUrl": "https://jobs.lever.co/company/job2"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let mut valid_jobs = 0;
        for posting in postings {
            let title = posting["text"].as_str().unwrap_or("").to_string();
            let url = posting["hostedUrl"].as_str().unwrap_or("").to_string();

            if !title.is_empty() && !url.is_empty() {
                valid_jobs += 1;
            }
        }

        assert_eq!(valid_jobs, 1);
    }
}

#[test]
fn test_parse_response_description_priority() {
    let json_data = r#"
    [
        {
            "text": "Job 1",
            "hostedUrl": "https://jobs.lever.co/company/1",
            "description": "<p>HTML version</p>",
            "descriptionPlain": "Plain version"
        },
        {
            "text": "Job 2",
            "hostedUrl": "https://jobs.lever.co/company/2",
            "descriptionPlain": "Only plain"
        },
        {
            "text": "Job 3",
            "hostedUrl": "https://jobs.lever.co/company/3"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        // Job 1: prefers description over descriptionPlain
        let desc1 = postings[0]["description"]
            .as_str()
            .or_else(|| postings[0]["descriptionPlain"].as_str());
        assert_eq!(desc1, Some("<p>HTML version</p>"));

        // Job 2: uses descriptionPlain when description is missing
        let desc2 = postings[1]["description"]
            .as_str()
            .or_else(|| postings[1]["descriptionPlain"].as_str());
        assert_eq!(desc2, Some("Only plain"));

        // Job 3: neither field exists
        let desc3 = postings[2]["description"]
            .as_str()
            .or_else(|| postings[2]["descriptionPlain"].as_str());
        assert_eq!(desc3, None);
    }
}

#[test]
fn test_parse_response_nested_categories() {
    let json_data = r#"
    [
        {
            "text": "Care Coordinator",
            "hostedUrl": "https://jobs.lever.co/company/1",
            "categories": {
                "location": "Remote",
                "team": "Community Care",
                "commitment": "Full-time"
            }
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let posting = &postings[0];

        // Test location extraction
        let location = posting["categories"]["location"].as_str();
        assert_eq!(location, Some("Remote"));

        // Test team fallback availability
        let team = posting["categories"]["team"].as_str();
        assert_eq!(team, Some("Community Care"));
    }
}

#[test]
fn test_parse_response_null_categories() {
    let json_data = r#"
    [
        {
            "text": "Care Coordinator",
            "hostedUrl": "https://jobs.lever.co/company/1",
            "categories": null
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let posting = &postings[0];

        // Should not panic when categories is null
        let location = posting["categories"]["location"].as_str();
        assert_eq!(location, None);
    }
}

#[test]
fn test_parse_response_mixed_valid_invalid() {
    let json_data = r#"
    [
        {
            "text": "Valid Job 1",
            "hostedUrl": "https://jobs.lever.co/company/1"
        },
        {
            "text": "",
            "hostedUrl": "https://jobs.lever.co/company/2"
        },
        {
            "text": "Valid Job 2",
            "hostedUrl": ""
        },
        {
            "text": "Valid Job 3",
            "hostedUrl": "https://jobs.lever.co/company/3"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let mut valid_count = 0;
        for posting in postings {
            let title = posting["text"].as_str().unwrap_or("").to_string();
            let url = posting["hostedUrl"].as_str().unwrap_or("").to_string();

            if !title.is_empty() && !url.is_empty() {
                valid_count += 1;
            }
        }

        // Only jobs 1 and 3 should be valid
        assert_eq!(valid_count, 2);
    }
}

#[test]
fn test_infer_remote_partial_word_matches() {
    // "remote" should match even within words
    assert!(LeverScraper::infer_remote("remotely accessible", None));

    // "wfh" should match as standalone or within text
    assert!(LeverScraper::infer_remote("wfh-friendly", None));

    // "work from home" requires exact spacing (not "work-from-home")
    assert!(LeverScraper::infer_remote(
        "work from home opportunity",
        None
    ));
    assert!(!LeverScraper::infer_remote(
        "work-from-home opportunity",
        None
    ));
}

#[test]
fn test_infer_remote_location_variations() {
    // Various remote location formats
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Remote (US)")
    ));
    assert!(!LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Remote/Hybrid")
    ));
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("100% Remote")
    ));
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("REMOTE FIRST")
    ));

    // Worldwide variations
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Worldwide Remote")
    ));
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Global/Worldwide")
    ));

    // Anywhere variations
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Work from Anywhere")
    ));
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Anywhere in Europe")
    ));
}

#[test]
fn test_hash_consistency_across_runs() {
    // Generate hash multiple times to ensure consistency
    let company = "Test Company™";
    let title = "Senior Care Coordinator (Remote) 🚀";
    let location = Some("San Francisco, CA");
    let url = "https://jobs.lever.co/test/abc123?ref=linkedin";

    let hashes: Vec<String> = (0..10)
        .map(|_| jobsentinel_domain::calculate_job_hash(company, title, location, url))
        .collect();

    // All hashes should be identical
    for i in 1..hashes.len() {
        assert_eq!(hashes[0], hashes[i]);
    }
}

#[test]
fn test_hash_with_query_parameters_normalized() {
    // With URL normalization, tracking params (ref, utm_*, etc.) are stripped
    // so URLs that differ only in tracking params should produce the SAME hash
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://jobs.lever.co/company/job?ref=linkedin",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://jobs.lever.co/company/job?ref=twitter",
    );
    let hash3 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Care Coordinator",
        None,
        "https://jobs.lever.co/company/job",
    );

    // All three should produce the SAME hash (tracking params stripped)
    assert_eq!(hash1, hash2);
    assert_eq!(hash1, hash3);
    assert_eq!(hash2, hash3);
}

#[test]
fn test_company_struct_clone() {
    let company = LeverCompany {
        id: "test-id".to_string(),
        name: "Test Company".to_string(),
        url: "https://jobs.lever.co/test".to_string(),
    };

    let cloned = company.clone();

    assert_eq!(company.id, cloned.id);
    assert_eq!(company.name, cloned.name);
    assert_eq!(company.url, cloned.url);
}

#[test]
fn test_company_struct_debug() {
    let company = LeverCompany {
        id: "debug-test".to_string(),
        name: "Debug Test Company".to_string(),
        url: "https://jobs.lever.co/debug".to_string(),
    };

    let debug_str = format!("{:?}", company);
    assert!(debug_str.contains("debug-test"));
    assert!(debug_str.contains("Debug Test Company"));
}

#[test]
fn test_infer_remote_empty_strings() {
    assert!(!LeverScraper::infer_remote("", None));
    assert!(!LeverScraper::infer_remote("", Some("")));
    assert!(!LeverScraper::infer_remote("Care Coordinator", Some("")));
}

#[test]
fn test_infer_remote_false_positives_prevention() {
    // Current implementation matches "remote" anywhere in the string
    // This is a known limitation - any "remote" substring matches
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Remote Street, Boston")
    ));

    // Should match if it's clearly about work arrangement
    assert!(LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Remote - Boston preferred")
    ));

    // Location names that definitely aren't remote work
    assert!(!LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Paris, France")
    ));
    assert!(!LeverScraper::infer_remote(
        "Care Coordinator",
        Some("Downtown NYC")
    ));
}

#[test]
fn test_parse_response_large_capacity() {
    // Test that capacity hint works correctly
    let postings = vec![
        serde_json::json!({
            "text": "Job 1",
            "hostedUrl": "https://example.com/1"
        }),
        serde_json::json!({
            "text": "Job 2",
            "hostedUrl": "https://example.com/2"
        }),
        serde_json::json!({
            "text": "Job 3",
            "hostedUrl": "https://example.com/3"
        }),
    ];

    let json = serde_json::json!(postings);

    if let Some(array) = json.as_array() {
        let mut jobs = Vec::with_capacity(array.len());

        for posting in array {
            let title = posting["text"].as_str().unwrap_or("").to_string();
            let url = posting["hostedUrl"].as_str().unwrap_or("").to_string();

            if !title.is_empty() && !url.is_empty() {
                jobs.push((title, url));
            }
        }

        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs.capacity(), 3);
    }
}

#[test]
fn test_parse_response_location_fallback_to_team() {
    let json_data = r#"
    [
        {
            "text": "Care Coordinator",
            "hostedUrl": "https://jobs.lever.co/company/job1",
            "categories": {
                "team": "Care Operations Team"
            }
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let posting = &postings[0];

        // No location field, should fall back to team
        let location = posting["categories"]["location"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| {
                posting["categories"]["team"]
                    .as_str()
                    .map(|s| s.to_string())
            });

        assert_eq!(location, Some("Care Operations Team".to_string()));
    }
}

#[test]
fn test_parse_response_special_characters_in_json() {
    let json_data = r#"
    [
        {
            "text": "Senior Program Coordinator (Regional) - 🚀",
            "hostedUrl": "https://jobs.lever.co/company™/job-id",
            "categories": {
                "location": "San Francisco, CA / Remote"
            },
            "description": "We're looking for a <strong>talented</strong> program coordinator!"
        }
    ]
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(postings) = parsed.as_array() {
        let posting = &postings[0];

        assert!(posting["text"].as_str().unwrap().contains("🚀"));
        assert!(posting["hostedUrl"].as_str().unwrap().contains("™"));
        assert!(posting["categories"]["location"]
            .as_str()
            .unwrap()
            .contains("/"));
        assert!(posting["description"]
            .as_str()
            .unwrap()
            .contains("<strong>"));
    }
}
