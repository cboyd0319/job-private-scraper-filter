//! Configuration Tauri commands
//!
//! Commands for saving, retrieving, and validating app configuration.

use crate::application::config::{AutoRefreshConfig, Config, EmailConfig};
use crate::application::credentials::{
    decode_smtp_password_for_binding, CredentialKey, CredentialService, SmtpCredentialBinding,
};
use crate::bootstrap::{AppState, StartupRecoveryState};
use crate::desktop::path_label_for_logging;
use crate::desktop::Database;
use crate::ipc::errors::user_friendly_error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, path::Path};
use tauri::State;
use tokio::sync::RwLock;

/// Email configuration for testing (matches frontend interface)
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct TestEmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub to_emails: Vec<String>,
    #[serde(default = "default_starttls")]
    pub use_starttls: bool,
}

impl fmt::Debug for TestEmailConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEmailConfig")
            .field("smtp_server", &self.smtp_server)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_username", &self.smtp_username)
            .field(
                "smtp_password",
                &if self.smtp_password.is_empty() {
                    "[empty]"
                } else {
                    "[REDACTED]"
                },
            )
            .field("from_email", &self.from_email)
            .field("to_emails", &self.to_emails)
            .field("use_starttls", &self.use_starttls)
            .finish()
    }
}

fn default_starttls() -> bool {
    true
}

async fn get_stored_credential_for_test(
    key: CredentialKey,
    label: &str,
    credentials: &CredentialService,
) -> Result<String, String> {
    match credentials.retrieve(key).await {
        Ok(Some(value)) if !value.is_empty() => Ok(value),
        Ok(_) => Err(format!("{label} credential is required")),
        Err(e) => {
            let message = user_friendly_error("Stored credential unavailable", &e);
            tracing::error!(
                credential = label,
                error = %message,
                "Stored credential unavailable"
            );
            Err(format!("{label} credential is unavailable"))
        }
    }
}

async fn resolve_slack_webhook_for_test(
    webhook_url: String,
    credentials: &CredentialService,
) -> Result<String, String> {
    let webhook_url = webhook_url.trim().to_string();
    if !webhook_url.is_empty() {
        return Ok(webhook_url);
    }

    get_stored_credential_for_test(CredentialKey::SlackWebhook, "Slack webhook", credentials).await
}

fn is_first_run_for_path(config_path: &Path) -> Result<bool, String> {
    config_path.try_exists().map(|exists| !exists).map_err(|e| {
        tracing::error!(
            config_path = %path_label_for_logging(config_path),
            error_kind = ?e.kind(),
            "Failed to check configuration file"
        );
        "JobSentinel could not read saved setup status. Check local file permissions, then try again.".to_string()
    })
}

fn first_run_or_recovery(config_path: &Path, recovery_required: bool) -> Result<bool, String> {
    if recovery_required {
        Err("JobSentinel started in local recovery mode.".to_string())
    } else {
        is_first_run_for_path(config_path)
    }
}

async fn resolve_smtp_password_for_test(
    email_config: &TestEmailConfig,
    credentials: &CredentialService,
) -> Result<String, String> {
    if !email_config.smtp_password.is_empty() {
        return Ok(email_config.smtp_password.clone());
    }

    let stored =
        get_stored_credential_for_test(CredentialKey::SmtpPassword, "SMTP password", credentials)
            .await?;
    let binding = SmtpCredentialBinding::new(
        &email_config.smtp_server,
        email_config.smtp_port,
        &email_config.smtp_username,
    );
    decode_smtp_password_for_binding(&stored, &binding)
}

async fn save_config_to_runtime_and_path(
    config: Value,
    runtime_config: &RwLock<Config>,
    config_path: &Path,
    database: &Database,
) -> Result<(), String> {
    let mut parsed_config: Config = serde_json::from_value(config)
        .map_err(|e| user_friendly_error("Invalid configuration", e))?;
    let previous = runtime_config.read().await.clone();
    jobsentinel_application::restricted_source_consent::reconcile_restricted_source_consents(
        database,
        &previous,
        &mut parsed_config,
    )
    .await
    .map_err(|error| user_friendly_error("Failed to update restricted-source review", error))?;

    parsed_config.save(config_path).map_err(|e| {
        let message = user_friendly_error("Failed to save configuration", &e);
        tracing::error!(
            config_path = %path_label_for_logging(config_path),
            error = %message,
            "Failed to save configuration"
        );
        message
    })?;

    {
        let mut runtime_config = runtime_config.write().await;
        *runtime_config = parsed_config;
    }

    Ok(())
}

async fn complete_setup_to_runtime_and_paths(
    config: Value,
    runtime_config: &RwLock<Config>,
    config_path: &Path,
    database: &Database,
) -> Result<(), String> {
    save_config_to_runtime_and_path(config, runtime_config, config_path, database).await
}

/// Save user configuration
#[tauri::command]
pub(crate) async fn save_config(config: Value, state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Command: save_config");

    let config_path = Config::default_path();
    save_config_to_runtime_and_path(
        config,
        state.config.as_ref(),
        &config_path,
        state.database.as_ref(),
    )
    .await?;

    tracing::info!("Configuration saved successfully");
    Ok(())
}

/// Get user configuration
#[tauri::command]
pub(crate) async fn get_config(state: State<'_, AppState>) -> Result<Value, String> {
    tracing::info!("Command: get_config");

    let config = state.config.read().await;
    serde_json::to_value(&*config).map_err(|e| user_friendly_error("Failed to serialize config", e))
}

/// Minimal dashboard preferences. Avoids exposing full settings to dashboard UI.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPreferences {
    pub auto_refresh: AutoRefreshConfig,
    pub salary_floor_usd: i64,
    pub any_job_source_enabled: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResumeMatchingPreference {
    pub enabled: bool,
}

impl DashboardPreferences {
    fn from_config(config: &Config) -> Self {
        Self {
            auto_refresh: config.auto_refresh.clone(),
            salary_floor_usd: config.salary_floor_usd,
            any_job_source_enabled: any_job_source_enabled(config),
        }
    }
}

async fn set_resume_matching_enabled_in_runtime_and_path(
    enabled: bool,
    runtime_config: &RwLock<Config>,
    config_path: &Path,
) -> Result<ResumeMatchingPreference, String> {
    let mut next_config = {
        let config = runtime_config.read().await;
        config.clone()
    };
    next_config.use_resume_matching = enabled;

    next_config.save(config_path).map_err(|e| {
        let message = user_friendly_error("Failed to save configuration", &e);
        tracing::error!(
            config_path = %path_label_for_logging(config_path),
            error = %message,
            "Failed to save resume matching preference"
        );
        message
    })?;

    {
        let mut runtime_config = runtime_config.write().await;
        *runtime_config = next_config;
    }

    Ok(ResumeMatchingPreference { enabled })
}

#[tauri::command]
pub(crate) async fn get_dashboard_preferences(
    state: State<'_, AppState>,
) -> Result<DashboardPreferences, String> {
    tracing::info!("Command: get_dashboard_preferences");
    let config = state.config.read().await;
    Ok(DashboardPreferences::from_config(&config))
}

#[tauri::command]
pub(crate) async fn get_resume_matching_preference(
    state: State<'_, AppState>,
) -> Result<ResumeMatchingPreference, String> {
    tracing::info!("Command: get_resume_matching_preference");
    let config = state.config.read().await;
    Ok(ResumeMatchingPreference {
        enabled: config.use_resume_matching,
    })
}

#[tauri::command]
pub(crate) async fn set_resume_matching_enabled(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<ResumeMatchingPreference, String> {
    tracing::info!(enabled, "Command: set_resume_matching_enabled");
    let config_path = Config::default_path();
    set_resume_matching_enabled_in_runtime_and_path(enabled, state.config.as_ref(), &config_path)
        .await
}

fn any_job_source_enabled(config: &Config) -> bool {
    config.remoteok.enabled
        || config.weworkremotely.enabled
        || config.hn_hiring.enabled
        || config.usajobs.enabled
        || config
            .greenhouse_urls
            .iter()
            .any(|url| !url.trim().is_empty())
        || config.lever_urls.iter().any(|url| !url.trim().is_empty())
        || config.jobswithgpt_payload_approved()
}

/// Validate Slack webhook URL
#[tauri::command]
pub(crate) async fn validate_slack_webhook(
    webhook_url: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    tracing::info!("Command: validate_slack_webhook");
    let webhook_url =
        resolve_slack_webhook_for_test(webhook_url, state.credentials.as_ref()).await?;

    match crate::application::notify::validate_slack_webhook(&webhook_url).await {
        Ok(valid) => Ok(valid),
        Err(e) => {
            let message = user_friendly_error("Validation failed", &e);
            tracing::error!(error = %message, "Webhook validation failed");
            Err(message)
        }
    }
}

/// Check if first-run setup is complete
#[tauri::command]
pub(crate) async fn is_first_run(
    recovery: State<'_, StartupRecoveryState>,
) -> Result<bool, String> {
    tracing::info!("Command: is_first_run");

    // Check if configuration file exists
    let config_path = Config::default_path();
    let first_run = first_run_or_recovery(&config_path, recovery.required())?;

    tracing::info!("First run: {}", first_run);
    Ok(first_run)
}

/// Complete first-run setup
#[tauri::command]
pub(crate) async fn complete_setup(
    config: Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: complete_setup");

    let config_path = Config::default_path();
    complete_setup_to_runtime_and_paths(
        config,
        state.config.as_ref(),
        &config_path,
        state.database.as_ref(),
    )
    .await?;

    tracing::info!("Setup complete");
    Ok(())
}

/// Test email notification configuration by sending a test email
#[tauri::command]
pub(crate) async fn test_email_notification(
    email_config: TestEmailConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: test_email_notification");
    let smtp_password =
        resolve_smtp_password_for_test(&email_config, state.credentials.as_ref()).await?;

    // Convert to EmailConfig for the validate function
    let config = EmailConfig {
        enabled: true,
        smtp_server: email_config.smtp_server,
        smtp_port: email_config.smtp_port,
        smtp_username: email_config.smtp_username,
        smtp_password,
        from_email: email_config.from_email,
        to_emails: email_config.to_emails,
        use_starttls: email_config.use_starttls,
    };

    crate::application::notify::validate_email_config(&config)
        .await
        .map_err(|e| user_friendly_error("Failed to send test email", &e))?;

    tracing::info!("Test email sent successfully");
    Ok(())
}

#[cfg(test)]
mod tests;
