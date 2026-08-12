use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock};

const APPLICATION_SCREENING_ALIAS_TAXONOMY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../resources/taxonomies/application-screening-aliases.json"
));

static APPLICATION_SCREENING_ALIAS_TAXONOMY: LazyLock<ApplicationScreeningAliasTaxonomy> =
    LazyLock::new(load_application_screening_alias_taxonomy);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationScreeningAliasTaxonomy {
    schema_version: u32,
    requires_user_answer_patterns: Vec<String>,
    plain_screening_pattern_aliases: Vec<PlainScreeningPatternAlias>,
    legacy_screening_patterns: Vec<LegacyScreeningPatternAlias>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlainScreeningPatternAlias {
    patterns: Vec<String>,
    label: String,
    editable_pattern: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyScreeningPatternAlias {
    pattern: String,
    label: String,
    editable_pattern: String,
    aliases: Vec<String>,
}

fn load_application_screening_alias_taxonomy() -> ApplicationScreeningAliasTaxonomy {
    let taxonomy: ApplicationScreeningAliasTaxonomy =
        match serde_json::from_str(APPLICATION_SCREENING_ALIAS_TAXONOMY_JSON) {
            Ok(taxonomy) => taxonomy,
            Err(error) => {
                panic!("application screening alias taxonomy must be valid JSON: {error}")
            }
        };

    assert_eq!(
        taxonomy.schema_version, 1,
        "unsupported application screening alias taxonomy schema version"
    );
    assert!(
        !taxonomy.requires_user_answer_patterns.is_empty()
            && taxonomy
                .requires_user_answer_patterns
                .iter()
                .all(|pattern| !pattern.trim().is_empty()),
        "application screening user-answer patterns must be non-empty"
    );
    assert!(
        !taxonomy.plain_screening_pattern_aliases.is_empty()
            && taxonomy
                .plain_screening_pattern_aliases
                .iter()
                .all(|alias| !alias.label.trim().is_empty()
                    && !alias.editable_pattern.trim().is_empty()
                    && !alias.patterns.is_empty()
                    && alias
                        .patterns
                        .iter()
                        .all(|pattern| !pattern.trim().is_empty())),
        "application screening plain aliases must be non-empty"
    );
    assert!(
        !taxonomy.legacy_screening_patterns.is_empty()
            && taxonomy.legacy_screening_patterns.iter().all(|alias| !alias
                .label
                .trim()
                .is_empty()
                && !alias.pattern.trim().is_empty()
                && !alias.editable_pattern.trim().is_empty()
                && !alias.aliases.is_empty()
                && alias
                    .aliases
                    .iter()
                    .all(|pattern| !pattern.trim().is_empty())),
        "application screening legacy aliases must be non-empty"
    );

    taxonomy
}

/// Lifecycle status of an automated job application.
///
/// Applications move through these states:
/// 1. `Pending` - Created but not yet started
/// 2. `InProgress` - Currently being automated
/// 3. `AwaitingApproval` - Filled, waiting for user review (default mode)
/// 4. `Submitted` - Successfully submitted
/// 5. `Failed` - Automation failed (error logged)
/// 6. `Cancelled` - User cancelled before submission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AutomationStatus {
    Pending,
    InProgress,
    AwaitingApproval,
    Submitted,
    Failed,
    Cancelled,
}

impl AutomationStatus {
    /// Convert status to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::AwaitingApproval => "awaiting_approval",
            Self::Submitted => "submitted",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse status from database string.
    ///
    /// Returns `Failed` for unknown strings (fail-safe).
    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "in_progress" => Self::InProgress,
            "awaiting_approval" => Self::AwaitingApproval,
            "submitted" => Self::Submitted,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Failed,
        }
    }
}

/// Applicant Tracking System (ATS) platform identifier.
///
/// Used to select the correct automation strategy for form filling.
/// Each platform has unique DOM structure and API patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AtsPlatform {
    Greenhouse,
    Lever,
    Workday,
    SmartRecruiters,
    Workable,
    Recruitee,
    Taleo,
    Icims,
    BambooHr,
    AshbyHq,
    BreezyHr,
    JazzHr,
    Bullhorn,
    Jobvite,
    Teamtailor,
    SuccessFactors,
    OracleRecruiting,
    Phenom,
    Personio,
    Comeet,
    Jobylon,
    Eightfold,
    AdpRecruiting,
    Ukg,
    Rippling,
    ZohoRecruit,
    Freshteam,
    Pinpoint,
    JobScore,
    Unknown,
}

impl AtsPlatform {
    /// Convert platform to database string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Greenhouse => "greenhouse",
            Self::Lever => "lever",
            Self::Workday => "workday",
            Self::SmartRecruiters => "smartrecruiters",
            Self::Workable => "workable",
            Self::Recruitee => "recruitee",
            Self::Taleo => "taleo",
            Self::Icims => "icims",
            Self::BambooHr => "bamboohr",
            Self::AshbyHq => "ashbyhq",
            Self::BreezyHr => "breezyhr",
            Self::JazzHr => "jazzhr",
            Self::Bullhorn => "bullhorn",
            Self::Jobvite => "jobvite",
            Self::Teamtailor => "teamtailor",
            Self::SuccessFactors => "successfactors",
            Self::OracleRecruiting => "oracle_recruiting",
            Self::Phenom => "phenom",
            Self::Personio => "personio",
            Self::Comeet => "comeet",
            Self::Jobylon => "jobylon",
            Self::Eightfold => "eightfold",
            Self::AdpRecruiting => "adp_recruiting",
            Self::Ukg => "ukg",
            Self::Rippling => "rippling",
            Self::ZohoRecruit => "zoho_recruit",
            Self::Freshteam => "freshteam",
            Self::Pinpoint => "pinpoint",
            Self::JobScore => "jobscore",
            Self::Unknown => "unknown",
        }
    }

    /// Parse platform from database string.
    ///
    /// Returns `Unknown` for unrecognized platforms.
    pub fn from_str(s: &str) -> Self {
        match s {
            "greenhouse" => Self::Greenhouse,
            "lever" => Self::Lever,
            "workday" => Self::Workday,
            "smartrecruiters" => Self::SmartRecruiters,
            "workable" => Self::Workable,
            "recruitee" => Self::Recruitee,
            "taleo" => Self::Taleo,
            "icims" => Self::Icims,
            "bamboohr" => Self::BambooHr,
            "ashbyhq" => Self::AshbyHq,
            "breezyhr" => Self::BreezyHr,
            "jazzhr" => Self::JazzHr,
            "bullhorn" => Self::Bullhorn,
            "jobvite" => Self::Jobvite,
            "teamtailor" => Self::Teamtailor,
            "successfactors" => Self::SuccessFactors,
            "oracle_recruiting" => Self::OracleRecruiting,
            "phenom" => Self::Phenom,
            "personio" => Self::Personio,
            "comeet" => Self::Comeet,
            "jobylon" => Self::Jobylon,
            "eightfold" => Self::Eightfold,
            "adp_recruiting" => Self::AdpRecruiting,
            "ukg" => Self::Ukg,
            "rippling" => Self::Rippling,
            "zoho_recruit" => Self::ZohoRecruit,
            "freshteam" => Self::Freshteam,
            "pinpoint" => Self::Pinpoint,
            "jobscore" => Self::JobScore,
            _ => Self::Unknown,
        }
    }
}

/// Record of a single automation attempt for a job application.
///
/// Tracks the full lifecycle including timing, status, errors, and user approval.
/// Screenshots are stored for debugging and user verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationAttempt {
    /// Database ID of this attempt.
    pub id: i64,
    /// Job hash linking to the jobs table.
    pub job_hash: String,
    /// Optional link to saved application record.
    pub application_id: Option<i64>,
    /// Current lifecycle status.
    pub status: AutomationStatus,
    /// Detected ATS platform for this job.
    pub ats_platform: AtsPlatform,
    /// Error message if automation failed.
    pub error_message: Option<String>,
    /// Path to screenshot taken during automation (debugging).
    pub screenshot_path: Option<String>,
    /// Path to confirmation page screenshot (proof of submission).
    pub confirmation_screenshot_path: Option<String>,
    /// Total time spent automating this application (milliseconds).
    pub automation_duration_ms: Option<i64>,
    /// Whether user has approved this for submission (human-in-the-loop).
    pub user_approved: bool,
    /// Timestamp when application was submitted (if successful).
    pub submitted_at: Option<DateTime<Utc>>,
    /// When this attempt was created.
    pub created_at: DateTime<Utc>,
}

/// Aggregated statistics for automation performance tracking.
///
/// Used for dashboard metrics and health monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationStats {
    /// Total number of automation attempts (all statuses).
    pub total_attempts: i64,
    /// Count of successfully submitted applications.
    pub submitted: i64,
    /// Count of failed attempts.
    pub failed: i64,
    /// Count of approved attempts waiting to be processed.
    pub pending: i64,
    /// Success rate percentage (submitted / total * 100).
    pub success_rate: f64,
}

/// User's application profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationProfile {
    pub id: i64,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub linkedin_url: Option<String>,
    pub github_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub website_url: Option<String>,
    pub default_resume_id: Option<i64>,
    pub resume_file_path: Option<String>,
    pub default_cover_letter_template: Option<String>,
    pub us_work_authorized: bool,
    pub requires_sponsorship: bool,
    pub max_applications_per_day: i32,
    pub require_manual_approval: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating or updating an application profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApplicationProfileInput {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub linkedin_url: Option<String>,
    pub github_url: Option<String>,
    pub portfolio_url: Option<String>,
    pub website_url: Option<String>,
    pub default_resume_id: Option<i64>,
    pub resume_file_path: Option<String>,
    pub resume_file_token: Option<String>,
    pub clear_resume_file: Option<bool>,
    pub default_cover_letter_template: Option<String>,
    pub us_work_authorized: bool,
    pub requires_sponsorship: bool,
    pub max_applications_per_day: i32,
    pub require_manual_approval: bool,
}

/// Saved answer to a screening question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningAnswer {
    pub id: i64,
    pub question_pattern: String,
    pub answer: String,
    pub answer_type: Option<String>,
    pub notes: Option<String>,
    pub times_used: i32,
    pub times_modified: i32,
    pub confidence_score: f64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Suggested screening answer with a confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerSuggestion {
    pub answer: String,
    pub confidence: f64,
    pub source: AnswerSource,
    pub times_used: i32,
    pub times_modified: i32,
    pub last_used_days_ago: Option<i32>,
    pub modification_rate: f64,
}

/// Origin of a screening-answer suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnswerSource {
    Manual { pattern: String, answer_id: i64 },
    Learned { pattern: String, learned_id: i64 },
    Historical { original_question: String },
}

/// Aggregate statistics for a saved answer pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerStatistics {
    pub pattern: String,
    pub answer: String,
    pub times_used: i32,
    pub times_modified: i32,
    pub modification_rate: f64,
    pub confidence_score: f64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub recent_modifications: Vec<ModificationExample>,
}

/// Example of a user-modified screening answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModificationExample {
    pub original_answer: String,
    pub modified_to: String,
    pub question_text: String,
    pub modified_at: DateTime<Utc>,
}

mod matching;

pub use matching::{requires_user_answer, screening_question_matches};
