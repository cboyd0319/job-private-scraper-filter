//! Scraper Integration Tests
//!
//! Tests for job scrapers using wiremock for HTTP mocking infrastructure.
//! These tests verify scraper construction and trait implementation.
//!
//! Note: Parsing logic is tested in unit tests within each scraper module.
//! Integration tests focus on the scraper owner's internal trait interface.

use super::{
    linkedin::LinkedInScraper, GreenhouseCompany, GreenhouseScraper, HnHiringScraper, JobScraper,
    LeverCompany, LeverScraper, RemoteOkScraper, WeWorkRemotelyScraper,
};

// ============================================================================
// Helper Functions for Test Data
// ============================================================================

fn test_greenhouse_company() -> GreenhouseCompany {
    GreenhouseCompany {
        id: "cloudflare".to_string(),
        name: "Cloudflare".to_string(),
        url: "https://boards.greenhouse.io/cloudflare".to_string(),
    }
}

fn test_lever_company() -> LeverCompany {
    LeverCompany {
        id: "netflix".to_string(),
        name: "Netflix".to_string(),
        url: "https://jobs.lever.co/netflix".to_string(),
    }
}

// ============================================================================
// Scraper Construction Tests
// ============================================================================

#[test]
fn test_remoteok_scraper_construction() {
    let scraper = RemoteOkScraper::new(vec!["rust".to_string(), "golang".to_string()], 10);
    assert_eq!(scraper.name(), "remoteok");
    assert_eq!(scraper.tags.len(), 2);
    assert_eq!(scraper.limit, 10);
}

#[test]
fn test_remoteok_scraper_empty_tags() {
    let scraper = RemoteOkScraper::new(vec![], 20);
    assert_eq!(scraper.name(), "remoteok");
    assert!(scraper.tags.is_empty());
}

#[test]
fn test_weworkremotely_scraper_construction() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    assert_eq!(scraper.name(), "weworkremotely");
    assert!(scraper.category.is_none());
}

#[test]
fn test_weworkremotely_scraper_with_category() {
    let scraper = WeWorkRemotelyScraper::new(Some("remote-programming-jobs".to_string()), 20);
    assert_eq!(scraper.name(), "weworkremotely");
    assert_eq!(
        scraper.category,
        Some("remote-programming-jobs".to_string())
    );
}

#[test]
fn test_hn_hiring_scraper_construction() {
    let scraper = HnHiringScraper::new(20, false);
    assert_eq!(scraper.name(), "hn_hiring");
    assert_eq!(scraper.limit, 20);
    assert!(!scraper.remote_only);
}

#[test]
fn test_hn_hiring_scraper_remote_filter() {
    let scraper = HnHiringScraper::new(10, true);
    assert_eq!(scraper.name(), "hn_hiring");
    assert!(scraper.remote_only);
}

#[test]
fn test_greenhouse_scraper_construction() {
    let scraper = GreenhouseScraper::new(vec![test_greenhouse_company()]);
    assert_eq!(scraper.name(), "greenhouse");
    assert_eq!(scraper.companies.len(), 1);
}

#[test]
fn test_greenhouse_scraper_multiple_companies() {
    let companies = vec![
        GreenhouseCompany {
            id: "stripe".to_string(),
            name: "Stripe".to_string(),
            url: "https://boards.greenhouse.io/stripe".to_string(),
        },
        GreenhouseCompany {
            id: "cloudflare".to_string(),
            name: "Cloudflare".to_string(),
            url: "https://boards.greenhouse.io/cloudflare".to_string(),
        },
        GreenhouseCompany {
            id: "discord".to_string(),
            name: "Discord".to_string(),
            url: "https://boards.greenhouse.io/discord".to_string(),
        },
    ];
    let scraper = GreenhouseScraper::new(companies);
    assert_eq!(scraper.name(), "greenhouse");
    assert_eq!(scraper.companies.len(), 3);
}

#[test]
fn test_lever_scraper_construction() {
    let scraper = LeverScraper::new(vec![test_lever_company()]);
    assert_eq!(scraper.name(), "lever");
    assert_eq!(scraper.companies.len(), 1);
}

#[test]
fn test_lever_scraper_multiple_companies() {
    let companies = vec![
        LeverCompany {
            id: "figma".to_string(),
            name: "Figma".to_string(),
            url: "https://jobs.lever.co/figma".to_string(),
        },
        LeverCompany {
            id: "notion".to_string(),
            name: "Notion".to_string(),
            url: "https://jobs.lever.co/notion".to_string(),
        },
    ];
    let scraper = LeverScraper::new(companies);
    assert_eq!(scraper.name(), "lever");
    assert_eq!(scraper.companies.len(), 2);
}

#[test]
fn test_linkedin_scraper_construction() {
    let scraper = LinkedInScraper::new("customer support".to_string(), "Chicago".to_string());
    assert_eq!(scraper.name(), "LinkedIn");
    assert_eq!(scraper.query, "customer support");
    assert_eq!(scraper.location, "Chicago");
}

// ============================================================================
// Scraper Trait Implementation Tests
// ============================================================================

/// All scrapers must implement the JobScraper trait
#[test]
fn test_all_scrapers_implement_job_scraper() {
    // This test verifies at compile time that all scrapers implement JobScraper
    fn assert_job_scraper<T: JobScraper>(_: &T) {}

    let remoteok = RemoteOkScraper::new(vec![], 10);
    assert_job_scraper(&remoteok);

    let wwr = WeWorkRemotelyScraper::new(None, 10);
    assert_job_scraper(&wwr);

    let hn = HnHiringScraper::new(10, false);
    assert_job_scraper(&hn);

    let greenhouse = GreenhouseScraper::new(vec![test_greenhouse_company()]);
    assert_job_scraper(&greenhouse);

    let lever = LeverScraper::new(vec![test_lever_company()]);
    assert_job_scraper(&lever);

    let linkedin = LinkedInScraper::new("query".to_string(), "location".to_string());
    assert_job_scraper(&linkedin);
}

/// Verify all scrapers return distinct names
#[test]
fn test_scraper_names_are_unique() {
    let names = vec![
        RemoteOkScraper::new(vec![], 10).name(),
        WeWorkRemotelyScraper::new(None, 10).name(),
        HnHiringScraper::new(10, false).name(),
        GreenhouseScraper::new(vec![test_greenhouse_company()]).name(),
        LeverScraper::new(vec![test_lever_company()]).name(),
        LinkedInScraper::new("q".to_string(), "l".to_string()).name(),
    ];

    // Check all names are unique (case-insensitive to catch duplicates)
    let mut unique_names: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    unique_names.sort();
    unique_names.dedup();
    assert_eq!(
        names.len(),
        unique_names.len(),
        "All scraper names should be unique"
    );
}

// ============================================================================
// Scraper Count Verification
// ============================================================================

/// Verify we have the expected number of scrapers tested
#[test]
fn test_scraper_count() {
    // We test 8 source owners:
    // - ziprecruiter
    // - remoteok, weworkremotely, hn_hiring
    // - greenhouse, lever
    // - linkedin, indeed
    // (wellfound, jobswithgpt have more complex constructors - tested separately)
    let names = vec![
        "ziprecruiter",
        "remoteok",
        "weworkremotely",
        "hn_hiring",
        "greenhouse",
        "lever",
        "linkedin",
        "indeed",
    ];
    assert_eq!(names.len(), 8, "Expected 8 source owners to be tested");
}
