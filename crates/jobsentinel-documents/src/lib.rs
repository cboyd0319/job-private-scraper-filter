//! Local resume document parsing, analysis, rendering, and export.

mod ats_analyzer;
mod ats_types;
mod export;
mod format_taxonomy;
mod parser;
mod resume_match_score;
mod skills;
mod structured_resume;
mod templates;
mod types;

pub use ats_analyzer::AtsAnalyzer;
pub use ats_types::{
    AtsAnalysisResult, AtsSuggestion, FormatIssue, HardConstraintCategory, HardConstraintRisk,
    IssueSeverity, KeywordImportance, KeywordMatch, MissingKeyword, ProfessionMatchingProfile,
    RegionalMatchingProfile, RequirementMatchState, RequirementReview, ResumeMatchingProfile,
    SuggestionCategory,
};
pub use export::ResumeExporter;
pub use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};
pub use parser::ResumeParser;
pub use resume_match_score::{calculate_resume_match_score, ResumeMatchScore};
pub use skills::{ExtractedSkill, SkillExtractor};
pub use structured_resume::{
    ResumeAnalysisInput, ResumeCertification, ResumeEducation, ResumeExperience,
    ResumePersonalInfo, ResumeProject, ResumeSkill, ResumeSkillCategory, StructuredResume,
    TemplateId,
};
pub use templates::{Template, TemplateRenderer};
pub use types::{
    DegreeLevel, EducationRequirement, ExperienceRequirement, JobSkill, MatchResult,
    MatchResultWithJob, NewSkill, NullableFieldUpdate, Resume, ResumeMatchFeedback,
    ResumeMatchFeedbackLabel, SkillUpdate, UserSkill,
};
