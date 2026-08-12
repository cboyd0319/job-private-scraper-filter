use crate::config::validation_error::{ValidationError, ValidationErrors};
use crate::config::{Config, ExternalAiProvider};
use jobsentinel_security::validate_credential_free_external_https_url;
use std::collections::BTreeSet;

pub(super) fn validate_external_ai(config: &Config, errors: &mut ValidationErrors) {
    let external_ai = &config.external_ai;
    if !external_ai.enabled {
        return;
    }

    if external_ai.enabled_providers.is_empty() {
        errors.add(ValidationError::required_field(
            "external_ai.enabled_providers",
            "enable at least one outside-AI provider or turn outside AI off",
        ));
    }

    if external_ai.provider == ExternalAiProvider::None {
        errors.add(ValidationError::required_field(
            "external_ai.provider",
            "choose a primary outside-AI provider or turn outside AI off",
        ));
    }

    if external_ai.provider != ExternalAiProvider::None
        && !external_ai
            .enabled_providers
            .contains(&external_ai.provider)
    {
        errors.add(ValidationError::inconsistent_values(
            "external_ai.provider",
            "external_ai.enabled_providers",
            "primary provider must be one of the enabled outside-AI providers",
        ));
    }

    if !external_ai.provider_order.is_empty() {
        let mut seen = BTreeSet::new();
        for provider in &external_ai.provider_order {
            if !seen.insert(provider) {
                errors.add(ValidationError::invalid_value(
                    "external_ai.provider_order",
                    format!("{provider:?}"),
                    "provider order cannot contain duplicate providers",
                ));
            }
        }
    }

    if !external_ai.require_payload_preview {
        errors.add(ValidationError::invalid_value(
            "external_ai.require_payload_preview",
            false,
            "outside-AI requests must show a payload preview before sending",
        ));
    }

    if !external_ai.redaction.enabled {
        errors.add(ValidationError::invalid_value(
            "external_ai.redaction.enabled",
            false,
            "outside-AI redaction must stay enabled",
        ));
    }

    if external_ai
        .enabled_providers
        .contains(&ExternalAiProvider::Custom)
        && validate_credential_free_external_https_url(&external_ai.custom_endpoint).is_err()
    {
        errors.add(ValidationError::invalid_url(
            "external_ai.custom_endpoint",
            "",
            "custom outside-AI provider endpoint must be a public HTTPS URL",
        ));
    }
}
