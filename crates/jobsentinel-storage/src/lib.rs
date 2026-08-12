//! SQLCipher-backed local storage for JobSentinel.
//!
//! Handles local database operations through a bounded SQLx-backed facade.

mod integrity;
mod scoring_config;

mod analytics_buckets;
mod sqlite_time;

pub mod application_tracking;
pub mod automation;
pub mod health;
pub mod market_intelligence;
pub mod outside_ai;
pub mod pack_tasks;
pub mod resume;
pub mod salary;
pub mod user_data;
pub mod v3_foundation;
pub mod v3_pack_lifecycle;
pub mod v3_source_consent;
pub mod v3_source_manifest;
pub mod v3_vectors;

// Internal modules
mod analytics;
mod connection;
mod credentials;
mod crud;
mod encryption;
mod ghost;
mod interactions;
mod queries;
mod types;

// Tests
#[cfg(test)]
mod outside_ai_tests;
#[cfg(test)]
mod pack_tasks_tests;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod v3_pack_lifecycle_tests;
#[cfg(test)]
mod v3_source_manifest_tests;

// Re-export public types
pub use types::{DuplicateGroup, GhostStatistics, Statistics};

// Re-export Database struct
pub use connection::{
    Database, PortableBackupHistory, PortableBackupInfo, PortableRestoreStatus, ReviewedExportInfo,
    ReviewedExportPlan, ReviewedExportSelection, StorageHealth, StorageMaintenanceReport,
};
pub use credentials::{
    CredentialKeyWrapRecord, CredentialRepository, CredentialSecretRecord, CredentialStorageError,
};
pub use v3_pack_lifecycle::pack_lifecycle_error_kind;

/// Stable, non-sensitive classification for storage errors used by callers.
pub fn database_error_kind(error: &sqlx::Error) -> &'static str {
    match error {
        sqlx::Error::Database(_) => "database",
        sqlx::Error::Decode(_) => "decode",
        sqlx::Error::Encode(_) => "encode",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::Protocol(_) => "protocol",
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::TypeNotFound { .. } => "type_not_found",
        sqlx::Error::ColumnIndexOutOfBounds { .. }
        | sqlx::Error::ColumnNotFound(_)
        | sqlx::Error::ColumnDecode { .. } => "column",
        _ => "unknown",
    }
}

impl Database {
    #[must_use]
    pub fn application_tracker(&self) -> application_tracking::ApplicationTracker {
        application_tracking::ApplicationTracker::new(self.pool().clone())
    }

    #[must_use]
    pub fn automation_manager(&self) -> automation::AutomationManager {
        automation::AutomationManager::new(self.pool().clone())
    }

    #[must_use]
    pub fn profile_manager(&self) -> automation::ProfileManager {
        automation::ProfileManager::new(self.pool().clone())
    }

    #[must_use]
    pub fn answer_learning_manager(&self) -> automation::AnswerLearningManager {
        automation::AnswerLearningManager::new(self.pool().clone())
    }

    #[must_use]
    pub fn market_intelligence(&self) -> market_intelligence::MarketIntelligence {
        market_intelligence::MarketIntelligence::new(self.pool().clone())
    }

    /// Create the bounded resume repository and document workflow for this database.
    #[must_use]
    pub fn resume_matcher(&self) -> resume::ResumeMatcher {
        resume::ResumeMatcher::new(self.pool().clone())
    }

    #[must_use]
    pub fn resume_builder(&self) -> resume::ResumeBuilder {
        resume::ResumeBuilder::new(self.pool().clone())
    }

    #[must_use]
    pub fn salary_analyzer(&self) -> salary::SalaryAnalyzer {
        salary::SalaryAnalyzer::new(self.pool().clone())
    }

    #[must_use]
    pub fn user_data_manager(&self) -> user_data::UserDataManager {
        user_data::UserDataManager::new(self.pool().clone())
    }

    pub async fn load_scoring_config(&self) -> Result<jobsentinel_domain::ScoringConfig, String> {
        scoring_config::load_scoring_config(self.pool()).await
    }

    pub async fn save_scoring_config(
        &self,
        config: &jobsentinel_domain::ScoringConfig,
    ) -> Result<(), String> {
        scoring_config::save_scoring_config(self.pool(), config).await
    }

    pub async fn reset_scoring_config(&self) -> Result<(), String> {
        scoring_config::reset_scoring_config(self.pool()).await
    }

    /// Close all database connections after in-flight work completes.
    pub async fn close(&self) {
        self.pool().close().await;
    }
}

#[cfg(test)]
mod analytics_bucket_contract_tests {
    use super::analytics_buckets::{
        market_location_bucket, salary_location_bucket, salary_title_bucket,
    };

    #[test]
    fn salary_buckets_are_distinct_from_canonical_normalization() {
        for (input, expected) in [
            ("Senior Software Engineer", "software engineer"),
            ("Sr. SWE", "software engineer"),
            ("Jr. Care Coordinator", "junior care coordinator"),
            ("   ", ""),
        ] {
            assert_eq!(salary_title_bucket(input), expected, "title: {input}");
        }

        for (input, expected) in [
            ("SF Bay Area", "san francisco, ca"),
            ("NYC, USA", "new york, ny"),
            ("Seattle Metropolitan Area", "seattle, wa"),
            ("Austin-Round Rock", "austin, tx"),
            ("Remote - US", "remote"),
            ("Satisfactory Location", "satisfactory location"),
        ] {
            assert_eq!(salary_location_bucket(input), expected, "location: {input}");
        }
    }

    #[test]
    fn market_location_bucket_preserves_market_group_keys() {
        for (input, expected) in [
            ("SF Bay Area", "san francisco, ca"),
            ("NYC", "new york, ny"),
            ("Remote - Anywhere", "remote"),
            ("Chicago, IL", "chicago, il"),
            ("Satisfactory Location", "satisfactory location"),
        ] {
            assert_eq!(market_location_bucket(input), expected, "location: {input}");
        }
    }
}
