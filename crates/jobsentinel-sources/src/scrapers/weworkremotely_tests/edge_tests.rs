use super::*;

#[test]
fn test_parse_rss_all_jobs_remote() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss>
            <channel>
                <item>
                    <title>Company: Program Coordinator</title>
                    <link>https://weworkremotely.com/jobs/1</link>
                    <description>Remote job</description>
                </item>
            </channel>
        </rss>
    "#;

    let jobs = scraper.parse_rss(rss).expect("parse_rss should succeed");

    assert_eq!(jobs.len(), 1);
    // All WeWorkRemotely jobs are remote
    assert_eq!(jobs[0].remote, Some(true));
}

#[test]
fn test_parse_rss_whitespace_trimming() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss>
            <channel>
                <item>
                    <title>
                        Community Care Network  :  Senior Care Coordinator
                    </title>
                    <link>  https://weworkremotely.com/jobs/123  </link>
                    <description>
                        Great opportunity
                    </description>
                </item>
            </channel>
        </rss>
    "#;

    let jobs = scraper.parse_rss(rss).expect("parse_rss should succeed");

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].company, "Community Care Network");
    assert_eq!(jobs[0].title, "Senior Care Coordinator");
}

#[test]
fn test_extract_tag_missing() {
    let xml = "<item><title>Test</title></item>";
    assert_eq!(WeWorkRemotelyScraper::extract_tag(xml, "nonexistent"), None);
}

#[test]
fn test_extract_tag_empty_content() {
    let xml = "<item><title></title></item>";
    // Empty tags are not found (returns None)
    assert_eq!(WeWorkRemotelyScraper::extract_tag(xml, "title"), None);
}

#[test]
fn test_remote_flag_always_true() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss>
            <channel>
                <item>
                    <title>Company: Job</title>
                    <link>https://weworkremotely.com/jobs/1</link>
                </item>
            </channel>
        </rss>
    "#;

    let jobs = scraper.parse_rss(rss).expect("parse_rss should succeed");

    assert_eq!(jobs.len(), 1);
    // WeWorkRemotely is remote-only, so all jobs should have remote=true
    assert_eq!(jobs[0].remote, Some(true));
}

#[test]
fn test_extract_tag_start_after_end() {
    let xml = "<item></title><title></item>";
    let result = WeWorkRemotelyScraper::extract_tag(xml, "title");
    // Malformed: end tag before start tag
    assert_eq!(result, None);
}

#[test]
fn test_strip_html_tags_nested_tags() {
    let html = "<div><p>Text <span>inside <strong>nested</strong> tags</span></p></div>";
    let result = WeWorkRemotelyScraper::strip_html_tags(html);
    assert_eq!(result, "Text inside nested tags");
}

#[test]
fn test_strip_html_tags_incomplete_tag() {
    let html = "<div>Text with incomplete <tag";
    let result = WeWorkRemotelyScraper::strip_html_tags(html);
    // Should handle incomplete tag gracefully
    assert!(result.contains("Text"));
}

#[test]
fn test_decode_html_entities_mixed() {
    let text = "Company &amp; Partners &lt;2024&gt; &quot;Best&quot; &nbsp;";
    let decoded = WeWorkRemotelyScraper::decode_html_entities(text);
    assert_eq!(decoded, "Company & Partners <2024> \"Best\"  ");
}

#[test]
fn test_parse_rss_description_with_no_location_patterns() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss>
            <channel>
                <item>
                    <title>FreshMart: Inventory Planner</title>
                    <link>https://weworkremotely.com/jobs/123</link>
                    <description>Great opportunity with competitive salary.</description>
                </item>
            </channel>
        </rss>
    "#;

    let jobs = scraper.parse_rss(rss).expect("parse_rss should succeed");

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].location, None);
}

#[test]
fn test_compute_hash_with_different_locations() {
    let hash1 = jobsentinel_domain::calculate_job_hash("Company", "Job", Some("USA"), "url");
    let hash2 = jobsentinel_domain::calculate_job_hash("Company", "Job", Some("Europe"), "url");

    assert_ne!(hash1, hash2);
}

#[test]
fn test_extract_tag_cdata_priority_over_regular() {
    let xml = r#"<item><title><![CDATA[CDATA Content]]></title><title>Regular</title></item>"#;
    let result = WeWorkRemotelyScraper::extract_tag(xml, "title");
    // CDATA check happens first, so should get CDATA content
    assert_eq!(result, Some("CDATA Content".to_string()));
}

#[test]
fn test_parse_rss_no_description_tag() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss>
            <channel>
                <item>
                    <title>Company: Position</title>
                    <link>https://weworkremotely.com/jobs/999</link>
                </item>
            </channel>
        </rss>
    "#;

    let jobs = scraper.parse_rss(rss).expect("parse_rss should succeed");

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].description, None);
    assert_eq!(jobs[0].location, None);
}

#[test]
fn test_new_constructor() {
    let scraper = WeWorkRemotelyScraper::new(Some("remote-design-jobs".to_string()), 25);

    assert_eq!(scraper.category, Some("remote-design-jobs".to_string()));
    assert_eq!(scraper.limit, 25);
}
