//! Beta Feedback System
//!
//! Privacy-first feedback collection for beta testers.
//! ALL output is sanitized to prevent PII leakage (this is a PUBLIC repo).

mod debug_log;
mod report;
mod sanitizer;
mod system_info;

use debug_log::{clear_debug_log, format_debug_log, get_debug_log, TimestampedEvent};
use sanitizer::{ConfigSummary, Sanitizer};
use system_info::SystemInfo;

use crate::bootstrap::{AppState, StartupRecoveryState};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid;

static SAVED_FEEDBACK_FILES: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedFeedbackFile {
    pub file_name: String,
    pub reveal_token: String,
}

/// Validate that a caller-supplied path is safe to reveal in the file manager.
///
/// Kept as a regression helper for the retired raw-path reveal command. The
/// live reveal flow uses opaque tokens created by `save_feedback_file`.
#[cfg(test)]
fn validate_reveal_path(path: &str) -> Result<PathBuf, String> {
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let canonical = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| "File not found or inaccessible".to_string())?;

    // Restrict to safe directories: home dir and app data dir
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .ok();
    let data_dir = crate::desktop::get_data_dir();

    let is_allowed = [home_dir.as_ref(), Some(&data_dir)]
        .iter()
        .any(|dir| dir.map(|d| canonical.starts_with(d)).unwrap_or(false));

    if !is_allowed {
        return Err("Access denied: path outside allowed directories".to_string());
    }

    Ok(canonical)
}

fn feedback_save_path(file_path: tauri_plugin_dialog::FilePath) -> Result<PathBuf, String> {
    file_path
        .into_path()
        .map_err(|_| "Invalid feedback file path".to_string())
}

fn feedback_suggested_filename(suggested_filename: Option<String>) -> String {
    suggested_filename
        .and_then(|name| {
            PathBuf::from(name)
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(report::get_feedback_filename_impl)
}

fn feedback_file_content(content: &str) -> String {
    Sanitizer::sanitize_support_report_text(content)
}

fn feedback_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(report::get_feedback_filename_impl)
}

fn remember_feedback_file(path: PathBuf) -> Result<SavedFeedbackFile, String> {
    let reveal_token = Uuid::new_v4().to_string();
    let file_name = feedback_file_name(&path);

    let mut files = SAVED_FEEDBACK_FILES
        .lock()
        .map_err(|_| "Failed to prepare saved feedback report".to_string())?;
    files.insert(reveal_token.clone(), path);

    Ok(SavedFeedbackFile {
        file_name,
        reveal_token,
    })
}

fn feedback_page_open_error() -> String {
    "Could not open the support page automatically. Open it manually if this keeps happening."
        .to_string()
}

fn feedback_reveal_error() -> String {
    "Could not show the saved support report automatically. Open it from the folder where you saved it."
        .to_string()
}

/// Open GitHub Issues page for bug reports
#[tauri::command]
#[allow(deprecated)]
pub(crate) async fn open_github_issues(
    app: AppHandle,
    template: Option<String>,
) -> Result<(), String> {
    let base_url = "https://github.com/cboyd0319/JobSentinel/issues/new";

    let url = if let Some(tmpl) = template {
        match tmpl.as_str() {
            "bug" => format!("{}?template=bug_report.yml", base_url),
            "feature" => format!("{}?template=feature_request.yml", base_url),
            "question" => format!("{}?template=question.yml", base_url),
            _ => base_url.to_string(),
        }
    } else {
        base_url.to_string()
    };

    app.shell()
        .open(&url, None)
        .map_err(|_| feedback_page_open_error())?;

    Ok(())
}

fn reveal_canonical_path(app: AppHandle, canonical: PathBuf) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = &app;
        use std::process::Command;
        Command::new("open")
            .arg("-R")
            .arg(canonical.as_os_str())
            .spawn()
            .map_err(|_| feedback_reveal_error())?;
    }

    #[cfg(target_os = "windows")]
    {
        let _ = &app;
        use std::process::Command;
        let path_str = canonical.to_str().ok_or_else(feedback_reveal_error)?;
        Command::new("explorer")
            .arg(format!("/select,{}", path_str))
            .spawn()
            .map_err(|_| feedback_reveal_error())?;
    }

    #[cfg(target_os = "linux")]
    {
        let parent = canonical
            .parent()
            .ok_or_else(feedback_reveal_error)?
            .to_str()
            .ok_or_else(feedback_reveal_error)?;

        #[allow(deprecated)]
        app.shell()
            .open(parent, None)
            .map_err(|_| feedback_reveal_error())?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn reveal_saved_feedback_file(
    app: AppHandle,
    reveal_token: String,
) -> Result<(), String> {
    let path = SAVED_FEEDBACK_FILES
        .lock()
        .ok()
        .and_then(|files| files.get(&reveal_token).cloned())
        .ok_or_else(|| "Saved feedback file is no longer available".to_string())?;

    reveal_canonical_path(app, path)
}

// ============================================================================
// System Info & Config Summary Commands
// ============================================================================

/// Get system information (Tauri command)
#[tauri::command]
pub(crate) async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo::current())
}

/// Get configuration summary (Tauri command)
#[tauri::command]
pub(crate) async fn get_config_summary(
    state: State<'_, AppState>,
) -> Result<ConfigSummary, String> {
    let config = state.config.read().await;
    Ok(system_info::summarize_config(&config))
}

/// Generate a complete feedback report
#[tauri::command]
pub(crate) async fn generate_feedback_report(
    state: State<'_, AppState>,
    recovery: State<'_, StartupRecoveryState>,
    category: String,
    description: String,
    include_debug_info: bool,
) -> Result<String, String> {
    report::generate_feedback_report_impl(
        state,
        recovery,
        category,
        description,
        include_debug_info,
    )
    .await
}

/// Sanitize renderer-composed feedback text before clipboard or file use.
#[tauri::command]
pub(crate) fn sanitize_feedback_text(content: String) -> String {
    feedback_file_content(&content)
}

/// Generate suggested filename for feedback report
#[tauri::command]
pub(crate) fn get_feedback_filename() -> String {
    report::get_feedback_filename_impl()
}

/// Save a feedback report using a native file dialog.
#[tauri::command]
pub(crate) async fn save_feedback_file(
    app: tauri::AppHandle,
    content: String,
    suggested_filename: Option<String>,
) -> Result<Option<SavedFeedbackFile>, String> {
    let suggested_filename = feedback_suggested_filename(suggested_filename);
    let Some(file_path) = app
        .dialog()
        .file()
        .add_filter("Text", &["txt"])
        .set_file_name(suggested_filename)
        .blocking_save_file()
    else {
        return Ok(None);
    };

    let path = feedback_save_path(file_path)?;
    std::fs::write(&path, feedback_file_content(&content))
        .map_err(|_| "Failed to save feedback report".to_string())?;
    let canonical = path
        .canonicalize()
        .map_err(|_| "Failed to confirm saved feedback report".to_string())?;

    Ok(Some(remember_feedback_file(canonical)?))
}

// ============================================================================
// Debug Log Commands
// ============================================================================

/// Get the debug log as a formatted string (sanitized)
///
/// Returns the last 100 debug events, fully anonymized.
/// Safe to include in bug reports and feedback submissions.
#[tauri::command]
pub(crate) async fn get_debug_log_formatted(_state: State<'_, AppState>) -> Result<String, String> {
    Ok(format_debug_log())
}

/// Get the raw debug log events (sanitized)
///
/// Returns structured event data. Frontend can format as needed.
#[tauri::command]
pub(crate) async fn get_debug_log_events(
    _state: State<'_, AppState>,
) -> Result<Vec<TimestampedEvent>, String> {
    Ok(get_debug_log())
}

/// Clear the debug log
///
/// Useful for testing or resetting diagnostic state.
#[tauri::command]
pub(crate) async fn clear_debug_log_cmd(_state: State<'_, AppState>) -> Result<(), String> {
    clear_debug_log();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_issue_url_generation() {
        let base = "https://github.com/cboyd0319/JobSentinel/issues/new";

        assert_eq!(
            format!("{}?template=bug_report.yml", base),
            "https://github.com/cboyd0319/JobSentinel/issues/new?template=bug_report.yml"
        );

        assert_eq!(
            format!("{}?template=feature_request.yml", base),
            "https://github.com/cboyd0319/JobSentinel/issues/new?template=feature_request.yml"
        );
    }

    // ========================================================================
    // Security: retired raw-path reveal validation (CWE-22 Path Traversal)
    // ========================================================================

    #[test]
    fn test_validate_reveal_path_rejects_traversal() {
        // Path traversal with ".." should be rejected
        let result = validate_reveal_path("/tmp/../../../etc/passwd");
        assert!(result.is_err(), "Path traversal with .. must be rejected");
    }

    #[test]
    fn test_validate_reveal_path_rejects_outside_home() {
        // Paths outside user's home directory should be rejected
        let result = validate_reveal_path("/etc/passwd");
        assert!(result.is_err(), "Paths outside home dir must be rejected");
    }

    #[test]
    fn test_validate_reveal_path_rejects_system_dirs() {
        let system_paths = ["/etc/shadow", "/var/log/system.log", "/usr/bin/sudo"];
        for path in &system_paths {
            let result = validate_reveal_path(path);
            assert!(result.is_err(), "System path {} must be rejected", path);
        }
    }

    #[test]
    fn test_validate_reveal_path_allows_home_dir() {
        // Paths within $HOME should be allowed (if they exist)
        if let Ok(home) = std::env::var("HOME") {
            // Use a path we know exists
            let result = validate_reveal_path(&home);
            assert!(result.is_ok(), "Home dir itself should be allowed");
        }
    }

    #[test]
    fn test_validate_reveal_path_rejects_nonexistent() {
        let result = validate_reveal_path("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err(), "Nonexistent paths must be rejected");
    }

    #[test]
    fn test_validate_reveal_path_rejects_empty() {
        let result = validate_reveal_path("");
        assert!(result.is_err(), "Empty path must be rejected");
    }

    #[test]
    fn test_feedback_suggested_filename_strips_path_components() {
        let filename = feedback_suggested_filename(Some("../jobsentinel-feedback.txt".to_string()));
        assert_eq!(filename, "jobsentinel-feedback.txt");
    }

    #[test]
    fn test_feedback_suggested_filename_uses_generated_fallback() {
        let filename = feedback_suggested_filename(Some("   ".to_string()));
        assert!(filename.starts_with("jobsentinel-feedback-"));
        assert!(filename.ends_with(".txt"));
    }

    #[test]
    fn test_feedback_reveal_error_uses_support_report_wording() {
        let message = feedback_reveal_error();
        assert!(message.contains("saved support report"));
        assert!(!message.contains("debug report"));
    }

    #[test]
    fn test_feedback_file_content_is_sanitized_before_write() {
        let content = concat!(
            "User john@example.com saved report from /",
            "Users",
            "/johnsmith/Desktop/report.txt\n",
            "Webhook https://discord.com/api/webhooks/123456789/secret-token\n",
            "Salary floor: $125,000\n",
            "Resume excerpt: Led retention project for oncology team\n",
            "Private note: laid off last month\n"
        );

        let sanitized = feedback_file_content(content);

        assert!(!sanitized.contains("john@example.com"));
        assert!(!sanitized.contains("johnsmith"));
        assert!(!sanitized.contains("discord.com/api/webhooks"));
        assert!(sanitized.contains("[EMAIL]"));
        assert!(sanitized.contains("[LOCAL_PATH]"));
        assert!(sanitized.contains("[WEBHOOK_CONFIGURED]"));
        assert!(sanitized.contains("Salary floor: [JOB_SEARCH_DETAIL_REDACTED]"));
        assert!(sanitized.contains("Resume excerpt: [JOB_SEARCH_DETAIL_REDACTED]"));
        assert!(sanitized.contains("Private note: [JOB_SEARCH_DETAIL_REDACTED]"));
        assert!(!sanitized.contains("$125,000"));
        assert!(!sanitized.contains("oncology team"));
        assert!(!sanitized.contains("laid off"));
        assert!(!sanitized.contains("report.txt"));
    }

    #[test]
    fn test_feedback_open_errors_do_not_echo_raw_system_details() {
        for message in [feedback_page_open_error(), feedback_reveal_error()] {
            assert!(!message.contains("Failed to"));
            assert!(!message.contains("{e}"));
            assert!(!message.contains(&format!("/{}/", "Users")));
            assert!(!message.contains("C:\\"));
            assert!(!message.contains("open directory"));
            assert!(!message.contains("reveal file"));
        }
    }

    #[test]
    fn test_sanitize_feedback_text_redacts_renderer_content() {
        let content = concat!(
            "Crash from C:\\Users\\Alice\\Desktop\\secret.txt\n",
            "Using token ghp_123456789 and john@example.com\n",
            "Screening answer: I need sponsorship next year"
        );

        let sanitized = sanitize_feedback_text(content.to_string());

        assert!(!sanitized.contains("Alice"));
        assert!(!sanitized.contains("ghp_123456789"));
        assert!(!sanitized.contains("john@example.com"));
        assert!(sanitized.contains("[LOCAL_PATH]"));
        assert!(!sanitized.contains("secret.txt"));
        assert!(sanitized.contains("[TOKEN]"));
        assert!(sanitized.contains("[EMAIL]"));
        assert!(sanitized.contains("Screening answer: [JOB_SEARCH_DETAIL_REDACTED]"));
        assert!(!sanitized.contains("sponsorship next year"));
    }

    #[test]
    fn test_sanitize_feedback_text_redacts_unlabeled_job_search_narrative() {
        let content = r#"Issue while applying to "Acme Health" for care manager role after layoff"#;

        let sanitized = sanitize_feedback_text(content.to_string());

        assert!(sanitized.contains("[JOB_SEARCH_DETAIL_REDACTED]"));
        assert!(!sanitized.contains("Acme Health"));
        assert!(!sanitized.contains("care manager"));
        assert!(!sanitized.contains("layoff"));
    }
}
