//! Resume matching Tauri commands
//!
//! Commands for resume upload, skill extraction, job-resume matching,
//! resume builder, and ATS analysis.

use crate::application::resume::{
    ActiveResumeAnalysisError, AtsAnalysisResult, AtsAnalyzer, MatchResult, MatchResultWithJob,
    NewSkill, Resume, ResumeAnalysisInput, ResumeExporter, ResumeMatchFeedback,
    ResumeMatchFeedbackLabel, ResumeMatchingProfile, SkillUpdate, StructuredResume, Template,
    TemplateId, TemplateRenderer, UserSkill,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use crate::ipc::limits::validate_optional_command_limit_i64;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[path = "resume_builder_commands.rs"]
pub(crate) mod resume_builder_commands;

#[path = "resume_file_commands.rs"]
pub(crate) mod resume_file_commands;
#[path = "resume_match_debugger_commands.rs"]
pub(crate) mod resume_match_debugger_commands;
#[path = "resume_military_transition_commands.rs"]
pub(crate) mod resume_military_transition_commands;
#[cfg(test)]
#[path = "resume_military_transition_tests.rs"]
mod resume_military_transition_tests;
#[cfg(test)]
use crate::application::resume::MAX_RESUME_FILE_BYTES;
#[cfg(test)]
use resume_file_commands::{
    delete_resume_with_file_cleanup, has_json_extension, managed_resume_upload_cleanup_path,
    safe_resume_upload_file_name, supported_resume_extension, validate_selected_resume,
};

const MAX_RESUME_TEXT_PREVIEW_CHARS: usize = 6_000;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResumeSummary {
    pub id: i64,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub format_label: String,
    pub has_readable_text: bool,
    pub readable_text_chars: usize,
}

impl From<Resume> for ResumeSummary {
    fn from(resume: Resume) -> Self {
        let readable_text_chars = resume
            .parsed_text
            .as_deref()
            .map(str::trim)
            .map(|text| text.chars().count())
            .unwrap_or(0);
        let format_label = resume_format_label(&resume);

        Self {
            id: resume.id,
            name: resume.name,
            is_active: resume.is_active,
            created_at: resume.created_at,
            updated_at: resume.updated_at,
            format_label,
            has_readable_text: readable_text_chars > 0,
            readable_text_chars,
        }
    }
}

fn resume_format_label(resume: &Resume) -> String {
    let extension = Path::new(&resume.file_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .or_else(|| {
            Path::new(&resume.name)
                .extension()
                .and_then(|extension| extension.to_str())
        })
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "pdf" => "PDF",
        "docx" => "DOCX",
        "txt" => "Plain text",
        "md" | "markdown" => "Markdown",
        "html" | "htm" => "HTML",
        _ => "Resume file",
    }
    .to_string()
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ResumeTextPreview {
    pub resume_id: i64,
    pub name: String,
    pub has_text: bool,
    pub text_preview: String,
    pub text_chars: usize,
    pub is_truncated: bool,
}

impl From<Resume> for ResumeTextPreview {
    fn from(resume: Resume) -> Self {
        let text = resume.parsed_text.unwrap_or_default();
        let readable_text = text.trim();
        let text_chars = readable_text.chars().count();
        let text_preview = readable_text
            .chars()
            .take(MAX_RESUME_TEXT_PREVIEW_CHARS)
            .collect::<String>();

        Self {
            resume_id: resume.id,
            name: resume.name,
            has_text: !readable_text.is_empty(),
            is_truncated: text_chars > text_preview.chars().count(),
            text_preview,
            text_chars,
        }
    }
}

/// Get active resume
#[tauri::command]
pub(crate) async fn get_active_resume(
    state: State<'_, AppState>,
) -> Result<Option<ResumeSummary>, String> {
    tracing::info!("Command: get_active_resume");

    let matcher = state.database.resume_matcher();
    matcher
        .get_active_resume()
        .await
        .map(|resume| resume.map(ResumeSummary::from))
        .map_err(|e| user_friendly_error("Failed to get resume", e))
}

/// Get an explicit local preview of readable resume text without file paths.
#[tauri::command]
pub(crate) async fn get_resume_text_preview(
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<ResumeTextPreview, String> {
    tracing::info!(resume_id, "Command: get_resume_text_preview");

    let matcher = state.database.resume_matcher();
    matcher
        .get_resume(resume_id)
        .await
        .map(ResumeTextPreview::from)
        .map_err(|e| user_friendly_error("Failed to get resume text preview", e))
}

/// Set active resume
#[tauri::command]
pub(crate) async fn set_active_resume(
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: set_active_resume (id: {})", resume_id);

    let matcher = state.database.resume_matcher();
    matcher
        .set_active_resume(resume_id)
        .await
        .map_err(|e| user_friendly_error("Failed to set active resume", e))
}

/// Get user skills from active resume
#[tauri::command]
pub(crate) async fn get_user_skills(
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<UserSkill>, String> {
    tracing::info!("Command: get_user_skills (resume_id: {})", resume_id);

    let matcher = state.database.resume_matcher();
    matcher
        .get_user_skills(resume_id)
        .await
        .map_err(|e| user_friendly_error("Failed to get skills", e))
}

/// Match resume to a job
#[tauri::command]
pub(crate) async fn match_resume_to_job(
    resume_id: i64,
    job_hash: String,
    state: State<'_, AppState>,
) -> Result<MatchResult, String> {
    let job_hash_chars = job_hash.chars().count();
    tracing::info!(resume_id, job_hash_chars, "Command: match_resume_to_job");

    let matcher = state.database.resume_matcher();
    matcher
        .match_resume_to_job(resume_id, &job_hash)
        .await
        .map_err(|e| user_friendly_error("Failed to match resume", e))
}

/// Get existing match result
#[tauri::command]
pub(crate) async fn get_match_result(
    resume_id: i64,
    job_hash: String,
    state: State<'_, AppState>,
) -> Result<Option<MatchResult>, String> {
    let job_hash_chars = job_hash.chars().count();
    tracing::info!(resume_id, job_hash_chars, "Command: get_match_result");

    let matcher = state.database.resume_matcher();
    matcher
        .get_match_result(resume_id, &job_hash)
        .await
        .map_err(|e| user_friendly_error("Failed to get match result", e))
}

/// Get recent match results for a resume
#[tauri::command]
pub(crate) async fn get_recent_matches(
    resume_id: i64,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<MatchResultWithJob>, String> {
    tracing::info!(
        "Command: get_recent_matches (resume: {}, limit: {:?})",
        resume_id,
        limit
    );

    let limit = validate_optional_command_limit_i64(limit, 10)?;
    let matcher = state.database.resume_matcher();
    matcher
        .get_recent_matches(resume_id, limit)
        .await
        .map_err(|e| user_friendly_error("Failed to get recent matches", e))
}

/// Set or clear an explicit local label on one saved resume match.
#[tauri::command]
pub(crate) async fn set_resume_match_feedback(
    match_id: i64,
    label: Option<ResumeMatchFeedbackLabel>,
    state: State<'_, AppState>,
) -> Result<Option<ResumeMatchFeedback>, String> {
    tracing::info!(
        match_id,
        has_label = label.is_some(),
        "Command: set resume match feedback"
    );

    state
        .database
        .resume_matcher()
        .set_match_feedback(match_id, label)
        .await
        .map_err(|e| user_friendly_error("Failed to save resume match feedback", e))
}

// ============================================================================
// Skill Management Commands (Phase 1: Skill Validation UI)
// ============================================================================

/// Update an existing user skill
#[tauri::command]
pub(crate) async fn update_user_skill(
    skill_id: i64,
    updates: SkillUpdate,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: update_user_skill (id: {})", skill_id);

    let matcher = state.database.resume_matcher();
    matcher
        .update_user_skill(skill_id, updates)
        .await
        .map_err(|e| user_friendly_error("Failed to update skill", e))
}

/// Delete a user skill
#[tauri::command]
pub(crate) async fn delete_user_skill(
    skill_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    tracing::info!("Command: delete_user_skill (id: {})", skill_id);

    let matcher = state.database.resume_matcher();
    matcher
        .delete_user_skill(skill_id)
        .await
        .map_err(|e| user_friendly_error("Failed to delete skill", e))
}

/// Add a new skill manually
#[tauri::command]
pub(crate) async fn add_user_skill(
    resume_id: i64,
    skill: NewSkill,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let skill_name_chars = skill.skill_name.chars().count();
    tracing::info!(resume_id, skill_name_chars, "Command: add_user_skill");

    let matcher = state.database.resume_matcher();
    matcher
        .add_user_skill(resume_id, skill)
        .await
        .map_err(|e| user_friendly_error("Failed to add skill", e))
}

// ============================================================================
// Resume Library Commands (Phase 2)
// ============================================================================

/// List all resumes
#[tauri::command]
pub(crate) async fn list_all_resumes(
    state: State<'_, AppState>,
) -> Result<Vec<ResumeSummary>, String> {
    tracing::info!("Command: list_all_resumes");

    let matcher = state.database.resume_matcher();
    matcher
        .list_all_resumes()
        .await
        .map(|resumes| resumes.into_iter().map(ResumeSummary::from).collect())
        .map_err(|e| user_friendly_error("Failed to list resumes", e))
}

// ============================================================================
// Template Commands
// ============================================================================

/// List available resume templates
#[tauri::command]
pub(crate) fn list_resume_templates() -> Vec<Template> {
    tracing::info!("Command: list_resume_templates");
    TemplateRenderer::list_templates()
}

/// Render resume to HTML using a template
#[tauri::command]
pub(crate) fn render_resume_html(resume: StructuredResume, template_id: TemplateId) -> String {
    tracing::info!("Command: render_resume_html (template: {:?})", template_id);
    TemplateRenderer::render_html(&resume, template_id)
}

/// Render resume to plain text
#[tauri::command]
pub(crate) fn render_resume_text(resume: StructuredResume) -> String {
    tracing::info!("Command: render_resume_text");
    TemplateRenderer::render_plain_text(&resume)
}

// ============================================================================
// Export Commands
// ============================================================================

/// Export resume to DOCX format
#[tauri::command]
pub(crate) fn export_resume_docx(
    resume: StructuredResume,
    template: TemplateId,
) -> Result<Vec<u8>, String> {
    tracing::info!("Command: export_resume_docx");
    ResumeExporter::export_docx(&resume, template)
        .map_err(|e| user_friendly_error("Failed to export resume", e))
}

/// Export resume to HTML format for browser-based PDF generation
#[tauri::command]
pub(crate) fn export_resume_html(resume: StructuredResume, template: TemplateId) -> String {
    tracing::info!("Command: export_resume_html (template: {:?})", template);
    ResumeExporter::export_html(&resume, template)
}

/// Export resume to plain text
#[tauri::command]
pub(crate) fn export_resume_text(resume: StructuredResume) -> String {
    tracing::info!("Command: export_resume_text");
    ResumeExporter::export_text(&resume)
}

// ============================================================================
// Resume Analysis Commands
// ============================================================================

/// Analyze resume against a job description for application readability
#[tauri::command]
pub(crate) fn analyze_resume_for_job(
    resume: ResumeAnalysisInput,
    job_description: String,
    matching_profile: Option<ResumeMatchingProfile>,
) -> Result<AtsAnalysisResult, String> {
    tracing::info!("Command: analyze_resume_for_job");
    crate::application::resume::analyze_structured_resume_for_job_with_profile(
        resume,
        &job_description,
        matching_profile,
    )
    .map_err(|error| user_friendly_error("Failed to analyze resume", error))
}

/// Analyze the active saved resume against a job description without returning raw resume text.
#[tauri::command]
pub(crate) async fn analyze_active_resume_for_job(
    job_description: String,
    matching_profile: Option<ResumeMatchingProfile>,
    state: State<'_, AppState>,
) -> Result<AtsAnalysisResult, String> {
    tracing::info!("Command: analyze_active_resume_for_job");

    if job_description.trim().is_empty() {
        return Err("Paste the job post, then review again.".to_string());
    }

    crate::application::resume::analyze_active_resume_for_job_with_profile(
        state.database.as_ref(),
        &job_description,
        matching_profile,
    )
    .await
    .map_err(|error| match error {
        ActiveResumeAnalysisError::Missing => {
            "Choose or add a resume before reviewing job fit.".to_string()
        }
        ActiveResumeAnalysisError::Unreadable => {
            "JobSentinel could not find readable text in the active resume. Add a PDF, DOCX, TXT, Markdown, or HTML resume with readable text, or use Import from Resume App."
                .to_string()
        }
        ActiveResumeAnalysisError::Changed => {
            "Your active resume changed while the review was running. Review it and try again."
                .to_string()
        }
        ActiveResumeAnalysisError::Internal(error) => {
            user_friendly_error("Failed to analyze active resume", error)
        }
    })
}

/// Analyze resume format without job context
#[tauri::command]
pub(crate) fn analyze_resume_format(resume: ResumeAnalysisInput) -> AtsAnalysisResult {
    tracing::info!("Command: analyze_resume_format");
    AtsAnalyzer::analyze_format(&resume)
}

/// Extract keywords from a job description
#[tauri::command]
pub(crate) fn extract_job_keywords(
    job_description: String,
) -> Vec<(String, crate::application::resume::KeywordImportance)> {
    tracing::info!("Command: extract_job_keywords");
    AtsAnalyzer::extract_job_keywords(&job_description)
}

/// Get ATS power words for bullet points
#[tauri::command]
pub(crate) fn get_ats_power_words() -> Vec<&'static str> {
    tracing::info!("Command: get_ats_power_words");
    AtsAnalyzer::get_power_words()
}

/// Improve a bullet point with ATS suggestions
#[tauri::command]
pub(crate) fn improve_bullet_point(bullet: String, job_context: Option<String>) -> String {
    tracing::info!("Command: improve_bullet_point");
    AtsAnalyzer::improve_bullet(&bullet, job_context.as_deref())
}

#[cfg(test)]
mod tests;
