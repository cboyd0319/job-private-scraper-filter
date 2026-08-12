//! Import type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Import-specific errors
#[derive(Error, Debug)]
pub enum ImportError {
    #[error("HTTP request failed")]
    HttpRequest,

    #[error("HTTP request returned status {0}")]
    HttpStatus(u16),

    #[error("HTTP response body read failed")]
    HttpBodyRead(#[from] jobsentinel_network::HttpBodyReadError),

    #[error("No Schema.org JobPosting data found at URL")]
    NoSchemaOrgData,

    #[error("Multiple JobPosting objects found - unable to choose automatically")]
    MultipleJobPostings(usize),

    #[error("Missing required field: {field}")]
    MissingRequiredField { field: String },

    #[error("Invalid JSON-LD format")]
    InvalidJsonLd(String),

    #[error("HTML parsing failed")]
    HtmlParseError(String),

    #[error("Database operation failed")]
    DatabaseError(String),

    #[error("Job already exists")]
    AlreadyExists,

    #[error("Import preview is missing or expired")]
    PendingImportNotFound,

    #[error("Pasted job details exceed the local draft limit")]
    SmartPasteTooLarge,

    #[error("Pasted {field} exceeds the local draft limit")]
    SmartPasteFieldTooLong { field: &'static str },

    #[error("Pasted job details contain session or credential material")]
    SmartPasteCredentialMaterial,

    #[error("Automated source access is unavailable")]
    SourcePolicyBlocked { visible_capture_allowed: bool },

    #[error("Source review is required")]
    SourceReviewRequired,

    #[error("Reviewed source authorization is unavailable")]
    SourceAuthorizationUnavailable,

    #[error("Timeout while fetching URL")]
    Timeout,

    #[error("Redirect blocked while fetching URL")]
    RedirectBlocked { location: String },

    #[error("URL validation failed")]
    InvalidUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_blocked_display_does_not_expose_location() {
        let error = ImportError::RedirectBlocked {
            location: "https://user:pass@example.com/final?token=secret123#private".to_string(),
        };

        let message = error.to_string();

        assert!(message.contains("Redirect blocked"));
        assert!(!message.contains("example.com"));
        assert!(!message.contains("secret123"));
        assert!(!message.contains("user"));
        assert!(!message.contains("pass"));
        assert!(!message.contains("private"));
    }

    #[test]
    fn test_import_error_display_does_not_expose_internal_details() {
        let cases = [
            ImportError::InvalidUrl(
                "https://user:pass@example.com/job?token=secret#private".to_string(),
            ),
            ImportError::InvalidJsonLd("candidate-specific JSON-LD content".to_string()),
            ImportError::HtmlParseError("private parser detail".to_string()),
            ImportError::DatabaseError("sqlite error at <local-private-db>".to_string()),
        ];

        for error in cases {
            let message = error.to_string();
            assert!(!message.contains("secret"));
            assert!(!message.contains("candidate-specific"));
            assert!(!message.contains("private parser detail"));
            assert!(!message.contains("<local-private-db>"));
        }
    }
}

pub(super) type ImportResult<T> = Result<T, ImportError>;

/// Preview of what will be imported (shown to user before confirming)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobImportPreview {
    /// Opaque identifier for saving the exact details shown in this preview.
    pub import_id: Option<String>,

    /// Job title
    pub title: String,

    /// Company name
    pub company: String,

    /// Canonical job URL after removing userinfo, fragments, and sensitive query parameters.
    pub url: String,

    /// Location (formatted)
    pub location: Option<String>,

    /// Description (truncated for preview)
    pub description_preview: Option<String>,

    /// Salary information (formatted)
    pub salary: Option<String>,

    /// Date posted
    pub date_posted: Option<DateTime<Utc>>,

    /// Expiry date
    pub valid_through: Option<DateTime<Utc>>,

    /// Employment type(s)
    pub employment_types: Vec<String>,

    /// Is remote?
    pub remote: bool,

    /// Missing required fields (if any)
    pub missing_fields: Vec<String>,

    /// Whether this job already exists in the database
    pub already_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedJobSummary {
    pub job_id: i64,
}

impl JobImportPreview {
    /// Check if this preview has all required fields for import
    pub fn is_valid(&self) -> bool {
        self.missing_fields.is_empty()
    }

    /// Get validation error message
    pub fn validation_error(&self) -> Option<String> {
        if self.missing_fields.is_empty() {
            None
        } else {
            Some(format!(
                "Missing required fields: {}",
                self.missing_fields.join(", ")
            ))
        }
    }
}
