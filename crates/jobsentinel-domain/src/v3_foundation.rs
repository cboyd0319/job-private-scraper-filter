use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    v3_contracts::SchemaId,
    v3_manifests::{PrivacyLabel, SourceClass},
};

pub const MAX_EVENT_METADATA_BYTES: usize = 2 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFile {
    pub case_file_id: String,
    pub job_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseFileEventKind {
    CaseCreated,
    StatusChanged,
    EvidenceLinked,
    SourceChecked,
    PrivacyReceiptRecorded,
    SourcePolicyChanged,
    RecoveryRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventOrigin {
    User,
    System,
    Source,
    Migration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseFileStatus {
    Saved,
    Reviewing,
    Preparing,
    Applied,
    Interviewing,
    Offer,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOutcome {
    Success,
    Failure,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    Retried,
    Restored,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventMetadata {
    Empty,
    StatusTransition {
        from: Option<CaseFileStatus>,
        to: CaseFileStatus,
    },
    LocalReference {
        reference_id: String,
    },
    SourceOutcome {
        source_id: String,
        outcome: SourceOutcome,
        item_count: u32,
        connectivity_required: bool,
    },
    RecoveryOutcome {
        outcome: RecoveryOutcome,
    },
}

impl EventMetadata {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::LocalReference { reference_id } => {
                validate_identifier("event reference", reference_id)?;
            }
            Self::SourceOutcome {
                source_id,
                item_count,
                ..
            } => {
                validate_identifier("event source", source_id)?;
                if *item_count > 10_000 {
                    return Err("event item count exceeds the local limit".to_string());
                }
            }
            Self::Empty | Self::StatusTransition { .. } | Self::RecoveryOutcome { .. } => {}
        }
        if self.to_json()?.len() > MAX_EVENT_METADATA_BYTES {
            return Err("event metadata exceeds the local byte limit".to_string());
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|_| "event metadata is invalid".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFileEventInput {
    pub case_file_id: String,
    pub kind: CaseFileEventKind,
    pub origin: EventOrigin,
    pub user_action: bool,
    pub privacy_labels: [PrivacyLabel; 2],
    pub metadata: EventMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseFileEvent {
    pub event_id: String,
    pub case_file_id: String,
    pub kind: CaseFileEventKind,
    pub origin: EventOrigin,
    pub user_action: bool,
    pub privacy_labels: [PrivacyLabel; 2],
    pub metadata: EventMetadata,
    pub created_at: DateTime<Utc>,
}

impl CaseFileEventInput {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier("case file", &self.case_file_id)?;
        self.metadata.validate()?;
        if self.user_action != matches!(self.origin, EventOrigin::User) {
            return Err("event user-action ownership is inconsistent".to_string());
        }
        if self.privacy_labels != [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive] {
            return Err("case history must stay sensitive and local".to_string());
        }
        let metadata_matches = matches!(
            (self.kind, &self.metadata),
            (CaseFileEventKind::CaseCreated, EventMetadata::Empty)
                | (
                    CaseFileEventKind::StatusChanged,
                    EventMetadata::StatusTransition { .. }
                )
                | (
                    CaseFileEventKind::EvidenceLinked
                        | CaseFileEventKind::PrivacyReceiptRecorded
                        | CaseFileEventKind::SourcePolicyChanged,
                    EventMetadata::LocalReference { .. }
                )
                | (
                    CaseFileEventKind::SourceChecked,
                    EventMetadata::SourceOutcome { .. }
                )
                | (
                    CaseFileEventKind::RecoveryRecorded,
                    EventMetadata::RecoveryOutcome { .. }
                )
        );
        if !metadata_matches {
            return Err("event kind and metadata do not match".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CareerRelation {
    Alias,
    Broader,
    Narrower,
    Related,
    Confusable,
    Evidence,
    Requirement,
    Blocker,
    Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRelation {
    Related,
    Policy,
    Lineage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphProvenance {
    UserConfirmed,
    Imported,
    PublicSource,
    ModelSuggestion,
    Migration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerGraphLink {
    pub link_id: String,
    pub subject_id: String,
    pub relation: CareerRelation,
    pub object_id: String,
    pub provenance: GraphProvenance,
    pub provenance_ref: Option<String>,
}

impl CareerGraphLink {
    pub fn validate(&self) -> Result<(), String> {
        validate_graph_link(
            &self.link_id,
            &self.subject_id,
            &self.object_id,
            self.provenance,
            self.provenance_ref.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceGraphLink {
    pub link_id: String,
    pub source_id: String,
    pub relation: SourceRelation,
    pub related_id: String,
    pub provenance: GraphProvenance,
    pub provenance_ref: Option<String>,
}

impl SourceGraphLink {
    pub fn validate(&self) -> Result<(), String> {
        validate_graph_link(
            &self.link_id,
            &self.source_id,
            &self.related_id,
            self.provenance,
            self.provenance_ref.as_deref(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAccess {
    Disabled,
    ScheduledPublic,
    UserOpened,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePolicy {
    pub source_id: String,
    pub source_class: SourceClass,
    pub access: SourceAccess,
    pub request_limit_per_hour: u16,
    pub user_review_required: bool,
    pub policy_ref: String,
    pub revision: u32,
    pub restriction_reason_code: Option<String>,
    pub reviewed_at: DateTime<Utc>,
}

impl SourcePolicy {
    pub fn validate(&self) -> Result<(), String> {
        validate_identifier("source", &self.source_id)?;
        validate_identifier("source policy", &self.policy_ref)?;
        if self.revision == 0 {
            return Err("source policy revision must be positive".to_string());
        }
        if let Some(reason) = &self.restriction_reason_code {
            validate_identifier("source restriction reason", reason)?;
        }
        match self.access {
            SourceAccess::Disabled if self.request_limit_per_hour != 0 => {
                return Err("disabled sources cannot request data".to_string());
            }
            SourceAccess::ScheduledPublic
                if self.request_limit_per_hour == 0
                    || self.request_limit_per_hour > 1_000
                    || matches!(
                        self.source_class,
                        SourceClass::RestrictedUserOpened | SourceClass::UserImport
                    ) =>
            {
                return Err("scheduled source policy is not public and bounded".to_string());
            }
            SourceAccess::UserOpened
                if self.request_limit_per_hour != 0 || !self.user_review_required =>
            {
                return Err("user-opened sources require review and no scheduler limit".to_string());
            }
            _ => {}
        }
        if matches!(self.source_class, SourceClass::RestrictedUserOpened)
            && (!matches!(
                self.access,
                SourceAccess::Disabled | SourceAccess::UserOpened
            ) || self.restriction_reason_code.is_none())
        {
            return Err(
                "restricted sources need a reason and cannot run scheduled checks".to_string(),
            );
        }
        if matches!(self.source_class, SourceClass::RestrictedPublicScheduled)
            && (matches!(self.access, SourceAccess::UserOpened)
                || (matches!(self.access, SourceAccess::ScheduledPublic)
                    && !self.user_review_required)
                || self.restriction_reason_code.is_none())
        {
            return Err(
                "restricted scheduled sources need review when active and a restriction reason"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityMetadata {
    pub schema: SchemaId,
    pub compatibility_line: u32,
    pub database_schema: u32,
    pub migration_version: i64,
}

impl CompatibilityMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SchemaId::CompatibilityV1
            || self.compatibility_line != 3
            || self.database_schema != 2
            || self.migration_version < 11
        {
            return Err("database compatibility metadata is unsupported".to_string());
        }
        Ok(())
    }
}

pub(crate) fn validate_identifier(label: &str, value: &str) -> Result<(), String> {
    let valid = !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'));
    if !valid {
        return Err(format!("{label} identifier is invalid"));
    }
    Ok(())
}

fn validate_graph_link(
    link_id: &str,
    subject_id: &str,
    object_id: &str,
    provenance: GraphProvenance,
    provenance_ref: Option<&str>,
) -> Result<(), String> {
    for (label, value) in [
        ("graph link", link_id),
        ("graph subject", subject_id),
        ("graph object", object_id),
    ] {
        validate_identifier(label, value)?;
    }
    if subject_id == object_id {
        return Err("graph links cannot point to themselves".to_string());
    }
    let needs_reference = matches!(
        provenance,
        GraphProvenance::Imported
            | GraphProvenance::PublicSource
            | GraphProvenance::ModelSuggestion
    );
    if provenance_ref.is_some() != needs_reference {
        return Err("graph provenance reference is inconsistent".to_string());
    }
    if let Some(reference) = provenance_ref {
        validate_identifier("graph provenance", reference)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_metadata_is_closed_and_bounded() {
        assert!(serde_json::from_str::<EventMetadata>(
            r#"{"kind":"local_reference","reference_id":"receipt-1","note":"private"}"#
        )
        .is_err());
        assert!(EventMetadata::LocalReference {
            reference_id: "x".repeat(129),
        }
        .validate()
        .is_err());
    }

    #[test]
    fn event_kind_must_match_typed_metadata() {
        let event = CaseFileEventInput {
            case_file_id: "case-1".to_string(),
            kind: CaseFileEventKind::StatusChanged,
            origin: EventOrigin::User,
            user_action: true,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::LocalReference {
                reference_id: "evidence-1".to_string(),
            },
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn case_history_cannot_be_labeled_as_public_data() {
        let event = CaseFileEventInput {
            case_file_id: "case-1".to_string(),
            kind: CaseFileEventKind::CaseCreated,
            origin: EventOrigin::System,
            user_action: false,
            privacy_labels: [PrivacyLabel::PublicDataOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::Empty,
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn graph_links_reject_self_edges_and_unowned_provenance() {
        let link = CareerGraphLink {
            link_id: "link-1".to_string(),
            subject_id: "evidence-1".to_string(),
            relation: CareerRelation::Evidence,
            object_id: "evidence-1".to_string(),
            provenance: GraphProvenance::PublicSource,
            provenance_ref: None,
        };

        assert!(link.validate().is_err());
    }

    #[test]
    fn compatibility_metadata_keeps_schema_and_migration_versions_distinct() {
        let metadata = CompatibilityMetadata {
            schema: SchemaId::CompatibilityV1,
            compatibility_line: 3,
            database_schema: 2,
            migration_version: 11,
        };

        assert!(metadata.validate().is_ok());
    }
}
