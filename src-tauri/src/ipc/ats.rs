//! Application Tracking System (ATS) Tauri commands
//!
//! Commands for managing job applications, interviews, reminders, and ghosting detection.

use crate::application::ats::{
    ApplicationStats, ApplicationStatus, ApplicationsByStatus, InterviewWithJob, PendingReminder,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use tauri::State;

/// Create a new application from a job
#[tauri::command]
pub(crate) async fn create_application(
    job_hash: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let job_hash_chars = job_hash.chars().count();
    tracing::info!(job_hash_chars, "Command: create_application");

    let tracker = state.database.application_tracker();
    tracker
        .create_application(&job_hash)
        .await
        .map_err(|e| user_friendly_error("Failed to create application", e))
}

/// Get applications grouped by status (for Kanban board)
#[tauri::command]
pub(crate) async fn get_applications_kanban(
    state: State<'_, AppState>,
) -> Result<ApplicationsByStatus, String> {
    tracing::info!("Command: get_applications_kanban");

    let tracker = state.database.application_tracker();
    tracker
        .get_applications_by_status()
        .await
        .map_err(|e| user_friendly_error("Failed to get applications", e))
}

/// Update application status
#[tauri::command]
pub(crate) async fn update_application_status(
    application_id: i64,
    status: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let status_chars = status.chars().count();
    tracing::info!(
        application_id,
        status_chars,
        "Command: update_application_status"
    );

    let tracker = state.database.application_tracker();
    let new_status: ApplicationStatus = status
        .parse()
        .map_err(|e| user_friendly_error("Invalid status", e))?;

    tracker
        .update_status(application_id, new_status)
        .await
        .map_err(|e| user_friendly_error("Failed to update status", e))
}

/// Add notes to an application
#[tauri::command]
pub(crate) async fn add_application_notes(
    application_id: i64,
    notes: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: add_application_notes (id: {})", application_id);

    let tracker = state.database.application_tracker();
    tracker
        .add_notes(application_id, &notes)
        .await
        .map_err(|e| user_friendly_error("Failed to add notes", e))
}

/// Get pending reminders
#[tauri::command]
pub(crate) async fn get_pending_reminders(
    state: State<'_, AppState>,
) -> Result<Vec<PendingReminder>, String> {
    tracing::info!("Command: get_pending_reminders");

    let tracker = state.database.application_tracker();
    tracker
        .get_pending_reminders()
        .await
        .map_err(|e| user_friendly_error("Failed to get reminders", e))
}

/// Mark reminder as completed
#[tauri::command]
pub(crate) async fn complete_reminder(
    reminder_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: complete_reminder (id: {})", reminder_id);

    let tracker = state.database.application_tracker();
    tracker
        .complete_reminder(reminder_id)
        .await
        .map_err(|e| user_friendly_error("Failed to complete reminder", e))
}

/// Auto-detect ghosted applications
#[tauri::command]
pub(crate) async fn detect_ghosted_applications(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    tracing::info!("Command: detect_ghosted_applications");

    let tracker = state.database.application_tracker();
    tracker
        .auto_detect_ghosted()
        .await
        .map_err(|e| user_friendly_error("Failed to detect ghosted", e))
}

/// Get application statistics for analytics dashboard
#[tauri::command]
pub(crate) async fn get_application_stats(
    state: State<'_, AppState>,
) -> Result<ApplicationStats, String> {
    tracing::info!("Command: get_application_stats");

    let tracker = state.database.application_tracker();
    tracker
        .get_application_stats()
        .await
        .map_err(|e| user_friendly_error("Failed to get application stats", e))
}

/// Schedule a new interview
#[tauri::command]
pub(crate) async fn schedule_interview(
    application_id: i64,
    interview_type: String,
    scheduled_at: String,
    duration_minutes: i32,
    location: Option<String>,
    interviewer_name: Option<String>,
    interviewer_title: Option<String>,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let interview_type_chars = interview_type.chars().count();
    let scheduled_at_chars = scheduled_at.chars().count();
    tracing::info!(
        application_id,
        interview_type_chars,
        scheduled_at_chars,
        duration_minutes,
        has_location = location.is_some(),
        has_interviewer_name = interviewer_name.is_some(),
        has_interviewer_title = interviewer_title.is_some(),
        has_notes = notes.is_some(),
        "Command: schedule_interview"
    );

    let tracker = state.database.application_tracker();
    tracker
        .schedule_interview(
            application_id,
            &interview_type,
            &scheduled_at,
            duration_minutes,
            location.as_deref(),
            interviewer_name.as_deref(),
            interviewer_title.as_deref(),
            notes.as_deref(),
        )
        .await
        .map_err(|e| user_friendly_error("Failed to schedule interview", e))
}

/// Get upcoming interviews
#[tauri::command]
pub(crate) async fn get_upcoming_interviews(
    state: State<'_, AppState>,
) -> Result<Vec<InterviewWithJob>, String> {
    tracing::info!("Command: get_upcoming_interviews");

    let tracker = state.database.application_tracker();
    tracker
        .get_upcoming_interviews()
        .await
        .map_err(|e| user_friendly_error("Failed to get interviews", e))
}

/// Get completed interviews.
#[tauri::command]
pub(crate) async fn get_past_interviews(
    state: State<'_, AppState>,
) -> Result<Vec<InterviewWithJob>, String> {
    tracing::info!("Command: get_past_interviews");

    let tracker = state.database.application_tracker();
    tracker
        .get_past_interviews()
        .await
        .map_err(|e| user_friendly_error("Failed to get past interviews", e))
}

/// Complete an interview with outcome
#[tauri::command]
pub(crate) async fn complete_interview(
    interview_id: i64,
    outcome: String,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let outcome_chars = outcome.chars().count();
    tracing::info!(
        interview_id,
        outcome_chars,
        has_notes = notes.is_some(),
        "Command: complete_interview"
    );

    let tracker = state.database.application_tracker();
    tracker
        .complete_interview(interview_id, &outcome, notes.as_deref())
        .await
        .map_err(|e| user_friendly_error("Failed to complete interview", e))
}

/// Delete an interview
#[tauri::command]
pub(crate) async fn delete_interview(
    interview_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: delete_interview (id: {})", interview_id);

    let tracker = state.database.application_tracker();
    tracker
        .delete_interview(interview_id)
        .await
        .map_err(|e| user_friendly_error("Failed to delete interview", e))
}
