//! Credential-aware orchestration for reviewed external-AI requests.

use chrono::{DateTime, Duration, Utc};
use jobsentinel_ai::{
    send_validated_external_ai_request, validate_external_ai_prepare_request,
    validate_external_ai_request, ExternalAiSendError, ValidatedExternalAiRequest,
    CHOOSE_EXTERNAL_AI_PROVIDER_MESSAGE,
};
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{DataCategory, PrivacyLabel, PrivacyReceipt, EXTERNAL_AI_GATEWAY_POLICY},
};
use jobsentinel_storage::{
    outside_ai::{OutsideAiCompletion, OutsideAiContext, OutsideAiOperation, OutsideAiStatus},
    Database,
};
use serde::{Deserialize, Serialize};
use std::future::{pending, Future};
use tokio::sync::watch;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{
    config::{ExternalAiConfig, ExternalAiProvider},
    credentials::{CredentialKey, CredentialService},
};

pub use jobsentinel_ai::{ExternalAiCommandRequest, ExternalAiCommandResponse};

const REVIEW_TTL_MINUTES: i64 = 10;
const REVIEW_UNAVAILABLE: &str =
    "This Outside AI request has no current matching review. Review it again before sending.";
const AUDIT_UNAVAILABLE: &str =
    "Outside AI audit is unavailable. Nothing was sent. Review the request again.";
const OUTCOME_UNKNOWN: &str =
    "The Outside AI outcome is unknown. Open Settings > Outside AI to check durable activity. Do not retry this request.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAiPrepareResponse {
    pub approval_id: String,
    pub provider_id: String,
    pub destination: String,
    pub model: String,
    pub field_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAiCancelOutcome {
    Cancelled,
    Ambiguous,
    AlreadyCompleted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAiCancelResponse {
    pub outcome: ExternalAiCancelOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalAiActivityStatus {
    Pending,
    Started,
    Succeeded,
    Failed,
    Ambiguous,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalAiActivityEntry {
    pub provider_id: String,
    pub destination: String,
    pub status: ExternalAiActivityStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Store an exact, short-lived backend review without trusting renderer flags.
pub async fn prepare_external_ai_request(
    request: &ExternalAiCommandRequest,
    config: &ExternalAiConfig,
    database: &Database,
) -> Result<ExternalAiPrepareResponse, String> {
    validate_stored_public_job_payload(request, database).await?;
    let validated = validate_external_ai_prepare_request(request, config)?;
    let context = outside_ai_context(request, &validated);
    let operation = database
        .create_outside_ai_review(&context, Utc::now() + Duration::minutes(REVIEW_TTL_MINUTES))
        .await
        .map_err(|_| AUDIT_UNAVAILABLE.to_string())?;
    Ok(ExternalAiPrepareResponse {
        approval_id: operation.approval_reference,
        provider_id: context.provider_id,
        destination: context.destination,
        model: validated.model().to_string(),
        field_count: request.payload.as_object().map_or(0, serde_json::Map::len),
    })
}

/// Consume one exact backend review, retrieve its credential, and send once.
pub async fn send_external_ai_request(
    approval_id: &str,
    request: &ExternalAiCommandRequest,
    config: &ExternalAiConfig,
    credentials: &CredentialService,
    database: &Database,
    cancellation: watch::Receiver<bool>,
) -> Result<ExternalAiCommandResponse, String> {
    validate_stored_public_job_payload(request, database).await?;
    let validated = validate_external_ai_request(request, config)?;
    let context = outside_ai_context(request, &validated);
    let reviewed = database
        .get_outside_ai_operation_by_approval(approval_id)
        .await
        .map_err(|_| REVIEW_UNAVAILABLE.to_string())?
        .ok_or_else(|| REVIEW_UNAVAILABLE.to_string())?;
    let started = database
        .start_reviewed_outside_ai_request(&reviewed.operation_id, approval_id, &context)
        .await
        .map_err(|_| REVIEW_UNAVAILABLE.to_string())?;
    let api_key = match retrieve_provider_key(&validated, credentials).await {
        Ok(api_key) => api_key,
        Err(error) => return stop_before_send(database, &started, error).await,
    };
    if *cancellation.borrow() {
        return stop_before_send(
            database,
            &started,
            "Outside AI request was cancelled before sending.".to_string(),
        )
        .await;
    }
    finish_provider_attempt(
        database,
        &started,
        cancellation,
        send_validated_external_ai_request(&validated, api_key.as_str()),
    )
    .await
}

pub async fn cancel_external_ai_request(
    approval_id: &str,
    database: &Database,
) -> Result<ExternalAiCancelResponse, String> {
    let reviewed = database
        .get_outside_ai_operation_by_approval(approval_id)
        .await
        .map_err(|_| REVIEW_UNAVAILABLE.to_string())?
        .ok_or_else(|| REVIEW_UNAVAILABLE.to_string())?;
    let status = database
        .cancel_outside_ai_operation(&reviewed.operation_id)
        .await
        .map_err(|_| REVIEW_UNAVAILABLE.to_string())?;
    let outcome = match status {
        jobsentinel_storage::outside_ai::OutsideAiStatus::Cancelled => {
            ExternalAiCancelOutcome::Cancelled
        }
        jobsentinel_storage::outside_ai::OutsideAiStatus::Ambiguous => {
            ExternalAiCancelOutcome::Ambiguous
        }
        jobsentinel_storage::outside_ai::OutsideAiStatus::Succeeded
        | jobsentinel_storage::outside_ai::OutsideAiStatus::Failed => {
            ExternalAiCancelOutcome::AlreadyCompleted
        }
        jobsentinel_storage::outside_ai::OutsideAiStatus::Pending
        | jobsentinel_storage::outside_ai::OutsideAiStatus::Started => {
            return Err(REVIEW_UNAVAILABLE.to_string());
        }
    };
    Ok(ExternalAiCancelResponse { outcome })
}

pub async fn list_external_ai_activity(
    database: &Database,
) -> Result<Vec<ExternalAiActivityEntry>, String> {
    Ok(database
        .list_recent_outside_ai_activity(20)
        .await
        .map_err(|_| "Outside AI activity is unavailable.".to_string())?
        .into_iter()
        .map(|entry| ExternalAiActivityEntry {
            provider_id: entry.provider_id,
            destination: entry.destination,
            status: activity_status(entry.status),
            created_at: entry.created_at,
            completed_at: entry.completed_at,
        })
        .collect())
}

const fn activity_status(status: OutsideAiStatus) -> ExternalAiActivityStatus {
    match status {
        OutsideAiStatus::Pending => ExternalAiActivityStatus::Pending,
        OutsideAiStatus::Started => ExternalAiActivityStatus::Started,
        OutsideAiStatus::Succeeded => ExternalAiActivityStatus::Succeeded,
        OutsideAiStatus::Failed => ExternalAiActivityStatus::Failed,
        OutsideAiStatus::Ambiguous => ExternalAiActivityStatus::Ambiguous,
        OutsideAiStatus::Cancelled => ExternalAiActivityStatus::Cancelled,
    }
}

async fn retrieve_provider_key(
    request: &ValidatedExternalAiRequest,
    credentials: &CredentialService,
) -> Result<Zeroizing<String>, String> {
    let key = credential_key_for_provider(request.provider())
        .ok_or_else(|| CHOOSE_EXTERNAL_AI_PROVIDER_MESSAGE.to_string())?;
    let value = credentials
        .retrieve(key)
        .await
        .map_err(|_| "Outside AI credential is unavailable.".to_string())?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Add this provider's API key in Settings first.".to_string())?;

    Ok(Zeroizing::new(value))
}

const fn credential_key_for_provider(provider: ExternalAiProvider) -> Option<CredentialKey> {
    match provider {
        ExternalAiProvider::OpenAi => Some(CredentialKey::ExternalAiOpenAiApiKey),
        ExternalAiProvider::Anthropic => Some(CredentialKey::ExternalAiAnthropicApiKey),
        ExternalAiProvider::GoogleGemini => Some(CredentialKey::ExternalAiGoogleApiKey),
        ExternalAiProvider::GithubCopilot => Some(CredentialKey::ExternalAiGithubCopilotApiKey),
        ExternalAiProvider::Custom => Some(CredentialKey::ExternalAiCustomApiKey),
        ExternalAiProvider::None => None,
    }
}

fn outside_ai_context(
    request: &ExternalAiCommandRequest,
    validated: &ValidatedExternalAiRequest,
) -> OutsideAiContext {
    OutsideAiContext {
        feature_id: request.feature.clone(),
        provider_id: provider_id(validated.provider()).to_string(),
        destination: validated.destination().to_string(),
        request_sha256: validated.request_sha256().to_string(),
        labels: vec![
            PrivacyLabel::ExternalAiOptional,
            PrivacyLabel::PublicDataOnly,
        ],
        data_categories: vec![DataCategory::PublicJobPosting],
        gateway_policy_revision: EXTERNAL_AI_GATEWAY_POLICY.to_string(),
    }
}

async fn validate_stored_public_job_payload(
    request: &ExternalAiCommandRequest,
    database: &Database,
) -> Result<(), String> {
    let job = database
        .get_job_by_id(request.source_job_id)
        .await
        .map_err(|_| "The stored public job could not be verified.".to_string())?
        .ok_or_else(|| "The stored public job could not be verified.".to_string())?;
    let submitted = request
        .payload
        .as_object()
        .filter(|payload| !payload.is_empty())
        .ok_or_else(|| "Choose at least one stored public job field to send.".to_string())?;
    let mut canonical = serde_json::Map::new();
    canonical.insert("title".to_string(), serde_json::Value::String(job.title));
    canonical.insert(
        "company".to_string(),
        serde_json::Value::String(job.company),
    );
    insert_trimmed_public_field(&mut canonical, "location", job.location.as_deref());
    insert_trimmed_public_field(&mut canonical, "description", job.description.as_deref());
    if submitted
        .iter()
        .any(|(key, value)| canonical.get(key) != Some(value))
    {
        return Err(
            "Outside AI details must be unchanged fields from the stored public job. Remove fields instead of adding or rewriting them."
                .to_string(),
        );
    }
    Ok(())
}

fn insert_trimmed_public_field(
    payload: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert(
            key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }
}

const fn provider_id(provider: ExternalAiProvider) -> &'static str {
    match provider {
        ExternalAiProvider::None => "none",
        ExternalAiProvider::OpenAi => "open_ai",
        ExternalAiProvider::Anthropic => "anthropic",
        ExternalAiProvider::GoogleGemini => "google_gemini",
        ExternalAiProvider::GithubCopilot => "github_copilot",
        ExternalAiProvider::Custom => "custom",
    }
}

async fn finish_provider_attempt<F>(
    database: &Database,
    operation: &OutsideAiOperation,
    mut cancellation: watch::Receiver<bool>,
    provider_call: F,
) -> Result<ExternalAiCommandResponse, String>
where
    F: Future<Output = Result<ExternalAiCommandResponse, ExternalAiSendError>>,
{
    let receipt = PrivacyReceipt {
        schema: SchemaId::PrivacyReceiptV1,
        receipt_id: format!("outside-ai-receipt:{}", Uuid::new_v4()),
        task_id: operation.operation_id.clone(),
        pack_id: None,
        labels: operation.context.labels.clone(),
        data_categories: operation.context.data_categories.clone(),
        stored_locally: true,
        data_left_device: true,
        external_destination: Some(operation.context.destination.clone()),
        gateway_policy_id: Some(EXTERNAL_AI_GATEWAY_POLICY.to_string()),
        approval_reference: Some(operation.approval_reference.clone()),
        delete_or_revoke_action: "Disable Outside AI in Settings to stop future requests."
            .to_string(),
    };
    if crate::v3_foundation::record_privacy_receipt(database, &receipt)
        .await
        .is_err()
    {
        return stop_before_send(database, operation, AUDIT_UNAVAILABLE.to_string()).await;
    }

    tokio::select! {
        biased;
        () = wait_for_cancellation(&mut cancellation) => {
            finish_terminal(database, operation, OutsideAiCompletion::Ambiguous).await?;
            Err("Outside AI request was cancelled. The provider may have received it.".to_string())
        },
        result = provider_call => finish_provider_result(database, operation, result).await,
    }
}

async fn wait_for_cancellation(cancellation: &mut watch::Receiver<bool>) {
    loop {
        if *cancellation.borrow() {
            return;
        }
        if cancellation.changed().await.is_err() {
            pending::<()>().await;
        }
    }
}

async fn finish_provider_result(
    database: &Database,
    operation: &OutsideAiOperation,
    result: Result<ExternalAiCommandResponse, ExternalAiSendError>,
) -> Result<ExternalAiCommandResponse, String> {
    match result {
        Ok(response) => {
            finish_terminal(database, operation, OutsideAiCompletion::Succeeded).await?;
            Ok(response)
        }
        Err(ExternalAiSendError::Rejected(error)) => {
            finish_terminal(database, operation, OutsideAiCompletion::Failed).await?;
            Err(error)
        }
        Err(ExternalAiSendError::OutcomeUnknown(_)) => {
            finish_terminal(database, operation, OutsideAiCompletion::Ambiguous).await?;
            Err(OUTCOME_UNKNOWN.to_string())
        }
    }
}

async fn finish_terminal(
    database: &Database,
    operation: &OutsideAiOperation,
    completion: OutsideAiCompletion,
) -> Result<(), String> {
    if database
        .finish_outside_ai_request(&operation.operation_id, completion)
        .await
        .is_ok()
    {
        return Ok(());
    }
    if database
        .get_outside_ai_operation(&operation.operation_id)
        .await
        .ok()
        .flatten()
        .is_some_and(|stored| stored.status == completion_status(completion))
    {
        return Ok(());
    }
    let _ = database
        .finish_outside_ai_request(&operation.operation_id, OutsideAiCompletion::Ambiguous)
        .await;
    Err(OUTCOME_UNKNOWN.to_string())
}

const fn completion_status(completion: OutsideAiCompletion) -> OutsideAiStatus {
    match completion {
        OutsideAiCompletion::Succeeded => OutsideAiStatus::Succeeded,
        OutsideAiCompletion::Failed => OutsideAiStatus::Failed,
        OutsideAiCompletion::Ambiguous => OutsideAiStatus::Ambiguous,
    }
}

async fn stop_before_send(
    database: &Database,
    operation: &OutsideAiOperation,
    error: String,
) -> Result<ExternalAiCommandResponse, String> {
    database
        .finish_outside_ai_request(&operation.operation_id, OutsideAiCompletion::Failed)
        .await
        .map_err(|_| AUDIT_UNAVAILABLE.to_string())?;
    Err(error)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests;
