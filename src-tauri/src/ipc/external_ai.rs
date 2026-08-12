//! Thin Tauri adapter for reviewed external-AI requests.

use crate::{application, bootstrap::AppState};
use tauri::State;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::{oneshot, watch};

pub(crate) use jobsentinel_application::{
    ExternalAiActivityEntry, ExternalAiCancelOutcome, ExternalAiCancelResponse,
    ExternalAiCommandRequest, ExternalAiCommandResponse, ExternalAiPrepareResponse,
};

#[tauri::command]
pub(crate) async fn prepare_external_ai_request(
    app: tauri::AppHandle,
    request: ExternalAiCommandRequest,
    state: State<'_, AppState>,
) -> Result<ExternalAiPrepareResponse, String> {
    tracing::info!("Command: prepare_external_ai_request");
    let config = {
        let config = state.config.read().await;
        config.external_ai.clone()
    };
    let prepared =
        application::prepare_external_ai_request(&request, &config, state.database.as_ref())
            .await?;
    let rejection = match confirm_native_outside_ai_send(&app, &prepared).await {
        Ok(true) => return Ok(prepared),
        Ok(false) => "Outside AI request was kept local.".to_string(),
        Err(error) => error,
    };
    application::cancel_external_ai_request(&prepared.approval_id, state.database.as_ref()).await?;
    Err(rejection)
}

#[tauri::command]
pub(crate) async fn send_external_ai_request(
    approval_id: String,
    request: ExternalAiCommandRequest,
    state: State<'_, AppState>,
) -> Result<ExternalAiCommandResponse, String> {
    tracing::info!("Command: send_external_ai_request");
    validate_approval_id(&approval_id)?;

    let config = {
        let config = state.config.read().await;
        config.external_ai.clone()
    };
    let (cancel, cancellation) = watch::channel(false);
    {
        let mut active = state.outside_ai_cancellations.lock().await;
        if active.contains_key(&approval_id) {
            return Err("This Outside AI approval is already in use.".to_string());
        }
        active.insert(approval_id.clone(), cancel);
    }
    let result = application::send_external_ai_request(
        &approval_id,
        &request,
        &config,
        state.credentials.as_ref(),
        state.database.as_ref(),
        cancellation,
    )
    .await;
    state
        .outside_ai_cancellations
        .lock()
        .await
        .remove(&approval_id);
    result
}

#[tauri::command]
pub(crate) async fn cancel_external_ai_request(
    approval_id: String,
    state: State<'_, AppState>,
) -> Result<ExternalAiCancelResponse, String> {
    validate_approval_id(&approval_id)?;
    let response =
        application::cancel_external_ai_request(&approval_id, state.database.as_ref()).await?;
    let active = state
        .outside_ai_cancellations
        .lock()
        .await
        .get(&approval_id)
        .cloned();
    if response.outcome != ExternalAiCancelOutcome::AlreadyCompleted {
        if let Some(cancel) = active {
            cancel
                .send(true)
                .map_err(|_| "Outside AI cancellation could not be delivered.".to_string())?;
        }
    }
    Ok(response)
}

#[tauri::command]
pub(crate) async fn list_external_ai_activity(
    state: State<'_, AppState>,
) -> Result<Vec<ExternalAiActivityEntry>, String> {
    application::list_external_ai_activity(state.database.as_ref()).await
}

fn validate_approval_id(value: &str) -> Result<(), String> {
    let Some(id) = value.strip_prefix("outside-ai-approval:") else {
        return Err("Outside AI approval could not be verified.".to_string());
    };
    if id.len() != 36
        || id.bytes().enumerate().any(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte != b'-'
            } else {
                !byte.is_ascii_hexdigit()
            }
        })
    {
        return Err("Outside AI approval could not be verified.".to_string());
    }
    Ok(())
}

async fn confirm_native_outside_ai_send(
    app: &tauri::AppHandle,
    prepared: &ExternalAiPrepareResponse,
) -> Result<bool, String> {
    let message = format!(
        "Send {} stored public job field(s) to {} using model {} at {}?\n\n\
         JobSentinel will keep private notes, resumes, veteran status, clearance claims, \
         credentials, and application history local.",
        prepared.field_count, prepared.provider_id, prepared.model, prepared.destination
    );
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(message)
        .title("Confirm Outside AI Send")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Send Public Fields".to_string(),
            "Keep Local".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "Outside AI confirmation could not be completed.".to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_approval_id;

    #[test]
    fn approval_ids_are_bounded_before_entering_the_active_map() {
        assert!(
            validate_approval_id("outside-ai-approval:4f677d43-dc4a-47dd-9139-f5998f13a810")
                .is_ok()
        );
        for invalid in [
            "",
            "outside-ai:4f677d43-dc4a-47dd-9139-f5998f13a810",
            "outside-ai-approval:",
            "outside-ai-approval:not valid",
        ] {
            assert!(validate_approval_id(invalid).is_err());
        }
        assert!(validate_approval_id(&format!("outside-ai-approval:{}", "a".repeat(129))).is_err());
    }
}
