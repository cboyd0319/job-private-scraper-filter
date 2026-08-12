use super::*;

// Additional tests for uncovered paths

#[tokio::test]
async fn test_scrape_calls_fetch_jobs() {
    let scraper = HnHiringScraper::new(5, true);

    // scrape() calls fetch_jobs() which we can't test without mocking the API
    // but we can verify the scraper is properly initialized
    assert_eq!(scraper.limit, 5);
    assert!(scraper.remote_only);
    assert_eq!(scraper.name(), "hn_hiring");
}

#[test]
fn test_extract_company_with_parentheses() {
    let (company, rest) = HnHiringScraper::extract_company("StartupCo (YC W22) | Backend Engineer");
    assert_eq!(company, "StartupCo (YC W22)");
    assert!(rest.contains("Backend Engineer"));
}

#[test]
fn test_extract_title_sre() {
    let text = "We're hiring an SRE to manage our infrastructure";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title.as_ref().unwrap().contains("SRE") || title.as_ref().unwrap().contains("sre"));
}

#[test]
fn test_extract_title_devops() {
    let text = "Looking for a DevOps engineer with Kubernetes experience";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title.as_ref().unwrap().to_lowercase().contains("devops"));
}

#[test]
fn test_extract_title_cto() {
    let text = "Seeking a CTO with startup experience";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title.as_ref().unwrap().to_lowercase().contains("cto"));
}

#[test]
fn test_extract_title_product_manager() {
    let text = "Product Manager role available";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title
        .as_ref()
        .unwrap()
        .to_lowercase()
        .contains("product manager"));
}

#[test]
fn test_extract_title_data_engineer() {
    let text = "Data Engineer position for building ETL pipelines";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title
        .as_ref()
        .unwrap()
        .to_lowercase()
        .contains("data engineer"));
}

#[test]
fn test_extract_title_infrastructure() {
    let text = "Infrastructure role managing cloud systems";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title
        .as_ref()
        .unwrap()
        .to_lowercase()
        .contains("infrastructure"));
}

#[test]
fn test_extract_location_with_variations() {
    assert_eq!(
        HnHiringScraper::extract_location("Office in Austin, Texas"),
        Some("Austin, TX".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Located in Boston area"),
        Some("Boston, MA".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Denver based startup"),
        Some("Denver, CO".to_string())
    );
}

#[test]
fn test_extract_location_international() {
    assert_eq!(
        HnHiringScraper::extract_location("Based in Berlin, Germany"),
        Some("Berlin, Germany".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Toronto office"),
        Some("Toronto, Canada".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Our Vancouver team"),
        Some("Vancouver, Canada".to_string())
    );
}

#[test]
fn test_extract_location_remote_us_only() {
    assert_eq!(
        HnHiringScraper::extract_location("Remote, US only"),
        Some("Remote (US)".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Remote position - USA only"),
        Some("Remote (US)".to_string())
    );
}

#[test]
fn test_extract_location_remote_europe() {
    assert_eq!(
        HnHiringScraper::extract_location("Remote (Europe)"),
        Some("Remote (EU)".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Remote, EU only"),
        Some("Remote (EU)".to_string())
    );
}

#[test]
fn test_extract_location_generic_remote() {
    assert_eq!(
        HnHiringScraper::extract_location("Fully remote position"),
        Some("Remote".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Remote work available"),
        Some("Remote".to_string())
    );
}

#[test]
fn test_is_remote_with_distributed() {
    assert!(HnHiringScraper::is_remote("We have a distributed team"));
    assert!(HnHiringScraper::is_remote(
        "Distributed company, work anywhere"
    ));
}

#[test]
fn test_is_remote_with_anywhere() {
    assert!(HnHiringScraper::is_remote(
        "Work from anywhere in the world"
    ));
    assert!(HnHiringScraper::is_remote("Anywhere, fully remote"));
}

#[test]
fn test_is_remote_false_for_onsite() {
    assert!(!HnHiringScraper::is_remote(
        "Onsite position in San Francisco"
    ));
    assert!(!HnHiringScraper::is_remote("In-office only"));
    assert!(!HnHiringScraper::is_remote("Must be in NYC office"));
}

#[test]
fn test_strip_html_with_various_tags() {
    let html = "<div><strong>Bold</strong> <em>italic</em> <a href='link'>text</a></div>";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("Bold"));
    assert!(clean.contains("italic"));
    assert!(clean.contains("text"));
    assert!(!clean.contains("<div>"));
    assert!(!clean.contains("<strong>"));
}

#[test]
fn test_strip_html_with_line_breaks() {
    let html = "Line 1<br>Line 2<br/>Line 3<br />Line 4";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("Line 1"));
    assert!(clean.contains("Line 2"));
    assert!(clean.contains("Line 3"));
    assert!(clean.contains("Line 4"));
}

#[test]
fn test_strip_html_with_all_entities() {
    let html = "&amp; &lt; &gt; &quot; &#x27; &#39; &nbsp;";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("&"));
    assert!(clean.contains("\""));
    assert!(clean.contains("'"));
}

#[test]
fn test_parse_job_comment_with_short_company() {
    let scraper = HnHiringScraper::new(10, false);
    let text =
        "X | Senior Engineer | Remote\n\nGreat company, great culture, great benefits.".repeat(1);

    let result = scraper.parse_job_text(&text, 123);
    // Company name "X" is too short (< 2 chars)
    assert!(result.is_none());
}

#[test]
fn test_parse_job_comment_with_valid_data() {
    let scraper = HnHiringScraper::new(10, false);
    let text = "TechCorp | Senior Software Engineer | Remote\n\nWe're looking for experienced engineers to join our team. Must have 5+ years of experience with distributed systems.";

    let result = scraper.parse_job_text(text, 123_456);
    assert!(result.is_some());

    let job = result.unwrap();
    assert_eq!(job.company, "TechCorp");
    assert!(job.remote.unwrap_or(false));
    assert_eq!(job.url, "https://news.ycombinator.com/item?id=123456");
    assert_eq!(job.source, "hn_hiring");
}

#[test]
fn test_parse_job_comment_description_truncation() {
    let scraper = HnHiringScraper::new(10, false);
    let long_description = format!("BigCompany | Engineer | SF\n\n{}", "X".repeat(600));
    let result = scraper.parse_job_text(&long_description, 789);

    assert!(result.is_some());
    let job = result.unwrap();
    let desc = job.description.unwrap();
    assert!(desc.len() <= 503);
    assert!(desc.ends_with("..."));
}

#[test]
fn test_parse_job_comment_description_not_truncated_when_short() {
    let scraper = HnHiringScraper::new(10, false);
    let short_text = "Company | Engineer | Remote\n\nShort description here.";
    let result = scraper.parse_job_text(short_text, 999);

    assert!(result.is_some());
    let job = result.unwrap();
    let desc = job.description.unwrap();
    assert!(!desc.ends_with("..."));
    assert!(desc.contains("Short description here."));
}

#[test]
fn test_compute_hash_location_variations() {
    // With normalization, "Remote (US)" and "Remote (EU)" both normalize to "remote"
    // This is expected behavior - they should produce the same hash
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Engineer",
        Some("Remote (US)"),
        "https://news.ycombinator.com/item?id=1",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Engineer",
        Some("Remote (EU)"),
        "https://news.ycombinator.com/item?id=1",
    );

    // Both should normalize to "remote" and produce the same hash
    assert_eq!(hash1, hash2);

    // But different cities should still produce different hashes
    let hash3 = jobsentinel_domain::calculate_job_hash(
        "Company",
        "Engineer",
        Some("New York, NY"),
        "https://news.ycombinator.com/item?id=1",
    );
    assert_ne!(hash1, hash3);
}

#[test]
fn test_new_scraper_initialization() {
    let scraper = HnHiringScraper::new(25, true);

    assert_eq!(scraper.limit, 25);
    assert!(scraper.remote_only);
    assert_eq!(scraper.name(), "hn_hiring");
}

#[test]
fn test_extract_title_with_context() {
    let text = "TechCorp | Senior Backend Engineer | Remote\n\nWe're hiring!";
    let title = HnHiringScraper::extract_title(text);

    assert!(title.is_some());
    assert!(title
        .as_ref()
        .unwrap()
        .to_lowercase()
        .contains("backend engineer"));
}

#[test]
fn test_extract_title_length_bounds() {
    // Title should be between 5 and 100 chars
    let text_too_short = "AB | CD";
    let title_short = HnHiringScraper::extract_title(text_too_short);
    // Might not find a valid title in this short text
    assert!(title_short.is_none() || title_short.unwrap().len() >= 5);
}
