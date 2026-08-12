//! LinkedIn Workbench commands.

use crate::application::linkedin_workbench::{
    record_event, remember_workbench_review, revoke_workbench_review, workbench_review_status,
    LinkedInWorkbenchEventInput, LinkedInWorkbenchEventResult, LinkedInWorkbenchReviewStatus,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use tauri::State;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::oneshot;

#[tauri::command]
pub(crate) async fn get_linkedin_workbench_review_status(
    state: State<'_, AppState>,
) -> Result<LinkedInWorkbenchReviewStatus, String> {
    workbench_review_status(state.database.as_ref())
        .await
        .map_err(|error| user_friendly_error("Failed to check LinkedIn Workbench review", error))
}

#[tauri::command]
pub(crate) async fn review_linkedin_workbench(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    ensure_linkedin_workbench_review(&app, &state).await
}

#[tauri::command]
pub(crate) async fn revoke_linkedin_workbench_review(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    revoke_workbench_review(state.database.as_ref())
        .await
        .map_err(|error| user_friendly_error("Failed to revoke LinkedIn Workbench review", error))
}

#[tauri::command]
pub(crate) async fn record_linkedin_workbench_event(
    input: LinkedInWorkbenchEventInput,
    state: State<'_, AppState>,
) -> Result<LinkedInWorkbenchEventResult, String> {
    record_event(&state.database, input)
        .await
        .map_err(|error| user_friendly_error("Failed to save LinkedIn work", error))
}

async fn confirm_native_workbench_review(app: &tauri::AppHandle) -> Result<bool, String> {
    let message = "Open LinkedIn yourself and use JobSentinel only to keep local records of \
                   actions you choose.\n\nJobSentinel will not automate LinkedIn, read hidden \
                   page or browser state, or store cookies, tokens, or session material.";
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(message)
        .title("Review LinkedIn Workbench")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Remember Review".to_string(),
            "Keep Disabled".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "LinkedIn Workbench confirmation could not be completed.".to_string())
}

pub(crate) async fn ensure_linkedin_workbench_review(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<bool, String> {
    match workbench_review_status(state.database.as_ref())
        .await
        .map_err(|error| user_friendly_error("Failed to check LinkedIn Workbench review", error))?
    {
        LinkedInWorkbenchReviewStatus::Reviewed => return Ok(true),
        LinkedInWorkbenchReviewStatus::PolicyRefreshRequired => return Err(
            "LinkedIn Workbench is paused while JobSentinel refreshes its provider policy review."
                .to_string(),
        ),
        LinkedInWorkbenchReviewStatus::ReviewRequired => {}
    }
    if !confirm_native_workbench_review(app).await? {
        return Ok(false);
    }
    remember_workbench_review(state.database.as_ref())
        .await
        .map_err(|error| user_friendly_error("Failed to save LinkedIn Workbench review", error))
}
