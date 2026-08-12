//! Narrow product APIs consumed by the desktop adapter.

mod startup;
mod startup_recovery;

pub use jobsentinel_assistance::{
    discard_pending_bookmarklet_imports, generate_all_links, generate_link_for_site, get_all_sites,
    get_sites_by_category, BookmarkletConfig, BookmarkletImportConfirmResult, BookmarkletServer,
    CompanionPairingCode, CompanionRequest, DeepLink, PendingBookmarkletImportPreview, RemoteType,
    SearchCriteria, SiteCategory, SiteInfo,
};
pub use jobsentinel_intelligence::GhostConfig;
#[cfg(feature = "embedded-ml")]
pub use jobsentinel_local_ai::{
    load_model_manifest, model_lock_hash, ModelCacheHealth, ModelKind, ModelManager, ModelManifest,
    ModelSpec, ModelStatus, SemanticMatcher,
};
pub use jobsentinel_network::{validate_external_https_url_for_fetch, HttpBodyReadError};
pub use jobsentinel_platform::{
    delete_device_secret, get_data_dir, initialize, retrieve_device_secret, store_device_secret,
    SecureStorageError,
};
pub use jobsentinel_security::{
    path_label_for_logging, sanitize_url_for_logging, validate_external_https_url,
};
pub use jobsentinel_sources::{detect_location, LocationInfo};
pub use jobsentinel_storage::{Database, DuplicateGroup, PortableRestoreStatus};
pub use startup::{
    DesktopServices, DesktopStartupError, DesktopStartupFailureKind, SchedulerStatus,
};
pub use startup_recovery::{
    repair_invalid_startup_config, StartupConfigRepair, StartupConfigRepairOutcome,
};
