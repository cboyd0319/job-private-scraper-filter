//! Bookmarklet server commands
//!
//! Tauri commands for managing a local HTTP server that receives job data
//! from a browser bookmarklet. This allows users to import jobs from any
//! website by clicking a bookmark while viewing a job posting.

use crate::application::config::Config;
use crate::bootstrap::AppState;
use crate::desktop::path_label_for_logging;
use crate::desktop::{
    discard_pending_bookmarklet_imports as discard_pending_bookmarklet_import_jobs,
    BookmarkletConfig, BookmarkletImportConfirmResult, CompanionRequest,
    PendingBookmarkletImportPreview,
};
use crate::ipc::errors::user_friendly_error;
use arboard::Clipboard;
use jobsentinel_application::{
    bookmarklet_repository, confirm_bookmarklet_imports, issue_browser_applied_pairing,
    issue_browser_import_pairing, prepare_browser_import_target,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tokio::sync::oneshot;
use uuid::Uuid;
use zeroize::Zeroizing;

const MIN_BOOKMARKLET_PORT: u16 = 1024;
const MAX_BOOKMARKLET_PORT: u32 = u16::MAX as u32;

#[derive(Clone, Copy)]
enum BrowserButtonAction {
    Import,
    Applied,
}

/// Bookmarklet configuration returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BookmarkletConfigResponse {
    pub port: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DiscardBookmarkletImportsResponse {
    pub discarded: usize,
}

fn bookmarklet_copy_error() -> String {
    "Could not copy browser button. Allow clipboard access and try again, or copy a safe support report if this keeps happening.".to_string()
}

fn validate_bookmarklet_port(port: u32) -> Result<u16, String> {
    if port < u32::from(MIN_BOOKMARKLET_PORT) || port > MAX_BOOKMARKLET_PORT {
        return Err("Choose a browser button number from 1024 to 65535.".to_string());
    }

    Ok(port as u16)
}

fn bookmarklet_code(
    port: u16,
    request: &CompanionRequest,
    action: BrowserButtonAction,
) -> Result<Zeroizing<String>, String> {
    let pairing = Zeroizing::new(
        serde_json::to_string(request)
            .map_err(|_| "Browser Import could not create a safe page pairing.".to_string())?,
    );
    let origin = serde_json::to_string(&request.origin)
        .map_err(|_| "Browser Import could not create a safe page pairing.".to_string())?;
    let (description_capture, success_message) = match action {
        BrowserButtonAction::Import => (
            r#"var desc=first('[class*="description"],[class*="desc"]');"#,
            "Request sent. Return to JobSentinel and check the review list. Copy a fresh button before retrying or importing another job.",
        ),
        BrowserButtonAction::Applied => (
            "var desc=null;",
            "Applied draft sent. Return to JobSentinel to review missing details.",
        ),
    };
    let success_message = serde_json::to_string(success_message)
        .map_err(|_| "Browser Import could not create a safe page pairing.".to_string())?;

    const TEMPLATE: &str = r#"javascript:(function(){var frame=null;function done(message){try{if(frame&&frame.parentNode){frame.parentNode.removeChild(frame);}}catch(e){}alert(message);}try{if(/(\.|^)(linkedin|ycombinator)\.com\.?$/i.test(location.hostname)){done('Browser Import is unavailable for this source');return;}if(location.protocol!=='https:'||location.origin!==__ORIGIN__){done('This Browser Import button is paired with a different site. Copy a fresh button for this page.');return;}frame=document.createElement('iframe');frame.setAttribute('aria-hidden','true');frame.style.display='none';(document.documentElement||document.body).appendChild(frame);var cleanWindow=frame.contentWindow;var cleanFetch=cleanWindow.fetch.bind(cleanWindow);var cleanStringify=cleanWindow.JSON.stringify.bind(cleanWindow.JSON);var cleanRects=cleanWindow.Element.prototype.getClientRects;var cleanStyle=cleanWindow.getComputedStyle.bind(cleanWindow);var cleanQuery=cleanWindow.Document.prototype.querySelectorAll;var cleanInnerText=cleanWindow.Object.getOwnPropertyDescriptor(cleanWindow.HTMLElement.prototype,'innerText').get;function norm(value){return String(value||'').replace(/\s+/g,' ').trim();}function visible(el){try{var style=cleanStyle(el);if(style.display==='none'||style.visibility!=='visible'||style.opacity==='0'||style.contentVisibility==='hidden'){return false;}var rects=cleanRects.call(el);for(var i=0;i<rects.length;i++){if(rects[i].width>0&&rects[i].height>0){return true;}}return false;}catch(e){return false;}}function first(selector){try{var nodes=cleanQuery.call(document,selector);for(var i=0;i<nodes.length;i++){if(visible(nodes[i])){return nodes[i];}}}catch(e){}return null;}function text(el){try{return visible(el)?norm(cleanInnerText.call(el)):'';}catch(e){return '';}}function safeJobUrl(value){try{return new cleanWindow.URL(value,window.location.href).toString();}catch(e){return '';}}var title=first('h1');var company=first('[class*="company"],[class*="employer"]');__DESCRIPTION_CAPTURE__var job={title:text(title),company:text(company),description:text(desc),url:safeJobUrl(window.location.href)};var payload={pairing:__PAIRING__,job:job};cleanFetch('http://localhost:__PORT__/api/bookmarklet/import',{method:'POST',mode:'no-cors',targetAddressSpace:'loopback',headers:{'Content-Type':'text/plain'},body:cleanStringify(payload)}).then(function(){done(__SUCCESS_MESSAGE__);}).catch(function(){done('Cannot connect to JobSentinel. Return to Settings and copy the browser button again.');});}catch(e){done('Cannot connect to JobSentinel. Return to Settings and copy the browser button again.');}})();"#;

    Ok(Zeroizing::new(
        TEMPLATE
            .replace("__PORT__", &port.to_string())
            .replace("__ORIGIN__", &origin)
            .replace("__PAIRING__", pairing.as_str())
            .replace("__DESCRIPTION_CAPTURE__", description_capture)
            .replace("__SUCCESS_MESSAGE__", &success_message),
    ))
}

async fn confirm_native_browser_import(
    app: &AppHandle,
    target_url: &str,
    origin: &str,
    action: BrowserButtonAction,
) -> Result<bool, String> {
    let (title, approve_label, purpose) = match action {
        BrowserButtonAction::Import => (
            "Confirm Browser Import Site",
            "Copy One-Use Button",
            "accept visible job details from this site only",
        ),
        BrowserButtonAction::Applied => (
            "Confirm Applied Draft Site",
            "Copy I Just Applied Button",
            "create a local applied draft from the visible job title, company, and page address; missing details stay marked for your review",
        ),
    };
    let message = format!(
        "Copy a one-use browser button for this page?\n\nSite: {origin}\nPage: \
         {target_url}\n\nJobSentinel will {purpose}. It will not read browser cookies, storage, \
         hidden page state, screenshots, or network traffic."
    );
    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            approve_label.to_string(),
            "Cancel".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "Browser Import confirmation could not be completed.".to_string())
}

async fn persist_bookmarklet_port(state: &AppState, port: u16) -> Result<(), String> {
    let config_path = Config::default_path();
    let mut next_config = {
        let runtime_config = state.config.read().await;
        runtime_config.clone()
    };
    next_config.bookmarklet_port = port;

    next_config.save(&config_path).map_err(|e| {
        let message = user_friendly_error("Failed to save Browser Import settings", &e);
        tracing::error!(
            config_path = %path_label_for_logging(&config_path),
            error = %message,
            "Failed to save Browser Import settings"
        );
        message
    })?;

    {
        let mut runtime_config = state.config.write().await;
        *runtime_config = next_config;
    }

    Ok(())
}

/// Get current bookmarklet configuration
#[tauri::command]
#[tracing::instrument(skip(state))]
pub(crate) async fn get_bookmarklet_config(
    state: State<'_, AppState>,
) -> Result<BookmarkletConfigResponse, String> {
    tracing::debug!("Getting bookmarklet configuration");

    let server_guard = state.bookmarklet_server.read().await;
    let config = server_guard.config().clone();
    let enabled = server_guard.is_running();

    Ok(BookmarkletConfigResponse {
        port: config.port,
        enabled,
    })
}

/// Copy an origin-bound browser button without exposing its pairing secret to the renderer.
#[tauri::command]
#[tracing::instrument(skip(app, state, target_url))]
pub(crate) async fn copy_bookmarklet_code(
    app: AppHandle,
    state: State<'_, AppState>,
    target_url: String,
) -> Result<bool, String> {
    tracing::debug!("Copying browser import button");
    copy_browser_button(app, state, target_url, BrowserButtonAction::Import).await
}

/// Copy an origin-bound applied draft button without exposing its pairing secret.
#[tauri::command]
#[tracing::instrument(skip(app, state, target_url))]
pub(crate) async fn copy_applied_bookmarklet_code(
    app: AppHandle,
    state: State<'_, AppState>,
    target_url: String,
) -> Result<bool, String> {
    tracing::debug!("Copying applied draft button");
    copy_browser_button(app, state, target_url, BrowserButtonAction::Applied).await
}

async fn copy_browser_button(
    app: AppHandle,
    state: State<'_, AppState>,
    target_url: String,
    action: BrowserButtonAction,
) -> Result<bool, String> {
    let (target_url, origin) = prepare_browser_import_target(&target_url)?;
    if !confirm_native_browser_import(&app, &target_url, &origin, action).await? {
        return Ok(false);
    }

    let mut server_guard = state.bookmarklet_server.write().await;
    if !server_guard.is_running() {
        return Err("Turn on Browser Import before copying a browser button.".to_string());
    }
    let (pairing, pairing_code) = match action {
        BrowserButtonAction::Import => issue_browser_import_pairing(&origin, chrono::Utc::now())?,
        BrowserButtonAction::Applied => issue_browser_applied_pairing(&origin, chrono::Utc::now())?,
    };
    let request = CompanionRequest {
        protocol_version: pairing_code.protocol_version,
        pairing_id: pairing_code.pairing_id.clone(),
        client_id: pairing_code.client_id.clone(),
        source_id: pairing_code.source_id.clone(),
        policy_ref: pairing_code.policy_ref.clone(),
        policy_revision: pairing_code.policy_revision,
        operation: pairing_code.operations[0],
        origin: pairing_code.origin.clone(),
        nonce: Uuid::new_v4().to_string(),
        token: pairing_code.token.clone(),
    };
    let code = bookmarklet_code(server_guard.config().port, &request, action)?;

    Clipboard::new()
        .and_then(|mut clipboard| clipboard.set_text(code.as_str()))
        .map_err(|_| bookmarklet_copy_error())?;

    server_guard
        .replace_pairing(pairing)
        .map_err(|error| user_friendly_error("Failed to activate Browser Import", error))?;

    Ok(true)
}

/// List browser imports waiting for user review.
#[tauri::command]
#[tracing::instrument(skip(state))]
pub(crate) async fn get_pending_bookmarklet_imports(
    state: State<'_, AppState>,
) -> Result<Vec<PendingBookmarkletImportPreview>, String> {
    let server_guard = state.bookmarklet_server.read().await;
    Ok(server_guard.pending_imports())
}

/// Save reviewed browser imports as durable jobs.
#[tauri::command]
#[tracing::instrument(skip(state, ids), fields(count = ids.len()))]
pub(crate) async fn confirm_pending_bookmarklet_imports(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<BookmarkletImportConfirmResult, String> {
    if ids.is_empty() {
        return Err("Choose at least one job to save.".to_string());
    }

    let pending_imports = {
        let server_guard = state.bookmarklet_server.read().await;
        server_guard.pending_import_store()
    };

    confirm_bookmarklet_imports(state.database.clone(), &pending_imports, &ids).await
}

/// Remove reviewed browser imports without saving.
#[tauri::command]
#[tracing::instrument(skip(state, ids), fields(count = ids.len()))]
pub(crate) async fn discard_pending_bookmarklet_imports(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<DiscardBookmarkletImportsResponse, String> {
    if ids.is_empty() {
        return Err("Choose at least one job to skip.".to_string());
    }

    let pending_imports = {
        let server_guard = state.bookmarklet_server.read().await;
        server_guard.pending_import_store()
    };
    let discarded = discard_pending_bookmarklet_import_jobs(&pending_imports, &ids);

    Ok(DiscardBookmarkletImportsResponse { discarded })
}

/// Start the bookmarklet server
#[tauri::command]
#[tracing::instrument(skip(state), fields(port))]
pub(crate) async fn start_bookmarklet_server(
    state: State<'_, AppState>,
    port: u32,
) -> Result<BookmarkletConfigResponse, String> {
    let port = validate_bookmarklet_port(port)?;
    tracing::info!(port = port, "Starting bookmarklet server");

    let mut server_guard = state.bookmarklet_server.write().await;

    if server_guard.is_running() {
        return Err("Bookmarklet server is already running".to_string());
    }

    // Update config with new port
    let mut config = server_guard.config().clone();
    config.port = port;

    // Start the server
    let selected_port = server_guard
        .start(config, bookmarklet_repository(state.database.clone()))
        .await
        .map_err(|e| {
            let message = user_friendly_error("Failed to start bookmarklet server", &e);
            tracing::error!(error = %message, "Failed to start bookmarklet server");
            message
        })?;

    if selected_port != port {
        if let Err(error) = persist_bookmarklet_port(&state, selected_port).await {
            let previous_config = BookmarkletConfig { port };
            if let Err(stop_error) = server_guard.stop().await {
                tracing::error!(
                    error = %user_friendly_error("Failed to stop Browser Import", &stop_error),
                    "Failed to roll back Browser Import after settings save failure"
                );
            }
            server_guard.set_config(previous_config);
            return Err(error);
        }
    }

    tracing::info!(
        port = selected_port,
        "Bookmarklet server started successfully"
    );
    Ok(BookmarkletConfigResponse {
        port: selected_port,
        enabled: true,
    })
}

/// Stop the bookmarklet server
#[tauri::command]
#[tracing::instrument(skip(state))]
pub(crate) async fn stop_bookmarklet_server(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Stopping bookmarklet server");

    let mut server_guard = state.bookmarklet_server.write().await;

    if !server_guard.is_running() {
        return Err("Bookmarklet server is not running".to_string());
    }

    server_guard.stop().await.map_err(|e| {
        let message = user_friendly_error("Failed to stop bookmarklet server", &e);
        tracing::error!(error = %message, "Failed to stop bookmarklet server");
        message
    })?;

    tracing::info!("Bookmarklet server stopped successfully");
    Ok(())
}

/// Set bookmarklet server port (only when server is stopped)
#[tauri::command]
#[tracing::instrument(skip(state), fields(port))]
pub(crate) async fn set_bookmarklet_port(
    state: State<'_, AppState>,
    port: u32,
) -> Result<(), String> {
    let port = validate_bookmarklet_port(port)?;
    tracing::debug!(port = port, "Setting bookmarklet port");

    {
        let server_guard = state.bookmarklet_server.read().await;
        if server_guard.is_running() {
            return Err(
                "Cannot change port while server is running. Stop the server first.".to_string(),
            );
        }
    }

    persist_bookmarklet_port(&state, port).await?;

    let mut server_guard = state.bookmarklet_server.write().await;
    if server_guard.is_running() {
        return Err(
            "Cannot change port while server is running. Stop the server first.".to_string(),
        );
    }
    let mut config = server_guard.config().clone();
    config.port = port;
    server_guard.set_config(config);

    tracing::info!(port = port, "Bookmarklet port updated");
    Ok(())
}

#[cfg(test)]
#[path = "bookmarklet_tests.rs"]
mod tests;
