use crate::application::v3_foundation::{
    confirm_pending_saved_match_military_transition_review,
    confirm_saved_match_military_evidence as confirm_military_evidence,
    prepare_pending_saved_match_military_transition_review, MilitaryBranch,
    MilitaryTransitionWording, MilitaryWordingMapping, SavedMatchMilitaryEvidenceKind,
    SavedMatchMilitaryTransitionConfirmation,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use crate::ipc::resume::resume_match_debugger_commands::validate_saved_match_debugger_args;
use serde::Deserialize;
use tauri::State;
use uuid::Uuid;

const MAX_OCCUPATION_CODE_CHARS: usize = 32;
const MAX_CIVILIAN_ROLE_CHARS: usize = 256;
const MAX_MAPPING_VALUE_CHARS: usize = 256;
const MAX_CURRENT_CLEARANCE_CHARS: usize = 128;
const MAX_MAPPING_ROWS: usize = 16;

/// One user-reviewed military phrase and its proposed civilian wording.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MilitaryWordingMappingInput {
    pub(crate) military_evidence: String,
    pub(crate) civilian_wording: String,
}

/// Renderer input accepted only for a bounded local military-transition review.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MilitaryTransitionWordingInput {
    pub(crate) occupation_code: String,
    pub(crate) civilian_role: String,
    pub(crate) responsibility_mappings: Vec<MilitaryWordingMappingInput>,
    pub(crate) credential_mappings: Vec<MilitaryWordingMappingInput>,
    pub(crate) current_clearance: Option<String>,
}

/// Confirm one canonical military or current-clearance field for a saved match.
#[tauri::command]
pub(crate) async fn confirm_saved_match_military_evidence(
    job_hash: String,
    resume_id: i64,
    kind: SavedMatchMilitaryEvidenceKind,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        "Command: confirm saved match military evidence"
    );

    confirm_military_evidence(state.database.as_ref(), &job_hash, resume_id, kind)
        .await
        .map_err(|error| user_friendly_error("Military evidence could not be confirmed", error))
}

/// Prepare one opaque, local military-transition review from confirmed saved-match evidence.
#[tauri::command]
pub(crate) async fn prepare_saved_match_military_transition_review(
    job_hash: String,
    resume_id: i64,
    branch: MilitaryBranch,
    wording: MilitaryTransitionWordingInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    validate_military_transition_prepare_args(&job_hash, resume_id, &wording)?;
    tracing::info!(
        resume_id,
        job_hash_chars = job_hash.chars().count(),
        responsibility_mapping_count = wording.responsibility_mappings.len(),
        credential_mapping_count = wording.credential_mappings.len(),
        "Command: prepare saved match military transition review"
    );

    prepare_pending_saved_match_military_transition_review(
        state.database.as_ref(),
        &state.pending_military_transition_reviews,
        &job_hash,
        resume_id,
        branch,
        into_application_wording(wording),
    )
    .await
    .map_err(|error| user_friendly_error("Military transition review is unavailable", error))
}

/// Consume one opaque local review token and return only its safe confirmed projection.
#[tauri::command]
pub(crate) async fn confirm_saved_match_military_transition_review(
    token: String,
    state: State<'_, AppState>,
) -> Result<SavedMatchMilitaryTransitionConfirmation, String> {
    if !is_exact_military_transition_review_token(&token) {
        return Err(
            "The military transition review is no longer available. Start again.".to_string(),
        );
    }
    tracing::info!("Command: confirm saved match military transition review");

    confirm_pending_saved_match_military_transition_review(
        state.database.as_ref(),
        &state.pending_military_transition_reviews,
        &token,
    )
    .await
    .map_err(|error| {
        user_friendly_error("Military transition review could not be confirmed", error)
    })
}

pub(super) fn validate_military_transition_prepare_args(
    job_hash: &str,
    resume_id: i64,
    wording: &MilitaryTransitionWordingInput,
) -> Result<(), String> {
    validate_saved_match_debugger_args(job_hash, resume_id)?;
    if !valid_trimmed_text(&wording.occupation_code, MAX_OCCUPATION_CODE_CHARS)
        || !valid_trimmed_text(&wording.civilian_role, MAX_CIVILIAN_ROLE_CHARS)
        || wording.responsibility_mappings.len() > MAX_MAPPING_ROWS
        || wording.credential_mappings.len() > MAX_MAPPING_ROWS
        || wording
            .responsibility_mappings
            .iter()
            .chain(&wording.credential_mappings)
            .any(|mapping| {
                !valid_trimmed_text(&mapping.military_evidence, MAX_MAPPING_VALUE_CHARS)
                    || !valid_trimmed_text(&mapping.civilian_wording, MAX_MAPPING_VALUE_CHARS)
            })
        || wording
            .current_clearance
            .as_ref()
            .is_some_and(|clearance| !valid_trimmed_text(clearance, MAX_CURRENT_CLEARANCE_CHARS))
    {
        return Err("Review the military wording and try again.".to_string());
    }
    Ok(())
}

pub(super) fn is_exact_military_transition_review_token(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|token| {
        token.get_version_num() == 4 && !token.is_nil() && token.to_string() == value
    })
}

fn into_application_wording(input: MilitaryTransitionWordingInput) -> MilitaryTransitionWording {
    MilitaryTransitionWording {
        occupation_code: input.occupation_code,
        civilian_role: input.civilian_role,
        responsibility_mappings: input
            .responsibility_mappings
            .into_iter()
            .map(into_application_mapping)
            .collect(),
        credential_mappings: input
            .credential_mappings
            .into_iter()
            .map(into_application_mapping)
            .collect(),
        current_clearance: input.current_clearance,
    }
}

fn into_application_mapping(input: MilitaryWordingMappingInput) -> MilitaryWordingMapping {
    MilitaryWordingMapping {
        military_evidence: input.military_evidence,
        civilian_wording: input.civilian_wording,
    }
}

fn valid_trimmed_text(value: &str, maximum_chars: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.chars().count() <= maximum_chars
        && !value.chars().any(char::is_control)
}
