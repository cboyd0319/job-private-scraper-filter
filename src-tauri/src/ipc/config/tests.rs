use super::*;
use crate::application::{
    config::{AlertConfig, LocationPreferences},
    credentials::{
        encode_smtp_password, CredentialService, SmtpCredentialBinding,
        SMTP_CREDENTIAL_REENTRY_REQUIRED,
    },
};
use crate::desktop::Database;

fn create_dashboard_test_config() -> Config {
    Config {
        title_allowlist: vec![],
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

async fn test_credentials() -> CredentialService {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    CredentialService::with_fixed_master_key(database.credentials(), [21_u8; 32], false)
}

fn test_email_config_for(server: &str, username: &str) -> TestEmailConfig {
    TestEmailConfig {
        smtp_server: server.to_string(),
        smtp_port: 587,
        smtp_username: username.to_string(),
        smtp_password: String::new(),
        from_email: "from@example.com".to_string(),
        to_emails: vec!["to@example.com".to_string()],
        use_starttls: true,
    }
}

#[test]
fn test_email_config_debug_does_not_leak_password() {
    let config = TestEmailConfig {
        smtp_server: "smtp.example.com".to_string(),
        smtp_port: 587,
        smtp_username: "user@example.com".to_string(),
        smtp_password: "smtp-secret-password".to_string(),
        from_email: "from@example.com".to_string(),
        to_emails: vec!["to@example.com".to_string()],
        use_starttls: true,
    };

    let debug_output = format!("{:?}", config);

    assert!(
        !debug_output.contains("smtp-secret-password"),
        "TestEmailConfig Debug output must not contain password. Got: {}",
        debug_output
    );
}

#[tokio::test]
async fn test_email_resolves_stored_smtp_password_only_for_matching_binding() {
    let credentials = test_credentials().await;
    let stored = encode_smtp_password(
        "smtp-secret",
        SmtpCredentialBinding::new("smtp.example.com", 587, "user@example.com"),
    )
    .unwrap();
    credentials
        .store(CredentialKey::SmtpPassword, &stored)
        .await
        .unwrap();

    let matching = test_email_config_for("smtp.example.com", "user@example.com");
    assert_eq!(
        resolve_smtp_password_for_test(&matching, &credentials)
            .await
            .unwrap(),
        "smtp-secret"
    );

    let changed = test_email_config_for("attacker.example.com", "user@example.com");
    let err = resolve_smtp_password_for_test(&changed, &credentials)
        .await
        .unwrap_err();
    assert_eq!(err, SMTP_CREDENTIAL_REENTRY_REQUIRED);
    assert!(!err.contains("smtp-secret"));
}

#[tokio::test]
async fn test_email_rejects_legacy_unbound_smtp_password() {
    let credentials = test_credentials().await;
    credentials
        .store(CredentialKey::SmtpPassword, "smtp-secret")
        .await
        .unwrap();

    let config = test_email_config_for("smtp.example.com", "user@example.com");
    let err = resolve_smtp_password_for_test(&config, &credentials)
        .await
        .unwrap_err();

    assert_eq!(err, SMTP_CREDENTIAL_REENTRY_REQUIRED);
    assert!(!err.contains("smtp-secret"));
}

#[test]
fn test_dashboard_preferences_are_minimal() {
    let mut config = create_dashboard_test_config();
    config.salary_floor_usd = 88_000;
    config.auto_refresh.enabled = true;
    config.auto_refresh.interval_minutes = 45;
    config.remoteok.enabled = true;

    let preferences = DashboardPreferences::from_config(&config);

    assert_eq!(preferences.salary_floor_usd, 88_000);
    assert!(preferences.auto_refresh.enabled);
    assert_eq!(preferences.auto_refresh.interval_minutes, 45);
    assert!(preferences.any_job_source_enabled);
}

#[tokio::test]
async fn save_config_updates_runtime_config_after_disk_save() {
    let runtime_config = RwLock::new(create_dashboard_test_config());
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();

    let mut new_config = create_dashboard_test_config();
    new_config.salary_floor_usd = 95_000;
    new_config.use_resume_matching = true;
    new_config.keywords_boost = vec!["case management".to_string()];
    let payload = serde_json::to_value(&new_config).unwrap();

    save_config_to_runtime_and_path(payload, &runtime_config, &config_path, &database)
        .await
        .unwrap();

    let runtime = runtime_config.read().await;
    assert_eq!(runtime.salary_floor_usd, 95_000);
    assert!(runtime.use_resume_matching);
    assert_eq!(runtime.keywords_boost, vec!["case management"]);
    drop(runtime);

    let saved = Config::load(&config_path).unwrap();
    assert_eq!(saved.salary_floor_usd, 95_000);
    assert!(saved.use_resume_matching);
    assert_eq!(saved.keywords_boost, vec!["case management"]);
}

#[tokio::test]
async fn save_config_clears_legacy_review_for_a_retired_source() {
    let previous = create_dashboard_test_config();
    let runtime_config = RwLock::new(previous.clone());
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let mut reviewed = previous;
    reviewed.dice.enabled = true;
    reviewed.dice.query = "security analyst".to_string();
    reviewed.dice.limit = 25;
    reviewed.restricted_source_acknowledgements.dice = true;
    save_config_to_runtime_and_path(
        serde_json::to_value(&reviewed).unwrap(),
        &runtime_config,
        &config_path,
        &database,
    )
    .await
    .unwrap();
    let runtime = runtime_config.read().await;
    assert!(!runtime.dice.enabled);
    assert!(!runtime.restricted_source_acknowledgements.dice);
    drop(runtime);
    assert!(
        !Config::load(&config_path)
            .unwrap()
            .restricted_source_acknowledgements
            .dice
    );
}

#[tokio::test]
async fn complete_setup_reuses_the_owned_database() {
    let runtime_config = RwLock::new(create_dashboard_test_config());
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();

    let mut setup_config = create_dashboard_test_config();
    setup_config.salary_floor_usd = 82_000;
    setup_config.title_allowlist = vec!["Program Coordinator".to_string()];
    setup_config.remoteok.enabled = true;
    setup_config.remoteok.limit = 50;
    let payload = serde_json::to_value(&setup_config).unwrap();

    complete_setup_to_runtime_and_paths(payload, &runtime_config, &config_path, &database)
        .await
        .unwrap();

    let runtime = runtime_config.read().await;
    assert_eq!(runtime.salary_floor_usd, 82_000);
    assert_eq!(runtime.title_allowlist, vec!["Program Coordinator"]);
    assert!(runtime.remoteok.enabled);
    assert!(DashboardPreferences::from_config(&runtime).any_job_source_enabled);
    drop(runtime);

    let saved = Config::load(&config_path).unwrap();
    assert_eq!(saved.salary_floor_usd, 82_000);
    assert_eq!(saved.title_allowlist, vec!["Program Coordinator"]);
    assert!(saved.remoteok.enabled);
    assert!(!is_first_run_for_path(&config_path).unwrap());
}

#[test]
fn first_run_check_is_true_only_when_config_is_missing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");

    assert!(is_first_run_for_path(&config_path).unwrap());

    std::fs::write(&config_path, "{}").unwrap();
    assert!(!is_first_run_for_path(&config_path).unwrap());
    assert_eq!(
        first_run_or_recovery(&config_path, true),
        Err("JobSentinel started in local recovery mode.".to_string())
    );
}

#[tokio::test]
async fn set_resume_matching_enabled_updates_runtime_config_after_disk_save() {
    let runtime_config = RwLock::new(create_dashboard_test_config());
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let preference =
        set_resume_matching_enabled_in_runtime_and_path(true, &runtime_config, &config_path)
            .await
            .unwrap();

    assert!(preference.enabled);
    assert!(runtime_config.read().await.use_resume_matching);
    assert!(Config::load(&config_path).unwrap().use_resume_matching);
}

#[test]
fn test_dashboard_source_check_handles_arrays_and_endpoint_sources() {
    let disabled = create_dashboard_test_config();
    assert!(!any_job_source_enabled(&disabled));

    let mut retired_yc = create_dashboard_test_config();
    retired_yc.yc_startup.enabled = true;
    assert!(!any_job_source_enabled(&retired_yc));

    let mut greenhouse = create_dashboard_test_config();
    greenhouse.greenhouse_urls = vec!["https://boards.greenhouse.io/example".to_string()];
    assert!(any_job_source_enabled(&greenhouse));

    let mut lever = create_dashboard_test_config();
    lever.lever_urls = vec!["https://jobs.lever.co/example".to_string()];
    assert!(any_job_source_enabled(&lever));

    let mut jobswithgpt_without_titles = create_dashboard_test_config();
    jobswithgpt_without_titles.jobswithgpt_endpoint = "https://api.jobswithgpt.com/mcp".to_string();
    assert!(!any_job_source_enabled(&jobswithgpt_without_titles));

    let mut jobswithgpt = jobswithgpt_without_titles;
    jobswithgpt.title_allowlist = vec!["Case Manager".to_string()];
    assert!(!any_job_source_enabled(&jobswithgpt));

    jobswithgpt.jobswithgpt_approval.enabled = true;
    jobswithgpt.jobswithgpt_approval.payload = jobswithgpt.jobswithgpt_payload_preview();
    assert!(any_job_source_enabled(&jobswithgpt));

    jobswithgpt.title_allowlist = vec!["Program Coordinator".to_string()];
    assert!(!any_job_source_enabled(&jobswithgpt));
}

#[test]
fn test_command_error_formatter_omits_raw_paths() {
    let msg = user_friendly_error(
        "Failed to initialize database",
        "sqlite://<local-private-db>?mode=rwc: unable to open database file",
    );

    assert!(!msg.contains("<local-private-db>"), "path leaked: {msg}");
    assert!(!msg.contains("jobs.db"), "database file leaked: {msg}");
    assert!(!msg.contains("sqlite:"), "database URL leaked: {msg}");
    assert!(msg.contains("Failed to initialize database"));
}

#[test]
fn test_email_error_formatter_omits_smtp_details() {
    let msg = user_friendly_error(
        "Failed to send test email",
        "smtp://user@example.com:smtp-secret-password@smtp.example.com:587 authentication failed",
    );

    assert!(
        !msg.contains("smtp-secret-password"),
        "password leaked: {msg}"
    );
    assert!(!msg.contains("user@example.com"), "username leaked: {msg}");
    assert!(!msg.contains("smtp.example.com"), "server leaked: {msg}");
    assert!(!msg.contains("smtp://"), "smtp URL leaked: {msg}");
    assert!(msg.contains("Failed to send test email"));
}

#[test]
fn test_webhook_error_formatter_omits_webhook_details() {
    let msg = user_friendly_error(
        "Validation failed",
        "https://hooks.slack.com/services/T00/B00/secret-token request failed",
    );

    assert!(!msg.contains("secret-token"), "token leaked: {msg}");
    assert!(!msg.contains("hooks.slack.com"), "host leaked: {msg}");
    assert!(!msg.contains("/services/"), "path leaked: {msg}");
    assert!(!msg.contains("https://"), "URL leaked: {msg}");
    assert!(msg.contains("Validation failed"));
}
