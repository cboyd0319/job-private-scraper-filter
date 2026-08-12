use super::*;

#[tokio::test]
async fn test_bookmarklet_import_consumes_pairing_after_first_valid_use() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let pending_imports = bookmarklet_pending_imports();
    let first_body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Community Care",
            "url": "https://example.com/jobs/1"
        }),
    );
    let second_body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Patient Scheduler",
            "company": "Community Care",
            "url": "https://example.com/jobs/2"
        }),
    );

    let (first_response, _) = handle_import_request(
        &bookmarklet_import_request(&first_body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let (second_response, _) = handle_import_request(
        &bookmarklet_import_request(&second_body),
        &active_pairing,
        pending_imports.clone(),
        database.clone(),
    )
    .await;
    let first: serde_json::Value =
        serde_json::from_str(&first_response).expect("first response should be JSON");
    let second: serde_json::Value =
        serde_json::from_str(&second_response).expect("second response should be JSON");

    assert_eq!(first["success"], true);
    assert_eq!(second["error"], BOOKMARKLET_UNAUTHORIZED_MESSAGE);
    assert_eq!(
        pending_bookmarklet_import_previews(&pending_imports).len(),
        1
    );
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_import_consumes_pairing_after_invalid_authenticated_payload() {
    let database = bookmarklet_test_database().await;
    let (active_pairing, code) = bookmarklet_pairing("https://example.com");
    let unsafe_body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Care Coordinator",
            "company": "Community Care",
            "url": "javascript:alert(1)"
        }),
    );
    let valid_body = bookmarklet_import_body(
        &code,
        serde_json::json!({
            "title": "Patient Scheduler",
            "company": "Community Care",
            "url": "https://example.com/jobs/2"
        }),
    );

    let (first_response, _) = handle_import_request(
        &bookmarklet_import_request(&unsafe_body),
        &active_pairing,
        bookmarklet_pending_imports(),
        database.clone(),
    )
    .await;
    let (second_response, _) = handle_import_request(
        &bookmarklet_import_request(&valid_body),
        &active_pairing,
        bookmarklet_pending_imports(),
        database.clone(),
    )
    .await;
    let first: serde_json::Value =
        serde_json::from_str(&first_response).expect("first response should be JSON");
    let second: serde_json::Value =
        serde_json::from_str(&second_response).expect("second response should be JSON");

    assert_eq!(first["error"], "Job link must be a public https address");
    assert_eq!(second["error"], BOOKMARKLET_UNAUTHORIZED_MESSAGE);
    assert_eq!(stored_job_count(&database).await, 0);
}

#[tokio::test]
async fn test_bookmarklet_start_uses_available_loopback_port_when_requested_port_is_occupied() {
    let occupied_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
    let occupied_port = occupied_listener
        .local_addr()
        .expect("test listener should expose address")
        .port();
    let database = bookmarklet_test_database().await;
    let mut server = BookmarkletServer::new(BookmarkletConfig {
        port: occupied_port,
        ..Default::default()
    });
    let config = server.config().clone();

    server
        .start(config, database)
        .await
        .expect("occupied port should fall back to an available loopback port");

    let selected_port = server.config().port;
    assert_ne!(selected_port, occupied_port);
    assert!(selected_port >= 1024);
    assert!(server.is_running());
    server.stop().await.expect("fallback listener should stop");
}

#[tokio::test]
async fn test_bookmarklet_server_lifecycle() {
    let config = BookmarkletConfig {
        port: 0,
        ..Default::default()
    };
    let database = bookmarklet_test_database().await;
    let mut server = BookmarkletServer::new(config.clone());

    assert!(!server.is_running());

    let selected_port = server
        .start(config, database)
        .await
        .expect("loopback listener should start");
    assert_ne!(selected_port, 0);
    assert_eq!(server.config().port, selected_port);
    assert!(server.is_running());

    server.stop().await.expect("loopback listener should stop");
    assert!(!server.is_running());
}
