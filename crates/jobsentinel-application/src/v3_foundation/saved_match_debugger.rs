use super::{map_error, FoundationError, RequirementWhyNot};
use jobsentinel_documents::{AtsAnalyzer, KeywordImportance, RequirementMatchState};
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};
use jobsentinel_storage::{v3_foundation::SavedMatchEvidenceConfirmation, Database};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// A renderer-safe local explanation of one existing saved resume/job match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedMatchDebugger {
    debugger_id: String,
    requirements: Vec<SavedMatchDebuggerRequirement>,
}

impl SavedMatchDebugger {
    #[must_use]
    pub fn debugger_id(&self) -> &str {
        &self.debugger_id
    }

    #[must_use]
    pub fn requirements(&self) -> &[SavedMatchDebuggerRequirement] {
        &self.requirements
    }
}

/// A renderer-safe requirement review with opaque references to current local evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedMatchDebuggerRequirement {
    requirement: String,
    importance: KeywordImportance,
    match_state: RequirementMatchState,
    hard_constraint: bool,
    evidence: Vec<SavedMatchDebuggerEvidence>,
    why_not: Option<RequirementWhyNot>,
    blocking: bool,
    #[serde(skip)]
    requires_review: bool,
}

impl SavedMatchDebuggerRequirement {
    #[must_use]
    pub fn requirement(&self) -> &str {
        &self.requirement
    }

    #[must_use]
    pub const fn importance(&self) -> KeywordImportance {
        self.importance
    }

    #[must_use]
    pub const fn match_state(&self) -> RequirementMatchState {
        self.match_state
    }

    #[must_use]
    pub const fn hard_constraint(&self) -> bool {
        self.hard_constraint
    }

    #[must_use]
    pub fn evidence(&self) -> &[SavedMatchDebuggerEvidence] {
        &self.evidence
    }

    #[must_use]
    pub const fn why_not(&self) -> Option<RequirementWhyNot> {
        self.why_not
    }

    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        self.blocking
    }

    #[must_use]
    pub(crate) const fn requires_review(&self) -> bool {
        self.requires_review
    }
}

/// One current opaque evidence reference and its explicit confirmation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedMatchDebuggerEvidence {
    evidence_id: String,
    #[serde(skip)]
    kind: SavedMatchDebuggerEvidenceKind,
    confirmed: bool,
}

impl SavedMatchDebuggerEvidence {
    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    #[must_use]
    pub(crate) const fn kind(&self) -> SavedMatchDebuggerEvidenceKind {
        self.kind
    }

    #[must_use]
    pub const fn confirmed(&self) -> bool {
        self.confirmed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedMatchDebuggerEvidenceKind {
    ResumeBullet,
    Project,
    Skill,
    Certification,
    ResumeEvidence,
}

pub(super) struct SavedMatchContext {
    job_hash: String,
    job_revision: String,
    job_description: String,
    resume_id: i64,
    resume_path: String,
    resume_text: String,
    snapshot: ResumeEvidenceSnapshot,
    skills: Vec<String>,
    saved_match_id: i64,
}

/// Prepare a read-only debugger for a saved job and saved resume. Pasted text is not accepted.
pub async fn prepare_saved_match_debugger(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
) -> Result<SavedMatchDebugger, FoundationError> {
    let context = load_saved_match_context(database, job_hash, resume_id).await?;
    let confirmed = confirmed_evidence_ids(database, &context).await?;
    let debugger = build_debugger(&context, &confirmed);
    let current = load_saved_match_context(database, job_hash, resume_id).await?;
    if !context.matches(&current) {
        return Err(FoundationError::Conflict);
    }
    Ok(debugger)
}

/// Confirm one opaque, current evidence reference for a saved resume/job match.
pub async fn confirm_saved_match_debugger_evidence(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
    expected_debugger_id: &str,
    evidence_id: &str,
) -> Result<bool, FoundationError> {
    if !is_opaque_id(expected_debugger_id) || !is_opaque_id(evidence_id) {
        return Err(FoundationError::InvalidInput);
    }
    let context = load_saved_match_context(database, job_hash, resume_id).await?;
    if debugger_id(&context) != expected_debugger_id {
        return Err(FoundationError::Conflict);
    }
    let current = load_saved_match_context(database, job_hash, resume_id).await?;
    if !context.matches(&current) || debugger_id(&current) != expected_debugger_id {
        return Err(FoundationError::Conflict);
    }
    let citation = current_citation(&current, evidence_id).ok_or(FoundationError::InvalidInput)?;
    database
        .confirm_saved_match_evidence(&SavedMatchEvidenceConfirmation {
            job_hash: current.job_hash,
            resume_id: current.resume_id,
            saved_match_id: current.saved_match_id,
            expected_job_revision: current.job_revision,
            expected_resume_snapshot: current.snapshot,
            expected_skills: current.skills,
            citation,
        })
        .await
        .map_err(map_saved_match_confirmation_error)
}

pub(super) async fn load_saved_match_context(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
) -> Result<SavedMatchContext, FoundationError> {
    if job_hash.is_empty()
        || job_hash.len() > 128
        || job_hash.chars().any(char::is_control)
        || resume_id <= 0
    {
        return Err(FoundationError::InvalidInput);
    }
    let job = database
        .get_job_by_hash(job_hash)
        .await
        .map_err(|_| FoundationError::Storage("job"))?
        .ok_or(FoundationError::Conflict)?;
    let job_description = job
        .description
        .filter(|description| !description.trim().is_empty())
        .ok_or(FoundationError::Conflict)?;
    let matcher = database.resume_matcher();
    let saved_match = matcher
        .get_match_result(resume_id, job_hash)
        .await
        .map_err(|_| FoundationError::Storage("match"))?
        .ok_or(FoundationError::Conflict)?;
    if saved_match.resume_id != resume_id || saved_match.job_hash != job_hash {
        return Err(FoundationError::Storage("match"));
    }
    let snapshot = matcher
        .get_resume_evidence_snapshot(resume_id)
        .await
        .map_err(|_| FoundationError::Storage("resume"))?
        .ok_or(FoundationError::Conflict)?;
    let resume = matcher
        .get_resume(resume_id)
        .await
        .map_err(|_| FoundationError::Conflict)?;
    let resume_text = resume
        .parsed_text
        .filter(|text| !text.trim().is_empty())
        .ok_or(FoundationError::Conflict)?;
    let skills = matcher
        .get_user_skills(resume_id)
        .await
        .map_err(|_| FoundationError::Storage("resume"))?
        .into_iter()
        .map(|skill| skill.skill_name)
        .collect();
    Ok(SavedMatchContext {
        job_hash: job.hash,
        job_revision: job.updated_at.to_rfc3339(),
        job_description,
        resume_id,
        resume_path: resume.file_path,
        resume_text,
        snapshot,
        skills,
        saved_match_id: saved_match.id,
    })
}

impl SavedMatchContext {
    pub(super) fn job_hash(&self) -> &str {
        &self.job_hash
    }

    pub(super) const fn resume_id(&self) -> i64 {
        self.resume_id
    }

    pub(super) fn snapshot(&self) -> &ResumeEvidenceSnapshot {
        &self.snapshot
    }

    pub(super) fn job_revision(&self) -> &str {
        &self.job_revision
    }

    pub(super) fn skills(&self) -> &[String] {
        &self.skills
    }

    pub(super) const fn saved_match_id(&self) -> i64 {
        self.saved_match_id
    }

    pub(super) fn matches(&self, other: &Self) -> bool {
        self.job_hash == other.job_hash
            && self.job_revision == other.job_revision
            && self.job_description == other.job_description
            && self.resume_id == other.resume_id
            && self.resume_path == other.resume_path
            && self.resume_text == other.resume_text
            && self.snapshot == other.snapshot
            && self.skills == other.skills
            && self.saved_match_id == other.saved_match_id
    }
}

async fn confirmed_evidence_ids(
    database: &Database,
    context: &SavedMatchContext,
) -> Result<HashSet<String>, FoundationError> {
    database
        .read_saved_match_confirmed_evidence(&context.job_hash, context.resume_id)
        .await
        .map_err(map_saved_match_confirmation_error)
        .map(|confirmed| confirmed.evidence_ids().iter().cloned().collect())
}

fn build_debugger(
    context: &SavedMatchContext,
    confirmed_evidence_ids: &HashSet<String>,
) -> SavedMatchDebugger {
    let analysis = AtsAnalyzer::analyze_text_for_job_with_snapshot(
        &context.resume_text,
        &context.skills,
        &context.job_description,
        &context.snapshot,
    );
    let mut requirements = analysis
        .requirement_reviews
        .iter()
        .map(|review| {
            let why_not = match review.match_state {
                RequirementMatchState::Direct | RequirementMatchState::Strong => None,
                RequirementMatchState::Partial => Some(RequirementWhyNot::PartialEvidence),
                RequirementMatchState::Implied => Some(RequirementWhyNot::ImpliedEvidence),
                RequirementMatchState::Missing => Some(RequirementWhyNot::MissingEvidence),
            };
            let blocking = review.hard_constraint
                && review.importance == KeywordImportance::Required
                && why_not.is_some();
            SavedMatchDebuggerRequirement {
                requirement: review.keyword.clone(),
                importance: review.importance,
                match_state: review.match_state,
                hard_constraint: review.hard_constraint,
                evidence: review
                    .evidence_citations
                    .iter()
                    .map(|citation| SavedMatchDebuggerEvidence {
                        confirmed: confirmed_evidence_ids.contains(&citation.evidence_id),
                        evidence_id: citation.evidence_id.clone(),
                        kind: evidence_kind(&citation.field_path),
                    })
                    .collect(),
                why_not,
                blocking,
                requires_review: (review.hard_constraint
                    && review.importance == KeywordImportance::Required)
                    || review
                        .evidence_sections
                        .iter()
                        .any(|section| section == "military service"),
            }
        })
        .collect::<Vec<_>>();
    requirements.sort_by(|left, right| {
        requirement_order(left.importance)
            .cmp(&requirement_order(right.importance))
            .then(match_state_order(left.match_state).cmp(&match_state_order(right.match_state)))
            .then(left.requirement.cmp(&right.requirement))
            .then_with(|| {
                left.evidence
                    .iter()
                    .map(|evidence| &evidence.evidence_id)
                    .cmp(right.evidence.iter().map(|evidence| &evidence.evidence_id))
            })
    });
    SavedMatchDebugger {
        debugger_id: debugger_id(context),
        requirements,
    }
}

fn evidence_kind(field_path: &str) -> SavedMatchDebuggerEvidenceKind {
    if field_path.starts_with("experience.") {
        SavedMatchDebuggerEvidenceKind::ResumeBullet
    } else if field_path.starts_with("projects.") {
        SavedMatchDebuggerEvidenceKind::Project
    } else if field_path.starts_with("skills.") {
        SavedMatchDebuggerEvidenceKind::Skill
    } else if field_path.starts_with("certifications.") {
        SavedMatchDebuggerEvidenceKind::Certification
    } else {
        SavedMatchDebuggerEvidenceKind::ResumeEvidence
    }
}

pub(super) fn current_citation(
    context: &SavedMatchContext,
    evidence_id: &str,
) -> Option<ResumeEvidenceCitation> {
    AtsAnalyzer::analyze_text_for_job_with_snapshot(
        &context.resume_text,
        &context.skills,
        &context.job_description,
        &context.snapshot,
    )
    .requirement_reviews
    .into_iter()
    .flat_map(|review| review.evidence_citations)
    .find(|citation| citation.evidence_id == evidence_id)
}

fn debugger_id(context: &SavedMatchContext) -> String {
    let mut digest = Sha256::new();
    digest.update(b"jobsentinel_saved_match_debugger_v1\0");
    digest.update(context.resume_id.to_le_bytes());
    digest.update(context.saved_match_id.to_le_bytes());
    digest.update((context.skills.len() as u64).to_le_bytes());
    for value in [
        context.job_hash.as_str(),
        context.job_revision.as_str(),
        context.job_description.as_str(),
        context.snapshot.source_id.as_str(),
        context.snapshot.revision.as_str(),
        context.resume_path.as_str(),
        context.resume_text.as_str(),
    ] {
        hash_debugger_value(&mut digest, value);
    }
    for skill in &context.skills {
        hash_debugger_value(&mut digest, skill);
    }
    hex::encode(digest.finalize())
}

fn hash_debugger_value(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value.as_bytes());
}

pub(super) fn map_saved_match_confirmation_error(error: anyhow::Error) -> FoundationError {
    match map_error(error) {
        FoundationError::InvalidInput => FoundationError::Conflict,
        error => error,
    }
}

pub(super) fn is_opaque_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

const fn requirement_order(importance: KeywordImportance) -> u8 {
    match importance {
        KeywordImportance::Required => 0,
        KeywordImportance::Preferred => 1,
        KeywordImportance::Industry => 2,
    }
}

const fn match_state_order(state: RequirementMatchState) -> u8 {
    match state {
        RequirementMatchState::Missing => 0,
        RequirementMatchState::Partial => 1,
        RequirementMatchState::Implied => 2,
        RequirementMatchState::Direct => 3,
        RequirementMatchState::Strong => 4,
    }
}
