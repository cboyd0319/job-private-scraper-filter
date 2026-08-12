use super::*;

#[test]
fn test_bookmarklet_config_serialization() {
    let config = BookmarkletConfigResponse {
        port: 4321,
        enabled: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("4321"));
    assert!(json.contains("true"));
    assert!(!json.contains("authToken"));
    assert!(!json.contains("test-token"));

    let deserialized: BookmarkletConfigResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.port, 4321);
    assert!(deserialized.enabled);
}

#[test]
fn test_bookmarklet_code_is_origin_bound_before_page_access() {
    let (_pairing, pairing_code) = jobsentinel_application::issue_browser_import_pairing(
        "https://jobs.example",
        chrono::Utc::now(),
    )
    .unwrap();
    let request = CompanionRequest {
        protocol_version: pairing_code.protocol_version,
        pairing_id: pairing_code.pairing_id.clone(),
        client_id: pairing_code.client_id.clone(),
        source_id: pairing_code.source_id.clone(),
        policy_ref: pairing_code.policy_ref.clone(),
        policy_revision: pairing_code.policy_revision,
        operation: pairing_code.operations[0],
        origin: pairing_code.origin.clone(),
        nonce: "test-nonce".to_string(),
        token: pairing_code.token.clone(),
    };
    let code = bookmarklet_code(4321, &request, BrowserButtonAction::Import).unwrap();

    assert!(code.contains("http://localhost:4321/api/bookmarklet/import"));
    assert!(code.contains("mode:'no-cors'"));
    assert!(code.contains("targetAddressSpace:'loopback'"));
    assert!(code.contains("cleanWindow.fetch.bind(cleanWindow)"));
    assert!(code.contains("cleanWindow.JSON.stringify.bind(cleanWindow.JSON)"));
    assert!(code.contains("cleanWindow.Element.prototype.getClientRects"));
    assert!(code.contains("cleanWindow.Document.prototype.querySelectorAll"));
    assert!(code.contains("parentNode.removeChild"));
    assert!(code.contains("(linkedin|ycombinator)\\.com"));
    assert!(code.contains("Browser Import is unavailable for this source"));
    assert!(code.contains("https://jobs.example"));
    assert!(code.contains("payload={pairing:{"));
    assert!(code.contains("\"protocol_version\":1"));
    assert!(code.contains("\"operation\":\"visible_page_capture\""));
    assert!(!code.contains("/jobs/view/"));
    assert!(!code.contains("visibleLinkedInJobs"));
    assert!(!code.contains("jobFromAnchor"));
    assert!(code.find("(linkedin|ycombinator)") < code.find("document.createElement"));
    assert!(code.find("location.origin") < code.find("document.createElement"));
    assert!(!code.contains("fetch('http://localhost"));
    assert!(!code.contains("JSON.stringify({token"));
    assert!(!code.contains("application/ld+json"));
    assert!(!code.contains("textContent"));
    assert!(code.contains("Request sent. Return to JobSentinel and check the review list."));
    assert!(code.contains("Return to Settings and copy the browser button again."));
    let old_setup_label = ["import", "helper"].join(" ");
    assert!(!code.contains(&old_setup_label));
    assert!(!code.contains("X-JobSentinel-Token"));
    assert!(code.contains(&pairing_code.token));
}

#[test]
fn applied_logging_button_captures_only_visible_minimum_fields() {
    let (_pairing, pairing_code) = jobsentinel_application::issue_browser_applied_pairing(
        "https://jobs.example",
        chrono::Utc::now(),
    )
    .unwrap();
    let request = CompanionRequest {
        protocol_version: pairing_code.protocol_version,
        pairing_id: pairing_code.pairing_id.clone(),
        client_id: pairing_code.client_id.clone(),
        source_id: pairing_code.source_id.clone(),
        policy_ref: pairing_code.policy_ref.clone(),
        policy_revision: pairing_code.policy_revision,
        operation: pairing_code.operations[0],
        origin: pairing_code.origin.clone(),
        nonce: "test-nonce".to_string(),
        token: pairing_code.token.clone(),
    };

    let code = bookmarklet_code(4321, &request, BrowserButtonAction::Applied).unwrap();

    assert!(code.contains("\"operation\":\"applied_logging\""));
    assert!(!code.contains("[class*=\"description\"]"));
    assert!(!code.contains("[class*=\"desc\"]"));
    assert!(code.contains("Applied draft sent. Return to JobSentinel to review missing details."));
    assert!(!code.contains("application/ld+json"));
    assert!(!code.contains("textContent"));
}

#[test]
fn test_bookmarklet_copy_error_has_safe_support_report_fallback() {
    let message = bookmarklet_copy_error();

    assert!(message.contains("safe support report"));
    assert!(message.contains("Allow clipboard access and try again, or copy"));
}

#[test]
fn test_bookmarklet_port_validation_rejects_reserved_ports() {
    let err = validate_bookmarklet_port(80).unwrap_err();

    assert!(err.contains("1024"));
    assert!(err.contains("65535"));
}

#[test]
fn test_bookmarklet_port_validation_rejects_out_of_range_ports() {
    let err = validate_bookmarklet_port(65_536).unwrap_err();

    assert!(err.contains("1024"));
    assert!(err.contains("65535"));
}

#[test]
fn test_bookmarklet_port_validation_accepts_user_port_range() {
    assert_eq!(validate_bookmarklet_port(1024), Ok(1024));
    assert_eq!(validate_bookmarklet_port(4321), Ok(4321));
    assert_eq!(validate_bookmarklet_port(65535), Ok(65535));
}
