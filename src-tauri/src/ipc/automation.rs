//! Application Automation Tauri Commands
//!
//! Commands for Application Assist with user-controlled submit.
//!
//! **Features:**
//! - Profile management (contact info, work authorization)
//! - Screening answer configuration
//! - Automation attempt tracking
//! - ATS platform detection

#[cfg(test)]
use crate::application::automation::AutomationStatus;
use crate::application::automation::{
    ApplicationProfileInput, AtsDetector, AtsPlatform, AutomationStats, ProfileManager,
};
use crate::bootstrap::AppState;
use crate::desktop::sanitize_url_for_logging;
use crate::ipc::errors::user_friendly_error;
use crate::ipc::limits::validate_optional_command_limit_usize;
use std::path::Path;
use tauri::State;

mod responses;

use responses::application_screening_answer_previews;
pub(crate) use responses::{
    ApplicationProfilePreviewResponse, ApplicationProfileResponse,
    ApplicationScreeningAnswerPreviewResponse, AtsDetectionResponse, AttemptResponse,
    ScreeningAnswerResponse,
};

#[path = "automation_browser_commands.rs"]
pub(crate) mod automation_browser_commands;
mod profile_resume;

#[cfg(test)]
use automation_browser_commands::{application_page_matches_platform, prepare_form_target};
#[cfg(test)]
use profile_resume::trusted_application_resume_path;
use profile_resume::{
    application_resume_dir, delete_managed_application_resume_file,
    prepare_application_profile_resume_input, resume_file_display_name,
    select_application_resume_file as select_application_resume_file_impl,
    ApplicationResumeFileSelection,
};

fn has_stored_path(path: Option<&str>) -> bool {
    path.is_some_and(|path| !path.trim().is_empty())
}

// ============================================================================
// Profile Management Commands
// ============================================================================

/// Select a resume with the native file picker and copy it into app-owned storage.
#[tauri::command]
pub(crate) async fn select_application_resume_file(
    app: tauri::AppHandle,
) -> Result<Option<ApplicationResumeFileSelection>, String> {
    select_application_resume_file_impl(app).await
}

/// Upsert (create or update) the application profile
#[tauri::command]
pub(crate) async fn upsert_application_profile(
    input: ApplicationProfileInput,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    tracing::info!("Command: upsert_application_profile");

    let manager = state.database.profile_manager();
    let managed_resume_dir = application_resume_dir();
    upsert_application_profile_with_resume_cleanup(input, &manager, &managed_resume_dir).await
}

async fn upsert_application_profile_with_resume_cleanup(
    input: ApplicationProfileInput,
    manager: &ProfileManager,
    managed_resume_dir: &Path,
) -> Result<i64, String> {
    let previous_resume_path = manager
        .get_profile()
        .await
        .map_err(|e| user_friendly_error("Failed to save profile", e))?
        .and_then(|profile| profile.resume_file_path);

    let input = prepare_application_profile_resume_input(input, managed_resume_dir)?;
    let requested_resume_change =
        input.clear_resume_file.unwrap_or(false) || input.resume_file_path.is_some();
    let next_resume_path = input.resume_file_path.clone();

    match manager.upsert_profile(&input).await {
        Ok(profile_id) => {
            if requested_resume_change
                && previous_resume_path.as_deref().map(str::trim)
                    != next_resume_path.as_deref().map(str::trim)
            {
                delete_managed_application_resume_file(
                    previous_resume_path.as_deref(),
                    managed_resume_dir,
                )?;
            }
            Ok(profile_id)
        }
        Err(error) => {
            delete_managed_application_resume_file(next_resume_path.as_deref(), managed_resume_dir)
                .ok();
            Err(user_friendly_error("Failed to save profile", error))
        }
    }
}

/// Get the current application profile
#[tauri::command]
pub(crate) async fn get_application_profile(
    state: State<'_, AppState>,
) -> Result<Option<ApplicationProfileResponse>, String> {
    tracing::info!("Command: get_application_profile");

    let manager = state.database.profile_manager();
    match manager.get_profile().await {
        Ok(Some(profile)) => Ok(Some(ApplicationProfileResponse::from(profile))),
        Ok(None) => Ok(None),
        Err(e) => Err(user_friendly_error("Failed to get profile", e)),
    }
}

/// Check whether an application profile exists without returning profile data
#[tauri::command]
pub(crate) async fn has_application_profile(state: State<'_, AppState>) -> Result<bool, String> {
    tracing::info!("Command: has_application_profile");

    let manager = state.database.profile_manager();
    manager
        .has_profile()
        .await
        .map_err(|e| user_friendly_error("Failed to check profile", e))
}

/// Get only the profile fields needed for a user-facing application preview
#[tauri::command]
pub(crate) async fn get_application_profile_preview(
    state: State<'_, AppState>,
) -> Result<Option<ApplicationProfilePreviewResponse>, String> {
    tracing::info!("Command: get_application_profile_preview");

    let manager = state.database.profile_manager();
    match manager.get_profile().await {
        Ok(Some(profile)) => Ok(Some(ApplicationProfilePreviewResponse::from(profile))),
        Ok(None) => Ok(None),
        Err(e) => Err(user_friendly_error("Failed to get profile preview", e)),
    }
}

// ============================================================================
// Screening Answer Commands
// ============================================================================

/// Add or update a screening answer pattern
#[tauri::command]
pub(crate) async fn upsert_screening_answer(
    question_pattern: String,
    answer: String,
    answer_type: String,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!(
        question_pattern_chars = question_pattern.chars().count(),
        answer_type,
        has_notes = notes.is_some(),
        "Command: upsert_screening_answer"
    );

    let manager = state.database.profile_manager();
    manager
        .upsert_screening_answer(&question_pattern, &answer, &answer_type, notes.as_deref())
        .await
        .map_err(|e| user_friendly_error("Failed to save screening answer", e))
}

/// Get all screening answers
#[tauri::command]
pub(crate) async fn get_screening_answers(
    state: State<'_, AppState>,
) -> Result<Vec<ScreeningAnswerResponse>, String> {
    tracing::info!("Command: get_screening_answers");

    let manager = state.database.profile_manager();
    match manager.get_screening_answers().await {
        Ok(answers) => Ok(answers
            .into_iter()
            .map(ScreeningAnswerResponse::from)
            .collect()),
        Err(e) => Err(user_friendly_error("Failed to get screening answers", e)),
    }
}

/// Get only non-user-controlled saved answers needed for application preview.
#[tauri::command]
pub(crate) async fn get_application_screening_answer_previews(
    state: State<'_, AppState>,
) -> Result<Vec<ApplicationScreeningAnswerPreviewResponse>, String> {
    tracing::info!("Command: get_application_screening_answer_previews");

    state
        .database
        .profile_manager()
        .get_screening_answers()
        .await
        .map(application_screening_answer_previews)
        .map_err(|e| user_friendly_error("Failed to get application screening previews", e))
}

/// Find the best answer for a specific question
#[tauri::command]
pub(crate) async fn find_answer_for_question(
    question: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    tracing::info!(
        question_chars = question.chars().count(),
        "Command: find_answer_for_question"
    );

    let manager = state.database.profile_manager();
    manager
        .find_answer_for_question(&question)
        .await
        .map_err(|e| user_friendly_error("Failed to find answer", e))
}

// ============================================================================
// Automation Attempt Commands
// ============================================================================

/// Create a new automation attempt for a job
#[tauri::command]
pub(crate) async fn create_automation_attempt(
    job_hash: String,
    ats_platform: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let platform = AtsPlatform::from_str(&ats_platform);

    tracing::info!(
        job_hash_chars = job_hash.chars().count(),
        ats_platform = platform.as_str(),
        "Command: create_automation_attempt"
    );

    let manager = state.database.automation_manager();

    manager
        .create_attempt(&job_hash, platform)
        .await
        .map_err(|e| user_friendly_error("Failed to create automation attempt", e))
}

/// Get an automation attempt by ID
#[tauri::command]
pub(crate) async fn get_automation_attempt(
    attempt_id: i64,
    state: State<'_, AppState>,
) -> Result<AttemptResponse, String> {
    tracing::info!("Command: get_automation_attempt (id: {})", attempt_id);

    let manager = state.database.automation_manager();
    match manager.get_attempt(attempt_id).await {
        Ok(attempt) => Ok(AttemptResponse::from(attempt)),
        Err(e) => Err(user_friendly_error("Failed to get attempt", e)),
    }
}

/// Approve an automation attempt (user reviewed and approved)
#[tauri::command]
pub(crate) async fn approve_automation_attempt(
    attempt_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: approve_automation_attempt (id: {})", attempt_id);

    let manager = state.database.automation_manager();
    manager
        .approve_attempt(attempt_id)
        .await
        .map_err(|e| user_friendly_error("Failed to approve attempt", e))
}

/// Cancel an automation attempt
#[tauri::command]
pub(crate) async fn cancel_automation_attempt(
    attempt_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: cancel_automation_attempt (id: {})", attempt_id);

    let manager = state.database.automation_manager();
    manager
        .update_status(
            attempt_id,
            crate::application::automation::AutomationStatus::Cancelled,
            Some("Cancelled by user"),
        )
        .await
        .map_err(|e| user_friendly_error("Failed to cancel attempt", e))
}

/// Get pending automation attempts (approved and ready to process)
#[tauri::command]
pub(crate) async fn get_pending_attempts(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<AttemptResponse>, String> {
    tracing::info!("Command: get_pending_attempts");

    let limit = validate_optional_command_limit_usize(limit, 10)?;
    let manager = state.database.automation_manager();
    match manager.get_pending_attempts(limit).await {
        Ok(attempts) => Ok(attempts.into_iter().map(AttemptResponse::from).collect()),
        Err(e) => Err(user_friendly_error("Failed to get pending attempts", e)),
    }
}

/// Get automation statistics
#[tauri::command]
pub(crate) async fn get_automation_stats(
    state: State<'_, AppState>,
) -> Result<AutomationStats, String> {
    tracing::info!("Command: get_automation_stats");

    let manager = state.database.automation_manager();
    manager
        .get_stats()
        .await
        .map_err(|e| user_friendly_error("Failed to get automation stats", e))
}

// ============================================================================
// ATS Detection Commands
// ============================================================================

/// Detect ATS platform from a URL
#[tauri::command]
pub(crate) async fn detect_ats_platform(url: String) -> Result<AtsDetectionResponse, String> {
    tracing::info!(
        "Command: detect_ats_platform (url: {})",
        sanitize_url_for_logging(&url)
    );

    let platform = AtsDetector::detect_from_url(&url);
    let common_fields = AtsDetector::get_common_fields(&platform)
        .into_iter()
        .map(String::from)
        .collect();
    let notes = AtsDetector::get_automation_notes(&platform);

    Ok(AtsDetectionResponse {
        platform: platform.as_str().to_string(),
        common_fields,
        automation_notes: Some(notes.to_string()),
    })
}

/// Detect ATS platform from HTML content
#[tauri::command]
pub(crate) async fn detect_ats_from_html(html: String) -> Result<String, String> {
    tracing::info!("Command: detect_ats_from_html");

    let platform = AtsDetector::detect_from_html(&html);
    Ok(platform.as_str().to_string())
}

#[cfg(test)]
mod tests;

mod answer_learning;

use answer_learning::{AnswerStatisticsResponse, AnswerSuggestionResponse};

#[tauri::command]
pub(crate) async fn get_suggested_answers(
    question: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<AnswerSuggestionResponse>, String> {
    let limit = validate_optional_command_limit_usize(limit, 5)?;
    answer_learning::get_suggested_answers(question, Some(limit), state).await
}

#[tauri::command]
pub(crate) async fn record_answer_usage(
    screening_answer_id: Option<i64>,
    question_text: String,
    answer_filled: String,
    was_modified: bool,
    modified_to: Option<String>,
    job_hash: Option<String>,
    application_attempt_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    answer_learning::record_answer_usage(
        screening_answer_id,
        question_text,
        answer_filled,
        was_modified,
        modified_to,
        job_hash,
        application_attempt_id,
        state,
    )
    .await
}

#[tauri::command]
pub(crate) async fn get_answer_statistics(
    pattern: String,
    state: State<'_, AppState>,
) -> Result<Option<AnswerStatisticsResponse>, String> {
    answer_learning::get_answer_statistics(pattern, state).await
}

#[tauri::command]
pub(crate) async fn clear_answer_history(
    pattern: Option<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    answer_learning::clear_answer_history(pattern, state).await
}

#[cfg(test)]
mod response_tests;
