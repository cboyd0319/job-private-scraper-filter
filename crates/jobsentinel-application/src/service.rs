use chrono::Utc;

use jobsentinel_domain::{
    canonicalize_job_url,
    v3_source_authorization::{
        automated_url_fetch_is_blocked, visible_page_capture_is_blocked, SourceActionDecision,
        SourceGrantState,
    },
    v3_source_manifest::SourceOperation,
    Job,
};
use jobsentinel_sources::{parse_single_job_page, JobPageParseError, ParsedJobPage};
use jobsentinel_storage::Database;

use super::{
    fetcher::fetch_job_page,
    pending::PendingUrlImports,
    types::{ImportError, ImportResult, ImportedJobSummary, JobImportPreview},
    v3_source_governance::authorize_user_source_action,
};

pub fn prepare_job_import_target(url: &str) -> ImportResult<String> {
    let canonical_url = canonicalize_job_url(url).map_err(ImportError::InvalidUrl)?;
    if automated_url_fetch_is_blocked(&canonical_url) {
        return Err(ImportError::SourcePolicyBlocked {
            visible_capture_allowed: !visible_page_capture_is_blocked(&canonical_url),
        });
    }
    Ok(canonical_url)
}

pub fn employer_discovery_review_grant() -> SourceGrantState {
    SourceGrantState::Granted {
        source_id: "user-source-actions".to_string(),
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        permission: jobsentinel_domain::v3_source_manifest::SourcePermission::UserReview,
        operation: SourceOperation::EmployerDiscovery,
        policy_revision: 1,
    }
}

pub async fn preview_job_import(
    database: &Database,
    pending: &PendingUrlImports,
    url: &str,
    grant: SourceGrantState,
) -> ImportResult<JobImportPreview> {
    let canonical_url = prepare_job_import_target(url)?;
    require_employer_discovery_authorization(database, grant.clone()).await?;
    let html = fetch_job_page(&canonical_url).await?;
    preview_job_from_html(database, pending, canonical_url, &html, grant).await
}

pub async fn confirm_job_import(
    database: &Database,
    pending: &PendingUrlImports,
    import_id: &str,
) -> ImportResult<ImportedJobSummary> {
    confirm_pending_job(
        database,
        pending,
        import_id,
        SourceOperation::EmployerDiscovery,
        true,
    )
    .await
}

pub(super) async fn confirm_pending_job(
    database: &Database,
    pending: &PendingUrlImports,
    import_id: &str,
    operation: SourceOperation,
    connectivity_required: bool,
) -> ImportResult<ImportedJobSummary> {
    let now = Utc::now();
    let (job, grant) = pending
        .find(import_id, now)
        .ok_or(ImportError::PendingImportNotFound)?;
    require_user_source_authorization(database, operation, connectivity_required, grant).await?;

    match database
        .insert_job_if_new(&job)
        .await
        .map_err(database_error)?
    {
        Some(job_id) => {
            pending.remove(import_id);
            tracing::info!(
                job_id,
                title_chars = job.title.chars().count(),
                company_chars = job.company.chars().count(),
                "Reviewed job import saved"
            );
            Ok(ImportedJobSummary { job_id })
        }
        None => {
            pending.remove(import_id);
            Err(ImportError::AlreadyExists)
        }
    }
}

async fn require_employer_discovery_authorization(
    database: &Database,
    grant: SourceGrantState,
) -> ImportResult<()> {
    require_user_source_authorization(database, SourceOperation::EmployerDiscovery, true, grant)
        .await
}

pub(super) async fn require_user_source_authorization(
    database: &Database,
    operation: SourceOperation,
    connectivity_required: bool,
    grant: SourceGrantState,
) -> ImportResult<()> {
    match authorize_user_source_action(database, operation, Utc::now().date_naive(), grant).await {
        Ok(SourceActionDecision::Allowed {
            connectivity_required: actual,
            ..
        }) if actual == connectivity_required => Ok(()),
        Ok(SourceActionDecision::ReviewRequired) => Err(ImportError::SourceReviewRequired),
        _ => Err(ImportError::SourceAuthorizationUnavailable),
    }
}

async fn preview_job_from_html(
    database: &Database,
    pending: &PendingUrlImports,
    canonical_url: String,
    html: &str,
    grant: SourceGrantState,
) -> ImportResult<JobImportPreview> {
    let parsed = parse_single_job_page(html).map_err(map_parse_error)?;
    let mut preview = JobImportPreview {
        import_id: None,
        title: parsed.title.clone(),
        company: parsed.company.clone(),
        url: canonical_url,
        location: parsed.location.clone(),
        description_preview: parsed.description_preview.clone(),
        salary: parsed.salary.clone(),
        date_posted: parsed.date_posted,
        valid_through: parsed.valid_through,
        employment_types: parsed.employment_types.clone(),
        remote: parsed.remote,
        missing_fields: parsed.missing_fields.clone(),
        already_exists: false,
    };
    if !preview.is_valid() {
        return Ok(preview);
    }

    let job = job_from_page(&parsed, &preview)?;
    preview.already_exists = database
        .job_exists_by_hash(&job.hash)
        .await
        .map_err(database_error)?;
    if !preview.already_exists {
        preview.import_id = Some(pending.queue(job, grant, Utc::now()));
    }

    Ok(preview)
}

fn map_parse_error(error: JobPageParseError) -> ImportError {
    match error {
        JobPageParseError::NoSchemaOrgData => ImportError::NoSchemaOrgData,
        JobPageParseError::MultipleJobPostings(count) => ImportError::MultipleJobPostings(count),
        JobPageParseError::HtmlParseError(details) => ImportError::HtmlParseError(details),
    }
}

fn job_from_page(parsed: &ParsedJobPage, preview: &JobImportPreview) -> ImportResult<Job> {
    if !preview.is_valid() {
        let field = preview
            .missing_fields
            .first()
            .cloned()
            .unwrap_or_else(|| "job details".to_string());
        return Err(ImportError::MissingRequiredField { field });
    }

    let discovered_at = Utc::now();
    let created_at = preview.date_posted.unwrap_or(discovered_at);

    Ok(Job {
        description: parsed.description.clone(),
        remote: Some(preview.remote),
        salary_min: parsed.salary_min,
        salary_max: parsed.salary_max,
        currency: parsed.currency.clone(),
        created_at,
        ..Job::newly_discovered(
            preview.title.clone(),
            preview.company.clone(),
            preview.url.clone(),
            preview.location.clone(),
            "user-source-actions",
            discovered_at,
        )
    })
}

fn database_error(error: impl ToString) -> ImportError {
    ImportError::DatabaseError(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use crate::v3_source_governance::{install_user_source_actions, user_source_actions_policy};

    static JOB_HTML: LazyLock<String> = LazyLock::new(|| {
        format!(
            r#"<script type="application/ld+json">{}</script>"#,
            include_str!(
                "../../jobsentinel-domain/src/fixtures/source_simulator/user_source_actions_v1.json"
            )
        )
    });

    async fn database() -> Database {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        install_user_source_actions(&database).await.unwrap();
        database
    }

    #[tokio::test]
    async fn url_preview_requires_reviewed_source_authorization_before_transport() {
        let database = database().await;
        let result = preview_job_import(
            &database,
            &PendingUrlImports::default(),
            "https://job-import-authorization.invalid/private",
            SourceGrantState::Missing,
        )
        .await;

        assert!(matches!(result, Err(ImportError::SourceReviewRequired)));
    }

    #[tokio::test]
    async fn url_preview_rejects_policy_blocked_sources_before_transport() {
        let database = database().await;
        let pending = PendingUrlImports::default();

        for (url, visible_capture_allowed) in [
            ("https://builtin.com/jobs/1", true),
            ("https://www.builtincolorado.com/jobs/1", true),
            ("https://www.dice.com/jobs/1", true),
            ("https://jobs.glassdoor.com/jobs/1", true),
            ("https://www.simplyhired.com/job/1", true),
            ("https://linkedin.com/jobs/view/1", false),
            ("https://www.linkedin.com/jobs/view/2", false),
            ("https://ycombinator.com/jobs", false),
            ("https://www.ycombinator.com/jobs", false),
        ] {
            let result =
                preview_job_import(&database, &pending, url, employer_discovery_review_grant())
                    .await;
            assert!(matches!(
                result,
                Err(ImportError::SourcePolicyBlocked {
                    visible_capture_allowed: actual
                }) if actual == visible_capture_allowed
            ));
        }
    }

    #[tokio::test]
    async fn preview_stages_and_confirm_saves_the_reviewed_job() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager?query=care".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await
        .unwrap();

        assert_eq!(preview.title, "Office Manager");
        assert_eq!(preview.company, "Example Services");
        assert_eq!(preview.location.as_deref(), Some("Denver, CO"));
        assert_eq!(preview.salary.as_deref(), Some("USD 25-30 per hour"));
        let import_id = preview.import_id.expect("valid preview should be staged");

        let saved = confirm_job_import(&database, &pending, &import_id)
            .await
            .unwrap();
        let job = database.get_job_by_id(saved.job_id).await.unwrap().unwrap();
        assert_eq!(job.title, "Office Manager");
        assert_eq!(job.source, "user-source-actions");
        assert_eq!(job.salary_min, Some(52_000));
        assert_eq!(job.salary_max, Some(62_400));
        assert_eq!(job.times_seen, 1);
    }

    #[tokio::test]
    async fn preview_marks_existing_jobs_without_staging_them() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let first = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await
        .unwrap();
        confirm_job_import(&database, &pending, first.import_id.as_deref().unwrap())
            .await
            .unwrap();

        let duplicate = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await
        .unwrap();

        assert!(duplicate.already_exists);
        assert!(duplicate.import_id.is_none());
    }

    #[tokio::test]
    async fn confirmation_reports_a_duplicate_created_after_preview() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await
        .unwrap();
        let import_id = preview.import_id.unwrap();
        let (staged, _) = pending.find(&import_id, Utc::now()).unwrap();
        database.insert_job_if_new(&staged).await.unwrap().unwrap();

        let result = confirm_job_import(&database, &pending, &import_id).await;

        assert!(matches!(result, Err(ImportError::AlreadyExists)));
        assert!(pending.find(&import_id, Utc::now()).is_none());
    }

    #[tokio::test]
    async fn confirmation_rechecks_source_policy_before_storage() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await
        .unwrap();
        let import_id = preview.import_id.unwrap();
        let mut newer_policy = user_source_actions_policy().unwrap();
        newer_policy.revision = 2;
        database.upsert_source_policy(&newer_policy).await.unwrap();

        let result = confirm_job_import(&database, &pending, &import_id).await;

        assert!(matches!(
            result,
            Err(ImportError::SourceAuthorizationUnavailable)
        ));
        assert!(pending.find(&import_id, Utc::now()).is_some());
    }

    #[tokio::test]
    async fn confirmation_uses_the_grant_retained_with_the_pending_job() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_job_from_html(
            &database,
            &pending,
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            SourceGrantState::Missing,
        )
        .await
        .unwrap();
        let import_id = preview.import_id.unwrap();

        let result = confirm_job_import(&database, &pending, &import_id).await;

        assert!(matches!(result, Err(ImportError::SourceReviewRequired)));
        assert!(pending.find(&import_id, Utc::now()).is_some());
    }

    #[tokio::test]
    async fn preview_does_not_mask_database_failures() {
        let database = database().await;
        database.close().await;

        let result = preview_job_from_html(
            &database,
            &PendingUrlImports::default(),
            "https://example.com/jobs/office-manager".to_string(),
            JOB_HTML.as_str(),
            employer_discovery_review_grant(),
        )
        .await;

        assert!(matches!(result, Err(ImportError::DatabaseError(_))));
    }

    #[test]
    fn multiple_job_postings_require_a_more_specific_page() {
        let html = format!("{}{}", JOB_HTML.as_str(), JOB_HTML.as_str());
        let result = parse_single_job_page(&html).map_err(map_parse_error);

        assert!(matches!(result, Err(ImportError::MultipleJobPostings(2))));
    }
}
