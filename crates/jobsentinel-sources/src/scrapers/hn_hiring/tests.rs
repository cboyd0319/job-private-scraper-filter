use super::*;

#[test]
fn test_scraper_name() {
    let scraper = HnHiringScraper::new(10, false);
    assert_eq!(scraper.name(), "hn_hiring");
}

#[test]
fn test_compute_hash_deterministic() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Startup",
        "Engineer",
        Some("Remote"),
        "https://news.ycombinator.com/item?id=123",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Startup",
        "Engineer",
        Some("Remote"),
        "https://news.ycombinator.com/item?id=123",
    );

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64);
}

#[test]
fn test_compute_hash_different_inputs() {
    let hash1 = jobsentinel_domain::calculate_job_hash(
        "Company A",
        "Engineer",
        Some("Remote"),
        "https://news.ycombinator.com/item?id=123",
    );
    let hash2 = jobsentinel_domain::calculate_job_hash(
        "Company B",
        "Engineer",
        Some("Remote"),
        "https://news.ycombinator.com/item?id=123",
    );

    assert_ne!(hash1, hash2);
}

#[test]
fn test_compute_hash_without_location() {
    let hash = jobsentinel_domain::calculate_job_hash(
        "Startup",
        "Engineer",
        None,
        "https://news.ycombinator.com/item?id=456",
    );

    assert_eq!(hash.len(), 64);
}

#[test]
fn test_extract_company() {
    let (company, rest) = HnHiringScraper::extract_company("Acme Inc | Senior Engineer | Remote");
    assert_eq!(company, "Acme Inc");
    assert!(rest.contains("Senior Engineer"));

    let (company2, _) = HnHiringScraper::extract_company("StartupCo (YC S21) | Backend");
    assert_eq!(company2, "StartupCo (YC S21)");
}

#[test]
fn test_extract_company_with_dash_separator() {
    let (company, rest) = HnHiringScraper::extract_company("TechCorp - Senior Backend Developer");
    assert_eq!(company, "TechCorp");
    assert_eq!(rest, "Senior Backend Developer");
}

#[test]
fn test_extract_company_no_separator() {
    let (company, rest) = HnHiringScraper::extract_company("SoloCorp Hiring Engineers");
    assert_eq!(company, "SoloCorp Hiring Engineers");
    assert_eq!(rest, "");
}

#[test]
fn test_is_remote() {
    assert!(HnHiringScraper::is_remote("Remote position available"));
    assert!(HnHiringScraper::is_remote("WFH friendly"));
    assert!(HnHiringScraper::is_remote("Work from anywhere"));
    assert!(!HnHiringScraper::is_remote("In office only"));
}

#[test]
fn test_is_remote_distributed() {
    assert!(HnHiringScraper::is_remote(
        "Distributed team, work anywhere"
    ));
}

#[test]
fn test_is_remote_case_insensitive() {
    assert!(HnHiringScraper::is_remote("REMOTE position"));
    assert!(HnHiringScraper::is_remote("Anywhere in the world"));
}

#[test]
fn test_extract_location() {
    assert_eq!(
        HnHiringScraper::extract_location("Based in San Francisco"),
        Some("San Francisco, CA".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Remote US only"),
        Some("Remote (US)".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Fully remote"),
        Some("Remote".to_string())
    );
}

#[test]
fn test_extract_location_multiple_cities() {
    assert_eq!(
        HnHiringScraper::extract_location("Looking for engineers in NYC"),
        Some("New York, NY".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Based in Seattle, WA"),
        Some("Seattle, WA".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("London office"),
        Some("London, UK".to_string())
    );
}

#[test]
fn test_extract_location_remote_eu() {
    assert_eq!(
        HnHiringScraper::extract_location("Remote, EU only"),
        Some("Remote (EU)".to_string())
    );
    assert_eq!(
        HnHiringScraper::extract_location("Remote (Europe)"),
        Some("Remote (EU)".to_string())
    );
}

#[test]
fn test_extract_location_no_match() {
    assert_eq!(HnHiringScraper::extract_location("Onsite position"), None);
}

#[test]
fn test_strip_html() {
    let html = "<p>Hello <b>World</b></p><br>New line";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("Hello"));
    assert!(clean.contains("World"));
    assert!(!clean.contains("<"));
}

#[test]
fn test_strip_html_entities() {
    let html = "We&amp;#x27;re hiring &lt;engineers&gt; &quot;now&quot;";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("We're hiring") || clean.contains("We&#x27;re hiring"));
    // Note: &lt; and &gt; are replaced with < and >, then stripped as tags
    assert!(clean.contains("\"now\""));
}

#[test]
fn test_strip_html_paragraph_breaks() {
    let html = "<p>First paragraph</p><p>Second paragraph</p>";
    let clean = HnHiringScraper::strip_html(html);
    assert!(clean.contains("First paragraph"));
    assert!(clean.contains("Second paragraph"));
    assert!(!clean.contains("<p>"));
}

#[test]
fn test_strip_html_empty_string() {
    let clean = HnHiringScraper::strip_html("");
    assert_eq!(clean, "");
}

#[test]
fn test_extract_title_common_patterns() {
    // The function extracts from the beginning of the line to the end of the pattern
    let title1 =
        HnHiringScraper::extract_title("Looking for a Senior Software Engineer to join our team");
    assert!(title1.is_some());
    assert!(title1.as_ref().unwrap().contains("Software Engineer"));

    let title2 = HnHiringScraper::extract_title("We need a Backend Engineer with Rust experience");
    assert!(title2.is_some());
    assert!(title2.as_ref().unwrap().contains("Backend Engineer"));
}

#[test]
fn test_extract_title_fullstack() {
    let text = "Hiring Full Stack developers";
    let title = HnHiringScraper::extract_title(text);
    assert!(title.is_some());
    assert!(title.as_ref().unwrap().contains("Full Stack"));
}

#[test]
fn test_extract_title_machine_learning() {
    let text = "Machine Learning engineer wanted";
    assert_eq!(
        HnHiringScraper::extract_title(text),
        Some("Machine Learning".to_string())
    );
}

#[test]
fn test_extract_title_no_match() {
    assert_eq!(HnHiringScraper::extract_title("Join our team!"), None);
}

#[test]
fn test_parse_thread_children_with_realistic_json() {
    let scraper = HnHiringScraper::new(10, false);
    let json_data = serde_json::json!({
        "children": [
            {
                "id": 12345,
                "text": "Acme Corp (YC S21) | Senior Backend Engineer | Remote | Full-time\n\nWe're building the future of AI. Looking for experienced engineers to join our team. Must have 5+ years experience with Rust or Go. Remote work available anywhere in the US.\n\nApply at: https://acmecorp.com/jobs"
            },
            {
                "id": 67890,
                "text": "TechStartup | Full Stack Engineer | San Francisco | $150k-$200k\n\nJoin our small team working on cutting-edge web tech. We use React, Node.js, and PostgreSQL."
            }
        ]
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].company, "Acme Corp (YC S21)");
    assert_eq!(jobs[0].remote, Some(true));
    assert_eq!(jobs[0].source, "hn_hiring");
    assert!(jobs[0].url.contains("12345"));
}

#[test]
fn test_parse_thread_children_respects_limit() {
    let scraper = HnHiringScraper::new(2, false);
    let json_data = serde_json::json!({
        "children": [
            {
                "id": 1,
                "text": "Company A | Senior Engineer | Remote\n\nWe're hiring experienced engineers with deep knowledge of distributed systems."
            },
            {
                "id": 2,
                "text": "Company B | Backend Developer | San Francisco\n\nLooking for backend developers with Python experience."
            },
            {
                "id": 3,
                "text": "Company C | Frontend Developer | New York\n\nSeeking frontend developers who know React."
            }
        ]
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 2);
}

#[test]
fn test_parse_thread_children_filters_remote_only() {
    let scraper = HnHiringScraper::new(10, true);
    let json_data = serde_json::json!({
        "children": [
            {
                "id": 1,
                "text": "RemoteCo | Senior Engineer | Remote\n\nFully remote position available for experienced engineers."
            },
            {
                "id": 2,
                "text": "OnSiteCo | Backend Developer | San Francisco\n\nOnsite only position in downtown SF."
            },
            {
                "id": 3,
                "text": "HybridCo | Frontend Developer | WFH\n\nWork from home position available."
            }
        ]
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 1);
    assert!(jobs.iter().all(|j| j.remote == Some(true)));
}

#[test]
fn test_parse_thread_children_skips_short_comments() {
    let scraper = HnHiringScraper::new(10, false);
    let json_data = serde_json::json!({
        "children": [
            {
                "id": 1,
                "text": "Too short"
            },
            {
                "id": 2,
                "text": "ValidCorp | Senior Engineer | Remote\n\nThis is a longer comment with sufficient detail about the job posting."
            }
        ]
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].company, "ValidCorp");
}

#[test]
fn test_parse_thread_children_empty_hits() {
    let scraper = HnHiringScraper::new(10, false);
    let json_data = serde_json::json!({
        "children": []
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 0);
}

#[test]
fn test_parse_thread_children_truncates_long_description() {
    let scraper = HnHiringScraper::new(10, false);
    let long_text = format!("BigCorp | Engineer | Remote\n\n{}", "A".repeat(600));
    let json_data = serde_json::json!({
        "children": [
            {
                "id": 1,
                "text": long_text
            }
        ]
    });

    let jobs = scraper
        .parse_thread_children(&json_data)
        .expect("parse_thread_children should succeed");

    assert_eq!(jobs.len(), 1);
    let description = jobs[0].description.as_ref().unwrap();
    assert!(description.len() <= 503); // 500 + "..."
    assert!(description.ends_with("..."));
}

mod additional_tests;
mod governance_tests;
