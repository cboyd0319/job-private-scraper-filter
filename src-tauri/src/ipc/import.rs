//! Thin Tauri adapter for reviewed job-page imports.

use crate::bootstrap::AppState;
use crate::desktop::sanitize_url_for_logging;
use jobsentinel_application::{
    confirm_job_import as confirm_reviewed_job_import,
    confirm_smart_paste as confirm_reviewed_smart_paste, employer_discovery_review_grant,
    prepare_job_import_target, preview_job_import as stage_job_import,
    preview_smart_paste_draft as stage_smart_paste, smart_paste_review_grant, ImportedJobSummary,
    JobImportPreview, SmartPasteEdits,
};
use tauri::State;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::oneshot;

#[path = "import_errors.rs"]
mod errors;
use errors::{format_import_error, format_smart_paste_error};

#[tauri::command]
#[tracing::instrument(skip(app, state, url), fields(url = %sanitize_url_for_logging(&url)), level = "info")]
pub(crate) async fn preview_job_import(
    app: tauri::AppHandle,
    url: String,
    state: State<'_, AppState>,
) -> Result<JobImportPreview, String> {
    let canonical_url =
        prepare_job_import_target(&url).map_err(|error| format_import_error(&error))?;
    if !confirm_native_job_link_fetch(&app, &canonical_url).await? {
        return Err("Job link check was canceled. No request was sent.".to_string());
    }
    let preview = stage_job_import(
        state.database.as_ref(),
        &state.pending_url_imports,
        &canonical_url,
        employer_discovery_review_grant(),
    )
    .await
    .map_err(|error| format_import_error(&error))?;

    tracing::info!(
        title_chars = preview.title.chars().count(),
        company_chars = preview.company.chars().count(),
        already_exists = preview.already_exists,
        missing_fields = preview.missing_fields.len(),
        "Job import preview staged"
    );
    Ok(preview)
}

#[tauri::command]
#[tracing::instrument(skip(state, import_id), level = "info")]
pub(crate) async fn confirm_job_import(
    import_id: String,
    state: State<'_, AppState>,
) -> Result<ImportedJobSummary, String> {
    confirm_reviewed_job_import(
        state.database.as_ref(),
        &state.pending_url_imports,
        &import_id,
    )
    .await
    .map_err(|error| format_import_error(&error))
}

#[tauri::command]
#[tracing::instrument(skip(state, text, title, company, job_url, location), fields(text_chars = text.chars().count()), level = "info")]
pub(crate) async fn preview_smart_paste(
    text: String,
    title: Option<String>,
    company: Option<String>,
    job_url: Option<String>,
    location: Option<String>,
    state: State<'_, AppState>,
) -> Result<JobImportPreview, String> {
    stage_smart_paste_text(&state, &text, title, company, job_url, location).await
}

pub(crate) async fn stage_smart_paste_text(
    state: &AppState,
    text: &str,
    title: Option<String>,
    company: Option<String>,
    job_url: Option<String>,
    location: Option<String>,
) -> Result<JobImportPreview, String> {
    stage_smart_paste(
        state.database.as_ref(),
        &state.pending_url_imports,
        text,
        SmartPasteEdits {
            title,
            company,
            url: job_url,
            location,
        },
        smart_paste_review_grant(),
    )
    .await
    .map_err(|error| format_smart_paste_error(&error))
}

#[tauri::command]
#[tracing::instrument(skip(state, import_id), level = "info")]
pub(crate) async fn confirm_smart_paste(
    import_id: String,
    state: State<'_, AppState>,
) -> Result<ImportedJobSummary, String> {
    confirm_reviewed_smart_paste(
        state.database.as_ref(),
        &state.pending_url_imports,
        &import_id,
    )
    .await
    .map_err(|error| format_smart_paste_error(&error))
}

fn job_link_review_message(url: &str) -> String {
    format!(
        "Check this public page?\n\n{}\n\nJobSentinel will make one HTTPS request without \
         browser cookies or account credentials. Nothing is saved until you review the result.",
        sanitize_url_for_logging(url)
    )
}

async fn confirm_native_job_link_fetch(app: &tauri::AppHandle, url: &str) -> Result<bool, String> {
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(job_link_review_message(url))
        .title("Check Job Link")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Check Page".to_string(),
            "Cancel".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "Job link confirmation could not be completed.".to_string())
}

#[cfg(test)]
#[path = "import_tests.rs"]
mod tests;
