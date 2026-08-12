use super::*;
use jobsentinel_network::send_test_http_text_with_retry;
use std::num::NonZeroU16;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LIST_FIXTURE: &str = include_str!("../../../fixtures/hn_hiring_list_v1.json");
const DETAIL_FIXTURE: &str = include_str!("../../../fixtures/hn_hiring_detail_v1.json");

#[test]
fn reviewed_thread_and_comment_fixtures_parse_to_canonical_records() {
    let scraper = HnHiringScraper::new(10, false);
    let list: serde_json::Value = serde_json::from_str(LIST_FIXTURE).unwrap();
    let detail: serde_json::Value = serde_json::from_str(DETAIL_FIXTURE).unwrap();

    let thread_id = HnHiringScraper::canonical_thread_id(&list).unwrap();
    assert_eq!(thread_id, 44_234_567);
    let jobs = scraper.parse_thread_item(&detail, thread_id).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].company, "Example Cooperative");
    assert_eq!(jobs[0].url, "https://news.ycombinator.com/item?id=44234678");
    assert!(jobs
        .iter()
        .all(|job| job.url != "https://news.ycombinator.com/item?id=44234679"));
}

#[test]
fn thread_item_must_match_the_selected_canonical_thread() {
    let scraper = HnHiringScraper::new(10, false);
    let mut detail: serde_json::Value = serde_json::from_str(DETAIL_FIXTURE).unwrap();
    detail["id"] = serde_json::json!(99);

    assert!(scraper.parse_thread_item(&detail, 44_234_567).is_err());
}

#[test]
fn comments_without_numeric_hn_ids_are_not_saved() {
    let scraper = HnHiringScraper::new(10, false);
    let detail = serde_json::json!({
        "children": [
            {
                "text": "Missing ID Cooperative | Senior Engineer | Remote\n\nThis posting is long enough to otherwise become a local job record."
            },
            {
                "id": "12?next=account",
                "text": "Invalid ID Cooperative | Senior Engineer | Remote\n\nThis posting is long enough to otherwise become a local job record."
            },
            {
                "id": 0,
                "text": "Zero ID Cooperative | Senior Engineer | Remote\n\nThis posting is long enough to otherwise become a local job record."
            }
        ]
    });

    assert!(scraper.parse_thread_children(&detail).unwrap().is_empty());
}

#[test]
fn multibyte_descriptions_truncate_on_a_character_boundary() {
    let scraper = HnHiringScraper::new(10, false);
    let detail = serde_json::json!({
        "children": [{
            "id": 44234679,
            "text": format!(
                "AB | Senior Engineer | Remote\n\n{}",
                "🙂".repeat(600)
            )
        }]
    });

    let jobs = scraper.parse_thread_children(&detail).unwrap();
    let description = jobs[0].description.as_deref().unwrap();
    assert!(description.chars().count() <= 503);
    assert!(description.ends_with("..."));
}

#[test]
fn direct_children_are_scanned_until_the_result_limit_is_met() {
    let scraper = HnHiringScraper::new(1, false);
    let detail = serde_json::json!({
        "children": [
            {"id": 1, "text": "short"},
            {"id": 2, "text": "also short"},
            {
                "id": 3,
                "text": "Late Cooperative | Senior Engineer | Remote\n\nThis valid direct reply appears after earlier non-job discussion."
            }
        ]
    });

    let jobs = scraper.parse_thread_children(&detail).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].company, "Late Cooperative");
}

#[test]
fn zero_result_limit_returns_no_jobs() {
    let scraper = HnHiringScraper::new(0, false);
    let detail = serde_json::json!({
        "children": [{
            "id": 5,
            "text": "Zero Limit Cooperative | Senior Engineer | Remote\n\nThis otherwise valid direct reply must not cross the zero result limit."
        }]
    });

    assert!(scraper.parse_thread_children(&detail).unwrap().is_empty());
}

#[test]
fn hostile_unicode_title_text_does_not_panic() {
    let scraper = HnHiringScraper::new(1, false);
    let detail = serde_json::json!({
        "children": [{
            "id": 4,
            "text": "İ | software engineer🙂 | Remote\n\nThis direct reply contains enough detail to be parsed as a job."
        }]
    });

    let jobs = scraper.parse_thread_children(&detail).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].url, "https://news.ycombinator.com/item?id=4");
}

#[test]
fn authorized_rate_replaces_the_hn_hiring_default() {
    let scraper =
        HnHiringScraper::new(10, false).with_request_limit_per_hour(NonZeroU16::new(24).unwrap());

    assert_eq!(scraper.request_limit_per_hour, 24);
}

#[tokio::test]
async fn hn_hiring_request_does_not_retry_rate_limits_or_server_errors() {
    for status in [429, 500] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/search_by_date"))
            .respond_with(ResponseTemplate::new(status).insert_header("Retry-After", "0"))
            .expect(1)
            .mount(&server)
            .await;
        let response = send_test_http_text_with_retry(HnHiringScraper::request(&format!(
            "{}/api/v1/search_by_date",
            server.uri()
        )))
        .await
        .unwrap();

        assert_eq!(response.status, status);
        server.verify().await;
    }
}
