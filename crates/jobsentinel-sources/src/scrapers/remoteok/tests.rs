mod hash_tests;
mod parse_tests;
mod smoke_tests;
mod tag_tests;

use super::*;
use jobsentinel_network::send_test_http_text_with_retry;
use std::num::NonZeroU16;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LIST_FIXTURE: &str = include_str!("../../fixtures/remoteok_list_v1.json");
const DETAIL_FIXTURE: &str = include_str!("../../fixtures/remoteok_detail_v1.json");

#[test]
fn reviewed_list_and_detail_fixtures_parse_to_the_same_canonical_job() {
    let list: serde_json::Value = serde_json::from_str(LIST_FIXTURE).unwrap();
    let detail: serde_json::Value = serde_json::from_str(DETAIL_FIXTURE).unwrap();
    let scraper = RemoteOkScraper::new(vec![], 10);
    let list_job = scraper
        .parse_job(&list.as_array().unwrap()[1])
        .unwrap()
        .unwrap();
    let detail_job = scraper.parse_job(&detail).unwrap().unwrap();

    assert_eq!(list_job.title, "Distributed Systems Engineer");
    assert_eq!(list_job.company, "Example Cooperative");
    assert_eq!(list_job.url, "https://remoteok.com/remote-jobs/example-123");
    assert_eq!(list_job.title, detail_job.title);
    assert_eq!(list_job.company, detail_job.company);
    assert_eq!(list_job.url, detail_job.url);
}

#[test]
fn authorized_rate_replaces_the_remoteok_default() {
    let scraper =
        RemoteOkScraper::new(vec![], 10).with_request_limit_per_hour(NonZeroU16::new(24).unwrap());

    assert_eq!(scraper.request_limit_per_hour, 24);
}

#[tokio::test]
async fn remoteok_request_does_not_retry_rate_limits_or_server_errors() {
    for status in [429, 500] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(status).insert_header("Retry-After", "0"))
            .expect(1)
            .mount(&server)
            .await;
        let response = send_test_http_text_with_retry(RemoteOkScraper::request(&format!(
            "{}/api",
            server.uri()
        )))
        .await
        .unwrap();

        assert_eq!(response.status, status);
        server.verify().await;
    }
}
