//! Live Scraper Tests - Tests against real APIs
//!
//! Run with: cargo test -p jobsentinel-sources scrapers::live_tests -- --ignored --nocapture
//!
//! Note: Some scrapers require authentication or may be rate-limited/blocked.
//! Tests are ignored by default because they depend on live external sites.

use super::{
    GreenhouseCompany, GreenhouseScraper, HnHiringScraper, JobScraper, LeverCompany, LeverScraper,
    RemoteOkScraper, WeWorkRemotelyScraper,
};

// ============================================================================
// API-BASED SCRAPERS (Most reliable)
// ============================================================================

#[tokio::test]
#[ignore = "Live network scraper check; run manually"]
async fn test_greenhouse_live() {
    let scraper = GreenhouseScraper::new(vec![GreenhouseCompany {
        id: "cloudflare".to_string(),
        name: "Cloudflare".to_string(),
        url: "https://boards.greenhouse.io/cloudflare".to_string(),
    }]);

    let result = scraper.scrape().await;
    match result {
        Ok(jobs) => {
            println!("Greenhouse: found {} jobs from Cloudflare", jobs.len());
            assert!(!jobs.is_empty(), "Expected jobs from Cloudflare");
        }
        Err(e) => panic!("Greenhouse scraper failed: {}", e),
    }
}

#[tokio::test]
#[ignore = "Live network scraper check; run manually"]
async fn test_lever_live() {
    let scraper = LeverScraper::new(vec![
        LeverCompany {
            id: "gohighlevel".to_string(),
            name: "HighLevel".to_string(),
            url: "https://api.lever.co/v0/postings/gohighlevel".to_string(),
        },
        LeverCompany {
            id: "hermeus".to_string(),
            name: "Hermeus".to_string(),
            url: "https://api.lever.co/v0/postings/hermeus".to_string(),
        },
        LeverCompany {
            id: "activecampaign".to_string(),
            name: "ActiveCampaign".to_string(),
            url: "https://api.lever.co/v0/postings/activecampaign".to_string(),
        },
    ]);

    let result = scraper.scrape().await;
    match result {
        Ok(jobs) => {
            println!(
                "Lever: Found {} jobs from the public sample boards",
                jobs.len()
            );
            assert!(
                !jobs.is_empty(),
                "Expected jobs from the public Lever sample boards"
            );
        }
        Err(e) => panic!("Lever scraper failed: {}", e),
    }
}

#[tokio::test]
#[ignore = "Live network scraper check; run manually"]
async fn test_remoteok_live() {
    let scraper = RemoteOkScraper::new(vec!["customer-support".to_string()], 50);

    let result = scraper.scrape().await;
    match result {
        Ok(jobs) => {
            println!("RemoteOK: found {} jobs", jobs.len());
            // RemoteOK may have 0 jobs for a specific tag
        }
        Err(e) => panic!("RemoteOK scraper failed: {}", e),
    }
}

#[tokio::test]
#[ignore = "Live network scraper check; run manually"]
async fn test_hn_hiring_live() {
    let scraper = HnHiringScraper::new(50, false);

    let result = scraper.scrape().await;
    match result {
        Ok(jobs) => {
            println!("HN Who's Hiring: found {} jobs", jobs.len());
            // May be 0 between hiring threads
        }
        Err(e) => panic!("HN Who's Hiring scraper failed: {}", e),
    }
}

// ============================================================================
// RSS-BASED SCRAPERS
// ============================================================================

#[tokio::test]
#[ignore = "Live network scraper check; run manually"]
async fn test_weworkremotely_live() {
    let scraper = WeWorkRemotelyScraper::new(Some("remote-customer-support-jobs".to_string()), 50);

    let result = scraper.scrape().await;
    match result {
        Ok(jobs) => {
            println!("WeWorkRemotely: found {} jobs", jobs.len());
            assert!(!jobs.is_empty(), "Expected jobs from WeWorkRemotely");
        }
        Err(e) => panic!("WeWorkRemotely scraper failed: {}", e),
    }
}

// ============================================================================
// EXTERNALLY CONFIGURED OR SOURCE-POLICY LIMITED SCRAPERS
// ============================================================================

#[tokio::test]
#[ignore = "Live LinkedIn automation is not in the default lane; use user-gated native import paths"]
async fn test_linkedin_live() {
    println!("LinkedIn: use user-gated native import paths instead of default hidden monitoring");
}

#[tokio::test]
#[ignore = "Requires USAJobs API key"]
async fn test_usajobs_live() {
    // USAJobs requires API key registration
    // This test is skipped by default
    println!("USAJobs: Skipped (requires API key)");
}

#[tokio::test]
#[ignore = "Requires MCP endpoint configuration"]
async fn test_jobswithgpt_live() {
    // JobsWithGPT uses MCP protocol
    // This test is skipped by default
    println!("JobsWithGPT: Skipped (requires MCP endpoint)");
}
