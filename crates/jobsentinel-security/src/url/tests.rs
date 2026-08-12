use super::*;

#[test]
fn allows_public_http_urls() {
    assert!(validate_external_http_url("https://example.com/jobs").is_ok());
    assert!(validate_external_http_url("http://example.com/jobs").is_ok());
}

#[test]
fn https_validator_blocks_public_http_urls() {
    assert!(validate_external_https_url("https://example.com/jobs").is_ok());
    assert!(validate_external_https_url("http://example.com/jobs").is_err());
}

#[test]
fn credential_free_https_rejects_private_url_components() {
    assert!(validate_credential_free_external_https_url("https://example.com/provider").is_ok());
    for url in [
        "https://user@example.com/provider",
        "https://example.com/provider?candidate=private",
        "https://example.com/provider#private",
    ] {
        assert!(
            validate_credential_free_external_https_url(url).is_err(),
            "{url}"
        );
    }
}

#[test]
fn canonical_job_url_requires_https() {
    assert!(
        canonicalize_user_supplied_job_url("http://example.com/jobs/123").is_err(),
        "stored job destinations should require https"
    );
    assert!(
        canonicalize_user_supplied_job_url("https://example.com/jobs/123").is_ok(),
        "https job destinations should remain valid"
    );
}

#[test]
fn blocks_localhost_names() {
    assert!(validate_external_http_url("http://localhost:3000/jobs").is_err());
    assert!(validate_external_http_url("http://app.localhost/jobs").is_err());
    assert!(validate_external_http_url("http://localhost./jobs").is_err());
}

#[test]
fn blocks_private_host_suffixes() {
    for url in [
        "http://printer.local/jobs",
        "http://search.lan/jobs",
        "http://jobs.home/jobs",
        "http://ats.internal/jobs",
        "http://intranet.corp/jobs",
    ] {
        assert!(validate_external_http_url(url).is_err(), "{url}");
    }
}

#[test]
fn blocks_private_ipv4_ranges() {
    for url in [
        "http://127.0.0.1/jobs",
        "http://10.0.0.1/jobs",
        "http://172.16.0.1/jobs",
        "http://192.168.1.1/jobs",
        "http://169.254.1.1/jobs",
        "http://100.64.0.1/jobs",
        "http://0.0.0.0/jobs",
    ] {
        assert!(validate_external_http_url(url).is_err(), "{url}");
    }
}

#[test]
fn blocked_private_ip_errors_do_not_echo_host() {
    let err = validate_external_http_url("http://192.168.1.10/internal?token=secret").unwrap_err();

    assert_eq!(err, "Blocked non-public IP address");
    assert!(!err.contains("192.168.1.10"), "host leaked: {err}");
    assert!(!err.contains("secret"), "query leaked: {err}");
}

#[test]
fn blocks_embedded_private_ipv4_hostnames() {
    for url in [
        "http://127.0.0.1.nip.io/jobs",
        "http://192.168.1.5.sslip.io/jobs",
        "http://169.254.1.1.example.com/jobs",
        "http://10.0.0.1.public.example/jobs",
    ] {
        assert!(validate_external_http_url(url).is_err(), "{url}");
    }

    assert!(validate_external_http_url("http://8.8.8.8.example.com/jobs").is_ok());
}

#[test]
fn blocks_private_ipv6_ranges() {
    for url in [
        "http://[::1]/jobs",
        "http://[::]/jobs",
        "http://[fc00::1]/jobs",
        "http://[fd00::1]/jobs",
        "http://[fe80::1]/jobs",
        "http://[::ffff:127.0.0.1]/jobs",
        "http://[::ffff:192.168.1.1]/jobs",
    ] {
        assert!(validate_external_http_url(url).is_err(), "{url}");
    }
}

#[test]
fn blocks_non_http_schemes() {
    assert!(validate_external_http_url("file:///etc/passwd").is_err());
    assert!(validate_external_http_url("javascript:alert(1)").is_err());
}

#[test]
fn blocks_embedded_credentials() {
    for url in [
        "https://user@example.com/jobs",
        "https://user:pass@example.com/jobs",
    ] {
        assert!(validate_external_http_url(url).is_err(), "{url}");
    }
}

#[test]
fn canonical_job_url_strips_sensitive_nested_query_values() {
    let canonical = canonicalize_user_supplied_job_url(
        "https://example.com/jobs/case-manager?jobId=456&redirect=https%3A%2F%2Fprivate.example%2Fcallback%3Ftoken%3Draw-secret&source=mail",
    )
    .expect("public URL should canonicalize");

    assert_eq!(canonical, "https://example.com/jobs/case-manager?jobId=456");
    assert!(!canonical.contains("raw-secret"));
    assert!(!canonical.contains("redirect"));
    assert!(!canonical.contains("source"));
}

#[test]
fn canonical_job_url_strips_the_shared_tracking_policy() {
    let canonical = canonicalize_user_supplied_job_url(
        "https://example.com/jobs/123?jobId=456&twclid=social&aff_id=partner&lever-source=email",
    )
    .expect("public URL should canonicalize");

    assert_eq!(canonical, "https://example.com/jobs/123?jobId=456");
}

#[test]
fn sanitized_log_url_removes_sensitive_parts() {
    let sanitized = sanitize_url_for_logging(
        "https://user:pass@example.com/jobs/123?token=secret&location=Denver#private",
    );

    assert_eq!(sanitized, "https://example.com/jobs/123");
    assert!(!sanitized.contains("secret"));
    assert!(!sanitized.contains("Denver"));
    assert!(!sanitized.contains("user"));
    assert!(!sanitized.contains("pass"));
}

#[test]
fn sanitized_log_url_truncates_long_paths() {
    let sanitized = sanitize_url_for_logging(&format!(
        "https://example.com/jobs/{}?q=rust",
        "a".repeat(120)
    ));

    assert!(sanitized.ends_with("..."));
    assert!(sanitized.len() <= MAX_LOG_URL_LEN + 3);
    assert!(!sanitized.contains("?q="));
}

#[test]
fn sanitized_log_url_handles_invalid_url() {
    let sanitized = sanitize_url_for_logging("not a url with token=secret");

    assert_eq!(sanitized, "<invalid-url>");
}

#[test]
fn resolved_private_ips_are_rejected_without_echoing_host() {
    let err = validate_resolved_ips([IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))]).unwrap_err();

    assert_eq!(err, "Blocked non-public IP address");
    assert!(!err.contains("127.0.0.1"));
}

#[test]
fn empty_resolution_is_rejected() {
    let ips: [IpAddr; 0] = [];

    assert_eq!(
        validate_resolved_ips(ips).unwrap_err(),
        "Could not verify URL host"
    );
}
