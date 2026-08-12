//! User-controlled LinkedIn Workbench ledger.
//!
//! This module records only explicit JobSentinel-side user actions. It does not
//! inspect LinkedIn pages, browser storage, cookies, network traffic, or session
//! state.

use crate::ats::{ApplicationStatus, ApplicationTracker};
use anyhow::{bail, Result};
use chrono::{Duration, NaiveDate, Utc};
use jobsentinel_domain::{
    canonicalize_job_url,
    v3_manifests::DataCategory,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_consent::{
        SourceConsentContext, SourceConsentOperation, SourceConsentReviewReason,
        SourceConsentStatus,
    },
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, SourcePermission,
        LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1,
    },
    Job,
};
use jobsentinel_security::sanitize_url_for_logging;
use jobsentinel_storage::Database;
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;
use uuid::Uuid;

const DEFAULT_LINKEDIN_URL: &str = "https://www.linkedin.com/jobs/";
const DEFAULT_APPLIED_URL: &str = "https://www.linkedin.com/jobs-tracker/?stage=applied";
const DEFAULT_TITLE: &str = "LinkedIn application";
const DEFAULT_COMPANY: &str = "Company needs details";
const SOURCE: &str = "LinkedIn";
const MAX_TITLE_CHARS: usize = 500;
const MAX_COMPANY_CHARS: usize = 200;
const MAX_NOTE_CHARS: usize = 5_000;
const WARNING_VERSION: u32 = 1;
const BEHAVIOR_REVISION: u32 = 1;
const WORKBENCH_DESTINATION: &str = "https://www.linkedin.com/jobs/";
const WORKBENCH_BEHAVIOR: &str =
    "linkedin-workbench-v1:user-opened-session;local-job-ledger;application-ledger;user-selected-text";
const CREDENTIAL_FIELD_PATTERN: &str = r"(?:[a-z0-9]+[_-])*(?:token|credential|password)(?:[_-][a-z0-9]+)*|(?:client|api|auth|private|oauth|access|refresh)[_-]?(?:secret|key)|secret|passphrase|session[_-]?id|authorization|proxy[_-]?authorization|cookie|set[_-]?cookie|li_at|jsessionid|bcookie|bscookie|liap|lidc|li_gc";

#[allow(clippy::expect_used)]
static NOTE_URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s"'<>\\)]+"#).expect("note URL regex must compile"));
#[allow(clippy::expect_used)]
static PROHIBITED_WORKBENCH_TEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"(?im)(?:["'](?:{CREDENTIAL_FIELD_PATTERN})["']\s*:\s*["'][^"'\r\n]{{4,}}["']|\b(?:{CREDENTIAL_FIELD_PATTERN})\b\s*=\s*[^\s,;]+|(?:cookie|set-cookie)\s*:\s*[-a-z0-9_]+\s*=|["']name["']\s*:\s*["'](?:li_at|jsessionid|bcookie|bscookie|liap|lidc|li_gc)["'])"#
    ))
    .expect("prohibited Workbench text regex must compile")
});

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkedInWorkbenchEventType {
    Applied,
    Saved,
    Tracking,
    Rejected,
    Interview,
    FollowUp,
    Reminder,
    Note,
    NotInterested,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedInWorkbenchEventInput {
    pub event_type: LinkedInWorkbenchEventType,
    pub title: Option<String>,
    pub company: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedInWorkbenchEventResult {
    pub job_id: i64,
    pub job_hash: String,
    pub application_id: Option<i64>,
    pub status: String,
    pub needs_details: bool,
    pub saved_as_bookmark: bool,
    pub hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkedInWorkbenchReviewStatus {
    Reviewed,
    ReviewRequired,
    PolicyRefreshRequired,
}

pub async fn record_event(
    database: &Database,
    input: LinkedInWorkbenchEventInput,
) -> Result<LinkedInWorkbenchEventResult> {
    authorize_workbench(database).await?;
    let event_type = input.event_type.clone();
    let title = trim_to_limit(input.title.as_deref(), MAX_TITLE_CHARS)
        .unwrap_or_else(|| DEFAULT_TITLE.to_string());
    let company = trim_to_limit(input.company.as_deref(), MAX_COMPANY_CHARS)
        .unwrap_or_else(|| DEFAULT_COMPANY.to_string());
    ensure_credential_free_source_text(&title)?;
    ensure_credential_free_source_text(&company)?;
    let notes = sanitize_workbench_notes(input.notes.as_deref())?;
    let user_url = input
        .url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let needs_url = user_url.is_none();
    let url = match user_url {
        Some(value) => canonicalize_job_url(value).map_err(anyhow::Error::msg)?,
        None if event_type == LinkedInWorkbenchEventType::Applied => {
            DEFAULT_APPLIED_URL.to_string()
        }
        None => DEFAULT_LINKEDIN_URL.to_string(),
    };
    let needs_details = title == DEFAULT_TITLE || company == DEFAULT_COMPANY || needs_url;
    let now = Utc::now();

    let mut job = Job {
        description: Some("Created from a user-controlled LinkedIn Workbench action.".to_string()),
        currency: Some("USD".to_string()),
        ..Job::newly_discovered(title, company, url, None, SOURCE, now)
    };
    if user_url.is_none() {
        job.hash = draft_hash();
    }
    let job_hash = job.hash.clone();

    let job_id = database.upsert_job(&job).await?;
    if let Some(notes) = notes.as_deref() {
        database.set_job_notes(job_id, Some(notes)).await?;
    }

    let mut application_id = None;
    let mut saved_as_bookmark = false;
    let mut hidden = false;
    let tracker = database.application_tracker();

    match event_type {
        LinkedInWorkbenchEventType::Applied => {
            let id = ensure_application(&tracker, &job_hash).await?;
            tracker
                .update_status(id, ApplicationStatus::Applied)
                .await?;
            application_id = Some(id);
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::Saved => {
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::Tracking => {
            application_id = Some(ensure_application(&tracker, &job_hash).await?);
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::Rejected => {
            let id = ensure_application(&tracker, &job_hash).await?;
            tracker
                .update_status(id, ApplicationStatus::Rejected)
                .await?;
            application_id = Some(id);
        }
        LinkedInWorkbenchEventType::Interview => {
            let id = ensure_application(&tracker, &job_hash).await?;
            tracker
                .update_status(id, ApplicationStatus::PhoneInterview)
                .await?;
            application_id = Some(id);
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::FollowUp => {
            let id = ensure_application(&tracker, &job_hash).await?;
            tracker.record_follow_up(id).await?;
            application_id = Some(id);
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::Reminder => {
            let id = ensure_application(&tracker, &job_hash).await?;
            tracker
                .set_reminder(
                    id,
                    "follow_up",
                    Utc::now() + Duration::days(3),
                    "Review this LinkedIn job in JobSentinel",
                )
                .await?;
            application_id = Some(id);
            database.set_bookmark(job_id, true).await?;
            saved_as_bookmark = true;
        }
        LinkedInWorkbenchEventType::Note => {}
        LinkedInWorkbenchEventType::NotInterested => {
            database.hide_job(job_id).await?;
            hidden = true;
        }
    }

    Ok(LinkedInWorkbenchEventResult {
        job_id,
        job_hash,
        application_id,
        status: status_for_event(&event_type).to_string(),
        needs_details,
        saved_as_bookmark,
        hidden,
    })
}

pub async fn workbench_review_is_current(database: &Database) -> Result<bool> {
    Ok(workbench_review_status(database).await? == LinkedInWorkbenchReviewStatus::Reviewed)
}

pub async fn workbench_review_status(database: &Database) -> Result<LinkedInWorkbenchReviewStatus> {
    workbench_review_status_on(database, Utc::now().date_naive()).await
}

pub async fn remember_workbench_review(database: &Database) -> Result<bool> {
    crate::v3_source_governance::install_linkedin_workbench(database).await?;
    remember_workbench_review_on(database, Utc::now().date_naive()).await
}

async fn remember_workbench_review_on(database: &Database, today: NaiveDate) -> Result<bool> {
    match workbench_review_status_on(database, today).await? {
        LinkedInWorkbenchReviewStatus::Reviewed => return Ok(true),
        LinkedInWorkbenchReviewStatus::PolicyRefreshRequired => {
            bail!("LinkedIn Workbench policy evidence needs refresh")
        }
        LinkedInWorkbenchReviewStatus::ReviewRequired => {}
    }
    let context = consent_context()?;
    match database.source_consent_status(&context).await? {
        SourceConsentStatus::Remembered { .. } => Ok(true),
        SourceConsentStatus::ReviewRequired {
            latest_event_id, ..
        } => {
            database
                .grant_source_consent(&context, latest_event_id.as_deref())
                .await?;
            Ok(workbench_review_status_on(database, today).await?
                == LinkedInWorkbenchReviewStatus::Reviewed)
        }
    }
}

pub async fn revoke_workbench_review(database: &Database) -> Result<bool> {
    database
        .revoke_source_consent(
            "linkedin-workbench",
            SourceConsentOperation::RestrictedWorkbench,
        )
        .await
}

pub async fn authorize_workbench(database: &Database) -> Result<()> {
    match workbench_review_status(database).await? {
        LinkedInWorkbenchReviewStatus::Reviewed => Ok(()),
        LinkedInWorkbenchReviewStatus::ReviewRequired => {
            bail!("LinkedIn Workbench review is required")
        }
        LinkedInWorkbenchReviewStatus::PolicyRefreshRequired => {
            bail!("LinkedIn Workbench policy evidence needs refresh")
        }
    }
}

async fn workbench_review_status_on(
    database: &Database,
    today: NaiveDate,
) -> Result<LinkedInWorkbenchReviewStatus> {
    let policy = crate::v3_source_governance::linkedin_workbench_policy()?;
    let manifest = parse_source_manifest(LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1, &policy)
        .map_err(anyhow::Error::msg)?;
    if database.get_source_policy(&policy.source_id).await? != Some(policy.clone())
        || database.get_source_manifest(&policy.source_id).await? != Some(manifest.clone())
    {
        return Ok(LinkedInWorkbenchReviewStatus::PolicyRefreshRequired);
    }
    let grant = match database.source_consent_status(&consent_context()?).await? {
        SourceConsentStatus::Remembered { .. } => SourceGrantState::Granted {
            source_id: policy.source_id.clone(),
            policy_ref: policy.policy_ref.clone(),
            permission: SourcePermission::UserReview,
            operation: SourceOperation::RestrictedWorkbench,
            policy_revision: policy.revision,
        },
        SourceConsentStatus::ReviewRequired {
            reason: SourceConsentReviewReason::Revoked,
            ..
        } => SourceGrantState::Revoked,
        SourceConsentStatus::ReviewRequired { .. } => SourceGrantState::Missing,
    };
    Ok(
        match manifest
            .authorize(&policy, SourceOperation::RestrictedWorkbench, today, grant)
            .map_err(anyhow::Error::msg)?
        {
            SourceActionDecision::Allowed { .. } => LinkedInWorkbenchReviewStatus::Reviewed,
            SourceActionDecision::ReviewRequired | SourceActionDecision::Revoked => {
                LinkedInWorkbenchReviewStatus::ReviewRequired
            }
            SourceActionDecision::Stale
            | SourceActionDecision::Blocked(_)
            | SourceActionDecision::Unsupported => {
                LinkedInWorkbenchReviewStatus::PolicyRefreshRequired
            }
        },
    )
}

fn consent_context() -> Result<SourceConsentContext> {
    let policy = crate::v3_source_governance::linkedin_workbench_policy()?;
    Ok(SourceConsentContext {
        source_id: policy.source_id,
        operation: SourceConsentOperation::RestrictedWorkbench,
        warning_version: WARNING_VERSION,
        behavior_revision: BEHAVIOR_REVISION,
        policy_ref: policy.policy_ref,
        policy_revision: policy.revision,
        source_class: policy.source_class,
        data_categories: vec![
            DataCategory::PublicJobPosting,
            DataCategory::ApplicationHistory,
            DataCategory::CareerGoals,
        ],
        destination_sha256: hex::encode(Sha256::digest(WORKBENCH_DESTINATION.as_bytes())),
        request_sha256: hex::encode(Sha256::digest(WORKBENCH_BEHAVIOR.as_bytes())),
    })
}

async fn ensure_application(tracker: &ApplicationTracker, job_hash: &str) -> Result<i64> {
    if let Some(id) = tracker.find_application_id_by_job_hash(job_hash).await? {
        return Ok(id);
    }

    tracker.create_application(job_hash).await
}

fn draft_hash() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"linkedin-workbench-draft:");
    hasher.update(Uuid::new_v4().as_bytes());
    hex::encode(hasher.finalize())
}

fn status_for_event(event_type: &LinkedInWorkbenchEventType) -> &'static str {
    match event_type {
        LinkedInWorkbenchEventType::Applied => "applied",
        LinkedInWorkbenchEventType::Saved => "saved",
        LinkedInWorkbenchEventType::Tracking => "tracking",
        LinkedInWorkbenchEventType::Rejected => "rejected",
        LinkedInWorkbenchEventType::Interview => "interview",
        LinkedInWorkbenchEventType::FollowUp => "follow_up",
        LinkedInWorkbenchEventType::Reminder => "reminder",
        LinkedInWorkbenchEventType::Note => "noted",
        LinkedInWorkbenchEventType::NotInterested => "not_interested",
    }
}

fn sanitize_workbench_notes(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let without_sensitive_urls = NOTE_URL_REGEX
        .replace_all(trimmed, |captures: &Captures<'_>| {
            let raw_url = captures.get(0).map_or("", |value| value.as_str());
            canonicalize_job_url(raw_url).unwrap_or_else(|_| sanitize_url_for_logging(raw_url))
        })
        .to_string();
    ensure_credential_free_source_text(&without_sensitive_urls)?;

    Ok(Some(
        without_sensitive_urls
            .chars()
            .take(MAX_NOTE_CHARS)
            .collect(),
    ))
}

pub(crate) fn ensure_credential_free_source_text(value: &str) -> Result<()> {
    let without_safe_markers = value.replace("[REDACTED]", "").replace("[removed]", "");
    let bearer_credential = |value: &str| {
        let value = value.trim_matches(['"', '\'', ',', ';', '(', ')']);
        let token_chars = |value: &str| {
            value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._~+/=-".contains(&byte))
        };
        value.len() >= 24 && token_chars(value)
            || value.len() >= 12
                && value.split('.').count() >= 2
                && value
                    .split('.')
                    .all(|part| part.len() >= 4 && token_chars(part))
    };
    let encoded_credential = |value: &str| {
        let value = value.trim_matches(['"', '\'', ',', ';', '(', ')']);
        value.len() >= 8
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"+/=".contains(&byte))
            && (value.ends_with('=')
                || value
                    .bytes()
                    .filter(|byte| byte.is_ascii_uppercase() || b"0123456789+/".contains(byte))
                    .count()
                    >= 2)
    };
    let authorization_header = without_safe_markers.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        let Some(index) = lower.find("authorization:") else {
            return false;
        };
        let mut parts = line[index + "authorization:".len()..]
            .trim_start_matches([' ', '\t', '"', '\''])
            .split_whitespace();
        match (parts.next(), parts.next()) {
            (Some(scheme), Some(value)) if scheme.eq_ignore_ascii_case("bearer") => {
                bearer_credential(value)
            }
            (Some(scheme), Some(value))
                if scheme.eq_ignore_ascii_case("basic")
                    || scheme.eq_ignore_ascii_case("negotiate") =>
            {
                encoded_credential(value)
            }
            _ => false,
        }
    });
    let bare_bearer = without_safe_markers
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| {
            pair[0]
                .trim_matches(|character: char| !character.is_ascii_alphanumeric())
                .eq_ignore_ascii_case("bearer")
                && bearer_credential(pair[1])
        });
    if PROHIBITED_WORKBENCH_TEXT_REGEX.is_match(&without_safe_markers)
        || authorization_header
        || bare_bearer
    {
        bail!("LinkedIn Workbench text cannot contain session or credential material");
    }
    Ok(())
}

fn trim_to_limit(value: Option<&str>, limit: usize) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(limit).collect())
}

#[cfg(test)]
mod tests;
