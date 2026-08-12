use super::BookmarkletJobData;
use chrono::{DateTime, Utc};
use jobsentinel_domain::{
    v3_source_authorization::SourceGrantState, v3_source_manifest::SourceOperation,
};
use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

const MAX_PENDING_BOOKMARKLET_IMPORTS: usize = 50;
const DESCRIPTION_PREVIEW_CHARS: usize = 220;

pub type PendingBookmarkletImports = Arc<RwLock<Vec<PendingBookmarkletImport>>>;

#[derive(Debug, Clone)]
pub struct PendingBookmarkletImport {
    id: String,
    hash: String,
    received_at: DateTime<Utc>,
    job_data: BookmarkletJobData,
    grant: SourceGrantState,
    operation: SourceOperation,
    missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingBookmarkletImportPreview {
    pub id: String,
    pub title: String,
    pub company: String,
    pub url: String,
    pub location: Option<String>,
    pub description_preview: Option<String>,
    pub remote: bool,
    pub operation: SourceOperation,
    pub missing_fields: Vec<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookmarkletImportConfirmResult {
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct BookmarkletImportQueueResult {
    pub pending: usize,
    pub skipped: usize,
}

impl PendingBookmarkletImport {
    pub fn new(
        hash: String,
        job_data: BookmarkletJobData,
        grant: SourceGrantState,
        operation: SourceOperation,
        missing_fields: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            hash,
            received_at: Utc::now(),
            job_data,
            grant,
            operation,
            missing_fields,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn job_data(&self) -> BookmarkletJobData {
        self.job_data.clone()
    }

    pub fn grant(&self) -> SourceGrantState {
        self.grant.clone()
    }

    pub fn preview(&self) -> PendingBookmarkletImportPreview {
        PendingBookmarkletImportPreview {
            id: self.id.clone(),
            title: self.job_data.title.clone(),
            company: self.job_data.get_company().unwrap_or_default(),
            url: self.job_data.url.clone(),
            location: self.job_data.get_location(),
            description_preview: description_preview(&self.job_data.description),
            remote: self.job_data.is_remote(),
            operation: self.operation,
            missing_fields: self.missing_fields.clone(),
            received_at: self.received_at,
        }
    }
}

pub(super) fn new_pending_bookmarklet_imports() -> PendingBookmarkletImports {
    Arc::new(RwLock::new(Vec::new()))
}

pub(super) fn pending_bookmarklet_import_previews(
    pending_imports: &PendingBookmarkletImports,
) -> Vec<PendingBookmarkletImportPreview> {
    let pending = match pending_imports.read() {
        Ok(pending) => pending,
        Err(poisoned) => poisoned.into_inner(),
    };

    pending
        .iter()
        .map(PendingBookmarkletImport::preview)
        .collect()
}

pub(super) fn queue_pending_bookmarklet_imports(
    pending_imports: &PendingBookmarkletImports,
    imports: Vec<PendingBookmarkletImport>,
) -> BookmarkletImportQueueResult {
    if imports.is_empty() {
        return BookmarkletImportQueueResult {
            pending: 0,
            skipped: 0,
        };
    }

    let mut pending = match pending_imports.write() {
        Ok(pending) => pending,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut existing_hashes: HashSet<String> = pending
        .iter()
        .map(|pending_import| pending_import.hash().to_string())
        .collect();
    let mut added = 0usize;
    let mut skipped = 0usize;

    for pending_import in imports {
        let hash = pending_import.hash().to_string();
        if existing_hashes.contains(&hash) || pending.len() >= MAX_PENDING_BOOKMARKLET_IMPORTS {
            skipped += 1;
            continue;
        }

        existing_hashes.insert(hash);
        pending.push(pending_import);
        added += 1;
    }

    BookmarkletImportQueueResult {
        pending: added,
        skipped,
    }
}

pub(super) fn selected_pending_bookmarklet_imports(
    pending_imports: &PendingBookmarkletImports,
    ids: &[String],
) -> Vec<PendingBookmarkletImport> {
    if ids.is_empty() {
        return Vec::new();
    }

    let wanted: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let pending = match pending_imports.read() {
        Ok(pending) => pending,
        Err(poisoned) => poisoned.into_inner(),
    };

    pending
        .iter()
        .filter(|pending_import| wanted.contains(pending_import.id()))
        .cloned()
        .collect()
}

pub(super) fn remove_pending_bookmarklet_imports(
    pending_imports: &PendingBookmarkletImports,
    ids: &[String],
) -> usize {
    if ids.is_empty() {
        return 0;
    }

    let wanted: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let mut pending = match pending_imports.write() {
        Ok(pending) => pending,
        Err(poisoned) => poisoned.into_inner(),
    };
    let before = pending.len();
    pending.retain(|pending_import| !wanted.contains(pending_import.id()));

    before - pending.len()
}

fn description_preview(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut preview = trimmed
        .chars()
        .take(DESCRIPTION_PREVIEW_CHARS)
        .collect::<String>();
    if trimmed.chars().count() > DESCRIPTION_PREVIEW_CHARS {
        preview.push_str("...");
    }

    Some(preview)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(index: usize) -> BookmarkletJobData {
        BookmarkletJobData {
            title: format!("Job {index}"),
            company: "Example Company".to_string(),
            description: "Helpful role description".to_string(),
            url: format!("https://example.com/jobs/{index}"),
            location: Some("Denver, CO".to_string()),
            salary: None,
            remote: Some(false),
            schema_type: None,
            hiring_organization: None,
            job_location: None,
            base_salary: None,
            date_posted: None,
            job_location_type: None,
        }
    }

    fn grant() -> SourceGrantState {
        SourceGrantState::Granted {
            source_id: "user-source-actions".to_string(),
            policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
            permission:
                jobsentinel_domain::v3_source_manifest::SourcePermission::PairedBrowserGrant,
            operation: jobsentinel_domain::v3_source_manifest::SourceOperation::VisiblePageCapture,
            policy_revision: 1,
        }
    }

    #[test]
    fn queue_cap_keeps_existing_pending_imports() {
        let pending_imports = new_pending_bookmarklet_imports();
        let initial_imports = (0..MAX_PENDING_BOOKMARKLET_IMPORTS)
            .map(|index| {
                PendingBookmarkletImport::new(
                    format!("hash-{index}"),
                    job(index),
                    grant(),
                    SourceOperation::VisiblePageCapture,
                    Vec::new(),
                )
            })
            .collect();
        let overflow_imports = vec![PendingBookmarkletImport::new(
            "hash-overflow".to_string(),
            job(999),
            grant(),
            SourceOperation::VisiblePageCapture,
            Vec::new(),
        )];

        let initial_result = queue_pending_bookmarklet_imports(&pending_imports, initial_imports);
        let overflow_result = queue_pending_bookmarklet_imports(&pending_imports, overflow_imports);
        let previews = pending_bookmarklet_import_previews(&pending_imports);

        assert_eq!(initial_result.pending, MAX_PENDING_BOOKMARKLET_IMPORTS);
        assert_eq!(initial_result.skipped, 0);
        assert_eq!(overflow_result.pending, 0);
        assert_eq!(overflow_result.skipped, 1);
        assert_eq!(previews.len(), MAX_PENDING_BOOKMARKLET_IMPORTS);
        assert_eq!(previews[0].title, "Job 0");
        assert_eq!(
            previews[MAX_PENDING_BOOKMARKLET_IMPORTS - 1].title,
            "Job 49"
        );
        assert!(!previews.iter().any(|preview| preview.title == "Job 999"));
    }
}
