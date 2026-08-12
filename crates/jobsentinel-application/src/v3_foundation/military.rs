use super::{map_error, FoundationError};
use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_evaluation_inputs::MilitaryBranch,
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata,
        EventOrigin, GraphProvenance,
    },
    v3_veteran_public_service::{
        reviewed_veteran_public_service_resource, VeteranResourceAccess, VeteranResourceUse,
    },
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};
use jobsentinel_storage::Database;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilitarySuggestionBoundary {
    AwaitingUserReview,
    SuggestionOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilitarySuggestionReviewStatus {
    UserReviewed,
}

pub struct MilitaryReviewResource {
    resource_id: String,
    display_name: String,
    url: String,
    intended_use: VeteranResourceUse,
    runtime_access: VeteranResourceAccess,
    reviewed_on: NaiveDate,
    source_release: Option<String>,
}

impl MilitaryReviewResource {
    #[must_use]
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn intended_use(&self) -> VeteranResourceUse {
        self.intended_use
    }

    #[must_use]
    pub fn runtime_access(&self) -> VeteranResourceAccess {
        self.runtime_access
    }

    #[must_use]
    pub fn reviewed_on(&self) -> NaiveDate {
        self.reviewed_on
    }

    #[must_use]
    pub fn source_release(&self) -> Option<&str> {
        self.source_release.as_deref()
    }
}

pub struct MilitaryWordingMapping {
    pub military_evidence: String,
    pub civilian_wording: String,
}

pub struct MilitaryTransitionWording {
    pub occupation_code: String,
    pub civilian_role: String,
    pub responsibility_mappings: Vec<MilitaryWordingMapping>,
    pub credential_mappings: Vec<MilitaryWordingMapping>,
    pub current_clearance: Option<String>,
}

/// Move-only review state for user-entered wording. The resource is context, not authorship.
pub struct MilitaryOccupationReviewDraft {
    review_id: String,
    case_file_id: String,
    branch: MilitaryBranch,
    occupation_code: String,
    civilian_role: String,
    responsibility_mappings: Vec<MilitaryWordingMapping>,
    credential_mappings: Vec<MilitaryWordingMapping>,
    current_clearance: Option<String>,
    snapshot: ResumeEvidenceSnapshot,
    citation: ResumeEvidenceCitation,
    clearance_citation: Option<ResumeEvidenceCitation>,
    resume_id: i64,
    resume_text_sha256: [u8; 32],
    occupation_review_resource: MilitaryReviewResource,
    credential_review_resource: Option<MilitaryReviewResource>,
}

impl MilitaryOccupationReviewDraft {
    #[must_use]
    pub fn review_id(&self) -> &str {
        &self.review_id
    }

    pub(super) fn case_file_id(&self) -> &str {
        &self.case_file_id
    }

    #[must_use]
    pub fn branch(&self) -> MilitaryBranch {
        self.branch
    }

    #[must_use]
    pub fn occupation_code(&self) -> &str {
        &self.occupation_code
    }

    #[must_use]
    pub fn civilian_role(&self) -> &str {
        &self.civilian_role
    }

    #[must_use]
    pub fn responsibility_mappings(&self) -> &[MilitaryWordingMapping] {
        &self.responsibility_mappings
    }

    #[must_use]
    pub fn credential_mappings(&self) -> &[MilitaryWordingMapping] {
        &self.credential_mappings
    }

    #[must_use]
    pub fn user_confirmed_current_clearance(&self) -> Option<&str> {
        self.current_clearance.as_deref()
    }

    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.citation.evidence_id
    }

    #[must_use]
    pub fn occupation_review_resource(&self) -> &MilitaryReviewResource {
        &self.occupation_review_resource
    }

    #[must_use]
    pub fn credential_review_resource(&self) -> Option<&MilitaryReviewResource> {
        self.credential_review_resource.as_ref()
    }

    #[must_use]
    pub const fn boundary(&self) -> MilitarySuggestionBoundary {
        MilitarySuggestionBoundary::AwaitingUserReview
    }
}

/// An in-memory suggestion confirmed from one exact review draft and fresh local evidence.
pub struct MilitaryOccupationSuggestion {
    case_file_id: String,
    branch: MilitaryBranch,
    occupation_code: String,
    civilian_role: String,
    civilian_responsibilities: Vec<String>,
    credential_wording: Vec<String>,
    current_clearance: Option<String>,
    evidence_id: String,
    clearance_evidence_id: Option<String>,
    occupation_review_resource: MilitaryReviewResource,
    credential_review_resource: Option<MilitaryReviewResource>,
}

impl MilitaryOccupationSuggestion {
    #[must_use]
    pub fn case_file_id(&self) -> &str {
        &self.case_file_id
    }

    #[must_use]
    pub fn branch(&self) -> MilitaryBranch {
        self.branch
    }

    #[must_use]
    pub fn occupation_code(&self) -> &str {
        &self.occupation_code
    }

    #[must_use]
    pub fn civilian_role(&self) -> &str {
        &self.civilian_role
    }

    #[must_use]
    pub fn civilian_responsibilities(&self) -> &[String] {
        &self.civilian_responsibilities
    }

    #[must_use]
    pub fn credential_wording(&self) -> &[String] {
        &self.credential_wording
    }

    #[must_use]
    pub fn user_confirmed_current_clearance(&self) -> Option<&str> {
        self.current_clearance.as_deref()
    }

    #[must_use]
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    #[must_use]
    pub fn clearance_evidence_id(&self) -> Option<&str> {
        self.clearance_evidence_id.as_deref()
    }

    #[must_use]
    pub fn occupation_review_resource(&self) -> &MilitaryReviewResource {
        &self.occupation_review_resource
    }

    #[must_use]
    pub fn credential_review_resource(&self) -> Option<&MilitaryReviewResource> {
        self.credential_review_resource.as_ref()
    }

    #[must_use]
    pub const fn review_status(&self) -> MilitarySuggestionReviewStatus {
        MilitarySuggestionReviewStatus::UserReviewed
    }

    #[must_use]
    pub const fn boundary(&self) -> MilitarySuggestionBoundary {
        MilitarySuggestionBoundary::SuggestionOnly
    }
}

pub async fn prepare_military_transition_review(
    database: &Database,
    case_file_id: &str,
    branch: MilitaryBranch,
    wording: MilitaryTransitionWording,
    snapshot: &ResumeEvidenceSnapshot,
    citation: &ResumeEvidenceCitation,
    clearance_citation: Option<&ResumeEvidenceCitation>,
    today: NaiveDate,
) -> Result<MilitaryOccupationReviewDraft, FoundationError> {
    if !valid_trimmed_text(&wording.occupation_code, 32)
        || !valid_trimmed_text(&wording.civilian_role, 256)
        || !valid_mapping_list(&wording.responsibility_mappings)
        || !valid_mapping_list(&wording.credential_mappings)
        || citation.field_path != "military_info"
        || ResumeEvidenceCitation::for_field(snapshot, "military_info").as_ref() != Some(citation)
    {
        return Err(FoundationError::InvalidInput);
    }
    let clearance_citation = match (wording.current_clearance.as_deref(), clearance_citation) {
        (Some(clearance), Some(citation))
            if valid_trimmed_text(clearance, 128)
                && citation.field_path == "clearance"
                && ResumeEvidenceCitation::for_field(snapshot, "clearance").as_ref()
                    == Some(citation) =>
        {
            Some(citation.clone())
        }
        (None, None) => None,
        _ => return Err(FoundationError::InvalidInput),
    };
    let resume_id = snapshot
        .source_id
        .strip_prefix("resume:")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .filter(|value| snapshot.source_id == format!("resume:{value}"))
        .ok_or(FoundationError::InvalidInput)?;
    let (current_snapshot, resume_text, links) = database
        .read_case_file_resume_evidence_context(case_file_id, resume_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    let resume_text = resume_text
        .filter(|text| {
            !text.trim().is_empty() && text.len() <= crate::resume::MAX_RESUME_FILE_BYTES as usize
        })
        .ok_or(FoundationError::Conflict)?;
    if &current_snapshot != snapshot
        || !has_confirmed_link(&links, citation)
        || clearance_citation
            .as_ref()
            .is_some_and(|citation| !has_confirmed_link(&links, citation))
    {
        return Err(FoundationError::Conflict);
    }
    let normalized_resume_text = normalize_evidence(&resume_text);
    if !contains_exact_evidence(&normalized_resume_text, &wording.occupation_code)
        || wording
            .responsibility_mappings
            .iter()
            .chain(&wording.credential_mappings)
            .any(|mapping| {
                !contains_exact_evidence(&normalized_resume_text, &mapping.military_evidence)
            })
        || wording
            .current_clearance
            .as_ref()
            .is_some_and(|clearance| !contains_exact_evidence(&normalized_resume_text, clearance))
    {
        return Err(FoundationError::Conflict);
    }
    let occupation_review_resource = review_resource(
        "onet-military-crosswalk",
        VeteranResourceUse::OccupationCrosswalk,
        today,
    )?;
    let credential_review_resource = (!wording.credential_mappings.is_empty())
        .then(|| {
            review_resource(
                "dod-cool-military-occupations",
                VeteranResourceUse::CredentialResearch,
                today,
            )
        })
        .transpose()?;

    Ok(MilitaryOccupationReviewDraft {
        review_id: Uuid::new_v4().to_string(),
        case_file_id: case_file_id.to_string(),
        branch,
        occupation_code: wording.occupation_code,
        civilian_role: wording.civilian_role,
        responsibility_mappings: wording.responsibility_mappings,
        credential_mappings: wording.credential_mappings,
        current_clearance: wording.current_clearance,
        snapshot: snapshot.clone(),
        citation: citation.clone(),
        clearance_citation,
        resume_id,
        resume_text_sha256: Sha256::digest(resume_text.as_bytes()).into(),
        occupation_review_resource,
        credential_review_resource,
    })
}

pub async fn confirm_military_transition_review(
    database: &Database,
    review: MilitaryOccupationReviewDraft,
    receipt: &CaseFileEventInput,
) -> Result<MilitaryOccupationSuggestion, FoundationError> {
    let exact_reference = matches!(
        &receipt.metadata,
        EventMetadata::LocalReference { reference_id } if reference_id == &review.review_id
    );
    if receipt.validate().is_err()
        || receipt.case_file_id != review.case_file_id
        || receipt.kind != CaseFileEventKind::PrivacyReceiptRecorded
        || receipt.origin != EventOrigin::User
        || !receipt.user_action
        || !exact_reference
    {
        return Err(FoundationError::InvalidInput);
    }
    let (current_snapshot, resume_text, links) = database
        .read_case_file_resume_evidence_context(&review.case_file_id, review.resume_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    if current_snapshot != review.snapshot
        || resume_text.as_ref().is_none_or(|text| {
            text.len() > crate::resume::MAX_RESUME_FILE_BYTES as usize
                || <[u8; 32]>::from(Sha256::digest(text.as_bytes())) != review.resume_text_sha256
        })
        || !has_confirmed_link(&links, &review.citation)
        || review
            .clearance_citation
            .as_ref()
            .is_some_and(|citation| !has_confirmed_link(&links, citation))
    {
        return Err(FoundationError::Conflict);
    }

    Ok(MilitaryOccupationSuggestion {
        case_file_id: review.case_file_id,
        branch: review.branch,
        occupation_code: review.occupation_code,
        civilian_role: review.civilian_role,
        civilian_responsibilities: review
            .responsibility_mappings
            .into_iter()
            .map(|mapping| mapping.civilian_wording)
            .collect(),
        credential_wording: review
            .credential_mappings
            .into_iter()
            .map(|mapping| mapping.civilian_wording)
            .collect(),
        current_clearance: review.current_clearance,
        evidence_id: review.citation.evidence_id,
        clearance_evidence_id: review
            .clearance_citation
            .map(|citation| citation.evidence_id),
        occupation_review_resource: review.occupation_review_resource,
        credential_review_resource: review.credential_review_resource,
    })
}

fn review_resource(
    resource_id: &str,
    intended_use: VeteranResourceUse,
    today: NaiveDate,
) -> Result<MilitaryReviewResource, FoundationError> {
    let resource = reviewed_veteran_public_service_resource(resource_id, today)
        .map_err(|_| FoundationError::InvalidInput)?;
    if resource.intended_use != intended_use
        || resource.runtime_access != VeteranResourceAccess::ManualReviewOnly
        || !resource.intended_use.requires_user_confirmed_evidence()
    {
        return Err(FoundationError::InvalidInput);
    }
    Ok(MilitaryReviewResource {
        resource_id: resource.resource_id,
        display_name: resource.display_name,
        url: resource.url,
        intended_use: resource.intended_use,
        runtime_access: resource.runtime_access,
        reviewed_on: resource.reviewed_on,
        source_release: resource.source_release,
    })
}

fn has_confirmed_link(links: &[CareerGraphLink], citation: &ResumeEvidenceCitation) -> bool {
    links.iter().any(|link| {
        link.relation == CareerRelation::Evidence
            && link.provenance == GraphProvenance::UserConfirmed
            && link.object_id == citation.evidence_id
    })
}

fn valid_mapping_list(values: &[MilitaryWordingMapping]) -> bool {
    values.len() <= 16
        && values.iter().all(|mapping| {
            valid_trimmed_text(&mapping.military_evidence, 256)
                && valid_trimmed_text(&mapping.civilian_wording, 256)
        })
}

fn normalize_evidence(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    for segment in value.split_whitespace() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.extend(segment.chars().flat_map(char::to_lowercase));
    }
    normalized
}

fn contains_exact_evidence(normalized_text: &str, evidence: &str) -> bool {
    let evidence = normalize_evidence(evidence);
    normalized_text
        .match_indices(&evidence)
        .any(|(start, matched)| {
            let before = normalized_text[..start].chars().next_back();
            let after = normalized_text[start + matched.len()..].chars().next();
            before.is_none_or(|character| !character.is_alphanumeric())
                && after.is_none_or(|character| !character.is_alphanumeric())
        })
}

fn valid_trimmed_text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.chars().count() <= maximum
        && !value.chars().any(char::is_control)
}
