use super::{map_error, FoundationError};
use jobsentinel_documents::{AtsAnalyzer, KeywordImportance, RequirementMatchState};
use jobsentinel_domain::{
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata,
        EventOrigin, GraphProvenance,
    },
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};
use jobsentinel_storage::Database;
use serde::Serialize;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketEvidenceBoundary {
    ClearanceCurrentnessUnverified,
    MilitaryCivilianEquivalenceUnverified,
}

/// An in-memory, user-authored packet claim. It is not a durable packet version.
pub struct EvidenceBoundPacketClaim {
    case_file_id: String,
    claim_id: String,
    text: String,
    evidence_ids: Vec<String>,
    boundaries: Vec<PacketEvidenceBoundary>,
}

impl EvidenceBoundPacketClaim {
    #[must_use]
    pub fn case_file_id(&self) -> &str {
        &self.case_file_id
    }

    #[must_use]
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    #[must_use]
    pub fn boundaries(&self) -> &[PacketEvidenceBoundary] {
        &self.boundaries
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementWhyNot {
    PartialEvidence,
    ImpliedEvidence,
    MissingEvidence,
}

/// An in-memory requirement result derived from current, reviewed local evidence.
pub struct EvidenceBoundRequirementDiagnostic {
    case_file_id: String,
    requirement: String,
    job_revision: String,
    importance: KeywordImportance,
    match_state: RequirementMatchState,
    hard_constraint: bool,
    evidence_ids: Vec<String>,
    why_not: Option<RequirementWhyNot>,
    blocking: bool,
}

impl EvidenceBoundRequirementDiagnostic {
    #[must_use]
    pub fn case_file_id(&self) -> &str {
        &self.case_file_id
    }

    #[must_use]
    pub fn requirement(&self) -> &str {
        &self.requirement
    }

    #[must_use]
    pub fn job_revision(&self) -> &str {
        &self.job_revision
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
    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    #[must_use]
    pub const fn why_not(&self) -> Option<RequirementWhyNot> {
        self.why_not
    }

    #[must_use]
    pub const fn hard_constraint(&self) -> bool {
        self.hard_constraint
    }

    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        self.blocking
    }
}

pub async fn confirm_resume_evidence_for_case(
    database: &Database,
    snapshot: &ResumeEvidenceSnapshot,
    citation: &ResumeEvidenceCitation,
    confirmation: &CaseFileEventInput,
) -> Result<bool, FoundationError> {
    let reference_matches = matches!(
        &confirmation.metadata,
        EventMetadata::LocalReference { reference_id } if reference_id == &citation.evidence_id
    );
    if ResumeEvidenceCitation::for_field(snapshot, &citation.field_path).as_ref() != Some(citation)
        || confirmation.validate().is_err()
        || confirmation.kind != CaseFileEventKind::EvidenceLinked
        || confirmation.origin != EventOrigin::User
        || !confirmation.user_action
        || !reference_matches
    {
        return Err(FoundationError::InvalidInput);
    }
    let link = CareerGraphLink {
        link_id: Uuid::new_v4().to_string(),
        subject_id: confirmation.case_file_id.clone(),
        relation: CareerRelation::Evidence,
        object_id: citation.evidence_id.clone(),
        provenance: GraphProvenance::UserConfirmed,
        provenance_ref: None,
    };
    database
        .insert_case_file_evidence(&link, confirmation)
        .await
        .map_err(map_error)
}

pub async fn prepare_evidence_bound_requirement_diagnostic(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
    requirement: &str,
) -> Result<EvidenceBoundRequirementDiagnostic, FoundationError> {
    if requirement.is_empty()
        || requirement != requirement.trim()
        || requirement.len() > 512
        || requirement.chars().any(char::is_control)
    {
        return Err(FoundationError::InvalidInput);
    }

    let resume_id = saved_resume_id(snapshot)?;
    let context = database
        .read_case_requirement_evidence_context(case_file_id, resume_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    if context.snapshot() != snapshot
        || context.job_description().trim().is_empty()
        || context.resume_text().trim().is_empty()
    {
        return Err(FoundationError::Conflict);
    }
    let analysis = AtsAnalyzer::analyze_text_for_job_with_snapshot(
        context.resume_text(),
        context.skills(),
        context.job_description(),
        context.snapshot(),
    );
    let mut reviews = analysis
        .requirement_reviews
        .iter()
        .filter(|review| review.keyword == requirement);
    let review = reviews.next().ok_or(FoundationError::InvalidInput)?;
    if reviews.next().is_some()
        || review.evidence_citations.len() > 32
        || (review.match_state == RequirementMatchState::Missing
            && !review.evidence_citations.is_empty())
        || (review.match_state != RequirementMatchState::Missing
            && review.evidence_citations.is_empty())
    {
        return Err(FoundationError::InvalidInput);
    }
    let evidence_ids = validated_confirmed_evidence_ids(
        context.snapshot(),
        &review.evidence_citations,
        context.links(),
    )?;
    let why_not = match review.match_state {
        RequirementMatchState::Direct | RequirementMatchState::Strong => None,
        RequirementMatchState::Partial => Some(RequirementWhyNot::PartialEvidence),
        RequirementMatchState::Implied => Some(RequirementWhyNot::ImpliedEvidence),
        RequirementMatchState::Missing => Some(RequirementWhyNot::MissingEvidence),
    };
    let risk_count = analysis
        .hard_constraint_risks
        .iter()
        .filter(|risk| risk.requirement == review.keyword)
        .count();
    let blocking = risk_count == 1;
    let should_block = review.hard_constraint
        && review.importance == KeywordImportance::Required
        && why_not.is_some();
    if risk_count > 1 || blocking != should_block {
        return Err(FoundationError::InvalidInput);
    }

    Ok(EvidenceBoundRequirementDiagnostic {
        case_file_id: case_file_id.to_string(),
        requirement: review.keyword.clone(),
        job_revision: context.job_revision().to_string(),
        importance: review.importance,
        match_state: review.match_state,
        hard_constraint: review.hard_constraint,
        evidence_ids,
        why_not,
        blocking,
    })
}

pub async fn prepare_evidence_bound_packet_claim(
    database: &Database,
    case_file_id: &str,
    claim_id: &str,
    reviewed_user_text: String,
    snapshot: &ResumeEvidenceSnapshot,
    citations: &[ResumeEvidenceCitation],
) -> Result<EvidenceBoundPacketClaim, FoundationError> {
    let parsed_claim_id = Uuid::parse_str(claim_id).map_err(|_| FoundationError::InvalidInput)?;
    if parsed_claim_id.is_nil()
        || parsed_claim_id.to_string() != claim_id
        || reviewed_user_text.trim().is_empty()
        || reviewed_user_text.len() > 8_192
        || citations.is_empty()
        || citations.len() > 32
    {
        return Err(FoundationError::InvalidInput);
    }
    let resume_id = saved_resume_id(snapshot)?;
    let (current_snapshot, _, links) = database
        .read_case_file_resume_evidence_context(case_file_id, resume_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    if &current_snapshot != snapshot {
        return Err(FoundationError::Conflict);
    }

    let evidence_ids = validated_confirmed_evidence_ids(snapshot, citations, &links)?;
    let mut boundaries = Vec::new();
    for citation in citations {
        let boundary = match citation.field_path.as_str() {
            "clearance" => Some(PacketEvidenceBoundary::ClearanceCurrentnessUnverified),
            "military_info" => Some(PacketEvidenceBoundary::MilitaryCivilianEquivalenceUnverified),
            _ => None,
        };
        if let Some(boundary) = boundary {
            if !boundaries.contains(&boundary) {
                boundaries.push(boundary);
            }
        }
    }

    Ok(EvidenceBoundPacketClaim {
        case_file_id: case_file_id.to_string(),
        claim_id: claim_id.to_string(),
        text: reviewed_user_text,
        evidence_ids,
        boundaries,
    })
}

fn saved_resume_id(snapshot: &ResumeEvidenceSnapshot) -> Result<i64, FoundationError> {
    snapshot
        .source_id
        .strip_prefix("resume:")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .filter(|value| snapshot.source_id == format!("resume:{value}"))
        .ok_or(FoundationError::InvalidInput)
}

fn validated_confirmed_evidence_ids(
    snapshot: &ResumeEvidenceSnapshot,
    citations: &[ResumeEvidenceCitation],
    links: &[CareerGraphLink],
) -> Result<Vec<String>, FoundationError> {
    let confirmed_ids = links
        .iter()
        .filter(|link| link.provenance == GraphProvenance::UserConfirmed)
        .map(|link| link.object_id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::with_capacity(citations.len());
    let mut evidence_ids = Vec::with_capacity(citations.len());
    for citation in citations {
        if ResumeEvidenceCitation::for_field(snapshot, &citation.field_path).as_ref()
            != Some(citation)
            || !seen.insert(citation.evidence_id.as_str())
        {
            return Err(FoundationError::InvalidInput);
        }
        if !confirmed_ids.contains(citation.evidence_id.as_str()) {
            return Err(FoundationError::Conflict);
        }
        evidence_ids.push(citation.evidence_id.clone());
    }
    Ok(evidence_ids)
}

pub async fn read_case_file_evidence_links(
    database: &Database,
    case_file_id: &str,
) -> Result<Vec<CareerGraphLink>, FoundationError> {
    database
        .list_case_file_evidence_links(case_file_id)
        .await
        .map_err(map_error)
}
