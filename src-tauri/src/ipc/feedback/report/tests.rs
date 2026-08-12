use super::*;
use crate::ipc::feedback::debug_log::{DebugEvent, TimestampedEvent};
use chrono::Utc;
use jobsentinel_application::privacy_doctor::{
    PrivacyDoctorAction, PrivacyDoctorCheck, PrivacyDoctorCheckId, PrivacyDoctorState,
};

#[test]
fn test_format_feedback_report_minimal() {
    let category = FeedbackCategory::Bug;
    let description = "Test bug description";
    let system_info = SystemInfo {
        app_version: "2.6.3".to_string(),
        platform: "macos".to_string(),
        os_version: "14.0".to_string(),
        architecture: "arm64".to_string(),
    };

    let report = format_feedback_report(&category, description, &system_info, None, &[], None);

    assert!(report.contains("JOBSENTINEL SAFE SUPPORT REPORT"));
    assert!(report.contains("Report type: Problem Report"));
    assert!(report.contains("WHAT YOU WROTE"));
    assert!(report.contains("Test bug description"));
    assert!(report.contains("APP AND DEVICE"));
    assert!(report.contains("App version: 2.6.3"));
    assert!(report.contains("Device: macos 14.0"));
    assert!(report.contains("System type: arm64"));
    assert!(report.contains("SUPPORT SUMMARY"));
    assert!(report.contains("END OF SAFE SUPPORT REPORT"));
}

#[test]
fn test_format_feedback_report_with_config() {
    let category = FeedbackCategory::Feature;
    let description = "Add dark mode";
    let system_info = SystemInfo::current();
    let config_summary = ConfigSummary {
        scrapers_enabled: 3,
        keywords_count: 5,
        has_location_prefs: true,
        has_salary_prefs: true,
        has_blocked_companies: false,
        has_preferred_companies: true,
        notifications_configured: 2,
        has_resume: true,
    };

    let report = format_feedback_report(
        &category,
        description,
        &system_info,
        Some(&config_summary),
        &[],
        None,
    );

    assert!(report.contains("JOBSENTINEL SETUP"));
    assert!(report.contains("Job sources turned on: 3"));
    assert!(report.contains("Search words saved: 5"));
    assert!(report.contains("Location preferences: configured"));
    assert!(report.contains("Salary preferences: configured"));
    assert!(report.contains("Hidden companies: not set"));
    assert!(report.contains("Preferred companies: set"));
    assert!(report.contains("Notifications: 2 channel(s)"));
}

#[test]
fn startup_recovery_report_omits_placeholder_setup_and_privacy() {
    let config_summary = ConfigSummary {
        scrapers_enabled: 0,
        keywords_count: 0,
        has_location_prefs: false,
        has_salary_prefs: false,
        has_blocked_companies: false,
        has_preferred_companies: false,
        notifications_configured: 0,
        has_resume: false,
    };
    let report = format_feedback_report_with_recovery(
        &FeedbackCategory::Bug,
        "Startup recovery",
        &SystemInfo::current(),
        Some(&config_summary),
        &[],
        None,
        Some(StartupRecoverySummary {
            platform: false,
            configuration: false,
            database: true,
        }),
    );

    assert!(report.contains("STARTUP RECOVERY"));
    assert!(report.contains("database: needs_recovery"));
    assert!(!report.contains("JOBSENTINEL SETUP"));
    assert!(!report.contains("LOCAL PRIVACY AND RECOVERY"));
}

#[test]
fn test_format_feedback_report_with_debug_events() {
    let category = FeedbackCategory::Bug;
    let description = "Scraper failed";
    let system_info = SystemInfo::current();

    let events = vec![
        TimestampedEvent {
            timestamp: Utc::now(),
            event: DebugEvent::ViewNavigated {
                from: "Jobs".to_string(),
                to: "Dashboard".to_string(),
            },
        },
        TimestampedEvent {
            timestamp: Utc::now(),
            event: DebugEvent::ScraperRun {
                scraper: "indeed".to_string(),
                jobs_found: 0,
                success: false,
            },
        },
    ];

    let report = format_feedback_report(&category, description, &system_info, None, &events, None);

    assert!(report.contains("RECENT APP ACTIVITY"));
    assert!(report.contains("Screen changed: Jobs to Dashboard"));
    assert!(report.contains("Job source checked: indeed; Result: failed; Jobs found: 0"));
    assert!(report.contains("indeed"));
    assert!(!report.contains("ViewNavigated"));
    assert!(!report.contains("ScraperRun"));
}

#[test]
fn test_report_sanitizes_description() {
    let category = FeedbackCategory::Bug;
    let description = concat!(
        "Error at /",
        "Users",
        "/johnsmith/file.txt\n",
        "Email john@example.com\n",
        "Salary floor: $125,000\n",
        "Resume text: Led retention project for oncology team\n",
        "Private note: laid off last month\n"
    );
    let system_info = SystemInfo::current();

    let report = format_feedback_report(&category, description, &system_info, None, &[], None);

    assert!(report.contains("[LOCAL_PATH]"));
    assert!(report.contains("[EMAIL]"));
    assert!(report.contains("Salary floor: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(report.contains("Resume text: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(report.contains("Private note: [JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(!report.contains("johnsmith"));
    assert!(!report.contains("file.txt"));
    assert!(!report.contains("john@example.com"));
    assert!(!report.contains("$125,000"));
    assert!(!report.contains("oncology team"));
    assert!(!report.contains("laid off"));
}

#[test]
fn test_report_redacts_unlabeled_job_search_narrative() {
    let category = FeedbackCategory::Bug;
    let description = r#"Issue while applying to "Acme Health" for care manager role after layoff"#;
    let system_info = SystemInfo::current();

    let report = format_feedback_report(&category, description, &system_info, None, &[], None);

    assert!(report.contains("[JOB_SEARCH_DETAIL_REDACTED]"));
    assert!(!report.contains("Acme Health"));
    assert!(!report.contains("care manager"));
    assert!(!report.contains("layoff"));
}

#[test]
fn test_feedback_category_as_str() {
    assert_eq!(FeedbackCategory::Bug.as_str(), "Problem Report");
    assert_eq!(FeedbackCategory::Feature.as_str(), "Improvement Idea");
    assert_eq!(FeedbackCategory::Question.as_str(), "General Feedback");
}

#[test]
fn safe_support_report_includes_only_typed_local_privacy_and_recovery_states() {
    let privacy_doctor = PrivacyDoctorReport {
        schema_version: 1,
        overall: PrivacyDoctorState::NeedsAttention,
        checks: vec![PrivacyDoctorCheck {
            id: PrivacyDoctorCheckId::BackupHistory,
            state: PrivacyDoctorState::NeedsAttention,
            message: "Portable backup history could not be checked.",
            action: Some(PrivacyDoctorAction::ReviewBackup),
            connectivity_required: false,
        }],
        connectivity_required: false,
    };
    let report = format_feedback_report(
        &FeedbackCategory::Bug,
        "User generated a safe support report from JobSentinel.",
        &SystemInfo::current(),
        None,
        &[],
        Some(&privacy_doctor),
    );

    assert!(report.contains("LOCAL PRIVACY AND RECOVERY"));
    assert!(report.contains("schema_version: 1.1"));
    assert!(report.contains("privacy_doctor_overall: needs_attention"));
    assert!(
        report.contains("privacy_doctor_check: backup_history | needs_attention | review_backup")
    );
    assert!(report.contains("privacy_doctor_connectivity_required: false"));
    let delivered = Sanitizer::sanitize_support_report_text(&report);
    assert!(delivered.contains("schema_version: 1.1"));
    assert!(delivered.contains("privacy_doctor_overall: needs_attention"));
    assert!(delivered
        .contains("privacy_doctor_check: backup_history | needs_attention | review_backup"));
    for private_detail in [
        "/Users/private/jobs.db",
        "Alice Resume.pdf",
        "external_ai_openai_api_key",
        "https://provider.example/private",
    ] {
        assert!(!report.contains(private_detail));
    }
}
