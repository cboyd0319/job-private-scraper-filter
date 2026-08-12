use chrono::Utc;
use jobsentinel_domain::{
    canonicalize_job_url,
    v3_source_authorization::SourceGrantState,
    v3_source_manifest::{SourceOperation, SourcePermission},
    Job,
};
use jobsentinel_storage::Database;

use super::{
    linkedin_workbench::ensure_credential_free_source_text,
    pending::PendingUrlImports,
    service::{confirm_pending_job, require_user_source_authorization},
    types::{ImportError, ImportResult, ImportedJobSummary, JobImportPreview},
};

const MAX_SMART_PASTE_CHARS: usize = 50_000;

#[derive(Default)]
pub struct SmartPasteEdits {
    pub title: Option<String>,
    pub company: Option<String>,
    pub url: Option<String>,
    pub location: Option<String>,
}

pub fn smart_paste_review_grant() -> SourceGrantState {
    SourceGrantState::Granted {
        source_id: "user-source-actions".to_string(),
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        permission: SourcePermission::UserReview,
        operation: SourceOperation::SmartPaste,
        policy_revision: 1,
    }
}

pub async fn preview_smart_paste(
    database: &Database,
    pending: &PendingUrlImports,
    text: &str,
    grant: SourceGrantState,
) -> ImportResult<JobImportPreview> {
    preview_smart_paste_draft(database, pending, text, SmartPasteEdits::default(), grant).await
}

pub async fn preview_smart_paste_draft(
    database: &Database,
    pending: &PendingUrlImports,
    text: &str,
    edits: SmartPasteEdits,
    grant: SourceGrantState,
) -> ImportResult<JobImportPreview> {
    require_user_source_authorization(database, SourceOperation::SmartPaste, false, grant.clone())
        .await?;
    let parsed = parse_smart_paste(text, edits)?;
    let mut preview = parsed.preview();
    if !preview.is_valid() {
        return Ok(preview);
    }

    let job = parsed.into_job()?;
    preview.already_exists = database
        .job_exists_by_hash(&job.hash)
        .await
        .map_err(database_error)?;
    if !preview.already_exists {
        preview.import_id = Some(pending.queue(job, grant, Utc::now()));
    }
    Ok(preview)
}

pub async fn confirm_smart_paste(
    database: &Database,
    pending: &PendingUrlImports,
    import_id: &str,
) -> ImportResult<ImportedJobSummary> {
    confirm_pending_job(
        database,
        pending,
        import_id,
        SourceOperation::SmartPaste,
        false,
    )
    .await
}

struct ParsedSmartPaste {
    title: String,
    company: String,
    url: String,
    location: Option<String>,
    description: Option<String>,
    missing_fields: Vec<String>,
}

impl ParsedSmartPaste {
    fn preview(&self) -> JobImportPreview {
        JobImportPreview {
            import_id: None,
            title: self.title.clone(),
            company: self.company.clone(),
            url: self.url.clone(),
            location: self.location.clone(),
            description_preview: self.description.as_deref().map(description_preview),
            salary: None,
            date_posted: None,
            valid_through: None,
            employment_types: Vec::new(),
            remote: false,
            missing_fields: self.missing_fields.clone(),
            already_exists: false,
        }
    }

    fn into_job(self) -> ImportResult<Job> {
        if let Some(field) = self.missing_fields.first() {
            return Err(ImportError::MissingRequiredField {
                field: field.clone(),
            });
        }
        let now = Utc::now();
        let mut job = Job::newly_discovered(
            self.title,
            self.company,
            self.url,
            self.location,
            "user-source-actions",
            now,
        );
        job.description = self.description;
        job.remote = Some(false);
        Ok(job)
    }
}

fn parse_smart_paste(text: &str, edits: SmartPasteEdits) -> ImportResult<ParsedSmartPaste> {
    if text.chars().count() > MAX_SMART_PASTE_CHARS {
        return Err(ImportError::SmartPasteTooLarge);
    }
    ensure_credential_free_source_text(text)
        .map_err(|_| ImportError::SmartPasteCredentialMaterial)?;

    let mut title = None;
    let mut company = None;
    let mut url = None;
    let mut location = None;
    let mut description = Vec::new();
    let mut unlabeled = Vec::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some((label, value)) = line.split_once(':') {
            let value = value.trim();
            match label.trim().to_ascii_lowercase().as_str() {
                "title" | "job title" if !value.is_empty() => {
                    title = Some(value.to_string());
                    continue;
                }
                "company" if !value.is_empty() => {
                    company = Some(value.to_string());
                    continue;
                }
                "location" if !value.is_empty() => {
                    location = Some(value.to_string());
                    continue;
                }
                "description" if !value.is_empty() => {
                    description.push(value.to_string());
                    continue;
                }
                "url" | "job link" if !value.is_empty() => {
                    url = Some(canonicalize_job_url(value).map_err(ImportError::InvalidUrl)?);
                    continue;
                }
                _ => {}
            }
        }
        if url.is_none() {
            if let Some(candidate) = line.split_whitespace().find_map(job_link_candidate) {
                url = Some(canonicalize_job_url(candidate).map_err(ImportError::InvalidUrl)?);
                continue;
            }
        }
        unlabeled.push(line.to_string());
    }

    if title.is_none() && !unlabeled.is_empty() {
        title = Some(unlabeled.remove(0));
    }
    if company.is_none() && !unlabeled.is_empty() {
        company = Some(unlabeled.remove(0));
    }
    description.extend(unlabeled);

    let title = edits.title.unwrap_or_else(|| title.unwrap_or_default());
    let company = edits.company.unwrap_or_else(|| company.unwrap_or_default());
    let url = match edits.url {
        Some(value) if value.trim().is_empty() => String::new(),
        Some(value) => canonicalize_job_url(value.trim()).map_err(ImportError::InvalidUrl)?,
        None => url.unwrap_or_default(),
    };
    let location = edits.location.or(location).and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    });
    for value in [
        title.as_str(),
        company.as_str(),
        location.as_deref().unwrap_or_default(),
    ] {
        ensure_credential_free_source_text(value)
            .map_err(|_| ImportError::SmartPasteCredentialMaterial)?;
    }
    check_field("job title", &title, 500)?;
    check_field("company name", &company, 200)?;
    if let Some(value) = &location {
        check_field("location", value, 200)?;
    }

    Ok(ParsedSmartPaste {
        missing_fields: [
            ("title", title.trim().is_empty()),
            ("company", company.trim().is_empty()),
            ("url", url.is_empty()),
        ]
        .into_iter()
        .filter(|(_, missing)| *missing)
        .map(|(field, _)| field.to_string())
        .collect(),
        title: title.trim().to_string(),
        company: company.trim().to_string(),
        url,
        location,
        description: (!description.is_empty()).then(|| description.join("\n")),
    })
}

fn job_link_candidate(value: &str) -> Option<&str> {
    let candidate = value.trim_matches(['<', '>', '(', ')', '[', ']', '"', '\'', ',']);
    candidate.starts_with("https://").then_some(candidate)
}

fn check_field(field: &'static str, value: &str, max_bytes: usize) -> ImportResult<()> {
    if value.len() > max_bytes {
        return Err(ImportError::SmartPasteFieldTooLong { field });
    }
    Ok(())
}

fn description_preview(description: &str) -> String {
    let mut chars = description.chars();
    let preview = chars.by_ref().take(500).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
    }
}

fn database_error(error: impl ToString) -> ImportError {
    ImportError::DatabaseError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3_source_governance::{install_user_source_actions, user_source_actions_policy};

    async fn database() -> Database {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        install_user_source_actions(&database).await.unwrap();
        database
    }

    #[tokio::test]
    async fn stages_text_without_fetching_and_saves_after_review() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_smart_paste(
            &database,
            &pending,
            "Title: Office Manager\nCompany: Example Services\n\
             Job link: https://smart-paste-must-not-fetch.invalid/jobs/office-manager?utm_source=copy\n\
             Location: Denver, CO\nDescription: Coordinate office operations.",
            smart_paste_review_grant(),
        )
        .await
        .unwrap();

        assert_eq!(
            preview.url,
            "https://smart-paste-must-not-fetch.invalid/jobs/office-manager"
        );
        let saved = confirm_smart_paste(&database, &pending, preview.import_id.as_deref().unwrap())
            .await
            .unwrap();
        let job = database.get_job_by_id(saved.job_id).await.unwrap().unwrap();
        assert_eq!(job.source, "user-source-actions");
        assert_eq!(
            job.description.as_deref(),
            Some("Coordinate office operations.")
        );
    }

    #[tokio::test]
    async fn requires_exact_authorization_before_staging() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let result = preview_smart_paste(
            &database,
            &pending,
            "Office Manager\nExample Services\nhttps://example.com/jobs/office-manager",
            SourceGrantState::Missing,
        )
        .await;

        assert!(matches!(result, Err(ImportError::SourceReviewRequired)));
        assert_eq!(pending.current_count(Utc::now()), 0);
    }

    #[tokio::test]
    async fn rejects_credential_material_before_queue_or_storage() {
        for credential in [
            "Authorization: Bearer eyJhbGciOi.fake.payload",
            "Cookie: li_at=AQED-opaque-value",
            "access_token=oauth-secret-value",
            "li_at=AQED-opaque-value",
            "password=hunter2",
        ] {
            let database = database().await;
            let pending = PendingUrlImports::default();
            let text = format!(
                "Office Manager\nExample Services\nhttps://example.com/jobs/office-manager\n{credential}"
            );
            let result =
                preview_smart_paste(&database, &pending, &text, smart_paste_review_grant()).await;

            assert!(
                matches!(result, Err(ImportError::SmartPasteCredentialMaterial)),
                "{credential}"
            );
            assert_eq!(pending.current_count(Utc::now()), 0, "{credential}");
            assert!(
                database.get_recent_jobs(1).await.unwrap().is_empty(),
                "{credential}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_credential_material_added_during_review() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let result = preview_smart_paste_draft(
            &database,
            &pending,
            "Office Manager\nExample Services\nhttps://example.com/jobs/office-manager",
            SmartPasteEdits {
                company: Some("Cookie: sessionid=secret-session-value-1234567890".to_string()),
                ..SmartPasteEdits::default()
            },
            smart_paste_review_grant(),
        )
        .await;

        assert!(matches!(
            result,
            Err(ImportError::SmartPasteCredentialMaterial)
        ));
        assert_eq!(pending.current_count(Utc::now()), 0);
        assert!(database.get_recent_jobs(1).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn confirmation_rechecks_source_policy_before_storage() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_smart_paste(
            &database,
            &pending,
            "Office Manager\nExample Services\nhttps://example.com/jobs/office-manager",
            smart_paste_review_grant(),
        )
        .await
        .unwrap();
        let import_id = preview.import_id.unwrap();
        let mut newer_policy = user_source_actions_policy().unwrap();
        newer_policy.revision = 2;
        database.upsert_source_policy(&newer_policy).await.unwrap();

        let result = confirm_smart_paste(&database, &pending, &import_id).await;

        assert!(matches!(
            result,
            Err(ImportError::SourceAuthorizationUnavailable)
        ));
        assert!(pending.find(&import_id, Utc::now()).is_some());
        assert!(database.get_recent_jobs(1).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn review_edits_complete_a_text_only_draft() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_smart_paste_draft(
            &database,
            &pending,
            "Office Manager\nExample Services\nCoordinate office operations.",
            SmartPasteEdits {
                url: Some("https://example.com/jobs/office-manager".to_string()),
                ..SmartPasteEdits::default()
            },
            smart_paste_review_grant(),
        )
        .await
        .unwrap();

        assert!(preview.import_id.is_some());
        assert!(preview.missing_fields.is_empty());
        assert_eq!(
            preview.description_preview.as_deref(),
            Some("Coordinate office operations.")
        );
    }

    #[tokio::test]
    async fn incomplete_text_stays_a_reviewable_unsaved_draft() {
        let database = database().await;
        let pending = PendingUrlImports::default();
        let preview = preview_smart_paste(
            &database,
            &pending,
            "Office Manager\nExample Services",
            smart_paste_review_grant(),
        )
        .await
        .unwrap();

        assert!(preview.import_id.is_none());
        assert_eq!(preview.missing_fields, ["url"]);
        assert_eq!(pending.current_count(Utc::now()), 0);
    }
}
