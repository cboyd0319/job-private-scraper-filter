use super::*;
use crate::desktop::Database;
use jobsentinel_domain::v3_manifests::{DataCategory, PrivacyLabel};
use jobsentinel_storage::outside_ai::{OutsideAiContext, OutsideAiStatus};
use std::collections::BTreeMap;
use std::future::pending;
use tokio::sync::watch;

fn config() -> ExternalAiConfig {
    ExternalAiConfig {
        enabled: true,
        provider: ExternalAiProvider::OpenAi,
        model: "gpt-test".to_string(),
        enabled_providers: vec![ExternalAiProvider::OpenAi],
        provider_models: BTreeMap::from([(ExternalAiProvider::OpenAi, "gpt-test".to_string())]),
        require_payload_preview: true,
        redaction: jobsentinel_domain::ExternalAiRedactionConfig { enabled: true },
        ..ExternalAiConfig::default()
    }
}

fn request() -> ExternalAiCommandRequest {
    ExternalAiCommandRequest {
        feature: "job-description-summary".to_string(),
        source_job_id: 1,
        provider: ExternalAiProvider::OpenAi,
        labels: vec![
            "External AI optional".to_string(),
            "Public-data only".to_string(),
        ],
        data_categories: vec!["job_posting".to_string()],
        payload: serde_json::json!({
            "title": "Security Analyst",
            "company": "Example Co",
            "description": "Review public security findings.",
        }),
        preview_shown: true,
        user_approved: true,
        explicitly_included_sensitive_data: false,
    }
}

async fn database() -> Database {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut job = jobsentinel_domain::Job::newly_discovered(
        "Security Analyst",
        "Example Co",
        "https://jobs.example.test/security-analyst",
        None,
        "public-test-source",
        chrono::Utc::now(),
    );
    job.description = Some("Review public security findings.".to_string());
    assert_eq!(database.upsert_job(&job).await.unwrap(), 1);
    database
}

#[tokio::test]
async fn prepare_creates_backend_owned_exact_public_review() {
    let database = database().await;
    let untrusted_renderer_flags = ExternalAiCommandRequest {
        preview_shown: false,
        user_approved: false,
        ..request()
    };

    let prepared = prepare_external_ai_request(&untrusted_renderer_flags, &config(), &database)
        .await
        .unwrap();
    let operation = database
        .get_outside_ai_operation_by_approval(&prepared.approval_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(operation.status, OutsideAiStatus::Pending);
    assert_eq!(
        operation.context.labels,
        [
            PrivacyLabel::ExternalAiOptional,
            PrivacyLabel::PublicDataOnly
        ]
    );
    assert_eq!(
        operation.context.data_categories,
        [DataCategory::PublicJobPosting]
    );
}

#[tokio::test]
async fn unknown_approval_is_rejected_before_credential_access() {
    let database = database().await;
    let credentials =
        CredentialService::with_fixed_master_key(database.credentials(), [31_u8; 32], false);
    let (_cancel, receiver) = watch::channel(false);

    let error = send_external_ai_request(
        "outside-ai-approval:unknown",
        &request(),
        &config(),
        &credentials,
        &database,
        receiver,
    )
    .await
    .unwrap_err();

    assert!(error.contains("review"));
    assert!(!error.contains("credential"));
}

#[tokio::test]
async fn payload_drift_cannot_consume_a_review() {
    let database = database().await;
    let prepared = prepare_external_ai_request(&request(), &config(), &database)
        .await
        .unwrap();
    let credentials =
        CredentialService::with_fixed_master_key(database.credentials(), [31_u8; 32], false);
    let changed = ExternalAiCommandRequest {
        payload: serde_json::json!({
            "title": "Different role",
            "company": "Example Co",
        }),
        ..request()
    };
    let (_cancel, receiver) = watch::channel(false);

    assert!(send_external_ai_request(
        &prepared.approval_id,
        &changed,
        &config(),
        &credentials,
        &database,
        receiver,
    )
    .await
    .is_err());
    assert_eq!(
        database
            .get_outside_ai_operation_by_approval(&prepared.approval_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Pending
    );
}

#[tokio::test]
async fn renderer_cannot_replace_public_job_fields_with_private_claims() {
    let database = database().await;
    let injected = ExternalAiCommandRequest {
        payload: serde_json::json!({
            "title": "Security Analyst",
            "company": "Example Co",
            "description": "I am a protected veteran with an active clearance.",
        }),
        ..request()
    };

    let error = prepare_external_ai_request(&injected, &config(), &database)
        .await
        .unwrap_err();

    assert!(error.contains("stored public job"));
}

#[tokio::test]
async fn receipt_is_durable_before_provider_future_and_success_is_terminal() {
    let database = database().await;
    let validated = jobsentinel_ai::validate_external_ai_request(&request(), &config()).unwrap();
    let context = OutsideAiContext {
        feature_id: "job-description-summary".to_string(),
        provider_id: "open_ai".to_string(),
        destination: validated.destination().to_string(),
        request_sha256: validated.request_sha256().to_string(),
        labels: vec![
            PrivacyLabel::ExternalAiOptional,
            PrivacyLabel::PublicDataOnly,
        ],
        data_categories: vec![DataCategory::PublicJobPosting],
        gateway_policy_revision: jobsentinel_domain::v3_manifests::EXTERNAL_AI_GATEWAY_POLICY
            .to_string(),
    };
    let reviewed = database
        .create_outside_ai_review(&context, chrono::Utc::now() + chrono::Duration::minutes(10))
        .await
        .unwrap();
    let started = database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context,
        )
        .await
        .unwrap();
    let (_cancel, receiver) = watch::channel(false);
    let provider_call = async {
        assert!(
            database
                .get_outside_ai_operation(&started.operation_id)
                .await
                .unwrap()
                .unwrap()
                .receipt_recorded
        );
        Ok(ExternalAiCommandResponse {
            text: "Reviewed summary".to_string(),
        })
    };

    let response = finish_provider_attempt(&database, &started, receiver, provider_call)
        .await
        .unwrap();

    assert_eq!(response.text, "Reviewed summary");
    assert_eq!(
        database
            .get_outside_ai_operation(&started.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Succeeded
    );
}

#[tokio::test]
async fn provider_failure_is_terminal_and_keeps_the_attempt_receipt() {
    let database = database().await;
    let (started, _validated) = started_request(&database).await;
    let (_cancel, receiver) = watch::channel(false);

    let error = finish_provider_attempt(&database, &started, receiver, async {
        Err(ExternalAiSendError::Rejected(
            "provider rejected request".to_string(),
        ))
    })
    .await
    .unwrap_err();

    assert_eq!(error, "provider rejected request");
    let stored = database
        .get_outside_ai_operation(&started.operation_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, OutsideAiStatus::Failed);
    assert!(stored.receipt_recorded);
}

#[tokio::test]
async fn in_flight_cancellation_is_recorded_as_ambiguous() {
    let database = database().await;
    let (started, _validated) = started_request(&database).await;
    let (cancel, receiver) = watch::channel(false);
    let provider_call = async move {
        cancel.send(true).unwrap();
        pending::<Result<ExternalAiCommandResponse, ExternalAiSendError>>().await
    };

    let error = finish_provider_attempt(&database, &started, receiver, provider_call)
        .await
        .unwrap_err();

    assert!(error.contains("may have received"));
    assert_eq!(
        database
            .get_outside_ai_operation(&started.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Ambiguous
    );
}

#[tokio::test]
async fn uncertain_provider_outcome_is_recorded_as_ambiguous() {
    let database = database().await;
    let (started, _validated) = started_request(&database).await;
    let (_cancel, receiver) = watch::channel(false);

    let error = finish_provider_attempt(&database, &started, receiver, async {
        Err(ExternalAiSendError::OutcomeUnknown(
            "The provider outcome is unknown. Do not retry.".to_string(),
        ))
    })
    .await
    .unwrap_err();

    assert!(error.contains("Do not retry"));
    assert_eq!(
        database
            .get_outside_ai_operation(&started.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Ambiguous
    );
}

#[tokio::test]
async fn pending_review_can_be_cancelled_by_approval_id() {
    let database = database().await;
    let prepared = prepare_external_ai_request(&request(), &config(), &database)
        .await
        .unwrap();

    let cancelled = cancel_external_ai_request(&prepared.approval_id, &database)
        .await
        .unwrap();

    assert_eq!(cancelled.outcome, ExternalAiCancelOutcome::Cancelled);
    assert_eq!(
        database
            .get_outside_ai_operation_by_approval(&prepared.approval_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Cancelled
    );
}

#[tokio::test]
async fn activity_projection_exposes_status_without_review_secrets() {
    let database = database().await;
    let prepared = prepare_external_ai_request(&request(), &config(), &database)
        .await
        .unwrap();
    cancel_external_ai_request(&prepared.approval_id, &database)
        .await
        .unwrap();

    let activity = list_external_ai_activity(&database).await.unwrap();
    let serialized = serde_json::to_string(&activity).unwrap();

    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].status, ExternalAiActivityStatus::Cancelled);
    assert!(!serialized.contains(&prepared.approval_id));
    assert!(!serialized.contains("requestSha256"));
}

#[tokio::test]
async fn delivered_cancellation_wins_when_provider_success_is_also_ready() {
    let database = database().await;
    let (started, _validated) = started_request(&database).await;
    let (cancel, receiver) = watch::channel(false);
    cancel.send(true).unwrap();

    let error = finish_provider_attempt(&database, &started, receiver, async {
        Ok(ExternalAiCommandResponse {
            text: "late success".to_string(),
        })
    })
    .await
    .unwrap_err();

    assert!(error.contains("may have received"));
    assert_eq!(
        database
            .get_outside_ai_operation(&started.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Ambiguous
    );
}

#[tokio::test]
async fn durable_and_in_memory_cancellation_converge_on_ambiguous() {
    let database = database().await;
    let (started, _validated) = started_request(&database).await;
    let (cancel, receiver) = watch::channel(false);
    let operation_id = started.operation_id.clone();
    let provider_call = async {
        assert_eq!(
            database
                .cancel_outside_ai_operation(&operation_id)
                .await
                .unwrap(),
            OutsideAiStatus::Ambiguous
        );
        cancel.send(true).unwrap();
        pending::<Result<ExternalAiCommandResponse, ExternalAiSendError>>().await
    };

    let error = finish_provider_attempt(&database, &started, receiver, provider_call)
        .await
        .unwrap_err();

    assert!(error.contains("provider may have received"));
    assert_eq!(
        database
            .get_outside_ai_operation(&started.operation_id)
            .await
            .unwrap()
            .unwrap()
            .status,
        OutsideAiStatus::Ambiguous
    );
}

async fn started_request(
    database: &Database,
) -> (
    jobsentinel_storage::outside_ai::OutsideAiOperation,
    ValidatedExternalAiRequest,
) {
    let validated = jobsentinel_ai::validate_external_ai_request(&request(), &config()).unwrap();
    let context = outside_ai_context(&request(), &validated);
    let reviewed = database
        .create_outside_ai_review(&context, chrono::Utc::now() + chrono::Duration::minutes(10))
        .await
        .unwrap();
    let started = database
        .start_reviewed_outside_ai_request(
            &reviewed.operation_id,
            &reviewed.approval_reference,
            &context,
        )
        .await
        .unwrap();
    (started, validated)
}
