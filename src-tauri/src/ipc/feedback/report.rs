//! Feedback Report Generator
//!
//! Generates fully anonymized feedback reports for beta testers.
//! Reports follow the beta feedback workflow in docs/plans/completed/beta-feedback-system.md.

use chrono::{DateTime, Local, Utc};
use jobsentinel_application::privacy_doctor::{
    inspect_privacy_doctor, BrowserImportPrivacyState, PrivacyDoctorReport,
};
use serde::Serialize;
use tauri::State;

use super::debug_log::{format_event_for_support, get_recent_events};
use super::sanitizer::{ConfigSummary, Sanitizer};
use super::system_info::{summarize_config, SystemInfo};
use crate::bootstrap::{AppState, StartupRecoveryState};

#[derive(Clone, Copy)]
struct StartupRecoverySummary {
    platform: bool,
    configuration: bool,
    database: bool,
}

/// Feedback category
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum FeedbackCategory {
    Bug,
    Feature,
    Question,
}

impl FeedbackCategory {
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::Bug => "Problem Report",
            Self::Feature => "Improvement Idea",
            Self::Question => "General Feedback",
        }
    }
}

/// Generate a complete feedback report (implementation)
///
/// This is the main report generation function. All output is sanitized.
/// Called by the Tauri command wrapper in mod.rs.
pub(super) async fn generate_feedback_report_impl(
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    category: String,
    description: String,
    include_debug_info: bool,
) -> Result<String, String> {
    // Parse category
    let category_enum = match category.as_str() {
        "bug" => FeedbackCategory::Bug,
        "feature" | "idea" => FeedbackCategory::Feature,
        "question" | "other" => FeedbackCategory::Question,
        _ => FeedbackCategory::Question,
    };

    // Get system info
    let system_info = SystemInfo::current();

    let startup_recovery = recovery.required().then(|| StartupRecoverySummary {
        platform: recovery.platform(),
        configuration: recovery.configuration(),
        database: recovery.database(),
    });
    let config = if include_debug_info && startup_recovery.is_none() {
        Some(state.config.read().await.clone())
    } else {
        None
    };
    let config_summary = config.as_ref().map(summarize_config);

    // Get debug log (if requested)
    let debug_events = if include_debug_info {
        get_recent_events(20) // Last 20 events
    } else {
        vec![]
    };

    let privacy_doctor = if let Some(config) = config.as_ref() {
        let browser_import = {
            let server = state.bookmarklet_server.read().await;
            BrowserImportPrivacyState {
                running: server.is_running(),
                code_current: server.pairing_is_current(),
            }
        };
        Some(
            inspect_privacy_doctor(&state.database, config, &state.credentials, browser_import)
                .await,
        )
    } else {
        None
    };

    // Generate report
    let report = format_feedback_report_with_recovery(
        &category_enum,
        &description,
        &system_info,
        config_summary.as_ref(),
        &debug_events,
        privacy_doctor.as_ref(),
        startup_recovery,
    );

    Ok(report)
}

/// Format a feedback report as plain text
#[cfg(test)]
fn format_feedback_report(
    category: &FeedbackCategory,
    description: &str,
    system_info: &SystemInfo,
    config_summary: Option<&ConfigSummary>,
    debug_events: &[super::debug_log::TimestampedEvent],
    privacy_doctor: Option<&PrivacyDoctorReport>,
) -> String {
    format_feedback_report_with_recovery(
        category,
        description,
        system_info,
        config_summary,
        debug_events,
        privacy_doctor,
        None,
    )
}

fn format_feedback_report_with_recovery(
    category: &FeedbackCategory,
    description: &str,
    system_info: &SystemInfo,
    config_summary: Option<&ConfigSummary>,
    debug_events: &[super::debug_log::TimestampedEvent],
    privacy_doctor: Option<&PrivacyDoctorReport>,
    startup_recovery: Option<StartupRecoverySummary>,
) -> String {
    let now: DateTime<Local> = Local::now();
    let now_utc: DateTime<Utc> = Utc::now();

    let mut report = String::new();

    // Header
    report.push_str("═══════════════════════════════════════════════════════════════════════\n");
    report.push_str("                    JOBSENTINEL SAFE SUPPORT REPORT\n");
    report.push_str("═══════════════════════════════════════════════════════════════════════\n");
    report.push_str("\n");

    report.push_str(&format!("Report type: {}\n", category.as_str()));
    report.push_str(&format!(
        "Created: {}\n",
        now.format("%B %d, %Y at %I:%M %p")
    ));
    report.push_str("\n");

    // User feedback
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("WHAT YOU WROTE\n");
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("\n");
    report.push_str(&Sanitizer::sanitize_support_report_text(description));
    report.push_str("\n\n");

    // System information
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("APP AND DEVICE (private details removed)\n");
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("\n");
    report.push_str(&format!("App version: {}\n", system_info.app_version));
    report.push_str(&format!(
        "Device: {} {}\n",
        system_info.platform, system_info.os_version
    ));
    report.push_str(&format!("System type: {}\n", system_info.architecture));
    report.push_str("\n");

    // Config summary (if provided)
    if let Some(summary) = config_summary.filter(|_| startup_recovery.is_none()) {
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("JOBSENTINEL SETUP (counts only)\n");
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("\n");
        report.push_str(&format!(
            "Job sources turned on: {}\n",
            summary.scrapers_enabled
        ));
        report.push_str(&format!("Search words saved: {}\n", summary.keywords_count));
        report.push_str(&format!(
            "Location preferences: {}\n",
            if summary.has_location_prefs {
                "configured"
            } else {
                "not set"
            }
        ));
        report.push_str(&format!(
            "Salary preferences: {}\n",
            if summary.has_salary_prefs {
                "configured"
            } else {
                "not set"
            }
        ));
        report.push_str(&format!(
            "Hidden companies: {}\n",
            if summary.has_blocked_companies {
                "set"
            } else {
                "not set"
            }
        ));
        report.push_str(&format!(
            "Preferred companies: {}\n",
            if summary.has_preferred_companies {
                "set"
            } else {
                "not set"
            }
        ));
        report.push_str(&format!(
            "Notifications: {} channel(s)\n",
            summary.notifications_configured
        ));
        report.push_str("\n");
    }

    // Debug events (if provided)
    if !debug_events.is_empty() {
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("RECENT APP ACTIVITY (private details removed)\n");
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("\n");

        for event in debug_events {
            let time_str = event.timestamp.format("%H:%M:%S");
            report.push_str(&format!(
                "[{}] {}\n",
                time_str,
                format_event_for_support(&event.event)
            ));
        }
        report.push_str("\n");
    }

    if let Some(startup) = startup_recovery {
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("STARTUP RECOVERY\n");
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("\n");
        report.push_str(&format!(
            "platform: {}\n",
            if startup.platform {
                "needs_repair"
            } else {
                "not_reported"
            }
        ));
        report.push_str(&format!(
            "configuration: {}\n",
            if startup.configuration {
                "needs_recovery"
            } else {
                "not_reported"
            }
        ));
        report.push_str(&format!(
            "database: {}\n",
            if startup.database {
                "needs_recovery"
            } else {
                "not_reported"
            }
        ));
        report.push_str("connectivity_required: false\n\n");
    } else if let Some(doctor) = privacy_doctor {
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("LOCAL PRIVACY AND RECOVERY\n");
        report
            .push_str("───────────────────────────────────────────────────────────────────────\n");
        report.push_str("\n");
        report.push_str(&format!(
            "privacy_doctor_overall: {}\n",
            doctor.overall.as_str()
        ));
        report.push_str(&format!(
            "privacy_doctor_connectivity_required: {}\n",
            doctor.connectivity_required
        ));
        for check in &doctor.checks {
            report.push_str(&format!(
                "privacy_doctor_check: {} | {} | {}\n",
                check.id.as_str(),
                check.state.as_str(),
                check.action.map_or("none", |action| action.as_str())
            ));
        }
        report.push_str("\n");
    }

    // Stable key-value support summary. Free text is sanitized before composition.
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("SUPPORT SUMMARY\n");
    report.push_str("───────────────────────────────────────────────────────────────────────\n");
    report.push_str("\n");
    report.push_str("schema_version: 1.1\n");
    report.push_str(&format!("app_version: {}\n", system_info.app_version));
    report.push_str(&format!(
        "category: {}\n",
        match category {
            FeedbackCategory::Bug => "bug",
            FeedbackCategory::Feature => "feature",
            FeedbackCategory::Question => "question",
        }
    ));
    report.push_str(&format!("timestamp: {}\n", now_utc.to_rfc3339()));
    report.push_str(&format!("platform_os: {}\n", system_info.platform));
    report.push_str(&format!(
        "platform_os_version: {}\n",
        system_info.os_version
    ));
    report.push_str(&format!("platform_arch: {}\n", system_info.architecture));
    report.push_str(&format!("debug_events_count: {}\n", debug_events.len()));
    report.push_str(&format!(
        "privacy_doctor_present: {}\n",
        privacy_doctor.is_some() && startup_recovery.is_none()
    ));
    report.push_str("\n");

    // Footer
    report.push_str("═══════════════════════════════════════════════════════════════════════\n");
    report.push_str("                    END OF SAFE SUPPORT REPORT\n");
    report.push_str("═══════════════════════════════════════════════════════════════════════\n");

    report
}

/// Generate suggested filename for feedback report (implementation)
pub(super) fn get_feedback_filename_impl() -> String {
    let now = Local::now();
    format!("jobsentinel-feedback-{}.txt", now.format("%Y-%m-%d-%H%M"))
}

// Note: save_feedback_file command moved to mod.rs

#[cfg(test)]
mod tests;
