//! Defines immutable startup recovery flags and the Tauri-managed application state.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{watch, Mutex, RwLock};

use crate::application::{config::Config, credentials::CredentialService, scheduler::Scheduler};
use crate::desktop::{
    BookmarkletServer, Database, DesktopServices, DesktopStartupFailureKind, SchedulerStatus,
};
use jobsentinel_application::{
    pack_runtime::PackRuntimeEnvironment, v3_foundation::PendingMilitaryTransitionReviews,
    PendingUrlImports,
};

pub(crate) struct StartupRecoveryState {
    platform: bool,
    failure: Option<DesktopStartupFailureKind>,
}

impl StartupRecoveryState {
    pub(crate) const fn new(platform: bool, failure: Option<DesktopStartupFailureKind>) -> Self {
        Self { platform, failure }
    }

    pub(crate) const fn required(&self) -> bool {
        self.platform || self.failure.is_some()
    }

    pub(crate) const fn platform(&self) -> bool {
        self.platform
    }

    pub(crate) const fn configuration(&self) -> bool {
        matches!(self.failure, Some(DesktopStartupFailureKind::Configuration))
    }

    pub(crate) const fn database(&self) -> bool {
        matches!(self.failure, Some(DesktopStartupFailureKind::Database))
    }
}

pub(crate) struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub database: Arc<Database>,
    pub credentials: Arc<CredentialService>,
    pub scheduler: Option<Arc<Scheduler>>,
    pub scheduler_status: Arc<RwLock<SchedulerStatus>>,
    pub bookmarklet_server: Arc<RwLock<BookmarkletServer>>,
    pub pending_url_imports: PendingUrlImports,
    pub pack_runtime: PackRuntimeEnvironment,
    pub pending_military_transition_reviews: PendingMilitaryTransitionReviews,
    pub outside_ai_cancellations: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>,
}

impl From<DesktopServices> for AppState {
    fn from(services: DesktopServices) -> Self {
        Self {
            config: services.config,
            database: services.database,
            credentials: services.credentials,
            scheduler: Some(services.scheduler),
            scheduler_status: services.scheduler_status,
            bookmarklet_server: services.bookmarklet_server,
            pending_url_imports: services.pending_url_imports,
            pack_runtime: services.pack_runtime,
            pending_military_transition_reviews: Default::default(),
            outside_ai_cancellations: Default::default(),
        }
    }
}
