//! Owns paired local bookmarklet imports and explicit pending-job review.

mod pairing;
#[cfg(test)]
mod pairing_test_support;
mod pending;
mod server;

use async_trait::async_trait;
use jobsentinel_domain::{v3_source_authorization::SourceGrantState, Job};

pub use pairing::{
    CompanionPairing, CompanionPairingCode, CompanionPairingError, CompanionRequest,
    COMPANION_PROTOCOL_VERSION,
};
#[cfg(test)]
pub(crate) use pairing_test_support::companion_request_for_test;
pub use pending::{
    BookmarkletImportConfirmResult, PendingBookmarkletImportPreview, PendingBookmarkletImports,
};
pub use server::{
    confirm_pending_bookmarklet_imports, discard_pending_bookmarklet_imports, BookmarkletConfig,
    BookmarkletError, BookmarkletServer,
};

#[async_trait]
pub trait BookmarkletRepository: Send + Sync {
    async fn authorize_browser_action(&self, grant: &SourceGrantState) -> Result<bool, String>;
    async fn job_exists_by_hash(&self, hash: &str) -> Result<bool, String>;
    async fn upsert_job(&self, job: &Job) -> Result<i64, String>;
    async fn mark_job_applied(&self, hash: &str) -> Result<(), String>;
}

#[async_trait]
impl<T> BookmarkletRepository for std::sync::Arc<T>
where
    T: BookmarkletRepository + ?Sized,
{
    async fn authorize_browser_action(&self, grant: &SourceGrantState) -> Result<bool, String> {
        self.as_ref().authorize_browser_action(grant).await
    }

    async fn job_exists_by_hash(&self, hash: &str) -> Result<bool, String> {
        self.as_ref().job_exists_by_hash(hash).await
    }

    async fn upsert_job(&self, job: &Job) -> Result<i64, String> {
        self.as_ref().upsert_job(job).await
    }

    async fn mark_job_applied(&self, hash: &str) -> Result<(), String> {
        self.as_ref().mark_job_applied(hash).await
    }
}

use serde::{Deserialize, Serialize};

pub(super) fn constant_time_ascii_eq(left: &str, right: &str) -> bool {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let mut diff = left_bytes.len() ^ right_bytes.len();
    for index in 0..left_bytes.len().max(right_bytes.len()) {
        diff |= usize::from(
            left_bytes.get(index).copied().unwrap_or(0)
                ^ right_bytes.get(index).copied().unwrap_or(0),
        );
    }
    diff == 0
}

/// Job data received from bookmarklet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkletJobData {
    /// Job title
    #[serde(default)]
    pub title: String,

    /// Company name
    #[serde(default)]
    pub company: String,

    /// Job description
    #[serde(default)]
    pub description: String,

    /// Job URL
    pub url: String,

    /// Location (optional)
    #[serde(default)]
    pub location: Option<String>,

    /// Salary information (optional)
    #[serde(default)]
    pub salary: Option<String>,

    /// Remote indicator (optional)
    #[serde(default)]
    pub remote: Option<bool>,

    /// Schema.org JobPosting data (if available)
    #[serde(rename = "@type", default)]
    pub schema_type: Option<String>,

    /// Hiring organization from Schema.org
    #[serde(rename = "hiringOrganization", default)]
    pub hiring_organization: Option<serde_json::Value>,

    /// Job location from Schema.org
    #[serde(rename = "jobLocation", default)]
    pub job_location: Option<serde_json::Value>,

    /// Base salary from Schema.org
    #[serde(rename = "baseSalary", default)]
    pub base_salary: Option<serde_json::Value>,

    /// Date posted from Schema.org
    #[serde(rename = "datePosted", default)]
    pub date_posted: Option<String>,

    /// Job location type (e.g., TELECOMMUTE)
    #[serde(rename = "jobLocationType", default)]
    pub job_location_type: Option<String>,
}

impl BookmarkletJobData {
    /// Validate that required fields are present
    pub fn validate(&self) -> Result<(), String> {
        if self.url.trim().is_empty() {
            return Err("URL is required".to_string());
        }

        jobsentinel_domain::canonicalize_job_url(self.url.trim())
            .map_err(|_| "Job link must be a public https address".to_string())?;

        // Try to get title from Schema.org or fallback field
        let title = if let Some(ref schema_type) = self.schema_type {
            if schema_type == "JobPosting" {
                !self.title.is_empty()
            } else {
                false
            }
        } else {
            !self.title.is_empty()
        };

        if !title {
            return Err("Title is required".to_string());
        }

        // Try to get company from Schema.org or fallback field
        let has_company = if let Some(ref org) = self.hiring_organization {
            org.get("name").and_then(|n| n.as_str()).is_some()
        } else {
            !self.company.is_empty()
        };

        if !has_company {
            return Err("Company name is required".to_string());
        }

        Ok(())
    }

    /// Extract company name from Schema.org or fallback
    pub fn get_company(&self) -> Option<String> {
        if let Some(ref org) = self.hiring_organization {
            if let Some(name) = org.get("name").and_then(|n| n.as_str()) {
                return Some(name.to_string());
            }
        }
        if !self.company.is_empty() {
            Some(self.company.clone())
        } else {
            None
        }
    }

    /// Extract location from Schema.org or fallback
    pub fn get_location(&self) -> Option<String> {
        if let Some(ref loc) = self.job_location {
            if let Some(obj) = loc.as_object() {
                if let Some(address) = obj.get("address") {
                    if let Some(addr_obj) = address.as_object() {
                        let mut parts = Vec::new();
                        if let Some(city) = addr_obj.get("addressLocality").and_then(|v| v.as_str())
                        {
                            parts.push(city);
                        }
                        if let Some(state) = addr_obj.get("addressRegion").and_then(|v| v.as_str())
                        {
                            parts.push(state);
                        }
                        if !parts.is_empty() {
                            return Some(parts.join(", "));
                        }
                    }
                }
                if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                    return Some(name.to_string());
                }
            }
        }
        self.location.clone()
    }

    /// Check if remote from Schema.org or fallback
    pub fn is_remote(&self) -> bool {
        if let Some(ref job_type) = self.job_location_type {
            job_type == "TELECOMMUTE"
        } else {
            self.remote.unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmarklet_job_data_validation() {
        let data = BookmarkletJobData {
            title: "Care Coordinator".to_string(),
            company: "Community Care".to_string(),
            description: "A great job".to_string(),
            url: "https://example.com/job".to_string(),
            location: Some("San Francisco, CA".to_string()),
            salary: None,
            remote: None,
            schema_type: None,
            hiring_organization: None,
            job_location: None,
            base_salary: None,
            date_posted: None,
            job_location_type: None,
        };

        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_bookmarklet_job_data_missing_url() {
        let data = BookmarkletJobData {
            title: "Care Coordinator".to_string(),
            company: "Community Care".to_string(),
            description: "A great job".to_string(),
            url: "".to_string(),
            location: None,
            salary: None,
            remote: None,
            schema_type: None,
            hiring_organization: None,
            job_location: None,
            base_salary: None,
            date_posted: None,
            job_location_type: None,
        };

        assert!(data.validate().is_err());
    }

    #[test]
    fn test_bookmarklet_job_data_rejects_unsafe_urls() {
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "http://127.0.0.1/internal",
            "http://127.0.0.1.nip.io/internal",
        ] {
            let data = BookmarkletJobData {
                title: "Care Coordinator".to_string(),
                company: "Community Care".to_string(),
                description: "A great job".to_string(),
                url: url.to_string(),
                location: None,
                salary: None,
                remote: None,
                schema_type: None,
                hiring_organization: None,
                job_location: None,
                base_salary: None,
                date_posted: None,
                job_location_type: None,
            };

            assert!(data.validate().is_err(), "{url}");
        }
    }

    #[test]
    fn test_schema_org_company_extraction() {
        let data = BookmarkletJobData {
            title: "Care Coordinator".to_string(),
            company: "".to_string(),
            description: "".to_string(),
            url: "https://example.com/job".to_string(),
            location: None,
            salary: None,
            remote: None,
            schema_type: Some("JobPosting".to_string()),
            hiring_organization: Some(serde_json::json!({"name": "Community Care"})),
            job_location: None,
            base_salary: None,
            date_posted: None,
            job_location_type: None,
        };

        assert_eq!(data.get_company(), Some("Community Care".to_string()));
    }

    #[test]
    fn test_remote_detection() {
        let data = BookmarkletJobData {
            title: "Care Coordinator".to_string(),
            company: "Community Care".to_string(),
            description: "".to_string(),
            url: "https://example.com/job".to_string(),
            location: None,
            salary: None,
            remote: None,
            schema_type: None,
            hiring_organization: None,
            job_location: None,
            base_salary: None,
            date_posted: None,
            job_location_type: Some("TELECOMMUTE".to_string()),
        };

        assert!(data.is_remote());
    }
}
