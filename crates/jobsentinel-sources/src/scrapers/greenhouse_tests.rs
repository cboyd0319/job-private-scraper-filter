use super::*;

#[path = "greenhouse_tests/api_tests.rs"]
mod api_tests;

#[test]
fn test_scraper_name() {
    let scraper = GreenhouseScraper::new(vec![]);
    assert_eq!(scraper.name(), "greenhouse");
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
        GreenhouseCompany {
            id: "communitycarenetwork".to_string(),
            name: "Community Care Network".to_string(),
            url: "https://boards.greenhouse.io/communitycarenetwork".to_string(),
        },
        GreenhouseCompany {
            id: "freshmart".to_string(),
            name: "FreshMart".to_string(),
            url: "https://boards.greenhouse.io/freshmart".to_string(),
        },
    ];

    let scraper = GreenhouseScraper::new(companies.clone());

    assert_eq!(scraper.companies.len(), 2);
    assert_eq!(scraper.companies[0].name, "Community Care Network");
    assert_eq!(scraper.companies[1].name, "FreshMart");
}

#[test]
fn test_new_scraper_empty() {
    let scraper = GreenhouseScraper::new(vec![]);
    assert_eq!(scraper.companies.len(), 0);
}

#[tokio::test]
async fn test_scrape_reports_error_when_all_companies_fail() {
    let scraper = GreenhouseScraper::new(vec![GreenhouseCompany {
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
        } if scraper == "greenhouse" && message.contains("All configured company boards failed")
    ));
}

#[test]
fn test_scraper_initialization() {
    let companies = vec![
        GreenhouseCompany {
            id: "communitycarenetwork".to_string(),
            name: "Community Care Network".to_string(),
            url: "https://boards.greenhouse.io/communitycarenetwork".to_string(),
        },
        GreenhouseCompany {
            id: "freshmart".to_string(),
            name: "FreshMart".to_string(),
            url: "https://boards.greenhouse.io/freshmart".to_string(),
        },
    ];

    let scraper = GreenhouseScraper::new(companies.clone());

    assert_eq!(scraper.companies.len(), 2);
    assert_eq!(scraper.companies[0].id, "communitycarenetwork");
    assert_eq!(scraper.companies[1].id, "freshmart");
}

#[test]
fn test_company_struct_clone() {
    let company = GreenhouseCompany {
        id: "test-id".to_string(),
        name: "Test Company".to_string(),
        url: "https://boards.greenhouse.io/test".to_string(),
    };

    let cloned = company.clone();

    assert_eq!(company.id, cloned.id);
    assert_eq!(company.name, cloned.name);
    assert_eq!(company.url, cloned.url);
}

#[test]
fn test_company_struct_debug() {
    let company = GreenhouseCompany {
        id: "debug-test".to_string(),
        name: "Debug Test Company".to_string(),
        url: "https://boards.greenhouse.io/debug".to_string(),
    };

    let debug_str = format!("{:?}", company);
    assert!(debug_str.contains("debug-test"));
    assert!(debug_str.contains("Debug Test Company"));
}
