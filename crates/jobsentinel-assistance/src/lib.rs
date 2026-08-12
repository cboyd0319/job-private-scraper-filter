//! User-reviewed job-search assistance workflows.

mod automation;
mod bookmarklet;
mod deeplinks;

pub use automation::{
    screening_question_matches, ApplicationAttempt, ApplicationProfile, ApplicationProfileInput,
    AtsDetector, AtsPlatform, AutomationError, AutomationPage, AutomationResult, AutomationStats,
    AutomationStatus, BrowserManager, FillResult, FormFiller, ScreeningAnswer,
};

pub use bookmarklet::{
    confirm_pending_bookmarklet_imports, discard_pending_bookmarklet_imports, BookmarkletConfig,
    BookmarkletError, BookmarkletImportConfirmResult, BookmarkletJobData, BookmarkletRepository,
    BookmarkletServer, CompanionPairing, CompanionPairingCode, CompanionPairingError,
    CompanionRequest, PendingBookmarkletImportPreview, PendingBookmarkletImports,
    COMPANION_PROTOCOL_VERSION,
};
pub use deeplinks::{
    generate_all_links, generate_link_for_site, get_all_sites, get_site_by_id,
    get_sites_by_category, DeepLink, ExperienceLevel, JobType, RemoteType, SearchCriteria,
    SiteCategory, SiteInfo,
};
