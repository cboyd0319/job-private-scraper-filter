//! Tauri commands for scraper health monitoring
//!
//! Provides frontend access to scraper health metrics, run history,
//! and smoke tests.

use crate::application::config::Config;
use crate::application::health::{
    get_all_scraper_health, get_health_summary as health_summary,
    get_latest_source_request as latest_source_request, get_scraper_configs as scraper_configs,
    get_scraper_runs as scraper_runs, is_known_scraper_name,
    run_all_smoke_tests_with_credentials as all_smoke_tests,
    run_smoke_test_with_credentials as run_smoke_test, set_scraper_enabled as scraper_enabled,
    HealthSummary, ScraperConfig, ScraperHealthMetrics, ScraperRun, SmokeTestResult,
    SourceRequestSummary,
};
use crate::bootstrap::AppState;
use crate::desktop::{path_label_for_logging, Database};
use crate::ipc::errors::user_friendly_error;
use crate::ipc::limits::validate_optional_command_limit_i32;
use std::path::Path;
use tauri::State;
use tokio::sync::RwLock;

fn health_command_error(context: &str, error: anyhow::Error) -> String {
    user_friendly_error(context, error)
}

fn validate_scraper_name(scraper_name: &str) -> Result<(), String> {
    if is_known_scraper_name(scraper_name) {
        Ok(())
    } else {
        Err("Unknown scraper".to_string())
    }
}

const DEFAULT_SCRAPER_LIMIT: usize = 50;
const DEFAULT_USAJOBS_LIMIT: usize = 100;
const DEFAULT_USAJOBS_DATE_POSTED_DAYS: u8 = 30;
const RETIRED_SCHEDULED_SOURCES: &[&str] = &["builtin", "dice", "simplyhired", "glassdoor"];

fn is_retired_scheduled_source(scraper_name: &str) -> bool {
    RETIRED_SCHEDULED_SOURCES.contains(&scraper_name)
}

fn ensure_source_limit(limit: &mut usize, default_limit: usize) {
    if *limit == 0 {
        *limit = default_limit;
    }
}

fn apply_config_backed_scraper_toggle(
    config: &mut Config,
    scraper_name: &str,
    enabled: bool,
) -> bool {
    match scraper_name {
        "yc_startup" => return false,
        source if is_retired_scheduled_source(source) => return false,
        "remoteok" => {
            config.remoteok.enabled = enabled;
            ensure_source_limit(&mut config.remoteok.limit, DEFAULT_SCRAPER_LIMIT);
        }
        "weworkremotely" => {
            config.weworkremotely.enabled = enabled;
            ensure_source_limit(&mut config.weworkremotely.limit, DEFAULT_SCRAPER_LIMIT);
        }
        "hn_hiring" => {
            config.hn_hiring.enabled = enabled;
            ensure_source_limit(&mut config.hn_hiring.limit, DEFAULT_SCRAPER_LIMIT);
        }
        "usajobs" => {
            if enabled && config.usajobs.email.trim().is_empty() {
                return false;
            }
            config.usajobs.enabled = enabled;
            if config.usajobs.date_posted_days == 0 {
                config.usajobs.date_posted_days = DEFAULT_USAJOBS_DATE_POSTED_DAYS;
            }
            ensure_source_limit(&mut config.usajobs.limit, DEFAULT_USAJOBS_LIMIT);
        }
        _ => return false,
    }

    true
}

async fn set_config_backed_scraper_enabled_in_runtime_and_path(
    scraper_name: &str,
    enabled: bool,
    runtime_config: &RwLock<Config>,
    config_path: &Path,
    database: &Database,
) -> Result<bool, String> {
    let previous = {
        let config = runtime_config.read().await;
        config.clone()
    };
    let mut next_config = previous.clone();

    if !apply_config_backed_scraper_toggle(&mut next_config, scraper_name, enabled) {
        return Ok(false);
    }
    jobsentinel_application::restricted_source_consent::reconcile_restricted_source_consents(
        database,
        &previous,
        &mut next_config,
    )
    .await
    .map_err(|error| user_friendly_error("Failed to update restricted-source review", error))?;

    next_config.save(config_path).map_err(|e| {
        let message = user_friendly_error("Failed to save configuration", &e);
        tracing::error!(
            config_path = %path_label_for_logging(config_path),
            error = %message,
            "Failed to save source configuration"
        );
        message
    })?;

    {
        let mut runtime_config = runtime_config.write().await;
        *runtime_config = next_config;
    }

    Ok(true)
}

/// Get health metrics for all scrapers
#[tauri::command]
pub(crate) async fn get_scraper_health(
    state: State<'_, AppState>,
) -> Result<Vec<ScraperHealthMetrics>, String> {
    get_all_scraper_health(&state.database)
        .await
        .map_err(|e| health_command_error("Failed to load scraper health", e))
}

/// Get health summary statistics
#[tauri::command]
pub(crate) async fn get_health_summary(
    state: State<'_, AppState>,
) -> Result<HealthSummary, String> {
    health_summary(&state.database)
        .await
        .map_err(|e| health_command_error("Failed to load health summary", e))
}

/// Get scraper configuration
#[tauri::command]
pub(crate) async fn get_scraper_configs(
    state: State<'_, AppState>,
) -> Result<Vec<ScraperConfig>, String> {
    scraper_configs(&state.database)
        .await
        .map_err(|e| health_command_error("Failed to load scraper configuration", e))
}

/// Enable or disable a scraper
#[tauri::command]
pub(crate) async fn set_scraper_enabled(
    state: State<'_, AppState>,
    scraper_name: String,
    enabled: bool,
) -> Result<(), String> {
    validate_scraper_name(&scraper_name)?;
    if is_retired_scheduled_source(&scraper_name) {
        return Ok(());
    }
    let config_path = Config::default_path();
    set_config_backed_scraper_enabled_in_runtime_and_path(
        &scraper_name,
        enabled,
        state.config.as_ref(),
        &config_path,
        state.database.as_ref(),
    )
    .await?;
    scraper_enabled(&state.database, &scraper_name, enabled)
        .await
        .map_err(|e| health_command_error("Failed to update scraper configuration", e))
}

/// Get recent runs for a specific scraper
#[tauri::command]
pub(crate) async fn get_scraper_runs(
    state: State<'_, AppState>,
    scraper_name: String,
    limit: Option<i32>,
) -> Result<Vec<ScraperRun>, String> {
    validate_scraper_name(&scraper_name)?;
    let limit = validate_optional_command_limit_i32(limit, 20)?;
    scraper_runs(&state.database, &scraper_name, limit)
        .await
        .map_err(|e| health_command_error("Failed to load scraper run history", e))
}

/// Get the latest minimized request record for an optional external source.
#[tauri::command]
pub(crate) async fn get_latest_source_request(
    state: State<'_, AppState>,
    source: String,
) -> Result<Option<SourceRequestSummary>, String> {
    validate_scraper_name(&source)?;
    latest_source_request(&state.database, &source)
        .await
        .map_err(|e| health_command_error("Failed to load source request history", e))
}

/// Run a smoke test for a specific scraper
#[tauri::command]
pub(crate) async fn run_scraper_smoke_test(
    state: State<'_, AppState>,
    scraper_name: String,
) -> Result<SmokeTestResult, String> {
    validate_scraper_name(&scraper_name)?;
    let config = state.config.read().await.clone();
    run_smoke_test(
        &state.database,
        &config,
        &scraper_name,
        state.credentials.as_ref(),
    )
    .await
    .map_err(|e| health_command_error("Failed to run scraper smoke test", e))
}

/// Run smoke tests for all scrapers
#[tauri::command]
pub(crate) async fn run_all_smoke_tests(
    state: State<'_, AppState>,
) -> Result<Vec<SmokeTestResult>, String> {
    let config = state.config.read().await.clone();
    all_smoke_tests(&state.database, &config, state.credentials.as_ref())
        .await
        .map_err(|e| health_command_error("Failed to run scraper smoke tests", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::config::{AlertConfig, AutoRefreshConfig, LocationPreferences};

    fn create_health_toggle_test_config() -> Config {
        Config {
            title_allowlist: vec!["Program Coordinator".to_string()],
            title_blocklist: vec![],
            keywords_boost: vec![],
            keywords_exclude: vec![],
            location_preferences: LocationPreferences {
                allow_remote: true,
                allow_hybrid: false,
                allow_onsite: false,
                cities: vec![],
                states: vec![],
                country: "US".to_string(),
            },
            salary_floor_usd: 70_000,
            salary_target_usd: None,
            penalize_missing_salary: false,
            auto_refresh: AutoRefreshConfig::default(),
            bookmarklet_port: 4321,
            immediate_alert_threshold: 0.8,
            scraping_interval_hours: 2,
            alerts: AlertConfig::default(),
            greenhouse_urls: vec![],
            lever_urls: vec![],
            linkedin: Default::default(),
            restricted_source_acknowledgements: Default::default(),
            remoteok: Default::default(),
            weworkremotely: Default::default(),
            builtin: Default::default(),
            hn_hiring: Default::default(),
            dice: Default::default(),
            yc_startup: Default::default(),
            usajobs: Default::default(),
            simplyhired: Default::default(),
            glassdoor: Default::default(),
            jobswithgpt_endpoint: String::new(),
            jobswithgpt_approval: Default::default(),
            external_ai: Default::default(),
            ghost_config: None,
            use_resume_matching: false,
            preferred_companies: vec![],
            blocked_companies: vec![],
        }
    }

    #[test]
    fn test_validate_scraper_name_omits_raw_input() {
        let err = validate_scraper_name("bad-scraper-with-token=secret").unwrap_err();
        assert_eq!(err, "Unknown scraper");
        assert!(!err.contains("secret"));
    }

    #[test]
    fn test_health_command_error_omits_raw_sql() {
        let msg = health_command_error(
            "Failed to load scraper health",
            anyhow::anyhow!("SELECT * FROM scraper_runs WHERE token = 'secret'"),
        );

        assert!(msg.contains("Failed to load scraper health"));
        assert!(!msg.contains("SELECT"));
        assert!(!msg.contains("secret"));
    }

    #[test]
    fn retired_sources_cannot_be_toggled() {
        let mut config = create_health_toggle_test_config();

        for source in ["yc_startup", "builtin", "dice", "simplyhired", "glassdoor"] {
            assert!(!apply_config_backed_scraper_toggle(
                &mut config,
                source,
                true,
            ));
        }
        assert!(!config.yc_startup.enabled);
        assert!(!config.builtin.enabled);
        assert!(!config.dice.enabled);
        assert!(!config.simplyhired.enabled);
        assert!(!config.glassdoor.enabled);
    }

    #[tokio::test]
    async fn config_backed_source_toggle_updates_runtime_config_and_disk() {
        let runtime_config = RwLock::new(create_health_toggle_test_config());
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let updated = set_config_backed_scraper_enabled_in_runtime_and_path(
            "remoteok",
            true,
            &runtime_config,
            &config_path,
            &database,
        )
        .await
        .unwrap();

        assert!(updated);
        assert!(runtime_config.read().await.remoteok.enabled);
        assert!(Config::load(&config_path).unwrap().remoteok.enabled);
    }
}
