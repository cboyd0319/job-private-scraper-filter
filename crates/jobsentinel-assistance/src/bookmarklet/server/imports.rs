use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::bookmarklet::{
    pending::{
        queue_pending_bookmarklet_imports, remove_pending_bookmarklet_imports,
        selected_pending_bookmarklet_imports, BookmarkletImportConfirmResult,
        BookmarkletImportQueueResult, PendingBookmarkletImport, PendingBookmarkletImports,
    },
    BookmarkletJobData, BookmarkletRepository, CompanionRequest,
};
use jobsentinel_domain::{
    calculate_job_hash, canonicalize_job_url,
    v3_source_authorization::{visible_page_capture_is_blocked, SourceGrantState},
    v3_source_manifest::SourceOperation,
    Job,
};

use super::{
    bookmarklet_payload_matches_request_origin, consume_active_pairing, json_error_response,
    ActiveCompanionPairing, BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE,
    BOOKMARKLET_DATABASE_FAILURE_MESSAGE, BOOKMARKLET_UNAUTHORIZED_MESSAGE,
    INVALID_BOOKMARKLET_PAYLOAD_MESSAGE, MAX_BOOKMARKLET_COMPANY_LENGTH,
    MAX_BOOKMARKLET_DESCRIPTION_LENGTH, MAX_BOOKMARKLET_JOBS_PER_REQUEST,
    MAX_BOOKMARKLET_LOCATION_LENGTH, MAX_BOOKMARKLET_TITLE_LENGTH, MAX_BOOKMARKLET_URL_LENGTH,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BookmarkletImportEnvelope {
    pairing: CompanionRequest,
    job: Option<Value>,
    jobs: Option<Vec<Value>>,
}

const MISSING_TITLE: &str = "Job title not added";
const MISSING_COMPANY: &str = "Company not added";

/// Handle import request
pub(super) async fn handle_import_request(
    request: &str,
    active_pairing: &ActiveCompanionPairing,
    pending_imports: PendingBookmarkletImports,
    repository: Arc<dyn BookmarkletRepository>,
) -> (String, String) {
    let body = if let Some(body_start) = request.find("\r\n\r\n") {
        &request[body_start + 4..]
    } else {
        return (
            json_error_response("Invalid request format"),
            "application/json".to_string(),
        );
    };

    let body_value: serde_json::Value = match serde_json::from_str(body) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(
                line = e.line(),
                column = e.column(),
                "Failed to parse bookmarklet job data"
            );
            return (
                json_error_response(INVALID_BOOKMARKLET_PAYLOAD_MESSAGE),
                "application/json".to_string(),
            );
        }
    };

    if !bookmarklet_payload_matches_request_origin(request, &body_value) {
        return (
            json_error_response("Invalid browser import origin"),
            "application/json".to_string(),
        );
    }

    let (pairing_request, job_values, batch_mode) = match parse_import_envelope(body_value) {
        Ok(envelope) => envelope,
        Err(message) => {
            return (json_error_response(message), "application/json".to_string());
        }
    };

    if super::external_https_origin(&pairing_request.origin)
        != super::request_header_value(request, "origin").and_then(super::external_https_origin)
    {
        return (
            json_error_response("Invalid browser import origin"),
            "application/json".to_string(),
        );
    }

    let grant = match consume_active_pairing(active_pairing, &pairing_request, Utc::now()) {
        Ok(grant) => grant,
        Err(_) => {
            return (
                json_error_response(BOOKMARKLET_UNAUTHORIZED_MESSAGE),
                "application/json".to_string(),
            );
        }
    };
    let mut job_data_values = Vec::with_capacity(job_values.len());

    for job_value in job_values {
        let job_data: BookmarkletJobData = match serde_json::from_value(job_value) {
            Ok(data) => data,
            Err(_) => {
                tracing::error!("Failed to parse bookmarklet job data");
                return (
                    json_error_response(INVALID_BOOKMARKLET_PAYLOAD_MESSAGE),
                    "application/json".to_string(),
                );
            }
        };

        job_data_values.push(job_data);
    }

    let queue_result = match queue_bookmarklet_jobs_for_review(
        repository.as_ref(),
        &pending_imports,
        job_data_values,
        batch_mode,
        grant,
    )
    .await
    {
        Ok(result) => result,
        Err(message) => {
            return (json_error_response(message), "application/json".to_string());
        }
    };

    if batch_mode {
        return (
            json!({
                "success": true,
                "message": "Jobs ready for review",
                "pending": queue_result.pending,
                "skipped": queue_result.skipped,
            })
            .to_string(),
            "application/json".to_string(),
        );
    }

    (
        json!({
            "success": true,
            "message": "Job ready for review",
            "pending": queue_result.pending,
            "skipped": queue_result.skipped,
        })
        .to_string(),
        "application/json".to_string(),
    )
}

fn parse_import_envelope(
    body: Value,
) -> Result<(CompanionRequest, Vec<Value>, bool), &'static str> {
    let envelope: BookmarkletImportEnvelope =
        serde_json::from_value(body).map_err(|_| INVALID_BOOKMARKLET_PAYLOAD_MESSAGE)?;
    match (envelope.job, envelope.jobs) {
        (Some(job), None) => Ok((envelope.pairing, vec![job], false)),
        (None, Some(jobs))
            if !jobs.is_empty() && jobs.len() <= MAX_BOOKMARKLET_JOBS_PER_REQUEST =>
        {
            Ok((envelope.pairing, jobs, true))
        }
        _ => Err(INVALID_BOOKMARKLET_PAYLOAD_MESSAGE),
    }
}

async fn queue_bookmarklet_jobs_for_review(
    repository: &dyn BookmarkletRepository,
    pending_imports: &PendingBookmarkletImports,
    job_data_values: Vec<BookmarkletJobData>,
    batch_mode: bool,
    grant: jobsentinel_domain::v3_source_authorization::SourceGrantState,
) -> Result<BookmarkletImportQueueResult, String> {
    let operation = granted_operation(&grant)?;
    require_browser_action_authorization(repository, &grant).await?;
    let mut pending_jobs = Vec::with_capacity(job_data_values.len());
    let mut skipped = 0usize;

    for job_data in job_data_values {
        let (job_data, missing_fields) = normalize_bookmarklet_job_data(job_data, operation)?;
        let job_hash = bookmarklet_job_hash(&job_data);

        match repository.job_exists_by_hash(&job_hash).await {
            Ok(true) if operation == SourceOperation::AppliedLogging => {}
            Ok(true) if batch_mode => {
                skipped += 1;
                continue;
            }
            Ok(true) => {
                return Err("Job already exists in database".to_string());
            }
            Ok(false) => {}
            Err(_e) => {
                tracing::error!(
                    error_kind = "database",
                    "Database error checking bookmarklet job existence"
                );
                return Err(BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string());
            }
        }

        pending_jobs.push(PendingBookmarkletImport::new(
            job_hash,
            job_data,
            grant.clone(),
            operation,
            missing_fields,
        ));
    }

    let mut result = queue_pending_bookmarklet_imports(pending_imports, pending_jobs);
    result.skipped += skipped;
    Ok(result)
}

pub async fn confirm_pending_bookmarklet_imports(
    repository: &dyn BookmarkletRepository,
    pending_imports: &PendingBookmarkletImports,
    ids: &[String],
) -> Result<BookmarkletImportConfirmResult, String> {
    let selected = selected_pending_bookmarklet_imports(pending_imports, ids);
    if selected.is_empty() {
        return Ok(BookmarkletImportConfirmResult {
            imported: 0,
            skipped: 0,
        });
    }

    let mut confirmed_ids = Vec::new();
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for pending_import in selected {
        match store_bookmarklet_job(
            repository,
            pending_import.job_data(),
            &pending_import.grant(),
        )
        .await
        {
            Ok(true) => {
                imported += 1;
                confirmed_ids.push(pending_import.id().to_string());
            }
            Ok(false) => {
                skipped += 1;
                confirmed_ids.push(pending_import.id().to_string());
            }
            Err(message) => return Err(message),
        }
    }

    remove_pending_bookmarklet_imports(pending_imports, &confirmed_ids);

    Ok(BookmarkletImportConfirmResult { imported, skipped })
}

pub fn discard_pending_bookmarklet_imports(
    pending_imports: &PendingBookmarkletImports,
    ids: &[String],
) -> usize {
    remove_pending_bookmarklet_imports(pending_imports, ids)
}

async fn store_bookmarklet_job(
    repository: &dyn BookmarkletRepository,
    job_data: BookmarkletJobData,
    grant: &jobsentinel_domain::v3_source_authorization::SourceGrantState,
) -> Result<bool, String> {
    require_browser_action_authorization(repository, grant).await?;
    let operation = granted_operation(grant)?;
    let job_data = normalize_bookmarklet_job_data(job_data, operation)?.0;
    let title = job_data.title.clone();
    let company = job_data.get_company().unwrap_or_default();
    let description = if job_data.description.trim().is_empty() {
        None
    } else {
        Some(job_data.description.trim().to_string())
    };
    let location = job_data
        .get_location()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let remote = job_data.is_remote();
    let url = job_data.url.clone();

    let job_hash = bookmarklet_job_hash(&job_data);

    match repository.job_exists_by_hash(&job_hash).await {
        Ok(true) if operation == SourceOperation::AppliedLogging => {
            repository
                .mark_job_applied(&job_hash)
                .await
                .map_err(|_| BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string())?;
            return Ok(true);
        }
        Ok(true) => return Ok(false),
        Ok(false) => {}
        Err(_e) => {
            tracing::error!(
                error_kind = "database",
                "Database error checking bookmarklet job existence"
            );
            return Err(BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string());
        }
    }

    let created_at = Utc::now();
    let job = Job {
        description: description.clone(),
        remote: Some(remote),
        ..Job::newly_discovered(
            title.clone(),
            company.clone(),
            url.clone(),
            location.clone(),
            "user-source-actions",
            created_at,
        )
    };

    match repository.upsert_job(&job).await {
        Ok(job_id) => {
            if operation == SourceOperation::AppliedLogging {
                repository
                    .mark_job_applied(&job_hash)
                    .await
                    .map_err(|_| BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string())?;
            }
            tracing::info!(
                job_id,
                job_hash_len = job_hash.len(),
                title_chars = title.chars().count(),
                company_chars = company.chars().count(),
                has_location = location.is_some(),
                remote,
                "Job imported from bookmarklet"
            );
            Ok(true)
        }
        Err(_e) => {
            tracing::error!(
                error_kind = "database",
                "Database error inserting bookmarklet job"
            );
            Err(BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string())
        }
    }
}

async fn require_browser_action_authorization(
    repository: &dyn BookmarkletRepository,
    grant: &SourceGrantState,
) -> Result<(), String> {
    match repository.authorize_browser_action(grant).await {
        Ok(true) => Ok(()),
        _ => Err(BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE.to_string()),
    }
}

fn normalize_bookmarklet_job_data(
    mut job_data: BookmarkletJobData,
    operation: SourceOperation,
) -> Result<(BookmarkletJobData, Vec<String>), String> {
    let mut missing_fields = Vec::new();
    if operation == SourceOperation::AppliedLogging {
        canonicalize_job_url(job_data.url.trim())
            .map_err(|_| "Job link must be a public https address".to_string())?;
        if !job_data.description.is_empty()
            || job_data.location.is_some()
            || job_data.salary.is_some()
            || job_data.remote.is_some()
            || job_data.schema_type.is_some()
            || job_data.hiring_organization.is_some()
            || job_data.job_location.is_some()
            || job_data.base_salary.is_some()
            || job_data.date_posted.is_some()
            || job_data.job_location_type.is_some()
        {
            return Err(INVALID_BOOKMARKLET_PAYLOAD_MESSAGE.to_string());
        }
        if job_data.title.trim().is_empty() {
            job_data.title = MISSING_TITLE.to_string();
            missing_fields.push("title".to_string());
        }
        if job_data.company.trim().is_empty() {
            job_data.company = MISSING_COMPANY.to_string();
            missing_fields.push("company".to_string());
        }
    } else {
        job_data.validate()?;
    }

    job_data.title = job_data.title.trim().to_string();
    job_data.company = job_data
        .get_company()
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    job_data.description = job_data.description.trim().to_string();
    job_data.location = job_data
        .get_location()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    job_data.url = canonicalize_job_url(job_data.url.trim())
        .map_err(|_| "Job link must be a public https address".to_string())?;
    if visible_page_capture_is_blocked(&job_data.url) {
        return Err("Browser Import is unavailable for this source".to_string());
    }
    validate_bookmarklet_job_storage_lengths(&job_data)?;

    Ok((job_data, missing_fields))
}

fn granted_operation(grant: &SourceGrantState) -> Result<SourceOperation, String> {
    match grant {
        SourceGrantState::Granted { operation, .. } => Ok(*operation),
        _ => Err(BOOKMARKLET_AUTHORIZATION_FAILURE_MESSAGE.to_string()),
    }
}

fn validate_bookmarklet_job_storage_lengths(job_data: &BookmarkletJobData) -> Result<(), String> {
    if job_data.title.len() > MAX_BOOKMARKLET_TITLE_LENGTH
        || job_data.company.len() > MAX_BOOKMARKLET_COMPANY_LENGTH
        || job_data.url.len() > MAX_BOOKMARKLET_URL_LENGTH
        || job_data
            .location
            .as_ref()
            .is_some_and(|location| location.len() > MAX_BOOKMARKLET_LOCATION_LENGTH)
        || job_data.description.len() > MAX_BOOKMARKLET_DESCRIPTION_LENGTH
    {
        return Err(BOOKMARKLET_DATABASE_FAILURE_MESSAGE.to_string());
    }

    Ok(())
}

fn bookmarklet_job_hash(job_data: &BookmarkletJobData) -> String {
    let company = job_data.get_company().unwrap_or_default();
    let location = job_data.get_location();

    calculate_job_hash(
        &company,
        &job_data.title,
        location.as_deref(),
        &job_data.url,
    )
}
