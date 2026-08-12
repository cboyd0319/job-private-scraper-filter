//! LinkedIn credential/session automation boundary.
//!
//! These commands remain registered only so older frontends and stored app
//! state fail closed instead of reaching hidden endpoints or collecting session
//! cookies. User-directed search links, manual entry, and the local Workbench
//! remain separate paths. Browser Import is blocked for LinkedIn.

use crate::application::credentials::{CredentialKey, CredentialService};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use tauri::{AppHandle, Manager, State};

pub(crate) const LINKEDIN_AUTH_DISABLED_MESSAGE: &str =
    "JobSentinel does not collect LinkedIn login details or session cookies. \
     You sign in and use LinkedIn yourself. JobSentinel can help you keep a \
     local record of jobs you choose to save, apply to, track, or review, but \
     it will not click for you, read LinkedIn pages in the background, or save \
     sign-in material. LinkedIn says third-party software that scrapes or \
     automates activity can violate its User Agreement, may lead to account \
     restrictions, and may raise privacy-law concerns.";

pub(crate) const LINKEDIN_INTERACTIVE_SESSION_REMINDER_MINUTES: i64 = 60;

/// Legacy LinkedIn credential expiry status.
#[derive(serde::Serialize)]
pub(crate) struct LinkedInExpiryStatus {
    /// Whether a LinkedIn session credential is active.
    pub connected: bool,
    /// Expiry date in ISO 8601 format, if applicable.
    pub expires_at: Option<String>,
    /// Days until expiry, if applicable.
    pub days_remaining: Option<i64>,
    /// Whether expiry is close.
    pub expiry_warning: bool,
    /// Whether the credential is expired.
    pub expired: bool,
}

/// Restricted interactive LinkedIn session policy.
#[derive(serde::Serialize)]
pub(crate) struct LinkedInInteractivePolicy {
    /// Whether a user action is required before any restricted LinkedIn use.
    pub requires_user_initiated_action: bool,
    /// Whether a fresh sign-in is required for each restricted session.
    pub requires_fresh_login: bool,
    /// Whether a terms/account/privacy warning is required before sign-in.
    pub pre_login_warning_required: bool,
    /// Whether auth tokens may be stored.
    pub stores_auth_tokens: bool,
    /// Whether session cookies may be stored.
    pub stores_session_cookies: bool,
    /// Whether browser localStorage or sessionStorage may be stored.
    pub stores_browser_storage: bool,
    /// Whether authorization headers may be stored.
    pub stores_authorization_headers: bool,
    /// Whether hidden or scheduled background automation is allowed.
    pub background_automation_allowed: bool,
    /// Whether the session may be used after the live user action ends.
    pub offline_use_allowed: bool,
    /// Privacy reminder interval in minutes for long-running manual sessions.
    pub privacy_reminder_minutes: i64,
    /// Whether JobSentinel must hard-close the window solely due to elapsed time.
    pub hard_session_expiry_required: bool,
    /// User-facing source-policy warning.
    pub warning: &'static str,
}

fn disabled_error() -> String {
    LINKEDIN_AUTH_DISABLED_MESSAGE.to_string()
}

fn linkedin_interactive_policy() -> LinkedInInteractivePolicy {
    LinkedInInteractivePolicy {
        requires_user_initiated_action: true,
        requires_fresh_login: true,
        pre_login_warning_required: true,
        stores_auth_tokens: false,
        stores_session_cookies: false,
        stores_browser_storage: false,
        stores_authorization_headers: false,
        background_automation_allowed: false,
        offline_use_allowed: false,
        privacy_reminder_minutes: LINKEDIN_INTERACTIVE_SESSION_REMINDER_MINUTES,
        hard_session_expiry_required: false,
        warning: LINKEDIN_AUTH_DISABLED_MESSAGE,
    }
}

/// LinkedIn login capture is disabled.
#[tauri::command]
pub(crate) async fn linkedin_login(app: AppHandle) -> Result<String, String> {
    close_linkedin_login(app).await?;
    Err(disabled_error())
}

/// Storing LinkedIn session cookies is disabled.
#[tauri::command]
pub(crate) async fn store_linkedin_cookie(cookie: String) -> Result<(), String> {
    drop(cookie);
    Err(disabled_error())
}

/// LinkedIn is not connected because session automation is disabled.
#[tauri::command]
pub(crate) async fn is_linkedin_connected() -> Result<bool, String> {
    Ok(false)
}

/// Remove legacy LinkedIn session entries from the OS credential store.
pub(crate) async fn disconnect_linkedin_with_credentials(
    credentials: &CredentialService,
) -> Result<(), String> {
    credentials
        .delete(CredentialKey::LinkedInCookie)
        .await
        .map_err(|e| user_friendly_error("Failed to remove legacy LinkedIn credential", e))?;
    credentials
        .delete(CredentialKey::LinkedInCookieExpiry)
        .await
        .map_err(|e| user_friendly_error("Failed to remove legacy LinkedIn credential", e))?;
    tracing::info!("Removed legacy LinkedIn credential entries");
    Ok(())
}

/// Remove legacy LinkedIn session entries from secure storage.
#[tauri::command]
pub(crate) async fn disconnect_linkedin(state: State<'_, AppState>) -> Result<(), String> {
    disconnect_linkedin_with_credentials(state.credentials.as_ref()).await
}

/// Close any legacy LinkedIn login window if it exists.
#[tauri::command]
pub(crate) async fn close_linkedin_login(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("linkedin-login") {
        window
            .close()
            .map_err(|e| user_friendly_error("Failed to close legacy LinkedIn window", e))?;
    }
    Ok(())
}

/// Expiry status is inactive because session automation is disabled.
#[tauri::command]
pub(crate) async fn get_linkedin_expiry_status() -> Result<LinkedInExpiryStatus, String> {
    Ok(LinkedInExpiryStatus {
        connected: false,
        expires_at: None,
        days_remaining: None,
        expiry_warning: false,
        expired: false,
    })
}

/// Return the restricted interactive LinkedIn policy without exposing auth state.
#[tauri::command]
pub(crate) async fn get_linkedin_interactive_policy() -> Result<LinkedInInteractivePolicy, String> {
    Ok(linkedin_interactive_policy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn storing_linkedin_cookie_fails_closed() {
        let err = store_linkedin_cookie("legacy-session-value".to_string())
            .await
            .unwrap_err();

        assert!(err.contains("does not collect LinkedIn login details"));
        assert!(err.contains("local record"));
        assert!(err.contains("User Agreement"));
        assert!(!err.contains("legacy-session-value"));
    }

    #[tokio::test]
    async fn linked_in_status_is_always_disconnected() {
        assert!(!is_linkedin_connected().await.unwrap());
    }

    #[tokio::test]
    async fn linked_in_expiry_status_is_inactive() {
        let status = get_linkedin_expiry_status().await.unwrap();

        assert!(!status.connected);
        assert!(status.expires_at.is_none());
        assert!(status.days_remaining.is_none());
        assert!(!status.expiry_warning);
        assert!(!status.expired);
    }

    #[tokio::test]
    async fn linked_in_interactive_policy_forbids_auth_persistence() {
        let policy = get_linkedin_interactive_policy().await.unwrap();

        assert!(policy.requires_user_initiated_action);
        assert!(policy.requires_fresh_login);
        assert!(policy.pre_login_warning_required);
        assert!(!policy.stores_auth_tokens);
        assert!(!policy.stores_session_cookies);
        assert!(!policy.stores_browser_storage);
        assert!(!policy.stores_authorization_headers);
        assert!(!policy.background_automation_allowed);
        assert!(!policy.offline_use_allowed);
        assert_eq!(
            policy.privacy_reminder_minutes,
            LINKEDIN_INTERACTIVE_SESSION_REMINDER_MINUTES
        );
        assert_eq!(policy.privacy_reminder_minutes, 60);
        assert!(!policy.hard_session_expiry_required);
        assert!(policy.warning.contains("User Agreement"));
    }
}

#[cfg(test)]
mod credential_integration_tests;
