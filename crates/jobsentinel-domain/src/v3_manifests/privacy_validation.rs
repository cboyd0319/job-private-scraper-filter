use crate::v3_contracts::{require_nonempty, require_schema, SchemaId};

use super::{AgentTask, DataCategory, PrivacyLabel, PrivacyReceipt, EXTERNAL_AI_GATEWAY_POLICY};

impl PrivacyReceipt {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::PrivacyReceiptV1)?;
        require_identifier("receipt id", &self.receipt_id, 128)?;
        require_identifier("task id", &self.task_id, 128)?;
        require_bounded_text(
            "delete or revoke action",
            &self.delete_or_revoke_action,
            256,
        )?;
        if let Some(pack_id) = &self.pack_id {
            require_identifier("receipt pack id", pack_id, 128)?;
        }
        if let Some(approval) = &self.approval_reference {
            require_identifier("receipt approval", approval, 128)?;
        }
        if self.labels.is_empty() || self.data_categories.is_empty() || !self.stored_locally {
            return Err("privacy receipt must retain typed local audit metadata".to_string());
        }
        let has_protected_data = self
            .data_categories
            .iter()
            .copied()
            .any(is_protected_local_category);
        if has_protected_data
            && (!self.labels.contains(&PrivacyLabel::LocalOnly)
                || !self.labels.contains(&PrivacyLabel::Sensitive)
                || self.labels.contains(&PrivacyLabel::PublicDataOnly)
                || self.labels.contains(&PrivacyLabel::ExternalAiOptional))
        {
            return Err(
                "military and protected-answer data must stay sensitive and local".to_string(),
            );
        }
        if self.data_left_device {
            if self.labels.contains(&PrivacyLabel::LocalOnly)
                || !self.labels.contains(&PrivacyLabel::ExternalAiOptional)
                || self.approval_reference.as_deref().is_none_or(str::is_empty)
                || self.gateway_policy_id.as_deref() != Some(EXTERNAL_AI_GATEWAY_POLICY)
                || !self
                    .external_destination
                    .as_deref()
                    .is_some_and(is_safe_external_destination)
            {
                return Err(
                    "external data flow requires the governed gateway and explicit approval"
                        .to_string(),
                );
            }
            if has_protected_data {
                return Err("military and protected-answer data must remain local".to_string());
            }
        } else if self.external_destination.is_some()
            || self.gateway_policy_id.is_some()
            || self.approval_reference.is_some()
        {
            return Err("local receipt cannot record an external route".to_string());
        }
        Ok(())
    }
}

impl AgentTask {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::AgentTaskV1)?;
        require_nonempty("task id", &self.task_id)?;
        if self.privacy_labels.is_empty()
            || self.data_categories.is_empty()
            || !(1..=3600).contains(&self.max_duration_seconds)
            || !(1..=10 * 1024 * 1024).contains(&self.max_output_bytes)
            || !(1..=3).contains(&self.max_attempts)
            || !self.user_review_required
        {
            return Err("agent task privacy, review, and resource bounds are invalid".to_string());
        }
        if self
            .data_categories
            .iter()
            .copied()
            .any(is_protected_local_category)
            && (!self.privacy_labels.contains(&PrivacyLabel::LocalOnly)
                || !self.privacy_labels.contains(&PrivacyLabel::Sensitive)
                || self.privacy_labels.contains(&PrivacyLabel::PublicDataOnly)
                || self
                    .privacy_labels
                    .contains(&PrivacyLabel::ExternalAiOptional))
        {
            return Err("protected agent data must stay sensitive and local".to_string());
        }
        Ok(())
    }
}

fn is_protected_local_category(category: DataCategory) -> bool {
    matches!(
        category,
        DataCategory::MilitaryService
            | DataCategory::ClearanceClaim
            | DataCategory::ProtectedVeteranAnswer
    )
}

fn is_safe_external_destination(value: &str) -> bool {
    jobsentinel_security::validate_credential_free_external_https_url(value).is_ok()
}

fn require_identifier(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    require_nonempty(label, value)?;
    if value.len() > max_bytes
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(format!("{label} is not a bounded opaque identifier"));
    }
    Ok(())
}

fn require_bounded_text(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    require_nonempty(label, value)?;
    if value.len() > max_bytes {
        return Err(format!("{label} exceeds the byte limit"));
    }
    Ok(())
}
