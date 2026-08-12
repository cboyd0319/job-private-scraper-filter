//! Exposes validated saved-match debugger and reviewed evidence commands to Tauri.

use crate::application::v3_foundation::{
    confirm_saved_match_debugger_evidence, list_saved_match_evidence_packet_claims,
    prepare_saved_match_debugger, save_saved_match_evidence_packet_claim, SavedMatchDebugger,
    SavedMatchEvidencePacketClaim,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use tauri::State;

const MAX_REVIEWED_CLAIM_BYTES: usize = 8_192;
const MAX_PACKET_EVIDENCE_IDS: usize = 32;

/// Get a local, evidence-bound explanation for one existing saved match.
#[tauri::command]
pub(crate) async fn get_saved_match_debugger(
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<SavedMatchDebugger, String> {
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        "Command: get saved match debugger"
    );

    prepare_saved_match_debugger(state.database.as_ref(), &job_hash, resume_id)
        .await
        .map_err(|error| user_friendly_error("Saved match evidence is unavailable", error))
}

/// Confirm one opaque local evidence reference shown by the saved-match debugger.
#[tauri::command]
pub(crate) async fn confirm_saved_match_evidence(
    job_hash: String,
    resume_id: i64,
    debugger_id: String,
    evidence_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    if !is_saved_match_debugger_opaque_id(&debugger_id)
        || !is_saved_match_debugger_opaque_id(&evidence_id)
    {
        return Err(
            "The selected evidence is no longer available. Refresh the debugger and try again."
                .to_string(),
        );
    }
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        "Command: confirm saved match evidence"
    );

    confirm_saved_match_debugger_evidence(
        state.database.as_ref(),
        &job_hash,
        resume_id,
        &debugger_id,
        &evidence_id,
    )
    .await
    .map_err(|error| user_friendly_error("Saved match evidence could not be confirmed", error))
}

/// List durable reviewed claims bound to currently confirmed saved-match evidence.
#[tauri::command]
pub(crate) async fn list_saved_match_evidence_packets(
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<SavedMatchEvidencePacketClaim>, String> {
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        "Command: list saved match evidence packets"
    );

    list_saved_match_evidence_packet_claims(state.database.as_ref(), &job_hash, resume_id)
        .await
        .map_err(|error| user_friendly_error("Saved reviewed claims are unavailable", error))
}

/// Save one explicitly reviewed claim bound to current confirmed saved-match evidence.
#[tauri::command]
pub(crate) async fn save_saved_match_evidence_packet(
    job_hash: String,
    resume_id: i64,
    reviewed_text: String,
    evidence_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<SavedMatchEvidencePacketClaim, String> {
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    validate_saved_match_packet_args(&reviewed_text, &evidence_ids)?;
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        reviewed_text_chars = reviewed_text.chars().count(),
        evidence_count = evidence_ids.len(),
        "Command: save saved match evidence packet"
    );

    save_saved_match_evidence_packet_claim(
        state.database.as_ref(),
        &job_hash,
        resume_id,
        reviewed_text,
        evidence_ids,
    )
    .await
    .map_err(|error| user_friendly_error("Reviewed claim could not be saved", error))
}

pub(crate) fn validate_saved_match_debugger_args(
    job_hash: &str,
    resume_id: i64,
) -> Result<(), String> {
    if job_hash.is_empty() || job_hash.len() > 128 || job_hash.chars().any(char::is_control) {
        return Err("Choose a saved job before inspecting its evidence.".to_string());
    }
    if resume_id <= 0 {
        return Err("Choose a saved resume before inspecting evidence.".to_string());
    }
    Ok(())
}

pub(super) fn is_saved_match_debugger_opaque_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn validate_saved_match_packet_args(
    reviewed_text: &str,
    evidence_ids: &[String],
) -> Result<(), String> {
    if reviewed_text.trim().is_empty()
        || reviewed_text.len() > MAX_REVIEWED_CLAIM_BYTES
        || reviewed_text.chars().any(char::is_control)
        || evidence_ids.is_empty()
        || evidence_ids.len() > MAX_PACKET_EVIDENCE_IDS
        || evidence_ids
            .iter()
            .any(|evidence_id| !is_saved_match_debugger_opaque_id(evidence_id))
        || evidence_ids
            .iter()
            .enumerate()
            .any(|(index, evidence_id)| evidence_ids[..index].contains(evidence_id))
    {
        return Err(
            "Review a claim and choose current confirmed evidence before saving.".to_string(),
        );
    }
    Ok(())
}
