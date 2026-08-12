use super::*;

fn mac_user_path(user: &str, suffix: &str) -> String {
    format!("/{}/{user}/{suffix}", "Users")
}

fn linux_home_path(user: &str, suffix: &str) -> String {
    format!("/{}/{user}/{suffix}", "home")
}

fn windows_user_path(drive: &str, user: &str, suffix: &str) -> String {
    format!(r"{drive}:\{}\{user}\{suffix}", "Users")
}

#[test]
fn test_sanitize_file_paths() {
    let input = format!(
        "Error reading {}",
        mac_user_path("johnsmith", "Documents/jobs.db")
    );
    let output = Sanitizer::sanitize(&input);
    assert_eq!(output, "Error reading /[USER_PATH]/Documents/jobs.db");
}

#[test]
fn test_sanitize_linux_paths() {
    let input = format!(
        "Error reading {}",
        linux_home_path("johnsmith", ".config/jobsentinel/jobs.db")
    );
    let output = Sanitizer::sanitize(&input);
    assert_eq!(
        output,
        "Error reading /[USER_PATH]/.config/jobsentinel/jobs.db"
    );
}

#[test]
fn test_sanitize_windows_paths() {
    let input = format!(
        "Error reading {}",
        windows_user_path("C", "JohnSmith", r"Documents\jobs.db")
    );
    let output = Sanitizer::sanitize(&input);
    assert!(!output.contains("JohnSmith"));
    assert!(output.contains("[USER_PATH]"));
    assert_eq!(output, r"Error reading C:\[USER_PATH]\Documents\jobs.db");
}

#[test]
fn test_sanitize_windows_paths_different_drives() {
    let tests = vec![
        (
            windows_user_path("D", "Alice", r"AppData\Local\JobSentinel\config.json"),
            r"C:\[USER_PATH]\AppData\Local\JobSentinel\config.json",
        ),
        (
            windows_user_path("E", "bob.johnson", r"Desktop\resume.pdf"),
            r"C:\[USER_PATH]\Desktop\resume.pdf",
        ),
        (
            windows_user_path("C", "admin", r"Downloads\jobs.csv"),
            r"C:\[USER_PATH]\Downloads\jobs.csv",
        ),
    ];

    for (input, expected) in tests {
        let output = Sanitizer::sanitize(&input);
        assert!(!output.contains("Alice"));
        assert!(!output.contains("bob.johnson"));
        assert!(!output.contains("admin"));
        assert_eq!(output, expected);
    }
}

#[test]
fn test_sanitize_mixed_platform_paths() {
    let input = format!(
        "Syncing from {} to {}",
        mac_user_path("johnsmith", "Desktop"),
        windows_user_path("C", "JohnSmith", "Documents")
    );
    let output = Sanitizer::sanitize(&input);
    assert!(!output.contains("johnsmith"));
    assert!(!output.contains("JohnSmith"));
    assert_eq!(
        output,
        r"Syncing from /[USER_PATH]/Desktop to C:\[USER_PATH]\Documents"
    );
}

#[test]
fn support_reports_remove_complete_local_paths_and_private_filenames() {
    let input = format!(
        concat!(
            "mac {}\nlinux {}\nwindows {}\ntemp /tmp/jobsentinel/Case Manager Resume.pdf\n",
            "unc \\\\server\\Veteran Files\\Alice Resume.pdf\n",
            "extended \\\\?\\C:\\Users\\Alice\\Documents\\Private Resume.pdf\n",
            "forward C:/Users/Alice/Documents/Clinical Resume.pdf\n",
            "mount /mnt/private/Alice Resume.pdf\n",
            "network //server/share/Alice Resume.pdf\n",
            "backtick-win `C:\\Users\\Alice\\Documents\\Backtick Resume.pdf`\n",
            "backtick-unix `/opt/private/Backtick Notes.txt`\n",
            "backtick-unc `\\\\server\\share\\Backtick File.txt`\n",
            "nbsp\u{00a0}\\\\server\\share\\NBSP Resume.pdf\n",
            "comma, /srv/private/Comma Notes.txt"
        ),
        mac_user_path("Alice", "Documents/Alice Resume.pdf"),
        linux_home_path("bob", "Downloads/Acme Health Notes.txt"),
        windows_user_path("D", "Casey", r"Desktop\Veteran Transition Plan.docx"),
    );

    let output = Sanitizer::sanitize_support_report_text(&input);

    assert_eq!(output.matches("[LOCAL_PATH]").count(), 14);
    assert!(output.ends_with("[LOCAL_PATH]"));
    for private_detail in [
        "Alice",
        "Resume.pdf",
        "bob",
        "Acme Health",
        "Casey",
        "Veteran Transition",
        "Case Manager",
        "server",
        "Private Resume",
        "Clinical Resume",
        "/mnt",
        "Backtick Resume",
        "Backtick Notes",
        "Backtick File",
        "NBSP Resume",
        "Comma Notes",
    ] {
        assert!(!output.contains(private_detail), "{output}");
    }
}

#[test]
fn test_sanitize_non_home_unix_paths() {
    let input = concat!(
        "Temp config /private/var/folders/zz/abc123/T/jobsentinel/config.json ",
        "cache /var/folders/aa/bb/T/resume-name.pdf ",
        "linux /tmp/jobsentinel/private-resume.docx ",
        "runtime /run/user/1000/jobsentinel/jobs.db"
    );
    let output = Sanitizer::sanitize(input);

    assert!(output.contains("/[LOCAL_PATH]"));
    assert!(!output.contains("/private/var"));
    assert!(!output.contains("/var/folders"));
    assert!(!output.contains("/tmp/jobsentinel"));
    assert!(!output.contains("/run/user/1000"));
    assert!(!output.contains("resume-name.pdf"));
}

#[test]
fn test_sanitize_non_home_windows_paths() {
    let input = concat!(
        r#"Temp "C:\Temp\JobSentinel\config.json" "#,
        r#"project D:\Job Search\Acme Services\resume final.pdf "#,
        r#"program data C:\ProgramData\JobSentinel\jobs.db"#
    );
    let output = Sanitizer::sanitize(input);

    assert!(output.contains(r#"C:\[LOCAL_PATH]"#));
    assert!(!output.contains(r#"C:\Temp"#));
    assert!(!output.contains(r#"D:\Job Search"#));
    assert!(!output.contains(r#"Acme Services"#));
    assert!(!output.contains("resume final.pdf"));
    assert!(!output.contains(r#"C:\ProgramData"#));
}

#[test]
fn test_sanitize_emails() {
    let input = "User email: john.doe@example.com configured";
    let output = Sanitizer::sanitize(input);
    assert_eq!(output, "User email: [EMAIL] configured");
}

#[test]
fn test_sanitize_phone_numbers_and_person_names() {
    let input = concat!(
        "Phone: +1 (303) 555-1212\n",
        "backup 720-555-9911\n",
        "Full name: Chad Boyd\n",
        "My name is Chad Boyd\n",
        "Company name is CareBridge Health\n",
    );
    let output = Sanitizer::sanitize(input);

    assert!(output.contains("Phone: [PHONE]"));
    assert!(output.contains("backup [PHONE]"));
    assert!(output.contains("Full name: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("My name [PERSON_NAME_REDACTED]"));
    assert!(output.contains("Company name is CareBridge Health"));
    assert!(!output.contains("303"));
    assert!(!output.contains("720"));
    assert!(!output.contains("Chad Boyd"));
}

#[test]
fn test_sanitize_webhooks() {
    let tests = vec![
        (
            "Slack webhook: https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXX",
            "Slack webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Discord webhook: https://discord.com/api/webhooks/123456789/abcdefg",
            "Discord webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Discord webhook: https://discordapp.com/api/webhooks/123456789/abcdefg",
            "Discord webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Teams webhook: https://outlook.office.com/webhook/abc123/IncomingWebhook/def456/ghi789",
            "Teams webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Teams webhook: https://outlook.office365.com/webhook/abc123/IncomingWebhook/def456/ghi789",
            "Teams webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Teams webhook: https://tenant.webhook.office.com/abc123/IncomingWebhook/def456/ghi789",
            "Teams webhook: [WEBHOOK_CONFIGURED]",
        ),
        (
            "Teams workflow: https://prod-12.westus.logic.azure.com:443/workflows/abc123/triggers/manual/paths/invoke",
            "Teams workflow: [WEBHOOK_CONFIGURED]",
        ),
    ];

    for (input, expected) in tests {
        let output = Sanitizer::sanitize(input);
        assert_eq!(output, expected);
    }
}

#[test]
fn test_sanitize_linkedin_cookies() {
    let input = "Cookie: li_at=AQEDARa1234567890abcdefg; Path=/";
    let output = Sanitizer::sanitize(input);
    assert_eq!(output, "Cookie: li_at=[REDACTED]; Path=/");
}

#[test]
fn test_sanitize_api_tokens() {
    let tests = vec![
        ("Bearer eyJ0eXAiOiJKV1QiLCJhbGc...", "[TOKEN]"),
        (
            "Authorization: token ghp_1234567890abcdefg",
            "Authorization: [TOKEN]",
        ),
        (
            "URL: /api?api_key=secret123&foo=bar",
            "URL: /api?[TOKEN]&foo=bar",
        ),
        (
            "Callback included access_token=abc123&state=ok",
            "Callback included [TOKEN]&state=ok",
        ),
        (
            "refresh_token=abc123 secret=hidden password=hunter2",
            "[TOKEN] [TOKEN] [TOKEN]",
        ),
        (r#""X-JobSentinel-Token":"local-secret""#, "[TOKEN]"),
    ];

    for (input, expected) in tests {
        let output = Sanitizer::sanitize(input);
        assert_eq!(output, expected);
    }
}

#[test]
fn test_sanitize_generic_urls() {
    let input = "Pasted job link https://example.com/jobs/123?candidate=alice&token=abc and http://localhost:4321/api/bookmarklet/import";
    let output = Sanitizer::sanitize(input);

    assert_eq!(output, "Pasted job link [URL] and [URL]");
}

#[test]
fn test_sanitize_bookmarklet_code() {
    let input = r#"fetch('http://localhost:4321/api/bookmarklet/import',{headers:{'X-JobSentinel-Token':"secret-token"},body:JSON.stringify(job)});"#;
    let output = Sanitizer::sanitize(input);

    assert!(!output.contains("localhost:4321"));
    assert!(!output.contains("secret-token"));
    assert!(output.contains("[URL]"));
    assert!(output.contains("[TOKEN]"));
}

#[test]
fn test_sanitize_ip_addresses() {
    let input = "Connected to 192.168.1.100:8080";
    let output = Sanitizer::sanitize(input);
    assert_eq!(output, "Connected to [IP_ADDRESS]:8080");
}

#[test]
fn test_sanitize_multiple_patterns() {
    let input = format!(
        "User john@example.com uploaded resume from {}, webhook https://hooks.slack.com/services/ABC",
        mac_user_path("johnsmith", "Desktop/resume.pdf")
    );
    let output = Sanitizer::sanitize(&input);
    assert_eq!(
        output,
        "User [EMAIL] uploaded resume from /[USER_PATH]/Desktop/resume.pdf, webhook [WEBHOOK_CONFIGURED]"
    );
}

#[test]
fn test_sanitize_preserves_safe_content() {
    let input = "Scraper run: Indeed found 42 jobs, command succeeded";
    let output = Sanitizer::sanitize(input);
    assert_eq!(output, input); // Should be unchanged
}

#[test]
fn test_sanitize_job_search_sensitive_context() {
    let input = concat!(
        "Problem summary: app froze while saving feedback\n",
        "Salary floor: $125,000 remote minimum\n",
        "Resume excerpt: Led retention project for oncology team\n",
        "Private note: laid off last month and urgent search\n",
        "Application history: rejected by CareBridge after phone screen\n",
        "Screening answer: I need sponsorship next year\n",
        "Location preference: Denver only because caregiving schedule\n",
        "Career goal: move out of night shifts\n",
        "Personal circumstances: unemployed for eight months\n",
        "My salary floor is 125000 before bonus\n",
    );

    let output = Sanitizer::sanitize(input);

    assert!(output.contains("Problem summary: app froze while saving feedback"));
    assert!(output.contains("Salary floor: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Resume excerpt: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Private note: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Application history: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Screening answer: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Location preference: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Career goal: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("Personal circumstances: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(output.contains("My salary floor [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(!output.contains("$125,000"));
    assert!(!output.contains("oncology team"));
    assert!(!output.contains("laid off"));
    assert!(!output.contains("CareBridge"));
    assert!(!output.contains("sponsorship next year"));
    assert!(!output.contains("Denver"));
    assert!(!output.contains("night shifts"));
    assert!(!output.contains("unemployed for eight months"));
    assert!(!output.contains("125000 before bonus"));
}

#[test]
fn test_sanitize_error_removes_quoted_strings() {
    let input = "Failed to find job titled \"Senior Care Coordinator\"";
    let output = Sanitizer::sanitize_error(input);
    assert_eq!(output, "Failed to find job titled \"[REDACTED]\"");

    let input = "Company 'Example Health' not found in blocklist";
    let output = Sanitizer::sanitize_error(input);
    assert_eq!(output, "Company '[REDACTED]' not found in blocklist");
}

#[test]
fn test_sanitize_error_complex() {
    let input = format!(
        "Error loading config from {}: Failed to parse webhook https://hooks.slack.com/T123/B456/xyz for user john@example.com. \
        LinkedIn cookie li_at=AQEDA123 is invalid. Search for \"customer support lead\" failed.",
        mac_user_path("john", "Library/JobSentinel/config.json")
    );

    let output = Sanitizer::sanitize_error(&input);

    // Should redact file paths, webhooks, emails, cookies, quoted strings
    assert!(output.contains("/[USER_PATH]/Library"));
    assert!(output.contains("[WEBHOOK_CONFIGURED]"));
    assert!(output.contains("[EMAIL]"));
    assert!(output.contains("li_at=[REDACTED]"));
    assert!(output.contains("\"[REDACTED]\""));
}

#[test]
fn test_sanitize_error_complex_windows() {
    let input = format!(
        r"Error loading config from {}: \
        Failed to parse webhook https://discord.com/api/webhooks/123/abc for user admin@company.com. \
        Database at {} is locked. Search for {} failed.",
        windows_user_path(
            "C",
            "Administrator",
            r"AppData\Roaming\JobSentinel\config.json"
        ),
        windows_user_path("D", "dbuser", r"Documents\jobs.db"),
        "\"case manager\""
    );

    let output = Sanitizer::sanitize_error(&input);

    // Should redact Windows paths, webhooks, emails, quoted strings
    assert!(output.contains(r"C:\[USER_PATH]\AppData\Roaming"));
    assert!(output.contains(r"C:\[USER_PATH]\Documents\jobs.db")); // D: drive normalized to C:
    assert!(output.contains("[WEBHOOK_CONFIGURED]"));
    assert!(output.contains("[EMAIL]"));
    assert!(output.contains("\"[REDACTED]\""));
    assert!(!output.contains("Administrator"));
    assert!(!output.contains("dbuser"));
    assert!(!output.contains("admin@company.com"));
}
