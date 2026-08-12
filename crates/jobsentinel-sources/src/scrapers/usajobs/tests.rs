use super::*;
use jobsentinel_network::send_test_http_text_with_retry;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LIST_FIXTURE: &str = include_str!("../../fixtures/usajobs_list_v1.json");
const DETAIL_FIXTURE: &str = include_str!("../../fixtures/usajobs_detail_v1.json");

#[test]
fn test_new_scraper() {
    let scraper = UsaJobsScraper::new("test-api-key".to_string(), "test@example.com".to_string());

    assert_eq!(scraper.api_key, "test-api-key");
    assert_eq!(scraper.email, "test@example.com");
    assert_eq!(scraper.limit, 100);
    assert_eq!(scraper.request_limit_per_hour, 60);
    assert_eq!(scraper.date_posted_days, Some(30));
    assert!(!scraper.remote_only);
}

#[test]
fn test_debug_does_not_leak_api_key_or_email() {
    let scraper = UsaJobsScraper::new(
        "usajobs-secret-api-key".to_string(),
        "user@example.com".to_string(),
    )
    .with_keywords("private search")
    .with_location("Private City", Some(25));

    let debug_output = format!("{:?}", scraper);

    assert!(
        !debug_output.contains("usajobs-secret-api-key"),
        "USAJobs scraper Debug output must not contain API key. Got: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("user@example.com"),
        "USAJobs scraper Debug output must not contain email. Got: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("private search"),
        "USAJobs scraper Debug output must not contain query. Got: {}",
        debug_output
    );
    assert!(
        !debug_output.contains("Private City"),
        "USAJobs scraper Debug output must not contain location. Got: {}",
        debug_output
    );
}

#[tokio::test]
async fn test_client_does_not_follow_redirects_with_authorization_key() {
    let target = MockServer::start().await;
    let source = MockServer::start().await;
    let location = format!("{}/capture", target.uri());

    Mock::given(method("GET"))
        .and(path("/api/Search"))
        .respond_with(ResponseTemplate::new(302).insert_header("Location", location))
        .mount(&source)
        .await;

    let scraper = UsaJobsScraper::new("secret-api-key".to_string(), "user@example.com".to_string());
    let url = format!("{}/api/Search", source.uri());
    let request = scraper
        .build_request(&url, Vec::new())
        .expect("request should build");
    let response = send_test_http_text_with_retry(request)
        .await
        .expect("request should complete");

    assert_eq!(response.status, 302);
    assert!(
        target.received_requests().await.unwrap().is_empty(),
        "USAJobs client must not forward Authorization-Key across redirects"
    );
}

#[tokio::test]
async fn usajobs_request_does_not_retry_rate_limits_or_server_errors() {
    for status in [429, 500] {
        let source = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/Search"))
            .respond_with(ResponseTemplate::new(status).insert_header("Retry-After", "0"))
            .expect(1)
            .mount(&source)
            .await;

        let scraper = UsaJobsScraper::new("secret-api-key", "user@example.com");
        let response = send_test_http_text_with_retry(
            scraper
                .build_request(&format!("{}/api/Search", source.uri()), Vec::new())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status, status);
        source.verify().await;
    }
}

#[test]
fn test_builder_methods() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string())
        .with_keywords("public health analyst")
        .with_location("Washington, DC", Some(25))
        .remote_only()
        .with_pay_grade(Some(11), Some(14))
        .posted_within_days(7)
        .with_limit(50)
        .with_request_limit_per_hour(NonZeroU16::new(24).unwrap());

    assert_eq!(scraper.keywords, Some("public health analyst".to_string()));
    assert_eq!(scraper.location, Some("Washington, DC".to_string()));
    assert_eq!(scraper.radius, Some(25));
    assert!(scraper.remote_only);
    assert_eq!(scraper.pay_grade_min, Some(11));
    assert_eq!(scraper.pay_grade_max, Some(14));
    assert_eq!(scraper.date_posted_days, Some(7));
    assert_eq!(scraper.limit, 50);
    assert_eq!(scraper.request_limit_per_hour, 24);
}

#[test]
fn reviewed_list_and_detail_fixtures_parse_to_the_same_canonical_job() {
    let response: UsaJobsResponse = serde_json::from_str(LIST_FIXTURE).unwrap();
    let detail: SearchResultItem = serde_json::from_str(DETAIL_FIXTURE).unwrap();
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    assert_eq!(response.search_result.search_result_count, 1);
    assert_eq!(response.search_result.search_result_count_all, 1);
    let list_job = scraper
        .parse_job(&response.search_result.search_result_items[0])
        .unwrap();
    let detail_job = scraper.parse_job(&detail).unwrap();

    assert_eq!(list_job.title, "Public Service Program Analyst");
    assert_eq!(list_job.company, "Department of Example Services");
    assert_eq!(list_job.url, "https://www.usajobs.gov/job/123456");
    assert_eq!(list_job.salary_min, Some(80_000));
    assert_eq!(list_job.salary_max, Some(105_000));
    assert_eq!(list_job.title, detail_job.title);
    assert_eq!(list_job.company, detail_job.company);
    assert_eq!(list_job.url, detail_job.url);
}

#[test]
fn test_date_posted_capped_at_60() {
    let scraper =
        UsaJobsScraper::new("key".to_string(), "email".to_string()).posted_within_days(100);

    assert_eq!(scraper.date_posted_days, Some(60));
}

#[test]
fn test_build_query_params_minimal() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());
    let params = scraper.build_query_params(1);

    // Should have DatePosted (default 30), Page, ResultsPerPage, Fields
    assert!(params.iter().any(|(k, v)| *k == "DatePosted" && v == "30"));
    assert!(params.iter().any(|(k, v)| *k == "Page" && v == "1"));
    assert!(params
        .iter()
        .any(|(k, v)| *k == "ResultsPerPage" && v == "100"));
    assert!(params.iter().any(|(k, v)| *k == "Fields" && v == "Full"));
}

#[test]
fn test_build_query_params_full() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string())
        .with_keywords("program coordinator")
        .with_location("Denver, CO", Some(50))
        .remote_only()
        .with_pay_grade(Some(12), Some(15))
        .posted_within_days(14);

    let params = scraper.build_query_params(2);

    assert!(params
        .iter()
        .any(|(k, v)| *k == "Keyword" && v == "program coordinator"));
    assert!(params
        .iter()
        .any(|(k, v)| *k == "LocationName" && v == "Denver, CO"));
    assert!(params.iter().any(|(k, v)| *k == "Radius" && v == "50"));
    assert!(params
        .iter()
        .any(|(k, v)| *k == "RemoteIndicator" && v == "true"));
    assert!(params.iter().any(|(k, v)| *k == "PayGradeLow" && v == "12"));
    assert!(params
        .iter()
        .any(|(k, v)| *k == "PayGradeHigh" && v == "15"));
    assert!(params.iter().any(|(k, v)| *k == "DatePosted" && v == "14"));
    assert!(params.iter().any(|(k, v)| *k == "Page" && v == "2"));
}

#[test]
fn test_compute_hash_deterministic() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Department of Health and Human Services",
        "Public Health Analyst",
        Some("Washington, DC"),
        "https://www.usajobs.gov/job/123456",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Department of Health and Human Services",
        "Public Health Analyst",
        Some("Washington, DC"),
        "https://www.usajobs.gov/job/123456",
    );

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
}

#[test]
fn test_compute_hash_unique() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "City Health Department",
        "Care Coordinator",
        Some("DC"),
        "https://usajobs.gov/1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "City Health Department",
        "Public Health Analyst",
        Some("DC"),
        "https://usajobs.gov/2",
    );

    assert_ne!(hash1, hash2);
}

#[test]
fn test_parse_salary_annual() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let remunerations = vec![Remuneration {
        minimum_range: "105029".to_string(),
        maximum_range: "136553".to_string(),
        rate_interval_code: "PA".to_string(),
    }];

    let (min, max) = scraper.parse_salary(&remunerations);
    assert_eq!(min, Some(105029));
    assert_eq!(max, Some(136553));
}

#[test]
fn test_parse_salary_hourly_converts_to_annual() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let remunerations = vec![Remuneration {
        minimum_range: "50".to_string(),
        maximum_range: "75".to_string(),
        rate_interval_code: "PH".to_string(),
    }];

    let (min, max) = scraper.parse_salary(&remunerations);
    // 50 * 2080 = 104000, 75 * 2080 = 156000
    assert_eq!(min, Some(104000));
    assert_eq!(max, Some(156000));
}

#[test]
fn test_parse_salary_empty() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let remunerations: Vec<Remuneration> = vec![];
    let (min, max) = scraper.parse_salary(&remunerations);

    assert_eq!(min, None);
    assert_eq!(max, None);
}

#[test]
fn test_parse_salary_invalid_numbers() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let remunerations = vec![Remuneration {
        minimum_range: "not-a-number".to_string(),
        maximum_range: "also-not".to_string(),
        rate_interval_code: "PA".to_string(),
    }];

    let (min, max) = scraper.parse_salary(&remunerations);
    assert_eq!(min, None);
    assert_eq!(max, None);
}

#[test]
fn test_scraper_name() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());
    assert_eq!(scraper.name(), "usajobs");
}

#[test]
fn test_api_error_message_does_not_echo_response_body() {
    let body = r#"{"error":"No results for Hidden Program Coordinator Role in Private City, CO"}"#;
    let message = UsaJobsScraper::api_error_message(400, Some(body.chars().count()));

    assert_eq!(message, "USAJobs API error: 400 (response_body_chars: 78)");
    assert!(!message.contains("Hidden Program Coordinator Role"));
    assert!(!message.contains("Private City"));
}

#[test]
fn test_api_error_message_handles_unavailable_body() {
    assert_eq!(
        UsaJobsScraper::api_error_message(503, None),
        "USAJobs API error: 503 (response_body_unavailable)"
    );
}

#[test]
fn test_parse_job_complete() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let item = SearchResultItem {
        matched_object_id: "123456".to_string(),
        matched_object_descriptor: JobDescriptor {
            position_title: "Public Health Analyst".to_string(),
            position_uri: "https://www.usajobs.gov/job/123456".to_string(),
            position_location_display: "Washington, DC".to_string(),
            organization_name: "Centers for Medicare & Medicaid Services".to_string(),
            department_name: "Department of Health and Human Services".to_string(),
            position_remuneration: vec![Remuneration {
                minimum_range: "100000".to_string(),
                maximum_range: "130000".to_string(),
                rate_interval_code: "PA".to_string(),
            }],
            user_area: Some(UserArea {
                details: Some(JobDetails {
                    job_summary: Some("Support community health programs".to_string()),
                    major_duties: None,
                }),
            }),
        },
    };

    let job = scraper.parse_job(&item).unwrap();

    assert_eq!(job.title, "Public Health Analyst");
    assert_eq!(job.company, "Centers for Medicare & Medicaid Services");
    assert_eq!(job.url, "https://www.usajobs.gov/job/123456");
    assert_eq!(job.location, Some("Washington, DC".to_string()));
    assert_eq!(job.salary_min, Some(100000));
    assert_eq!(job.salary_max, Some(130000));
    assert_eq!(
        job.description,
        Some("Support community health programs".to_string())
    );
    assert_eq!(job.source, "usajobs");
    assert_eq!(job.currency, Some("USD".to_string()));
}

#[test]
fn test_parse_job_remote_detection() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let item = SearchResultItem {
        matched_object_id: "789".to_string(),
        matched_object_descriptor: JobDescriptor {
            position_title: "Remote Program Coordinator".to_string(),
            position_uri: "https://www.usajobs.gov/job/789".to_string(),
            position_location_display: "Anywhere in the U.S. (remote job)".to_string(),
            organization_name: "Administration for Community Living".to_string(),
            department_name: "Department of Health and Human Services".to_string(),
            position_remuneration: vec![],
            user_area: None,
        },
    };

    let job = scraper.parse_job(&item).unwrap();
    assert_eq!(job.remote, Some(true));
}

#[test]
fn test_parse_job_fallback_to_department() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let item = SearchResultItem {
        matched_object_id: "456".to_string(),
        matched_object_descriptor: JobDescriptor {
            position_title: "Analyst".to_string(),
            position_uri: "https://www.usajobs.gov/job/456".to_string(),
            position_location_display: "Denver, CO".to_string(),
            organization_name: "".to_string(), // Empty org name
            department_name: "Department of Veterans Affairs".to_string(),
            position_remuneration: vec![],
            user_area: None,
        },
    };

    let job = scraper.parse_job(&item).unwrap();
    assert_eq!(job.company, "Department of Veterans Affairs");
}

#[test]
fn test_parse_job_missing_title_returns_none() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let item = SearchResultItem {
        matched_object_id: "999".to_string(),
        matched_object_descriptor: JobDescriptor {
            position_title: "".to_string(),
            position_uri: "https://www.usajobs.gov/job/999".to_string(),
            position_location_display: "DC".to_string(),
            organization_name: "Test".to_string(),
            department_name: "Test".to_string(),
            position_remuneration: vec![],
            user_area: None,
        },
    };

    assert!(scraper.parse_job(&item).is_none());
}

#[test]
fn test_parse_job_missing_url_returns_none() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string());

    let item = SearchResultItem {
        matched_object_id: "888".to_string(),
        matched_object_descriptor: JobDescriptor {
            position_title: "Inventory Planner".to_string(),
            position_uri: "".to_string(),
            position_location_display: "DC".to_string(),
            organization_name: "Test".to_string(),
            department_name: "Test".to_string(),
            position_remuneration: vec![],
            user_area: None,
        },
    };

    assert!(scraper.parse_job(&item).is_none());
}

#[test]
fn test_results_per_page_capped() {
    let scraper = UsaJobsScraper::new("key".to_string(), "email".to_string()).with_limit(1000); // Over the max

    let params = scraper.build_query_params(1);

    // Should be capped at MAX_RESULTS_PER_PAGE (500)
    let results_param = params.iter().find(|(k, _)| *k == "ResultsPerPage");
    assert!(results_param.is_some());
    let (_, value) = results_param.unwrap();
    assert_eq!(value, "500");
}
