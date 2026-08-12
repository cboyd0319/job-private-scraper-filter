use super::{has_stored_path, resume_file_display_name};
use crate::application::automation::{
    requires_user_answer, ApplicationAttempt, ApplicationProfile, ScreeningAnswer,
};
use serde::{Deserialize, Serialize};

/// Response type for application profile (frontend-friendly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationProfileResponse {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub linkedin_url: Option<String>,
    pub github_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub website_url: Option<String>,
    pub has_resume_file: bool,
    pub resume_file_name: Option<String>,
    pub us_work_authorized: bool,
    pub requires_sponsorship: bool,
    pub max_applications_per_day: i32,
    pub require_manual_approval: bool,
}

impl From<ApplicationProfile> for ApplicationProfileResponse {
    fn from(p: ApplicationProfile) -> Self {
        Self {
            full_name: p.full_name,
            email: p.email,
            phone: p.phone,
            linkedin_url: p.linkedin_url,
            github_url: p.github_url,
            portfolio_url: p.portfolio_url,
            website_url: p.website_url,
            has_resume_file: p
                .resume_file_path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty()),
            resume_file_name: p
                .resume_file_path
                .as_deref()
                .and_then(resume_file_display_name),
            us_work_authorized: p.us_work_authorized,
            requires_sponsorship: p.requires_sponsorship,
            max_applications_per_day: p.max_applications_per_day,
            require_manual_approval: p.require_manual_approval,
        }
    }
}

/// Minimal profile preview response for non-settings UI surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationProfilePreviewResponse {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub linkedin_url: Option<String>,
    pub github_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub website_url: Option<String>,
    pub us_work_authorized: bool,
    pub requires_sponsorship: bool,
}

impl From<ApplicationProfile> for ApplicationProfilePreviewResponse {
    fn from(p: ApplicationProfile) -> Self {
        Self {
            full_name: p.full_name,
            email: p.email,
            phone: p.phone,
            linkedin_url: p.linkedin_url,
            github_url: p.github_url,
            portfolio_url: p.portfolio_url,
            website_url: p.website_url,
            us_work_authorized: p.us_work_authorized,
            requires_sponsorship: p.requires_sponsorship,
        }
    }
}

/// Response type for screening answers (frontend-friendly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreeningAnswerResponse {
    pub id: i64,
    pub question_pattern: String,
    pub answer: String,
    pub answer_type: Option<String>,
    pub notes: Option<String>,
    pub times_used: i32,
    pub times_modified: i32,
    pub confidence_score: f64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ScreeningAnswer> for ScreeningAnswerResponse {
    fn from(a: ScreeningAnswer) -> Self {
        Self {
            id: a.id,
            question_pattern: a.question_pattern,
            answer: a.answer,
            answer_type: a.answer_type,
            notes: a.notes,
            times_used: a.times_used,
            times_modified: a.times_modified,
            confidence_score: a.confidence_score,
            last_used_at: a.last_used_at.map(|dt| dt.to_rfc3339()),
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

/// Minimal saved-answer data for application review surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplicationScreeningAnswerPreviewResponse {
    pub question_pattern: String,
    pub answer: String,
}

pub(crate) fn application_screening_answer_previews(
    answers: Vec<ScreeningAnswer>,
) -> Vec<ApplicationScreeningAnswerPreviewResponse> {
    answers
        .into_iter()
        .filter(|answer| !requires_user_answer(&answer.question_pattern))
        .map(|answer| ApplicationScreeningAnswerPreviewResponse {
            question_pattern: answer.question_pattern,
            answer: answer.answer,
        })
        .collect()
}

/// Response type for automation attempts (frontend-friendly)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttemptResponse {
    pub id: i64,
    pub job_hash: String,
    pub application_id: Option<i64>,
    pub status: String,
    pub ats_platform: String,
    pub error_message: Option<String>,
    pub has_screenshot: bool,
    pub has_confirmation_screenshot: bool,
    pub automation_duration_ms: Option<i64>,
    pub user_approved: bool,
    pub submitted_at: Option<String>,
    pub created_at: String,
}

impl From<ApplicationAttempt> for AttemptResponse {
    fn from(a: ApplicationAttempt) -> Self {
        let has_screenshot = has_stored_path(a.screenshot_path.as_deref());
        let has_confirmation_screenshot =
            has_stored_path(a.confirmation_screenshot_path.as_deref());

        Self {
            id: a.id,
            job_hash: a.job_hash,
            application_id: a.application_id,
            status: a.status.as_str().to_string(),
            ats_platform: a.ats_platform.as_str().to_string(),
            error_message: a.error_message,
            has_screenshot,
            has_confirmation_screenshot,
            automation_duration_ms: a.automation_duration_ms,
            user_approved: a.user_approved,
            submitted_at: a.submitted_at.map(|dt| dt.to_rfc3339()),
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

/// ATS detection response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AtsDetectionResponse {
    pub platform: String,
    pub common_fields: Vec<String>,
    pub automation_notes: Option<String>,
}
