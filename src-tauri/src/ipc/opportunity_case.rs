use crate::application::v3_foundation::{
    open_opportunity_case as open_case_snapshot, OpportunityCaseSnapshot,
};
use crate::bootstrap::AppState;
use crate::desktop::GhostConfig;
use crate::ipc::errors::user_friendly_error;
use tauri::State;

/// Open one existing saved job as a local opportunity case.
#[tauri::command]
pub(crate) async fn open_opportunity_case(
    job_hash: String,
    state: State<'_, AppState>,
) -> Result<OpportunityCaseSnapshot, String> {
    validate_open_opportunity_case_args(&job_hash)?;
    let stale_threshold_days = {
        let config = state.config.read().await;
        config
            .ghost_config
            .as_ref()
            .map(|config| config.stale_threshold_days)
            .unwrap_or_else(|| GhostConfig::default().stale_threshold_days)
    };
    validate_stale_threshold_days(stale_threshold_days)?;
    tracing::info!(
        job_hash_chars = job_hash.chars().count(),
        "Command: open opportunity case"
    );

    open_case_snapshot(state.database.as_ref(), &job_hash, stale_threshold_days)
        .await
        .map_err(|error| user_friendly_error("Opportunity case is unavailable", error))
}

fn validate_open_opportunity_case_args(job_hash: &str) -> Result<(), String> {
    if job_hash.is_empty() || job_hash.len() > 128 || job_hash.chars().any(char::is_control) {
        return Err("Choose a saved job before opening its case.".to_string());
    }
    Ok(())
}

fn validate_stale_threshold_days(stale_threshold_days: i64) -> Result<(), String> {
    if stale_threshold_days < 1 {
        return Err("Posting freshness settings need a positive stale-day threshold.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_open_opportunity_case_args, validate_stale_threshold_days};

    #[test]
    fn command_arguments_require_a_saved_job_and_positive_stale_threshold() {
        assert_eq!(validate_open_opportunity_case_args("saved-job"), Ok(()));
        assert!(validate_open_opportunity_case_args("").is_err());
        assert!(validate_open_opportunity_case_args("bad\u{0000}job").is_err());
        assert!(validate_open_opportunity_case_args(&"a".repeat(129)).is_err());
        assert_eq!(validate_stale_threshold_days(1), Ok(()));
        assert!(validate_stale_threshold_days(0).is_err());
    }
}
