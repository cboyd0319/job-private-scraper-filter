use super::{
    map_error, prepare_evidence_bound_packet_claim, FoundationError, PacketEvidenceBoundary,
};
use crate::v3_foundation::saved_match_debugger::{
    current_citation, is_opaque_id, load_saved_match_context,
};
use jobsentinel_domain::ResumeEvidenceCitation;
use jobsentinel_storage::{
    v3_foundation::{
        EvidenceBoundPacketClaimRecord, EvidenceBoundPacketClaimsRead, EvidencePacketBoundary,
        NewEvidenceBoundPacketClaim,
    },
    Database,
};
use serde::Serialize;
use uuid::Uuid;

const MAX_CLAIM_BYTES: usize = 8_192;
const MAX_EVIDENCE_IDS: usize = 32;

/// A renderer-safe, durable reviewed packet claim for one saved match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedMatchEvidencePacketClaim {
    claim_id: String,
    reviewed_text: String,
    evidence_ids: Vec<String>,
    boundaries: Vec<SavedMatchEvidencePacketBoundary>,
}

impl SavedMatchEvidencePacketClaim {
    #[must_use]
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    #[must_use]
    pub fn reviewed_text(&self) -> &str {
        &self.reviewed_text
    }

    #[must_use]
    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    #[must_use]
    pub fn boundaries(&self) -> &[SavedMatchEvidencePacketBoundary] {
        &self.boundaries
    }
}

/// A reviewed-evidence boundary that remains visible with a durable packet claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedMatchEvidencePacketBoundary {
    ClearanceCurrentnessUnverified,
    MilitaryCivilianEquivalenceUnverified,
}

/// Save one reviewed, evidence-bound claim for an existing saved match.
pub async fn save_saved_match_evidence_packet_claim(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
    reviewed_text: String,
    evidence_ids: Vec<String>,
) -> Result<SavedMatchEvidencePacketClaim, FoundationError> {
    validate_packet_request(&reviewed_text, &evidence_ids)?;
    let context = load_saved_match_context(database, job_hash, resume_id).await?;
    let citations = citations_for_evidence_ids(&context, &evidence_ids)?;
    let confirmed = database
        .read_saved_match_confirmed_evidence(job_hash, resume_id)
        .await
        .map_err(map_packet_error)?;
    if evidence_ids
        .iter()
        .any(|evidence_id| !confirmed.evidence_ids().contains(evidence_id))
    {
        return Err(FoundationError::Conflict);
    }
    let case_file_id = confirmed.case_file_id().ok_or(FoundationError::Conflict)?;
    let current = load_saved_match_context(database, job_hash, resume_id).await?;
    if !context.matches(&current) {
        return Err(FoundationError::Conflict);
    }
    let claim_id = Uuid::new_v4().to_string();
    let prepared = prepare_evidence_bound_packet_claim(
        database,
        case_file_id,
        &claim_id,
        reviewed_text,
        current.snapshot(),
        &citations,
    )
    .await?;
    let record = database
        .persist_evidence_bound_packet_claim(&NewEvidenceBoundPacketClaim {
            case_file_id: case_file_id.to_string(),
            claim_id,
            reviewed_text: prepared.text().to_string(),
            resume_snapshot: current.snapshot().clone(),
            job_revision: current.job_revision().to_string(),
            evidence_ids: prepared.evidence_ids().to_vec(),
            citations,
            boundaries: prepared
                .boundaries()
                .iter()
                .copied()
                .map(storage_boundary)
                .collect(),
        })
        .await
        .map_err(map_packet_error)?;
    Ok(renderer_packet_claim(&record))
}

/// Read current durable packet claims for one existing saved match.
pub async fn list_saved_match_evidence_packet_claims(
    database: &Database,
    job_hash: &str,
    resume_id: i64,
) -> Result<Vec<SavedMatchEvidencePacketClaim>, FoundationError> {
    match database
        .list_saved_match_evidence_packet_claims(job_hash, resume_id)
        .await
        .map_err(map_packet_error)?
    {
        EvidenceBoundPacketClaimsRead::Current(records) => {
            Ok(records.iter().map(renderer_packet_claim).collect())
        }
        EvidenceBoundPacketClaimsRead::Invalid => Err(FoundationError::Conflict),
    }
}

fn validate_packet_request(
    reviewed_text: &str,
    evidence_ids: &[String],
) -> Result<(), FoundationError> {
    if reviewed_text.trim().is_empty()
        || reviewed_text.len() > MAX_CLAIM_BYTES
        || reviewed_text.chars().any(char::is_control)
        || evidence_ids.is_empty()
        || evidence_ids.len() > MAX_EVIDENCE_IDS
    {
        return Err(FoundationError::InvalidInput);
    }
    let mut seen = std::collections::HashSet::with_capacity(evidence_ids.len());
    if evidence_ids
        .iter()
        .any(|evidence_id| !is_opaque_id(evidence_id) || !seen.insert(evidence_id.as_str()))
    {
        return Err(FoundationError::InvalidInput);
    }
    Ok(())
}

fn citations_for_evidence_ids(
    context: &super::saved_match_debugger::SavedMatchContext,
    evidence_ids: &[String],
) -> Result<Vec<ResumeEvidenceCitation>, FoundationError> {
    evidence_ids
        .iter()
        .map(|evidence_id| {
            current_citation(context, evidence_id).ok_or(FoundationError::InvalidInput)
        })
        .collect()
}

fn storage_boundary(boundary: PacketEvidenceBoundary) -> EvidencePacketBoundary {
    match boundary {
        PacketEvidenceBoundary::ClearanceCurrentnessUnverified => {
            EvidencePacketBoundary::ClearanceCurrentnessUnverified
        }
        PacketEvidenceBoundary::MilitaryCivilianEquivalenceUnverified => {
            EvidencePacketBoundary::MilitaryCivilianEquivalenceUnverified
        }
    }
}

fn renderer_packet_claim(record: &EvidenceBoundPacketClaimRecord) -> SavedMatchEvidencePacketClaim {
    SavedMatchEvidencePacketClaim {
        claim_id: record.claim_id().to_string(),
        reviewed_text: record.reviewed_text().to_string(),
        evidence_ids: record.evidence_ids().to_vec(),
        boundaries: record
            .boundaries()
            .iter()
            .copied()
            .map(renderer_boundary)
            .collect(),
    }
}

fn renderer_boundary(boundary: EvidencePacketBoundary) -> SavedMatchEvidencePacketBoundary {
    match boundary {
        EvidencePacketBoundary::ClearanceCurrentnessUnverified => {
            SavedMatchEvidencePacketBoundary::ClearanceCurrentnessUnverified
        }
        EvidencePacketBoundary::MilitaryCivilianEquivalenceUnverified => {
            SavedMatchEvidencePacketBoundary::MilitaryCivilianEquivalenceUnverified
        }
    }
}

fn map_packet_error(error: anyhow::Error) -> FoundationError {
    match map_error(error) {
        FoundationError::InvalidInput => FoundationError::Conflict,
        error => error,
    }
}
