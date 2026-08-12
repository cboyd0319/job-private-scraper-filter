//! Defines exact source-consent contexts, statuses, and review reasons.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    v3_contracts::require_sha256,
    v3_foundation::{validate_identifier, SourcePolicy},
    v3_manifests::{DataCategory, SourceClass},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceConsentOperation {
    ScheduledCheck,
    RestrictedWorkbench,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceConsentContext {
    pub source_id: String,
    pub operation: SourceConsentOperation,
    pub warning_version: u32,
    pub behavior_revision: u32,
    pub policy_ref: String,
    pub policy_revision: u32,
    pub source_class: SourceClass,
    pub data_categories: Vec<DataCategory>,
    pub destination_sha256: String,
    pub request_sha256: String,
}

impl SourceConsentContext {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier("source", &self.source_id)?;
        validate_identifier("source policy", &self.policy_ref)?;
        require_sha256("source destination", &self.destination_sha256)?;
        require_sha256("source request", &self.request_sha256)?;
        require_lowercase_sha256("source destination", &self.destination_sha256)?;
        require_lowercase_sha256("source request", &self.request_sha256)?;
        if self.warning_version == 0 || self.behavior_revision == 0 || self.policy_revision == 0 {
            return Err("source consent revisions must be positive".to_string());
        }
        let expected_class = match self.operation {
            SourceConsentOperation::ScheduledCheck => SourceClass::RestrictedPublicScheduled,
            SourceConsentOperation::RestrictedWorkbench => SourceClass::RestrictedUserOpened,
        };
        if self.source_class != expected_class {
            return Err("source consent class is not reviewable".to_string());
        }
        if self.data_categories.is_empty()
            || self.data_categories.len() > 16
            || self
                .data_categories
                .iter()
                .enumerate()
                .any(|(index, category)| self.data_categories[..index].contains(category))
        {
            return Err("source consent data categories must be a nonempty set".to_string());
        }
        if self
            .data_categories
            .iter()
            .any(|category| match self.operation {
                SourceConsentOperation::ScheduledCheck => !matches!(
                    category,
                    DataCategory::PublicJobPosting
                        | DataCategory::CareerGoals
                        | DataCategory::LocationPreferences
                ),
                SourceConsentOperation::RestrictedWorkbench => !matches!(
                    category,
                    DataCategory::PublicJobPosting
                        | DataCategory::ApplicationHistory
                        | DataCategory::CareerGoals
                ),
            })
        {
            return Err(
                "remembered source consent contains an unreviewed data category".to_string(),
            );
        }
        Ok(())
    }

    #[must_use]
    pub fn matches(&self, current: &Self) -> bool {
        self.validate().is_ok()
            && current.validate().is_ok()
            && self.source_id == current.source_id
            && self.operation == current.operation
            && self.warning_version == current.warning_version
            && self.behavior_revision == current.behavior_revision
            && self.policy_ref == current.policy_ref
            && self.policy_revision == current.policy_revision
            && self.source_class == current.source_class
            && self.destination_sha256 == current.destination_sha256
            && self.request_sha256 == current.request_sha256
            && self.data_categories.len() == current.data_categories.len()
            && self
                .data_categories
                .iter()
                .all(|category| current.data_categories.contains(category))
    }

    #[must_use]
    pub fn matches_policy(&self, policy: &SourcePolicy) -> bool {
        let policy_ref_matches = self.policy_ref == policy.policy_ref;
        let revision_matches = self.policy_revision == policy.revision;
        policy.validate().is_ok()
            && self.source_id == policy.source_id
            && policy_ref_matches
            && revision_matches
            && self.source_class == policy.source_class
            && policy.user_review_required
            && matches!(
                (self.operation, policy.access),
                (
                    SourceConsentOperation::ScheduledCheck,
                    crate::v3_foundation::SourceAccess::ScheduledPublic
                ) | (
                    SourceConsentOperation::RestrictedWorkbench,
                    crate::v3_foundation::SourceAccess::UserOpened
                )
            )
    }
}

fn require_lowercase_sha256(label: &str, value: &str) -> Result<(), String> {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!("{label} SHA-256 must be lowercase"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceConsentDecision {
    Granted,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceConsentReviewReason {
    Missing,
    Revoked,
    ContextChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceConsentStatus {
    Remembered {
        event_id: String,
    },
    ReviewRequired {
        reason: SourceConsentReviewReason,
        latest_event_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceConsentEvent {
    pub sequence: i64,
    pub event_id: String,
    pub previous_event_id: Option<String>,
    pub context: SourceConsentContext,
    pub decision: SourceConsentDecision,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePolicyLedgerEntry {
    pub sequence: i64,
    pub policy: SourcePolicy,
    pub recorded_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> SourceConsentContext {
        SourceConsentContext {
            source_id: "dice".to_string(),
            operation: SourceConsentOperation::ScheduledCheck,
            warning_version: 1,
            behavior_revision: 2,
            policy_ref: "jobsentinel.source-policy.dice".to_string(),
            policy_revision: 3,
            source_class: SourceClass::RestrictedPublicScheduled,
            data_categories: vec![
                DataCategory::PublicJobPosting,
                DataCategory::CareerGoals,
                DataCategory::LocationPreferences,
            ],
            destination_sha256: "a".repeat(64),
            request_sha256: "b".repeat(64),
        }
    }

    fn workbench_context() -> SourceConsentContext {
        let mut context = context();
        context.source_id = "linkedin-workbench".to_string();
        context.operation = SourceConsentOperation::RestrictedWorkbench;
        context.behavior_revision = 1;
        context.policy_ref = "jobsentinel.source-policy.linkedin-workbench".to_string();
        context.policy_revision = 1;
        context.source_class = SourceClass::RestrictedUserOpened;
        context.data_categories = vec![
            DataCategory::PublicJobPosting,
            DataCategory::ApplicationHistory,
            DataCategory::CareerGoals,
        ];
        context.destination_sha256 = "c".repeat(64);
        context.request_sha256 = "d".repeat(64);
        context
    }

    #[test]
    fn exact_context_match_is_order_independent() {
        let expected = context();
        let mut current = expected.clone();
        current.data_categories.reverse();

        assert!(expected.validate().is_ok());
        assert!(expected.matches(&current));
    }

    #[test]
    fn every_material_context_change_requires_review() {
        let expected = context();
        let mut changed = Vec::new();

        let mut value = expected.clone();
        value.warning_version += 1;
        changed.push(value);
        let mut value = expected.clone();
        value.behavior_revision += 1;
        changed.push(value);
        let mut value = expected.clone();
        value.policy_ref = "jobsentinel.source-policy.dice-v2".to_string();
        changed.push(value);
        let mut value = expected.clone();
        value.policy_revision += 1;
        changed.push(value);
        let mut value = expected.clone();
        value.source_class = SourceClass::RestrictedUserOpened;
        changed.push(value);
        let mut value = expected.clone();
        value.data_categories.push(DataCategory::PayPreferences);
        changed.push(value);
        let mut value = expected.clone();
        value.destination_sha256 = "b".repeat(64);
        changed.push(value);
        let mut value = expected.clone();
        value.request_sha256 = "c".repeat(64);
        changed.push(value);

        assert!(changed.iter().all(|value| !expected.matches(value)));
    }

    #[test]
    fn remembered_source_consent_cannot_hold_private_text_or_protected_data() {
        let mut invalid = context();
        invalid.source_id = "dice private query".to_string();
        assert!(invalid.validate().is_err());

        invalid = context();
        invalid.data_categories = vec![DataCategory::ProtectedVeteranAnswer];
        assert!(invalid.validate().is_err());

        invalid = context();
        invalid.destination_sha256 = "https://example.test/private?token=secret".to_string();
        assert!(invalid.validate().is_err());

        invalid = context();
        invalid.data_categories = vec![DataCategory::ResumeEvidence];
        assert!(invalid.validate().is_err());

        invalid = context();
        invalid.request_sha256 = "A".repeat(64);
        assert!(invalid.validate().is_err());

        invalid = context();
        invalid.source_class = SourceClass::OfficialPublicApi;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn restricted_workbench_review_matches_only_current_user_opened_policy() {
        let context = workbench_context();
        let policy = SourcePolicy {
            source_id: context.source_id.clone(),
            source_class: context.source_class,
            access: crate::v3_foundation::SourceAccess::UserOpened,
            request_limit_per_hour: 0,
            user_review_required: true,
            policy_ref: context.policy_ref.clone(),
            revision: context.policy_revision,
            restriction_reason_code: Some("account-backed-source".to_string()),
            reviewed_at: Utc::now(),
        };

        assert!(context.validate().is_ok());
        assert!(context.matches_policy(&policy));

        let mut scheduled = context;
        scheduled.operation = SourceConsentOperation::ScheduledCheck;
        assert!(scheduled.validate().is_err());
        assert!(!scheduled.matches_policy(&policy));
    }

    #[test]
    fn disabled_policy_cannot_match_an_active_consent_context() {
        let current = context();
        let policy = SourcePolicy {
            source_id: current.source_id.clone(),
            source_class: current.source_class,
            access: crate::v3_foundation::SourceAccess::Disabled,
            request_limit_per_hour: 0,
            user_review_required: true,
            policy_ref: current.policy_ref.clone(),
            revision: current.policy_revision,
            restriction_reason_code: Some("terms-review".to_string()),
            reviewed_at: Utc::now(),
        };

        assert!(policy.validate().is_ok());
        assert!(!current.matches_policy(&policy));
    }

    #[test]
    fn restricted_source_policy_keeps_user_opened_and_scheduled_access_distinct() {
        let mut policy = SourcePolicy {
            source_id: "restricted.example".to_string(),
            source_class: SourceClass::RestrictedUserOpened,
            access: crate::v3_foundation::SourceAccess::ScheduledPublic,
            request_limit_per_hour: 10,
            user_review_required: false,
            policy_ref: "policy-1".to_string(),
            revision: 1,
            restriction_reason_code: None,
            reviewed_at: Utc::now(),
        };
        assert!(policy.validate().is_err());

        policy.source_class = SourceClass::RestrictedPublicScheduled;
        policy.restriction_reason_code = Some("terms-review".to_string());
        policy.request_limit_per_hour = 1;
        assert!(policy.validate().is_err());
        policy.user_review_required = true;
        assert!(policy.validate().is_ok());
        policy.access = crate::v3_foundation::SourceAccess::Disabled;
        policy.request_limit_per_hour = 0;
        assert!(policy.validate().is_ok());
    }
}
