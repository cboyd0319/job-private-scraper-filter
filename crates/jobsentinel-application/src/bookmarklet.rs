use std::sync::Arc;

use async_trait::async_trait;
use jobsentinel_assistance::{
    confirm_pending_bookmarklet_imports, BookmarkletImportConfirmResult, BookmarkletRepository,
    CompanionPairing, CompanionPairingCode, PendingBookmarkletImports,
};
use jobsentinel_domain::{
    canonicalize_job_url,
    v3_source_authorization::visible_page_capture_is_blocked,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::SourceOperation,
    Job,
};
use jobsentinel_security::validate_external_https_url;
use jobsentinel_storage::application_tracking::ApplicationStatus;
use jobsentinel_storage::Database;

struct StorageBookmarkletRepository {
    database: Arc<Database>,
}

pub fn prepare_browser_import_target(target_url: &str) -> Result<(String, String), String> {
    if target_url
        .trim()
        .strip_prefix("https://")
        .is_none_or(|authority| authority.starts_with('/'))
    {
        return Err("Enter a public https job page address.".to_string());
    }
    let parsed = validate_external_https_url(target_url)
        .map_err(|_| "Enter a public https job page address.".to_string())?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Enter a public https job page address.".to_string());
    }
    let canonical_url = canonicalize_job_url(target_url)
        .map_err(|_| "Enter a public https job page address.".to_string())?;
    if visible_page_capture_is_blocked(&canonical_url) {
        return Err("Browser Import is unavailable for this source.".to_string());
    }
    let origin = url::Url::parse(&canonical_url)
        .map_err(|_| "Enter a public https job page address.".to_string())?
        .origin()
        .ascii_serialization();
    Ok((canonical_url, origin))
}

pub fn issue_browser_import_pairing(
    origin: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(CompanionPairing, CompanionPairingCode), String> {
    issue_browser_pairing(origin, SourceOperation::VisiblePageCapture, now)
}

pub fn issue_browser_applied_pairing(
    origin: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(CompanionPairing, CompanionPairingCode), String> {
    issue_browser_pairing(origin, SourceOperation::AppliedLogging, now)
}

fn issue_browser_pairing(
    origin: &str,
    operation: SourceOperation,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(CompanionPairing, CompanionPairingCode), String> {
    CompanionPairing::issue(
        "browser-import",
        "user-source-actions",
        "jobsentinel.source-policy.user-source-actions",
        1,
        origin,
        vec![operation],
        now,
    )
    .map_err(|_| "Browser Import could not create a safe page pairing.".to_string())
}

#[async_trait]
impl BookmarkletRepository for StorageBookmarkletRepository {
    async fn authorize_browser_action(&self, grant: &SourceGrantState) -> Result<bool, String> {
        let SourceGrantState::Granted { operation, .. } = grant else {
            return Ok(false);
        };
        Ok(matches!(
            crate::v3_source_governance::authorize_user_source_action(
                &self.database,
                *operation,
                chrono::Utc::now().date_naive(),
                grant.clone(),
            )
            .await
            .map_err(|error| error.to_string())?,
            SourceActionDecision::Allowed {
                connectivity_required: false,
                ..
            }
        ))
    }

    async fn job_exists_by_hash(&self, hash: &str) -> Result<bool, String> {
        self.database
            .job_exists_by_hash(hash)
            .await
            .map_err(|error| error.to_string())
    }

    async fn upsert_job(&self, job: &Job) -> Result<i64, String> {
        for value in [
            job.title.as_str(),
            job.company.as_str(),
            job.description.as_deref().unwrap_or_default(),
        ] {
            crate::linkedin_workbench::ensure_credential_free_source_text(value).map_err(|_| {
                "Browser Import cannot save credential or session text.".to_string()
            })?;
        }
        self.database
            .upsert_job(job)
            .await
            .map_err(|error| error.to_string())
    }

    async fn mark_job_applied(&self, hash: &str) -> Result<(), String> {
        let tracker = self.database.application_tracker();
        let application_id = match tracker
            .find_application_id_by_job_hash(hash)
            .await
            .map_err(|error| error.to_string())?
        {
            Some(application_id) => application_id,
            None => tracker
                .create_application(hash)
                .await
                .map_err(|error| error.to_string())?,
        };
        tracker
            .update_status(application_id, ApplicationStatus::Applied)
            .await
            .map_err(|error| error.to_string())
    }
}

pub fn bookmarklet_repository(database: Arc<Database>) -> Arc<dyn BookmarkletRepository> {
    Arc::new(StorageBookmarkletRepository { database })
}

pub async fn confirm_bookmarklet_imports(
    database: Arc<Database>,
    pending_imports: &PendingBookmarkletImports,
    ids: &[String],
) -> Result<BookmarkletImportConfirmResult, String> {
    let repository = StorageBookmarkletRepository { database };
    confirm_pending_bookmarklet_imports(&repository, pending_imports, ids).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_import_target_is_minimized_and_origin_bound() {
        assert_eq!(
            prepare_browser_import_target(
                "https://jobs.example/posting/1?utm_source=mail&token=private&gh_jid=42#apply"
            )
            .unwrap(),
            (
                "https://jobs.example/posting/1?gh_jid=42".to_string(),
                "https://jobs.example".to_string(),
            )
        );
    }

    #[test]
    fn browser_import_target_rejects_unsafe_or_blocked_scope() {
        for target in [
            "http://jobs.example/posting/1",
            "https://localhost/posting/1",
            "https://user:secret@jobs.example/posting/1",
            "https:///jobs",
            "https://www.linkedin.com/jobs/view/1",
            "https://www.ycombinator.com/jobs/1",
        ] {
            assert!(prepare_browser_import_target(target).is_err(), "{target}");
        }
    }

    #[test]
    fn visible_capture_pairing_uses_the_frozen_source_contract() {
        let now = chrono::Utc::now();
        let (_pairing, code) = issue_browser_import_pairing("https://jobs.example", now).unwrap();

        assert_eq!(code.client_id, "browser-import");
        assert_eq!(code.source_id, "user-source-actions");
        assert_eq!(
            code.policy_ref,
            "jobsentinel.source-policy.user-source-actions"
        );
        assert_eq!(code.policy_revision, 1);
        assert_eq!(code.operations, vec![SourceOperation::VisiblePageCapture]);
        assert_eq!(code.origin, "https://jobs.example");
    }

    #[test]
    fn applied_logging_pairing_has_one_exact_operation() {
        let now = chrono::Utc::now();
        let (_pairing, code) = issue_browser_applied_pairing("https://jobs.example", now).unwrap();

        assert_eq!(code.client_id, "browser-import");
        assert_eq!(code.source_id, "user-source-actions");
        assert_eq!(
            code.policy_ref,
            "jobsentinel.source-policy.user-source-actions"
        );
        assert_eq!(code.policy_revision, 1);
        assert_eq!(code.operations, vec![SourceOperation::AppliedLogging]);
        assert_eq!(code.origin, "https://jobs.example");
    }

    #[tokio::test]
    async fn storage_adapter_rechecks_the_installed_visible_capture_contract() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        crate::v3_source_governance::install_user_source_actions(&database)
            .await
            .unwrap();
        let repository = StorageBookmarkletRepository { database };
        let grant = SourceGrantState::Granted {
            source_id: "user-source-actions".to_string(),
            policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
            permission:
                jobsentinel_domain::v3_source_manifest::SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 1,
        };

        assert!(repository.authorize_browser_action(&grant).await.unwrap());
        assert!(!repository
            .authorize_browser_action(&SourceGrantState::Missing)
            .await
            .unwrap());
        let applied_grant = SourceGrantState::Granted {
            source_id: "user-source-actions".to_string(),
            policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
            permission:
                jobsentinel_domain::v3_source_manifest::SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::AppliedLogging,
            policy_revision: 1,
        };
        assert!(repository
            .authorize_browser_action(&applied_grant)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn storage_adapter_rejects_credential_text_before_job_storage() {
        let database = Arc::new(Database::connect_memory().await.unwrap());
        database.migrate().await.unwrap();
        let repository = StorageBookmarkletRepository {
            database: database.clone(),
        };
        for (title, company, description) in [
            (
                "Authorization: Bearer secret-token-value-1234567890",
                "Example",
                None,
            ),
            ("Role", "sessionid=secret-session-value-1234567890", None),
            (
                "Role",
                "Example",
                Some("access_token=secret-token-value-1234567890"),
            ),
        ] {
            let mut job = Job::newly_discovered(
                title,
                company,
                "https://jobs.example/posting/1",
                None,
                "user-source-actions",
                chrono::Utc::now(),
            );
            job.description = description.map(str::to_string);

            assert!(repository.upsert_job(&job).await.is_err());
        }
        assert!(database.get_recent_jobs(1).await.unwrap().is_empty());
    }
}
