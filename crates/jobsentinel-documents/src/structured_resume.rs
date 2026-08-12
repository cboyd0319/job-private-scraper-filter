use jobsentinel_domain::ResumeEvidenceSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Resume template identifier accepted by rendering and export boundaries.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemplateId {
    Classic,
    Modern,
    Technical,
    Executive,
    Military,
    #[default]
    Professional,
    Traditional,
}

impl TemplateId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Modern => "modern",
            Self::Technical => "technical",
            Self::Executive => "executive",
            Self::Military => "military",
            Self::Professional => "professional",
            Self::Traditional => "traditional",
        }
    }

    pub fn rendering_id(self) -> Self {
        match self {
            Self::Professional => Self::Classic,
            Self::Traditional => Self::Executive,
            rendering_id => rendering_id,
        }
    }
}

impl FromStr for TemplateId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "classic" => Ok(Self::Classic),
            "modern" => Ok(Self::Modern),
            "technical" => Ok(Self::Technical),
            "executive" => Ok(Self::Executive),
            "military" => Ok(Self::Military),
            "professional" => Ok(Self::Professional),
            "traditional" => Ok(Self::Traditional),
            _ => Err("Invalid template ID".to_string()),
        }
    }
}

/// Canonical structured resume shared by rendering, export, analysis, and storage adapters.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StructuredResume {
    pub personal: ResumePersonalInfo,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub experience: Vec<ResumeExperience>,
    #[serde(default)]
    pub education: Vec<ResumeEducation>,
    #[serde(default)]
    pub skills: Vec<ResumeSkillCategory>,
    #[serde(default)]
    pub certifications: Vec<ResumeCertification>,
    #[serde(default)]
    pub projects: Vec<ResumeProject>,
    #[serde(default)]
    pub clearance: Option<String>,
    #[serde(default)]
    pub military_info: Option<String>,
}

/// Structured resume plus fields used only by analysis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResumeAnalysisInput {
    pub resume: StructuredResume,
    #[serde(default)]
    pub custom_sections: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub evidence_snapshot: Option<ResumeEvidenceSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumePersonalInfo {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub linkedin: Option<String>,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeExperience {
    pub title: String,
    pub company: String,
    #[serde(default)]
    pub location: Option<String>,
    pub start_date: String,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default, alias = "bullets")]
    pub achievements: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeEducation {
    pub institution: String,
    pub degree: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_of_study: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub graduation_date: Option<String>,
    #[serde(default)]
    pub gpa: Option<String>,
    #[serde(default)]
    pub honors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResumeSkillCategory {
    pub name: String,
    #[serde(default)]
    pub skills: Vec<ResumeSkill>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResumeSkill {
    pub name: String,
    #[serde(default)]
    pub proficiency: Option<String>,
    #[serde(default)]
    pub years_experience: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCertification {
    pub name: String,
    pub issuer: String,
    #[serde(default)]
    pub date_obtained: Option<String>,
    #[serde(default)]
    pub expiration_date: Option<String>,
    #[serde(default)]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeProject {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub technologies: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}
