// Verifies governed bookmarklet imports remain pending until explicit confirmation.

use super::*;

async fn queue_example_import(
    database: Arc<TestBookmarkletRepository>,
    pending_imports: PendingBookmarkletImports,
) -> serde_json::Value {
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Community Care",
            "url": "https://example.com/jobs/1"
        }),
    );
    let (response, _) = handle_import_request(
        &bookmarklet_import_request(&body),
        &active_pairing,
        pending_imports,
        database,
    )
    .await;
    serde_json::from_str(&response).unwrap()
}

async fn queue_url_import(
    url: &str,
) -> (
    serde_json::Value,
    PendingBookmarkletImports,
    Arc<TestBookmarkletRepository>,
) {
    let database = bookmarklet_test_database().await;
    let origin = external_https_origin(url).unwrap();
    let (active_pairing, code) = bookmarklet_pairing(&origin);
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Example",
            "url": url
        }),
    );
    let (response, _) = handle_import_request(
        &bookmarklet_import_request_from_origin(&body, &origin, url),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    (
        serde_json::from_str(&response).unwrap(),
        pending_imports,
        database,
    )
}

#[tokio::test]
async fn test_bookmarklet_import_requires_persisted_authorization_before_queue() {
    let database = bookmarklet_test_database().await;
    database.set_authorization_allowed(false);
    let pending_imports = bookmarklet_pending_imports();
    let parsed = queue_example_import(database.clone(), pending_imports.clone()).await;

    assert_eq!(parsed["error"], BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE);
    assert!(pending_bookmarklet_import_previews(&pending_imports).is_empty());
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_confirm_keeps_pending_import_after_authorization_drift() {
    let database = bookmarklet_test_database().await;
    let pending_imports = bookmarklet_pending_imports();
    let parsed = queue_example_import(database.clone(), pending_imports.clone()).await;
    let preview = pending_bookmarklet_import_previews(&pending_imports)
        .pop()
        .unwrap();
    database.set_authorization_allowed(false);

    let result =
        confirm_pending_bookmarklet_imports(&database, &pending_imports, &[preview.id]).await;

    assert_eq!(parsed["success"], true);
    assert_eq!(
        result.unwrap_err(),
        BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE
    );
    assert_eq!(
        pending_bookmarklet_import_previews(&pending_imports).len(),
        1
    );
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_import_rejects_policy_blocked_page_capture() {
    for url in [
        "https://ycombinator.com/jobs/1",
        "https://www.ycombinator.com/jobs/2",
        "https://linkedin.com/jobs/view/1",
        "https://www.linkedin.com/jobs/view/2",
    ] {
        let (parsed, pending_imports, database) = queue_url_import(url).await;

        assert_eq!(
            parsed["error"],
            "Browser Import is unavailable for this source"
        );
        assert!(pending_bookmarklet_import_previews(&pending_imports).is_empty());
        assert_eq!(stored_job_count(&database).await, 0);
    }
}

#[tokio::test]
async fn test_bookmarklet_import_allows_visible_capture_for_fetch_blocked_boards() {
    for url in [
        "https://builtin.com/jobs/1",
        "https://www.dice.com/jobs/2",
        "https://www.simplyhired.com/job/3",
        "https://jobs.glassdoor.com/jobs/4",
    ] {
        let (parsed, pending_imports, _) = queue_url_import(url).await;
        let previews = pending_bookmarklet_import_previews(&pending_imports);

        assert_eq!(parsed["success"], true, "{url}");
        assert_eq!(previews.len(), 1, "{url}");
        assert_eq!(previews[0].url, url);
    }
}

#[tokio::test]
async fn test_bookmarklet_import_queues_valid_job_for_review_without_insert() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Community Care",
            "description": "Coordinate care appointments",
            "url": "https://example.com/jobs/1",
            "location": "Denver, CO",
            "remote": true
        }),
    );

    let (response, content_type) = handle_import_request(
        &bookmarklet_import_request(&body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("success response should be JSON");
    let previews = pending_bookmarklet_import_previews(&pending_imports);

    assert_eq!(content_type, "application/json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["pending"], 1);
    assert_eq!(parsed["imported"], serde_json::Value::Null);
    assert_eq!(stored_job_count(&database).await, 0);
    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].title, "Care Coordinator");
    assert_eq!(previews[0].company, "Community Care");
    assert_eq!(previews[0].url, "https://example.com/jobs/1");
    assert_eq!(previews[0].location, Some("Denver, CO".to_string()));
    assert!(previews[0].remote);

    let confirm_result =
        confirm_pending_bookmarklet_imports(&database, &pending_imports, &[previews[0].id.clone()])
            .await
            .expect("confirm should save queued import");
    let stored = database.jobs().remove(0);

    assert_eq!(confirm_result.imported, 1);
    assert_eq!(confirm_result.skipped, 0);
    assert_eq!(
        pending_bookmarklet_import_previews(&pending_imports).len(),
        0
    );
    assert_eq!(stored.title, "Care Coordinator");
    assert_eq!(stored.company, "Community Care");
    assert_eq!(stored.source, "user-source-actions");
    assert_eq!(stored.remote, Some(true));
    assert_eq!(
        stored.hash,
        calculate_job_hash(
            "Community Care",
            "Care Coordinator",
            Some("Denver, CO"),
            "https://example.com/jobs/1"
        )
    );
    assert_eq!(stored.location, Some("Denver, CO".to_string()));
}

#[tokio::test]
async fn test_bookmarklet_import_queues_visible_job_batch_for_review_without_insert() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://careers.example.com");
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_batch_body(
        &code,
        serde_json::json!([
            {
                "title": "Principal Systems Security Engineer",
                "company": "Sierra Nevada Corporation",
                "description": "Rendered employer card selected by the user",
                "url": "https://careers.example.com/jobs/100?token=private",
                "location": "Centennial, CO"
            },
            {
                "title": "Lead Platform Security Engineer",
                "company": "HDR",
                "description": "Rendered employer card selected by the user",
                "url": "https://careers.example.com/jobs/200?token=private",
                "location": "Denver, CO"
            }
        ]),
    );

    let (response, content_type) = handle_import_request(
        &bookmarklet_import_request_from_origin(
            &body,
            "https://careers.example.com",
            "https://careers.example.com/jobs/",
        ),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("batch response should be JSON");
    let previews = pending_bookmarklet_import_previews(&pending_imports);

    assert_eq!(content_type, "application/json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["pending"], 2);
    assert_eq!(stored_job_count(&database).await, 0);
    assert_eq!(previews.len(), 2);
    assert_eq!(previews[0].title, "Principal Systems Security Engineer");
    assert_eq!(previews[0].company, "Sierra Nevada Corporation");
    assert_eq!(previews[0].url, "https://careers.example.com/jobs/100");
    assert_eq!(previews[1].title, "Lead Platform Security Engineer");
    assert_eq!(previews[1].company, "HDR");
    assert_eq!(previews[1].url, "https://careers.example.com/jobs/200");
}

#[tokio::test]
async fn test_bookmarklet_import_rejects_batch_origin_mismatch() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://www.linkedin.com");
    let body = bookmarklet_import_batch_body(
        &code,
        serde_json::json!([
            {
                "title": "Principal Systems Security Engineer",
                "company": "Sierra Nevada Corporation",
                "url": "https://www.linkedin.com/jobs/view/100"
            },
            {
                "title": "Lead Platform Security Engineer",
                "company": "HDR",
                "url": "https://evil.example/jobs/200"
            }
        ]),
    );

    let (response, content_type) = handle_import_request(
        &bookmarklet_import_request_from_origin(
            &body,
            "https://www.linkedin.com",
            "https://www.linkedin.com/jobs/",
        ),
        &active_pairing,
        bookmarklet_pending_imports(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("error response should be JSON");

    assert_eq!(content_type, "application/json");
    assert_eq!(parsed["error"], "Invalid browser import origin");
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_import_rejects_invalid_batch_without_partial_insert() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let body = bookmarklet_import_batch_body(
        &code,
        serde_json::json!([
            {
                "title": "Principal Systems Security Engineer",
                "company": "Sierra Nevada Corporation",
                "url": "https://example.com/jobs/100"
            },
            {
                "title": "Missing Company",
                "company": "",
                "url": "https://example.com/jobs/200"
            }
        ]),
    );

    let (response, content_type) = handle_import_request(
        &bookmarklet_import_request_from_origin(
            &body,
            "https://example.com",
            "https://example.com/jobs/",
        ),
        &active_pairing,
        bookmarklet_pending_imports(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("error response should be JSON");

    assert_eq!(content_type, "application/json");
    assert_eq!(parsed["error"], "Company name is required");
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_import_minimizes_url_before_storage() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let pending_imports = bookmarklet_pending_imports();
    let body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Community Care",
            "description": "Coordinate care appointments",
            "url": "https://example.com/jobs?utm_source=browser&gh_jid=123&token=secret&candidate_email=person@example.com&query=care#private",
            "location": "Denver, CO"
        }),
    );

    let (response, content_type) = handle_import_request(
        &bookmarklet_import_request(&body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let parsed: serde_json::Value =
        serde_json::from_str(&response).expect("success response should be JSON");
    let previews = pending_bookmarklet_import_previews(&pending_imports);
    let confirm_result =
        confirm_pending_bookmarklet_imports(&database, &pending_imports, &[previews[0].id.clone()])
            .await
            .expect("confirm should save queued import");
    let stored_url = database.jobs().remove(0).url;

    assert_eq!(content_type, "application/json");
    assert_eq!(parsed["success"], true);
    assert_eq!(parsed["pending"], 1);
    assert_eq!(confirm_result.imported, 1);
    assert_eq!(stored_url, "https://example.com/jobs?gh_jid=123&query=care");
    assert!(!stored_url.contains("utm_source"));
    assert!(!stored_url.contains("token"));
    assert!(!stored_url.contains("candidate_email"));
    assert!(!stored_url.contains("person@example.com"));
    assert!(!stored_url.contains("private"));
}
