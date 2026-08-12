//! Provider-specific outside-AI HTTP transports.

use jobsentinel_domain::ExternalAiProvider;
use jobsentinel_network::{
    send_external_https_text_with_retry, ExternalFetchError, ExternalHttpRequest,
    ExternalTextResponse,
};
use serde_json::Value;

use crate::{
    ExternalAiSendError, CHOOSE_EXTERNAL_AI_PROVIDER_MESSAGE,
    CUSTOM_EXTERNAL_AI_ENDPOINT_REQUIRED_MESSAGE,
};

const MAX_PROVIDER_RESPONSE_CHARS: usize = 16_000;
const OPENAI_RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const ANTHROPIC_MESSAGES_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const GEMINI_GENERATE_CONTENT_ENDPOINT_PREFIX: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/";
const GEMINI_GENERATE_CONTENT_ENDPOINT_SUFFIX: &str = ":generateContent";
const GITHUB_MODELS_CHAT_COMPLETIONS_ENDPOINT: &str =
    "https://models.github.ai/inference/chat/completions";

#[derive(Debug)]
pub(crate) struct ProviderRequest {
    pub(crate) provider: ExternalAiProvider,
    pub(crate) model: String,
    pub(crate) custom_endpoint: Option<String>,
    pub(crate) prompt: String,
}

pub(crate) async fn send_provider_request(
    request: &ProviderRequest,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let destination = provider_destination(request).map_err(ExternalAiSendError::Rejected)?;
    match request.provider {
        ExternalAiProvider::OpenAi => {
            send_openai_request(&destination, &request.model, &request.prompt, api_key).await
        }
        ExternalAiProvider::Anthropic => {
            send_anthropic_request(&destination, &request.model, &request.prompt, api_key).await
        }
        ExternalAiProvider::GoogleGemini => {
            send_gemini_request(&destination, &request.prompt, api_key).await
        }
        ExternalAiProvider::GithubCopilot => {
            send_github_models_request(&destination, &request.model, &request.prompt, api_key).await
        }
        ExternalAiProvider::Custom => {
            send_custom_chat_request(&destination, &request.model, &request.prompt, api_key).await
        }
        ExternalAiProvider::None => Err(ExternalAiSendError::Rejected(
            CHOOSE_EXTERNAL_AI_PROVIDER_MESSAGE.to_string(),
        )),
    }
}

pub(crate) fn provider_destination(request: &ProviderRequest) -> Result<String, String> {
    match request.provider {
        ExternalAiProvider::OpenAi => Ok(OPENAI_RESPONSES_ENDPOINT.to_string()),
        ExternalAiProvider::Anthropic => Ok(ANTHROPIC_MESSAGES_ENDPOINT.to_string()),
        ExternalAiProvider::GoogleGemini => Ok(format!(
            "{GEMINI_GENERATE_CONTENT_ENDPOINT_PREFIX}{}{GEMINI_GENERATE_CONTENT_ENDPOINT_SUFFIX}",
            urlencoding::encode(&request.model)
        )),
        ExternalAiProvider::GithubCopilot => {
            Ok(GITHUB_MODELS_CHAT_COMPLETIONS_ENDPOINT.to_string())
        }
        ExternalAiProvider::Custom => request
            .custom_endpoint
            .clone()
            .ok_or_else(|| CUSTOM_EXTERNAL_AI_ENDPOINT_REQUIRED_MESSAGE.to_string()),
        ExternalAiProvider::None => Err(CHOOSE_EXTERNAL_AI_PROVIDER_MESSAGE.to_string()),
    }
}

async fn send_openai_request(
    endpoint: &str,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let response = send_provider_http_request(
        ExternalHttpRequest::post(endpoint)
            .header("authorization", format!("Bearer {api_key}"))
            .json(serde_json::json!({
                "model": model,
                "input": prompt,
            })),
    )
    .await?;
    let value = provider_json(response, "open_ai").await?;
    extract_openai_response_text(&value).ok_or_else(provider_text_error)
}

async fn send_anthropic_request(
    endpoint: &str,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let response = send_provider_http_request(
        ExternalHttpRequest::post(endpoint)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(serde_json::json!({
                "model": model,
                "max_tokens": 1024,
                "messages": [{ "role": "user", "content": prompt }],
            })),
    )
    .await?;
    let value = provider_json(response, "anthropic").await?;
    extract_anthropic_response_text(&value).ok_or_else(provider_text_error)
}

async fn send_gemini_request(
    endpoint: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let response = send_provider_http_request(
        ExternalHttpRequest::post(endpoint)
            .query([("key".to_string(), api_key.to_string())])
            .json(serde_json::json!({
                "contents": [{ "parts": [{ "text": prompt }] }],
            })),
    )
    .await?;
    let value = provider_json(response, "google_gemini").await?;
    extract_gemini_response_text(&value).ok_or_else(provider_text_error)
}

async fn send_github_models_request(
    endpoint: &str,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let response = send_provider_http_request(
        ExternalHttpRequest::post(endpoint)
            .header("authorization", format!("Bearer {api_key}"))
            .json(chat_completion_body(model, prompt)),
    )
    .await?;
    let value = provider_json(response, "github_copilot").await?;
    extract_chat_response_text(&value).ok_or_else(provider_text_error)
}

async fn send_custom_chat_request(
    endpoint: &str,
    model: &str,
    prompt: &str,
    api_key: &str,
) -> Result<String, ExternalAiSendError> {
    let response = send_provider_http_request(
        ExternalHttpRequest::post(endpoint)
            .header("authorization", format!("Bearer {api_key}"))
            .json(chat_completion_body(model, prompt)),
    )
    .await?;
    let value = provider_json(response, "custom").await?;
    extract_chat_response_text(&value)
        .or_else(|| extract_openai_response_text(&value))
        .or_else(|| {
            value
                .get("text")
                .and_then(Value::as_str)
                .map(trimmed_response_text)
        })
        .ok_or_else(provider_text_error)
}

fn chat_completion_body(model: &str, prompt: &str) -> Value {
    serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
    })
}

async fn send_provider_http_request(
    request: ExternalHttpRequest,
) -> Result<ExternalTextResponse, ExternalAiSendError> {
    send_external_https_text_with_retry(request.without_retries())
        .await
        .map_err(classify_fetch_error)
}

fn classify_fetch_error(error: ExternalFetchError) -> ExternalAiSendError {
    match error {
        ExternalFetchError::InvalidTarget(_) | ExternalFetchError::Client => {
            ExternalAiSendError::Rejected(
                "Outside AI provider could not be reached before sending.".to_string(),
            )
        }
        ExternalFetchError::Timeout | ExternalFetchError::Request | ExternalFetchError::Body(_) => {
            ExternalAiSendError::OutcomeUnknown(
                "The Outside AI provider request outcome is unknown. Do not retry.".to_string(),
            )
        }
    }
}

async fn provider_json(
    response: ExternalTextResponse,
    provider: &str,
) -> Result<Value, ExternalAiSendError> {
    if !(200..300).contains(&response.status) {
        tracing::warn!(
            provider,
            status = response.status,
            "Outside AI provider returned an unsuccessful status"
        );
        return Err(provider_status_error(response.status));
    }

    serde_json::from_str(&response.body).map_err(|_| {
        ExternalAiSendError::OutcomeUnknown(
            "Outside AI returned an unreadable response. Do not retry.".to_string(),
        )
    })
}

fn provider_status_error(status: u16) -> ExternalAiSendError {
    if status == 408 || (500..600).contains(&status) {
        return ExternalAiSendError::OutcomeUnknown(
            "The Outside AI provider returned an uncertain server outcome. Do not retry."
                .to_string(),
        );
    }
    ExternalAiSendError::Rejected(
        "Outside AI provider rejected the request. Check provider setup before trying again."
            .to_string(),
    )
}

fn provider_text_error() -> ExternalAiSendError {
    ExternalAiSendError::OutcomeUnknown(
        "Outside AI returned a response JobSentinel could not read. Do not retry.".to_string(),
    )
}

fn trimmed_response_text(text: &str) -> String {
    text.trim()
        .chars()
        .take(MAX_PROVIDER_RESPONSE_CHARS)
        .collect()
}

fn extract_openai_response_text(value: &Value) -> Option<String> {
    value
        .get("output_text")
        .and_then(Value::as_str)
        .map(trimmed_response_text)
        .filter(|text| !text.is_empty())
        .or_else(|| extract_content_array_text(value.get("output")))
}

fn extract_anthropic_response_text(value: &Value) -> Option<String> {
    extract_content_array_text(value.get("content"))
}

fn extract_gemini_response_text(value: &Value) -> Option<String> {
    value
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|candidate| {
            candidate
                .pointer("/content/parts")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .map(trimmed_response_text)
        .find(|text| !text.is_empty())
}

fn extract_chat_response_text(value: &Value) -> Option<String> {
    value
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|choice| choice.pointer("/message/content").and_then(Value::as_str))
        .map(trimmed_response_text)
        .find(|text| !text.is_empty())
}

fn extract_content_array_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .map(trimmed_response_text)
        .find(|text| !text.is_empty())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extracts_provider_response_text() {
        let openai = serde_json::json!({ "output_text": " Summary text " });
        let anthropic =
            serde_json::json!({ "content": [{ "type": "text", "text": " Anthropic text " }] });
        let gemini = serde_json::json!({
            "candidates": [{ "content": { "parts": [{ "text": " Gemini text " }] } }]
        });
        let chat = serde_json::json!({ "choices": [{ "message": { "content": " Chat text " } }] });

        assert_eq!(
            extract_openai_response_text(&openai),
            Some("Summary text".to_string())
        );
        assert_eq!(
            extract_anthropic_response_text(&anthropic),
            Some("Anthropic text".to_string())
        );
        assert_eq!(
            extract_gemini_response_text(&gemini),
            Some("Gemini text".to_string())
        );
        assert_eq!(
            extract_chat_response_text(&chat),
            Some("Chat text".to_string())
        );
    }

    #[test]
    fn classifies_only_proven_pre_dispatch_failures_as_rejected() {
        assert!(matches!(
            classify_fetch_error(jobsentinel_network::ExternalFetchError::Client),
            ExternalAiSendError::Rejected(_)
        ));
        for error in [
            jobsentinel_network::ExternalFetchError::Timeout,
            jobsentinel_network::ExternalFetchError::Request,
        ] {
            assert!(matches!(
                classify_fetch_error(error),
                ExternalAiSendError::OutcomeUnknown(_)
            ));
        }
    }

    #[test]
    fn provider_server_error_is_an_unknown_outcome() {
        assert!(matches!(
            provider_status_error(503),
            ExternalAiSendError::OutcomeUnknown(_)
        ));
    }
}
