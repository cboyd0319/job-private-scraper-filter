use super::*;
use crate::v3_foundation::{prepare_saved_match_debugger, RequirementWhyNot};
use jobsentinel_documents::{KeywordImportance, RequirementMatchState};

use super::super::saved_match_debugger::SavedMatchDebuggerEvidenceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityCaseEvidenceReviewStatus {
    Ready,
    NoSavedMatch,
    NeedsRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseEvidenceRequirement {
    requirement: String,
    importance: OpportunityCaseRequirementImportance,
    match_state: OpportunityCaseRequirementMatchState,
    hard_constraint: bool,
    blocking: bool,
    why_not: Option<RequirementWhyNot>,
    evidence: Vec<OpportunityCaseEvidenceReference>,
    #[serde(skip)]
    requires_review: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OpportunityCaseRequirementImportance {
    Required,
    Preferred,
    Industry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum OpportunityCaseRequirementMatchState {
    Direct,
    Strong,
    Partial,
    Implied,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
struct OpportunityCaseEvidenceReference {
    kind: SavedMatchDebuggerEvidenceKind,
    confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OpportunityCaseEvidenceReview {
    pub(super) status: OpportunityCaseEvidenceReviewStatus,
    pub(super) requirements: Vec<OpportunityCaseEvidenceRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum OpportunityCaseDecisionKind {
    Apply,
    Maybe,
    Skip,
    ResearchMore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseDecision {
    kind: OpportunityCaseDecisionKind,
    reasons: Vec<String>,
}

pub(super) async fn load_opportunity_case_evidence(
    database: &Database,
    job_hash: &str,
) -> Result<OpportunityCaseEvidenceReview, FoundationError> {
    let matcher = database.resume_matcher();
    let Some(active_resume) = matcher
        .get_active_resume()
        .await
        .map_err(|_| FoundationError::Storage("resume"))?
    else {
        return Ok(unavailable_evidence(
            OpportunityCaseEvidenceReviewStatus::NoSavedMatch,
        ));
    };
    if matcher
        .get_match_result(active_resume.id, job_hash)
        .await
        .map_err(|_| FoundationError::Storage("match"))?
        .is_none()
    {
        return Ok(unavailable_evidence(
            OpportunityCaseEvidenceReviewStatus::NoSavedMatch,
        ));
    }
    let debugger = match prepare_saved_match_debugger(database, job_hash, active_resume.id).await {
        Ok(debugger) => debugger,
        Err(FoundationError::Conflict) => {
            return Ok(unavailable_evidence(
                OpportunityCaseEvidenceReviewStatus::NeedsRefresh,
            ));
        }
        Err(error) => return Err(error),
    };

    Ok(OpportunityCaseEvidenceReview {
        status: OpportunityCaseEvidenceReviewStatus::Ready,
        requirements: debugger
            .requirements()
            .iter()
            .map(|requirement| OpportunityCaseEvidenceRequirement {
                requirement: requirement.requirement().to_string(),
                importance: requirement_importance(requirement.importance()),
                match_state: requirement_state(requirement.match_state()),
                hard_constraint: requirement.hard_constraint(),
                blocking: requirement.is_blocking(),
                why_not: requirement.why_not(),
                requires_review: requirement.requires_review(),
                evidence: requirement
                    .evidence()
                    .iter()
                    .map(|evidence| OpportunityCaseEvidenceReference {
                        kind: evidence.kind(),
                        confirmed: evidence.confirmed(),
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn unavailable_evidence(
    status: OpportunityCaseEvidenceReviewStatus,
) -> OpportunityCaseEvidenceReview {
    OpportunityCaseEvidenceReview {
        status,
        requirements: Vec::new(),
    }
}

pub(super) fn build_opportunity_case_decision(
    read: &OpportunityCaseRead,
    source_stale: bool,
    evidence: &OpportunityCaseEvidenceReview,
) -> OpportunityCaseDecision {
    if matches!(
        read.outcome_status.as_deref(),
        Some("offer_rejected" | "rejected" | "ghosted" | "withdrawn")
    ) {
        return decision(
            OpportunityCaseDecisionKind::Skip,
            "This opportunity already has a closed outcome.",
        );
    }
    if read.outcome_status.as_deref() == Some("offer_accepted") {
        return decision(
            OpportunityCaseDecisionKind::Skip,
            "This opportunity closed with an accepted offer.",
        );
    }

    let mut research_reasons = Vec::new();
    match evidence.status {
        OpportunityCaseEvidenceReviewStatus::NoSavedMatch => research_reasons
            .push("No current saved-resume evidence review is available.".to_string()),
        OpportunityCaseEvidenceReviewStatus::NeedsRefresh => research_reasons
            .push("The saved-resume evidence review needs to be refreshed.".to_string()),
        OpportunityCaseEvidenceReviewStatus::Ready if evidence.requirements.is_empty() => {
            research_reasons.push(
                "The posting does not contain enough recognized requirements for a decision."
                    .to_string(),
            );
        }
        OpportunityCaseEvidenceReviewStatus::Ready => {}
    }
    for requirement in evidence
        .requirements
        .iter()
        .filter(|requirement| requirement.blocking || requirement.requires_review)
    {
        research_reasons.push(if requirement.blocking {
            format!(
                "Verify this required qualification before deciding: {}.",
                requirement.requirement
            )
        } else {
            format!(
                "Review the evidence before treating it as a civilian qualification: {}.",
                requirement.requirement
            )
        });
    }
    for requirement in evidence.requirements.iter().filter(|requirement| {
        matches!(
            requirement.match_state,
            OpportunityCaseRequirementMatchState::Direct
                | OpportunityCaseRequirementMatchState::Strong
        ) && !requirement
            .evidence
            .iter()
            .any(|evidence| evidence.confirmed)
    }) {
        research_reasons.push(format!(
            "Confirm current resume evidence before relying on {}.",
            requirement.requirement
        ));
    }
    if source_stale {
        research_reasons.push("The saved source snapshot may be stale.".to_string());
    }
    if read.times_seen > 1 {
        research_reasons.push(format!(
            "The posting has been seen {} times and may be a repost.",
            read.times_seen
        ));
    }
    if read.stale_packet_count > 0 {
        research_reasons.push("Reviewed evidence needs confirmation before reuse.".to_string());
    }
    research_reasons.extend(read.posting_risk_reasons.iter().cloned());
    if !research_reasons.is_empty() {
        return OpportunityCaseDecision {
            kind: OpportunityCaseDecisionKind::ResearchMore,
            reasons: unique(research_reasons),
        };
    }

    let gaps = evidence
        .requirements
        .iter()
        .filter(|requirement| requirement.why_not.is_some())
        .map(|requirement| {
            format!(
                "Evidence is missing or incomplete for {}.",
                requirement.requirement
            )
        })
        .collect::<Vec<_>>();
    if gaps.is_empty() {
        decision(
            OpportunityCaseDecisionKind::Apply,
            "Current confirmed evidence supports every reviewed requirement.",
        )
    } else {
        OpportunityCaseDecision {
            kind: OpportunityCaseDecisionKind::Maybe,
            reasons: gaps,
        }
    }
}

fn decision(kind: OpportunityCaseDecisionKind, reason: &str) -> OpportunityCaseDecision {
    OpportunityCaseDecision {
        kind,
        reasons: vec![reason.to_string()],
    }
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

const fn requirement_importance(
    importance: KeywordImportance,
) -> OpportunityCaseRequirementImportance {
    match importance {
        KeywordImportance::Required => OpportunityCaseRequirementImportance::Required,
        KeywordImportance::Preferred => OpportunityCaseRequirementImportance::Preferred,
        KeywordImportance::Industry => OpportunityCaseRequirementImportance::Industry,
    }
}

const fn requirement_state(state: RequirementMatchState) -> OpportunityCaseRequirementMatchState {
    match state {
        RequirementMatchState::Direct => OpportunityCaseRequirementMatchState::Direct,
        RequirementMatchState::Strong => OpportunityCaseRequirementMatchState::Strong,
        RequirementMatchState::Partial => OpportunityCaseRequirementMatchState::Partial,
        RequirementMatchState::Implied => OpportunityCaseRequirementMatchState::Implied,
        RequirementMatchState::Missing => OpportunityCaseRequirementMatchState::Missing,
    }
}
