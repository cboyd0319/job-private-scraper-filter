//! Constructs desktop services and reconciles startup-owned local state before use.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, NaiveDate, Utc};
use jobsentinel_domain::v3_signed_packs::TrustedPublisherKey;
use thiserror::Error;
use tokio::sync::RwLock;

use super::{path_label_for_logging, BookmarkletConfig, BookmarkletServer, Database};
use crate::{
    credentials::{
        clear_config_credentials, extract_plaintext_credentials, is_migrated, set_migrated,
        CredentialService,
    },
    pack_runtime::{
        production_trusted_publishers, reconcile_active_pack_artifacts, PackRuntimeEnvironment,
    },
    scheduler::Scheduler,
    Config, PendingUrlImports,
};

#[derive(Debug, Clone, Default)]
pub struct SchedulerStatus {
    pub is_running: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum DesktopStartupError {
    #[error("configuration error: {0}")]
    Configuration(String),
    #[error("database initialization error: {0}")]
    Database(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopStartupFailureKind {
    Configuration,
    Database,
}

impl DesktopStartupError {
    #[must_use]
    pub const fn context(&self) -> &'static str {
        match self {
            Self::Configuration(_) => "Configuration error",
            Self::Database(_) => "Failed to initialize database",
        }
    }

    #[must_use]
    pub const fn kind(&self) -> DesktopStartupFailureKind {
        match self {
            Self::Configuration(_) => DesktopStartupFailureKind::Configuration,
            Self::Database(_) => DesktopStartupFailureKind::Database,
        }
    }
}

pub struct DesktopServices {
    pub config: Arc<RwLock<Config>>,
    pub database: Arc<Database>,
    pub credentials: Arc<CredentialService>,
    pub scheduler: Arc<Scheduler>,
    pub scheduler_status: Arc<RwLock<SchedulerStatus>>,
    pub bookmarklet_server: Arc<RwLock<BookmarkletServer>>,
    pub pending_url_imports: PendingUrlImports,
    pub pack_runtime: PackRuntimeEnvironment,
    pub is_first_run: bool,
}

impl DesktopServices {
    pub async fn initialize() -> Result<Self, DesktopStartupError> {
        Self::initialize_at(Config::default_path(), Database::default_path()).await
    }

    pub async fn initialize_recovery() -> Result<Self, DesktopStartupError> {
        let database = Database::connect_memory()
            .await
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        database
            .migrate()
            .await
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        let mut key = [0_u8; 32];
        key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        let credentials =
            CredentialService::with_fixed_master_key(database.credentials(), key, false);

        Ok(Self::assemble(
            Config::first_run(),
            false,
            database,
            credentials,
            PackRuntimeEnvironment::for_data_dir(&jobsentinel_platform::get_data_dir()),
        ))
    }

    async fn initialize_at(
        config_path: PathBuf,
        database_path: PathBuf,
    ) -> Result<Self, DesktopStartupError> {
        let (config, is_first_run) = load_startup_config(&config_path)?;
        tracing::info!(
            db_path = %path_label_for_logging(&database_path),
            "Connecting to database"
        );
        let database = Database::connect_with_staged_restore(&database_path)
            .await
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;

        Self::initialize_ready(config_path, config, is_first_run, database).await
    }

    #[cfg(test)]
    async fn initialize_with_database(
        config_path: PathBuf,
        database: Database,
    ) -> Result<Self, DesktopStartupError> {
        let (config, is_first_run) = load_startup_config(&config_path)?;
        Self::initialize_loaded(config_path, config, is_first_run, database).await
    }

    #[cfg(test)]
    async fn initialize_loaded(
        config_path: PathBuf,
        config: Config,
        is_first_run: bool,
        database: Database,
    ) -> Result<Self, DesktopStartupError> {
        database
            .migrate()
            .await
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        tracing::info!("Database initialized successfully");
        let pack_runtime = PackRuntimeEnvironment::for_data_dir(
            config_path.parent().unwrap_or_else(|| Path::new(".")),
        );
        Self::initialize_ready_with_pack_runtime(
            config_path,
            config,
            is_first_run,
            database,
            pack_runtime,
            production_trusted_publishers(),
            Utc::now().date_naive(),
        )
        .await
    }

    async fn initialize_ready(
        config_path: PathBuf,
        config: Config,
        is_first_run: bool,
        database: Database,
    ) -> Result<Self, DesktopStartupError> {
        Self::initialize_ready_with_pack_runtime(
            config_path,
            config,
            is_first_run,
            database,
            PackRuntimeEnvironment::for_data_dir(&jobsentinel_platform::get_data_dir()),
            production_trusted_publishers(),
            Utc::now().date_naive(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn initialize_ready_with_pack_runtime(
        config_path: PathBuf,
        mut config: Config,
        is_first_run: bool,
        database: Database,
        pack_runtime: PackRuntimeEnvironment,
        trusted_publishers: &[TrustedPublisherKey],
        today: NaiveDate,
    ) -> Result<Self, DesktopStartupError> {
        let reconciled = database
            .reconcile_outside_ai_operations()
            .await
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        if reconciled.ambiguous > 0 || reconciled.cancelled > 0 {
            tracing::warn!(
                ambiguous = reconciled.ambiguous,
                cancelled = reconciled.cancelled,
                "Reconciled interrupted Outside AI operations"
            );
        }
        crate::v3_source_governance::install_startup_source_governance(&database).await;
        jobsentinel_platform::ensure_private_dir_tree(pack_runtime.artifact_root())
            .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        let reconciled_packs = reconcile_active_pack_artifacts(
            &database,
            pack_runtime.artifact_root(),
            trusted_publishers,
            today,
        )
        .await
        .map_err(|error| DesktopStartupError::Database(error.to_string()))?;
        if reconciled_packs.rolled_back > 0 || reconciled_packs.quarantined > 0 {
            tracing::warn!(
                checked = reconciled_packs.checked,
                rolled_back = reconciled_packs.rolled_back,
                quarantined = reconciled_packs.quarantined,
                "Reconciled active signed pack artifacts"
            );
        }
        let credentials = CredentialService::new(database.credentials());
        if migrate_plaintext_credentials_to_secure_storage(&config_path, &credentials).await {
            config = Config::load(&config_path)
                .map_err(|error| DesktopStartupError::Configuration(error.to_string()))?;
        }
        crate::restricted_source_consent::refresh_restricted_source_acknowledgements(
            &database,
            &mut config,
        )
        .await
        .map_err(|error| DesktopStartupError::Database(error.to_string()))?;

        Ok(Self::assemble(
            config,
            is_first_run,
            database,
            credentials,
            pack_runtime,
        ))
    }

    fn assemble(
        config: Config,
        is_first_run: bool,
        database: Database,
        credentials: CredentialService,
        pack_runtime: PackRuntimeEnvironment,
    ) -> Self {
        let bookmarklet_port = config.bookmarklet_port;
        let database = Arc::new(database);
        let credentials = Arc::new(credentials);
        let config = Arc::new(RwLock::new(config));
        let scheduler_status = Arc::new(RwLock::new(SchedulerStatus::default()));
        let scheduler = Arc::new(Scheduler::new_shared_with_credentials(
            Arc::clone(&config),
            Arc::clone(&database),
            Arc::clone(&credentials),
        ));
        let bookmarklet_server = Arc::new(RwLock::new(BookmarkletServer::new(BookmarkletConfig {
            port: bookmarklet_port,
        })));

        Self {
            config,
            database,
            credentials,
            scheduler,
            scheduler_status,
            bookmarklet_server,
            pending_url_imports: PendingUrlImports::default(),
            pack_runtime,
            is_first_run,
        }
    }
}

fn load_startup_config(config_path: &Path) -> Result<(Config, bool), DesktopStartupError> {
    let is_first_run = !config_path.exists();
    if is_first_run {
        tracing::info!("No configuration file found, first-run setup required");
        return Ok((Config::first_run(), true));
    }

    let config = Config::load(config_path)
        .map_err(|error| DesktopStartupError::Configuration(error.to_string()))?;
    tracing::info!(
        config_path = %path_label_for_logging(config_path),
        "Loaded configuration"
    );
    Ok((config, false))
}

async fn migrate_plaintext_credentials_to_secure_storage(
    config_path: &Path,
    credentials: &CredentialService,
) -> bool {
    if !config_path.exists() || is_migrated() {
        return false;
    }

    tracing::info!("Checking for plaintext credentials to migrate to secure storage");

    let credentials_to_migrate = match extract_plaintext_credentials(config_path) {
        Ok(credentials_to_migrate) => credentials_to_migrate,
        Err(error) => {
            tracing::error!(
                "Failed to extract plaintext credentials for migration: {}",
                error
            );
            return false;
        }
    };

    let mark_migration_complete = if credentials_to_migrate.is_empty() {
        tracing::info!("No active plaintext credentials found");
        if let Err(error) = clear_config_credentials(config_path) {
            tracing::error!("Failed to clear legacy credential fields: {}", error);
            tracing::warn!("Secure-storage migration will retry on next startup");
            false
        } else {
            true
        }
    } else {
        tracing::info!(
            "Found {} plaintext credentials to migrate",
            credentials_to_migrate.len()
        );

        let mut migration_success = true;
        for (key, value) in &credentials_to_migrate {
            if let Err(error) = credentials.store(*key, value).await {
                tracing::error!(
                    "Failed to migrate credential {:?} to secure storage: {}",
                    key,
                    error
                );
                migration_success = false;
            } else {
                tracing::info!("Migrated {:?} to secure storage", key);
            }
        }

        if migration_success {
            if let Err(error) = clear_config_credentials(config_path) {
                tracing::error!(
                    "Failed to clear plaintext credentials from config: {}",
                    error
                );
                tracing::warn!("Secure-storage migration will retry on next startup");
                false
            } else {
                true
            }
        } else {
            tracing::warn!("Secure-storage migration incomplete; will retry on next startup");
            false
        }
    };

    if mark_migration_complete {
        if let Err(error) = set_migrated() {
            tracing::warn!("Failed to set migration flag: {}", error);
        }
    }

    mark_migration_complete
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
