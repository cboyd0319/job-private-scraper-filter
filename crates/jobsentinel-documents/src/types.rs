//! Type definitions for the resume module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// Resume metadata for uploaded local resumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume {
    pub id: i64,
    pub name: String,
    pub file_path: String,
    pub parsed_text: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Extracted skill from resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSkill {
    pub id: i64,
    pub resume_id: i64,
    pub skill_name: String,
    pub skill_category: Option<String>,
    pub confidence_score: f64,
    pub years_experience: Option<f64>,
    pub proficiency_level: Option<String>,
    pub source: String,
}

/// Job skill requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSkill {
    pub id: i64,
    pub job_hash: String,
    pub skill_name: String,
    pub is_required: bool,
    pub skill_category: Option<String>,
}

/// Resume-job match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub id: i64,
    pub resume_id: i64,
    pub job_hash: String,
    pub overall_match_score: f64,
    pub skills_match_score: Option<f64>,
    pub experience_match_score: Option<f64>,
    pub education_match_score: Option<f64>,
    pub missing_skills: Vec<String>,
    pub matching_skills: Vec<String>,
    pub gap_analysis: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Explicit local feedback attached to one saved resume match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeMatchFeedbackLabel {
    Useful,
    NotRelevant,
}

impl ResumeMatchFeedbackLabel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Useful => "useful",
            Self::NotRelevant => "not_relevant",
        }
    }
}

impl TryFrom<&str> for ResumeMatchFeedbackLabel {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "useful" => Ok(Self::Useful),
            "not_relevant" => Ok(Self::NotRelevant),
            _ => Err("invalid resume match feedback label"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeMatchFeedback {
    pub match_id: i64,
    pub label: ResumeMatchFeedbackLabel,
    pub recorded_at: DateTime<Utc>,
}

/// Resume-job match result with job details (for frontend display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResultWithJob {
    pub id: i64,
    pub resume_id: i64,
    pub job_hash: String,
    pub job_title: String,
    pub company: String,
    pub overall_match_score: f64,
    pub skills_match_score: Option<f64>,
    pub experience_match_score: Option<f64>,
    pub education_match_score: Option<f64>,
    pub missing_skills: Vec<String>,
    pub matching_skills: Vec<String>,
    pub gap_analysis: Option<String>,
    pub feedback: Option<ResumeMatchFeedback>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Skill Management Types
// ============================================================================

/// Update payload for modifying a user skill
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SkillUpdate {
    pub skill_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub skill_category: NullableFieldUpdate<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub proficiency_level: NullableFieldUpdate<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update")]
    pub years_experience: NullableFieldUpdate<f64>,
}

/// Payload for adding a new skill
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewSkill {
    pub skill_name: String,
    pub skill_category: Option<String>,
    pub proficiency_level: Option<String>,
    pub years_experience: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NullableFieldUpdate<T> {
    Unset,
    Clear,
    Set(T),
}

impl<T> Default for NullableFieldUpdate<T> {
    fn default() -> Self {
        Self::Unset
    }
}

impl<T> NullableFieldUpdate<T> {
    pub fn is_unset(&self) -> bool {
        matches!(self, Self::Unset)
    }
}

fn deserialize_nullable_update<'de, D, T>(
    deserializer: D,
) -> Result<NullableFieldUpdate<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|value| match value {
        Some(value) => NullableFieldUpdate::Set(value),
        None => NullableFieldUpdate::Clear,
    })
}

// ============================================================================
// Experience & Education Matching Types
// ============================================================================

/// Experience requirement extracted from job description
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExperienceRequirement {
    /// The skill/technology this requirement applies to (None = general experience)
    pub skill: Option<String>,
    /// Minimum years required
    pub min_years: f64,
    /// Maximum years (for ranges like "3-5 years")
    pub max_years: Option<f64>,
    /// Whether this is a strict requirement or preferred
    pub is_required: bool,
}

/// Education/degree level for matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegreeLevel {
    None = 0,
    HighSchool = 1,
    Associate = 2,
    Bachelor = 3,
    Master = 4,
    PhD = 5,
}

impl DegreeLevel {
    /// Parse degree level from text
    pub fn from_text(text: &str) -> Option<Self> {
        let lower = text.to_lowercase();

        // PhD / Doctorate
        if lower.contains("ph.d") || lower.contains("phd") || lower.contains("doctorate") {
            return Some(DegreeLevel::PhD);
        }

        // Master's
        if lower.contains("master")
            || lower.contains("m.s.")
            || lower.contains("m.a.")
            || lower.contains("mba")
            || lower.contains("ms ")
            || lower.contains("ma ")
        {
            return Some(DegreeLevel::Master);
        }

        // Bachelor's
        if lower.contains("bachelor")
            || lower.contains("b.s.")
            || lower.contains("b.a.")
            || lower.contains("bs ")
            || lower.contains("ba ")
            || lower.contains("undergraduate")
        {
            return Some(DegreeLevel::Bachelor);
        }

        // Associate
        if lower.contains("associate") || lower.contains("a.s.") || lower.contains("a.a.") {
            return Some(DegreeLevel::Associate);
        }

        // High School
        if lower.contains("high school") || lower.contains("ged") || lower.contains("diploma") {
            return Some(DegreeLevel::HighSchool);
        }

        None
    }

    /// Get human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            DegreeLevel::None => "No degree",
            DegreeLevel::HighSchool => "High School",
            DegreeLevel::Associate => "Associate's",
            DegreeLevel::Bachelor => "Bachelor's",
            DegreeLevel::Master => "Master's",
            DegreeLevel::PhD => "PhD/Doctorate",
        }
    }
}

impl Default for DegreeLevel {
    fn default() -> Self {
        DegreeLevel::None
    }
}

/// Education requirement extracted from job description
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EducationRequirement {
    /// Minimum degree level required
    pub degree_level: DegreeLevel,
    /// Specific fields of study (CS, Engineering, etc.)
    pub fields: Vec<String>,
    /// Whether this is required or preferred
    pub is_required: bool,
}
