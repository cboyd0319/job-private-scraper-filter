use super::*;
use jobsentinel_network::send_test_http_text_with_retry;
use std::num::NonZeroU16;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LIST_FIXTURE: &str = include_str!("../../fixtures/weworkremotely_list_v1.xml");
const DETAIL_FIXTURE: &str = include_str!("../../fixtures/weworkremotely_detail_v1.xml");

#[test]
fn reviewed_list_and_detail_fixtures_parse_to_the_same_canonical_job() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let list_job = scraper.parse_rss(LIST_FIXTURE).unwrap().remove(0);
    let detail_job = scraper.parse_rss(DETAIL_FIXTURE).unwrap().remove(0);

    assert_eq!(list_job.title, "Distributed Systems Engineer");
    assert_eq!(list_job.company, "Example Cooperative");
    assert_eq!(
        list_job.url,
        "https://weworkremotely.com/remote-jobs/example-cooperative-distributed-systems-engineer"
    );
    assert_eq!(list_job.title, detail_job.title);
    assert_eq!(list_job.company, detail_job.company);
    assert_eq!(list_job.url, detail_job.url);
}

#[test]
fn parser_keeps_only_canonical_weworkremotely_listing_links() {
    let scraper = WeWorkRemotelyScraper::new(None, 10);
    let rss = r#"
        <rss><channel>
            <item>
                <title>Example: External Link</title>
                <link>https://employer.example/jobs/1</link>
            </item>
            <item>
                <title>Example: Insecure Link</title>
                <link>http://weworkremotely.com/remote-jobs/example-insecure</link>
            </item>
            <item>
                <title>Example: Reviewed Link</title>
                <link>https://weworkremotely.com/remote-jobs/example-reviewed</link>
            </item>
        </channel></rss>
    "#;

    let jobs = scraper.parse_rss(rss).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].url,
        "https://weworkremotely.com/remote-jobs/example-reviewed"
    );
}

#[test]
fn build_url_uses_only_reviewed_feeds() {
    let default = WeWorkRemotelyScraper::new(None, 10);
    assert_eq!(
        default.build_url().unwrap(),
        "https://weworkremotely.com/remote-jobs.rss"
    );

    let category = WeWorkRemotelyScraper::new(Some("remote-programming-jobs".to_string()), 10);
    assert_eq!(
        category.build_url().unwrap(),
        "https://weworkremotely.com/categories/remote-programming-jobs.rss"
    );

    let unreviewed = WeWorkRemotelyScraper::new(Some("../account".to_string()), 10);
    assert!(matches!(
        unreviewed.build_url(),
        Err(ScraperError::InvalidConfiguration { .. })
    ));
}

#[test]
fn legacy_categories_map_to_reviewed_feed_urls() {
    for (category, expected) in [
        (
            "programming",
            "https://weworkremotely.com/categories/remote-programming-jobs.rss",
        ),
        (
            "design",
            "https://weworkremotely.com/categories/remote-design-jobs.rss",
        ),
        (
            "devops",
            "https://weworkremotely.com/categories/remote-devops-sysadmin-jobs.rss",
        ),
    ] {
        let scraper = WeWorkRemotelyScraper::new(Some(category.to_string()), 10);
        assert_eq!(scraper.build_url().unwrap(), expected);
    }
}

#[test]
fn authorized_rate_replaces_the_weworkremotely_default() {
    let scraper = WeWorkRemotelyScraper::new(None, 10)
        .with_request_limit_per_hour(NonZeroU16::new(24).unwrap());

    assert_eq!(scraper.request_limit_per_hour, 24);
}

#[tokio::test]
async fn weworkremotely_request_does_not_retry_rate_limits_or_server_errors() {
    for status in [429, 500] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/remote-jobs.rss"))
            .respond_with(ResponseTemplate::new(status).insert_header("Retry-After", "0"))
            .expect(1)
            .mount(&server)
            .await;
        let response = send_test_http_text_with_retry(WeWorkRemotelyScraper::request(&format!(
            "{}/remote-jobs.rss",
            server.uri()
        )))
        .await
        .unwrap();

        assert_eq!(response.status, status);
        server.verify().await;
    }
}
