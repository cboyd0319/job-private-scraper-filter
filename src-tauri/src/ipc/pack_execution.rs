//! Exposes exact reviewed pack tasks without accepting renderer-owned publisher trust.

use crate::application::pack_runtime::{
    cancel_reviewed_pack_task as cancel_pack_task, execute_production_draft_packet_task,
    execute_production_evidence_review_task, prepare_production_draft_packet_task,
    prepare_production_evidence_review_task, DraftPacketTaskResult, DraftPacketTaskReview,
    EvidenceReviewTaskResult, PackTaskReview,
};
use crate::bootstrap::AppState;
use crate::ipc::errors::user_friendly_error;
use crate::ipc::pack_management::validate_pack_identity;
use crate::ipc::resume::resume_match_debugger_commands::validate_saved_match_debugger_args;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub(crate) async fn prepare_evidence_reviewer(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<PackTaskReview, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    prepare_production_evidence_review_task(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        expected_generation,
        &job_hash,
        resume_id,
    )
    .await
    .map_err(|error| user_friendly_error("Evidence Reviewer could not be prepared", error))
}

#[tauri::command]
pub(crate) async fn execute_evidence_reviewer(
    run_id: String,
    approval_reference: String,
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<EvidenceReviewTaskResult, String> {
    validate_reviewed_task_id(&run_id, "pack-task:")?;
    validate_reviewed_task_id(&approval_reference, "pack-task-approval:")?;
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    execute_production_evidence_review_task(
        state.database.as_ref(),
        &state.pack_runtime,
        &run_id,
        &approval_reference,
        &job_hash,
        resume_id,
    )
    .await
    .map_err(|error| user_friendly_error("Evidence Reviewer stopped safely", error))
}

#[tauri::command]
pub(crate) async fn cancel_reviewed_pack_task(
    run_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    validate_reviewed_task_id(&run_id, "pack-task:")?;
    cancel_pack_task(state.database.as_ref(), &run_id)
        .await
        .map_err(|error| user_friendly_error("Pack task could not be cancelled", error))
}

#[tauri::command]
pub(crate) async fn prepare_packet_builder(
    publisher_key_id: String,
    pack_id: String,
    expected_generation: u64,
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<DraftPacketTaskReview, String> {
    validate_pack_identity(&publisher_key_id, &pack_id)?;
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    prepare_production_draft_packet_task(
        state.database.as_ref(),
        &state.pack_runtime,
        &publisher_key_id,
        &pack_id,
        expected_generation,
        &job_hash,
        resume_id,
    )
    .await
    .map_err(|error| user_friendly_error("Packet Builder could not be prepared", error))
}

#[tauri::command]
pub(crate) async fn execute_packet_builder(
    run_id: String,
    approval_reference: String,
    job_hash: String,
    resume_id: i64,
    state: State<'_, AppState>,
) -> Result<DraftPacketTaskResult, String> {
    validate_reviewed_task_id(&run_id, "pack-task:")?;
    validate_reviewed_task_id(&approval_reference, "pack-task-approval:")?;
    validate_saved_match_debugger_args(&job_hash, resume_id)?;
    execute_production_draft_packet_task(
        state.database.as_ref(),
        &state.pack_runtime,
        &run_id,
        &approval_reference,
        &job_hash,
        resume_id,
    )
    .await
    .map_err(|error| user_friendly_error("Packet Builder stopped safely", error))
}

fn validate_reviewed_task_id(value: &str, prefix: &str) -> Result<(), String> {
    value
        .strip_prefix(prefix)
        .and_then(|suffix| Uuid::parse_str(suffix).ok())
        .map(|_| ())
        .ok_or_else(|| "Refresh the reviewed task before continuing.".to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_reviewed_task_id;

    #[test]
    fn reviewed_task_ids_require_the_exact_prefix_and_uuid() {
        assert!(validate_reviewed_task_id(
            "pack-task:550e8400-e29b-41d4-a716-446655440000",
            "pack-task:"
        )
        .is_ok());
        for invalid in [
            "550e8400-e29b-41d4-a716-446655440000",
            "pack-task:not-a-uuid",
            "pack-task:550e8400-e29b-41d4-a716-446655440000:extra",
        ] {
            assert!(validate_reviewed_task_id(invalid, "pack-task:").is_err());
        }
    }
}
