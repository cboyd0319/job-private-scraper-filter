//! Proves Greenhouse API payloads normalize into bounded canonical job records.

use super::*;

#[test]
fn reviewed_fixture_exercises_the_production_parser() {
    let company = GreenhouseCompany {
        id: "example".to_string(),
        name: "Example".to_string(),
        url: "https://job-boards.greenhouse.io/example".to_string(),
    };
    let payload = include_str!("../../fixtures/greenhouse_list_v1.json");
    let json = serde_json::from_str(payload).unwrap();

    let jobs = GreenhouseScraper::parse_api_jobs(
        &json,
        &company,
        "https://boards-api.greenhouse.io/v1/boards/example/jobs",
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Inventory Planner");
    assert_eq!(
        jobs[0].url,
        "https://job-boards.greenhouse.io/example/jobs/123456"
    );
}

#[test]
fn provider_error_object_is_parser_drift_not_an_empty_success() {
    let company = GreenhouseCompany {
        id: "example".to_string(),
        name: "Example".to_string(),
        url: "https://job-boards.greenhouse.io/example".to_string(),
    };

    assert!(GreenhouseScraper::parse_api_jobs(
        &serde_json::json!({"error": "schema changed"}),
        &company,
        "https://boards-api.greenhouse.io/v1/boards/example/jobs",
    )
    .is_err());
    assert!(GreenhouseScraper::parse_api_jobs(
        &serde_json::json!({"jobs": []}),
        &company,
        "https://boards-api.greenhouse.io/v1/boards/example/jobs",
    )
    .unwrap()
    .is_empty());
}

#[test]
fn test_parse_api_response_single_job() {
    let json_data = r#"
    {
        "jobs": [
            {
                "id": 123456,
                "title": "Inventory Planner",
                "location": {
                    "name": "Remote"
                }
            }
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        assert_eq!(jobs_array.len(), 1);

        let job = &jobs_array[0];
        assert_eq!(job["id"].as_i64(), Some(123456));
        assert_eq!(job["title"].as_str(), Some("Inventory Planner"));
        assert_eq!(job["location"]["name"].as_str(), Some("Remote"));
    } else {
        panic!("jobs should be an array");
    }
}

#[test]
fn test_parse_api_response_multiple_jobs() {
    let json_data = r#"
    {
        "jobs": [
            {
                "id": 1,
                "title": "Care Coordinator",
                "location": {
                    "name": "Denver, CO"
                }
            },
            {
                "id": 2,
                "title": "Public Health Analyst",
                "location": {
                    "name": "Remote"
                }
            },
            {
                "id": 3,
                "title": "Customer Support Manager",
                "location": {
                    "name": "New York, NY"
                }
            }
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        assert_eq!(jobs_array.len(), 3);

        assert_eq!(jobs_array[0]["title"].as_str(), Some("Care Coordinator"));
        assert_eq!(
            jobs_array[1]["title"].as_str(),
            Some("Public Health Analyst")
        );
        assert_eq!(
            jobs_array[2]["title"].as_str(),
            Some("Customer Support Manager")
        );
    }
}

#[test]
fn test_parse_api_response_empty_jobs() {
    let json_data = r#"
    {
        "jobs": []
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        assert_eq!(jobs_array.len(), 0);
    }
}

#[test]
fn test_parse_api_response_missing_jobs_key() {
    let json_data = r#"
    {
        "error": "Not found"
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(_jobs_array) = parsed["jobs"].as_array() {
        panic!("jobs should not exist");
    }
}

#[test]
fn test_parse_api_response_missing_location() {
    let json_data = r#"
    {
        "jobs": [
            {
                "id": 123,
                "title": "Program Coordinator"
            }
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        let job = &jobs_array[0];

        let location = job["location"]["name"].as_str();
        assert_eq!(location, None);
    }
}

#[test]
fn test_parse_api_response_missing_title() {
    let json_data = r#"
    {
        "jobs": [
            {
                "id": 456,
                "location": {
                    "name": "Remote"
                }
            }
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        let job = &jobs_array[0];

        let title = job["title"].as_str().unwrap_or("");
        assert_eq!(title, "");
    }
}

#[test]
fn test_parse_api_response_missing_id() {
    let json_data = r#"
    {
        "jobs": [
            {
                "title": "Inventory Planner",
                "location": {
                    "name": "Remote"
                }
            }
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        let job = &jobs_array[0];

        let id = job["id"].as_i64().unwrap_or(0);
        assert_eq!(id, 0);
    }
}

#[test]
fn test_api_url_construction() {
    let company_id = "communitycarenetwork";
    let api_url = format!(
        "https://boards-api.greenhouse.io/v1/boards/{}/jobs",
        company_id
    );

    assert_eq!(
        api_url,
        "https://boards-api.greenhouse.io/v1/boards/communitycarenetwork/jobs"
    );
}

#[test]
fn test_api_url_from_company_url() {
    let company_url = "https://boards.greenhouse.io/communitycarenetwork";
    let company_id = company_url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap();

    assert_eq!(company_id, "communitycarenetwork");

    let api_url = format!(
        "https://boards-api.greenhouse.io/v1/boards/{}/jobs",
        company_id
    );
    assert_eq!(
        api_url,
        "https://boards-api.greenhouse.io/v1/boards/communitycarenetwork/jobs"
    );
}

#[test]
fn test_api_url_with_trailing_slash() {
    let company_url = "https://boards.greenhouse.io/freshmart/";
    let company_id = company_url
        .trim_end_matches('/')
        .split('/')
        .next_back()
        .unwrap();

    assert_eq!(company_id, "freshmart");
}

#[test]
fn test_job_url_construction_from_api() {
    let company_id = "cityhealthdepartment";
    let job_id = 987654;
    let url = format!(
        "https://job-boards.greenhouse.io/{}/jobs/{}",
        company_id, job_id
    );

    assert_eq!(
        url,
        "https://job-boards.greenhouse.io/cityhealthdepartment/jobs/987654"
    );
}

#[test]
fn test_api_uses_the_canonical_board_url_instead_of_an_unreviewed_employer_host() {
    let job_data: serde_json::Value = serde_json::json!({
        "id": 7687193003_i64,
        "title": "Account Executive, Commercial",
        "absolute_url": "https://www.fivetran.com/careers/job?gh_jid=7687193003",
        "location": {
            "name": "Denver, Colorado, United States"
        }
    });

    let company_id = "fivetran";
    let job_id = job_data["id"].as_i64().unwrap_or(0);
    let url = GreenhouseScraper::api_job_url(company_id, job_id, &job_data);

    assert_eq!(
        url,
        "https://job-boards.greenhouse.io/fivetran/jobs/7687193003"
    );
}

#[test]
fn test_hash_consistency_across_runs() {
    let company = "Community Care Network™";
    let title = "Senior Care Coordinator (Remote) 🚀";
    let location = Some("Denver, CO");
    let url = "https://boards.greenhouse.io/test/jobs/123";

    let hashes: Vec<String> = (0..10)
        .map(|_| jobsentinel_domain::calculate_job_hash(company, title, location, url))
        .collect();

    for i in 1..hashes.len() {
        assert_eq!(hashes[0], hashes[i]);
    }
}

#[test]
fn test_hash_with_query_parameters_normalized() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Program Coordinator",
        None,
        "https://boards.greenhouse.io/company/jobs/1?ref=linkedin",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Program Coordinator",
        None,
        "https://boards.greenhouse.io/company/jobs/1?ref=twitter",
    );
    let hash3 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Program Coordinator",
        None,
        "https://boards.greenhouse.io/company/jobs/1",
    );

    assert_eq!(hash1, hash2);
    assert_eq!(hash1, hash3);
    assert_eq!(hash2, hash3);
}

#[test]
fn test_parse_api_response_with_capacity() {
    let json_data = r#"
    {
        "jobs": [
            {"id": 1, "title": "Job 1"},
            {"id": 2, "title": "Job 2"},
            {"id": 3, "title": "Job 3"}
        ]
    }
    "#;

    let parsed: serde_json::Value = serde_json::from_str(json_data).unwrap();

    if let Some(jobs_array) = parsed["jobs"].as_array() {
        let mut jobs = Vec::with_capacity(jobs_array.len());

        for job_data in jobs_array {
            let title = job_data["title"].as_str().unwrap_or("").to_string();
            jobs.push(title);
        }

        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs.capacity(), 3);
    }
}
