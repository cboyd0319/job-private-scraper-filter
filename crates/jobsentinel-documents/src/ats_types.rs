//! Resume readability analyzer result types.

use jobsentinel_domain::ResumeEvidenceCitation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfessionMatchingProfile {
    Technical,
    Content,
    Operations,
    Healthcare,
    Service,
    Trades,
    Education,
    Sales,
    EarlyCareer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionalMatchingProfile {
    #[serde(rename = "us")]
    UnitedStates,
    #[serde(rename = "uk")]
    UnitedKingdom,
    #[serde(rename = "eu")]
    EuropeanUnion,
    India,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeMatchingProfile {
    pub profession: ProfessionMatchingProfile,
    pub region: RegionalMatchingProfile,
}

/// Complete readability analysis result for a resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtsAnalysisResult {
    /// Overall application readability score (0-100)
    pub overall_score: f64,
    /// Keyword matching score (0-100)
    pub keyword_score: f64,
    /// Format safety score (0-100)
    pub format_score: f64,
    /// Resume completeness score (0-100)
    pub completeness_score: f64,
    /// Keywords found in resume
    pub keyword_matches: Vec<KeywordMatch>,
    /// Important keywords missing from resume
    pub missing_keywords: Vec<String>,
    /// Important keywords missing from resume with job-post importance
    pub missing_keyword_details: Vec<MissingKeyword>,
    /// Format issues that may make a resume hard to parse
    pub format_issues: Vec<FormatIssue>,
    /// Requirement-by-requirement local review with evidence state
    pub requirement_reviews: Vec<RequirementReview>,
    /// Missing required hard constraints that cap confidence
    pub hard_constraint_risks: Vec<HardConstraintRisk>,
    /// Improvement suggestions
    pub suggestions: Vec<AtsSuggestion>,
    /// Explicit caller-selected matching context. Never inferred from resume content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matching_profile: Option<ResumeMatchingProfile>,
}

/// A keyword found in the resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordMatch {
    /// The keyword or phrase
    pub keyword: String,
    /// Resume sections where found
    pub found_in: Vec<String>,
    /// Number of times mentioned
    pub frequency: usize,
    /// How important this keyword is
    pub importance: KeywordImportance,
}

/// A keyword from the job post that was not clearly found in the resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingKeyword {
    /// The keyword or phrase
    pub keyword: String,
    /// How important this keyword is in the job post
    pub importance: KeywordImportance,
}

/// How clearly a job-post requirement appears in the resume
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequirementMatchState {
    /// Visible in resume text or structured experience
    Direct,
    /// Visible in more than one evidence area or repeated naturally
    Strong,
    /// Visible only in a lighter evidence area such as a skills list
    Partial,
    /// Related evidence may exist, but the requirement is not clearly named
    Implied,
    /// Not clearly found
    Missing,
}

/// A single job-post requirement reviewed against resume evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementReview {
    /// The requirement or role-language phrase
    pub keyword: String,
    /// How important this requirement is in the job post
    pub importance: KeywordImportance,
    /// How clearly the resume shows this requirement
    pub match_state: RequirementMatchState,
    /// Resume areas where evidence was found
    pub evidence_sections: Vec<String>,
    /// Opaque references to exact local resume fields that support the match
    #[serde(default)]
    pub evidence_citations: Vec<ResumeEvidenceCitation>,
    /// Whether this looks like a hard requirement to verify before tailoring
    pub hard_constraint: bool,
    /// Whether exact evidence appears in a section preferred by the selected profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_preferred_section: Option<bool>,
    /// Plain next step for the job seeker
    pub recommendation: String,
}

/// Hard requirement category for cautious score caps
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardConstraintCategory {
    /// Work authorization, visa sponsorship, or legal work eligibility
    WorkAuthorization,
    /// Citizenship requirement
    Citizenship,
    /// Security clearance requirement
    SecurityClearance,
    /// Required license or certification
    LicenseOrCertification,
    /// Required degree or education credential
    Education,
    /// Required years or level of experience
    Experience,
    /// Required language fluency
    Language,
    /// Required minimum age or legal work-age eligibility
    Age,
    /// Required background, drug, or pre-employment screening
    BackgroundScreening,
    /// Required physical demand such as lifting or prolonged standing
    PhysicalRequirement,
    /// Required location, onsite, relocation, travel, schedule, or availability constraint
    Location,
}

/// Missing hard requirement that should cap local fit confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardConstraintRisk {
    /// Requirement phrase from the job post
    pub requirement: String,
    /// Hard requirement category
    pub category: HardConstraintCategory,
    /// Maximum score allowed while this requirement is missing
    pub score_cap: f64,
    /// Why the cap exists
    pub reason: String,
    /// User-facing next step
    pub action: String,
}

/// Importance level of a keyword
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum KeywordImportance {
    /// Must-have requirement from job description
    Required,
    /// Nice-to-have from job description
    Preferred,
    /// Common industry term
    Industry,
}

/// A formatting issue that may affect resume parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatIssue {
    /// How serious this issue is
    pub severity: IssueSeverity,
    /// Description of the issue
    pub issue: String,
    /// How to fix it
    pub fix: String,
}

/// Severity level of a format issue
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Will likely cause a parser to miss content
    Critical,
    /// May cause parsing issues
    Warning,
    /// Suggestion for improvement
    Info,
}

/// Suggestion for improving application readability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtsSuggestion {
    /// Category of suggestion
    pub category: SuggestionCategory,
    /// The suggestion text
    pub suggestion: String,
    /// Expected impact if implemented
    pub impact: String,
}

/// Category of readability suggestion
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SuggestionCategory {
    /// Add missing keyword
    AddKeyword,
    /// Improve bullet point wording
    RewordBullet,
    /// Add missing section
    AddSection,
    /// Reorder content for better impact
    ReorderContent,
    /// Fix formatting issue
    FormatFix,
}
