// Owns durable v3 foundation persistence and storage-safe read models.

use anyhow::{anyhow, Result};
use chrono::Utc;
use jobsentinel_domain::{
    v3_foundation::{
        CareerGraphLink, CareerRelation, CaseFile, CaseFileEvent, CaseFileEventInput,
        CaseFileEventKind, CompatibilityMetadata, SourceGraphLink, SourcePolicy, SourceRelation,
    },
    v3_manifests::PrivacyReceipt,
};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::{sqlite_time::parse_sqlite_datetime, Database};

const MAX_RECEIPT_BYTES: usize = 8 * 1024;

pub fn error_kind(error: &anyhow::Error) -> &'static str {
    error
        .downcast_ref::<sqlx::Error>()
        .map(crate::database_error_kind)
        .unwrap_or("invalid")
}

#[derive(FromRow)]
struct CaseFileRow {
    case_file_id: String,
    job_hash: Option<String>,
    created_at: String,
}

#[derive(FromRow)]
struct EventRow {
    event_id: String,
    case_file_id: String,
    event_kind: String,
    origin: String,
    user_action: bool,
    local_only: bool,
    sensitive: bool,
    metadata_json: String,
    created_at: String,
}

#[derive(FromRow)]
pub(crate) struct SourcePolicyRow {
    source_id: String,
    source_class: String,
    access: String,
    request_limit_per_hour: i64,
    user_review_required: bool,
    policy_ref: String,
    revision: i64,
    restriction_reason_code: Option<String>,
    reviewed_at: String,
}

impl Database {
    pub async fn ensure_case_file(&self, job_hash: &str) -> Result<CaseFile> {
        if job_hash.is_empty() || job_hash.len() > 128 {
            return Err(anyhow!("invalid job reference"));
        }
        sqlx::query(
            "INSERT INTO opportunity_case_files (case_file_id, job_hash, created_at)
             SELECT ?, ?, ?
             WHERE EXISTS (SELECT 1 FROM jobs WHERE hash = ?)
             ON CONFLICT(job_hash) DO NOTHING",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(job_hash)
        .bind(Utc::now().to_rfc3339())
        .bind(job_hash)
        .execute(self.pool())
        .await?;
        let row = sqlx::query_as::<_, CaseFileRow>(
            "SELECT case_file_id, job_hash, created_at
             FROM opportunity_case_files
             WHERE job_hash = ?",
        )
        .bind(job_hash)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| anyhow!("job reference does not exist"))?;
        case_file_from_row(row)
    }

    pub async fn append_case_file_event(
        &self,
        input: &CaseFileEventInput,
    ) -> Result<CaseFileEvent> {
        input
            .validate()
            .map_err(|_| anyhow!("invalid case event"))?;
        if input.kind == CaseFileEventKind::EvidenceLinked {
            return Err(anyhow!("case evidence requires atomic confirmation"));
        }
        let event_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let metadata_json = input
            .metadata
            .to_json()
            .map_err(|_| anyhow!("invalid case event metadata"))?;
        sqlx::query(
            "INSERT INTO v3_job_events (
                event_id, case_file_id, event_kind, origin, user_action,
                local_only, sensitive, metadata_json, created_at
             ) VALUES (?, ?, ?, ?, ?, 1, 1, ?, ?)",
        )
        .bind(&event_id)
        .bind(&input.case_file_id)
        .bind(enum_text(input.kind)?)
        .bind(enum_text(input.origin)?)
        .bind(input.user_action)
        .bind(&metadata_json)
        .bind(created_at.to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(CaseFileEvent {
            event_id,
            case_file_id: input.case_file_id.clone(),
            kind: input.kind,
            origin: input.origin,
            user_action: input.user_action,
            privacy_labels: input.privacy_labels,
            metadata: input.metadata.clone(),
            created_at,
        })
    }

    pub async fn list_case_file_events(&self, case_file_id: &str) -> Result<Vec<CaseFileEvent>> {
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT event_id, case_file_id, event_kind, origin, user_action,
                    local_only, sensitive, metadata_json, created_at
             FROM v3_job_events
             WHERE case_file_id = ?
             ORDER BY created_at, event_id",
        )
        .bind(case_file_id)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(event_from_row).collect()
    }

    pub async fn insert_career_graph_link(&self, link: &CareerGraphLink) -> Result<()> {
        link.validate()
            .map_err(|_| anyhow!("invalid career graph link"))?;
        if link.relation == CareerRelation::Evidence {
            return Err(anyhow!("case evidence requires explicit confirmation"));
        }
        sqlx::query(
            "INSERT INTO career_graph_links (
                link_id, subject_id, relation, object_id,
                provenance, provenance_ref, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&link.link_id)
        .bind(&link.subject_id)
        .bind(enum_text(link.relation)?)
        .bind(&link.object_id)
        .bind(enum_text(link.provenance)?)
        .bind(&link.provenance_ref)
        .bind(Utc::now().to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn insert_source_graph_link(&self, link: &SourceGraphLink) -> Result<()> {
        link.validate()
            .map_err(|_| anyhow!("invalid source graph link"))?;
        if link.relation == SourceRelation::Lineage {
            return Err(anyhow!("source lineage is owned by source manifests"));
        }
        sqlx::query(
            "INSERT INTO source_graph_links (
                link_id, source_id, relation, related_id,
                provenance, provenance_ref, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&link.link_id)
        .bind(&link.source_id)
        .bind(enum_text(link.relation)?)
        .bind(&link.related_id)
        .bind(enum_text(link.provenance)?)
        .bind(&link.provenance_ref)
        .bind(Utc::now().to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn store_privacy_receipt(&self, receipt: &PrivacyReceipt) -> Result<()> {
        receipt
            .validate()
            .map_err(|_| anyhow!("invalid privacy receipt"))?;
        let receipt_json =
            serde_json::to_string(receipt).map_err(|_| anyhow!("invalid privacy receipt"))?;
        if receipt_json.len() > MAX_RECEIPT_BYTES {
            return Err(anyhow!("privacy receipt exceeds the local limit"));
        }
        sqlx::query(
            "INSERT INTO v3_privacy_receipts (
                receipt_id, schema, receipt_json, stored_locally,
                data_left_device, created_at
             ) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&receipt.receipt_id)
        .bind(enum_text(receipt.schema)?)
        .bind(receipt_json)
        .bind(receipt.stored_locally)
        .bind(receipt.data_left_device)
        .bind(Utc::now().to_rfc3339())
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn get_privacy_receipt(&self, receipt_id: &str) -> Result<Option<PrivacyReceipt>> {
        let receipt_json: Option<String> =
            sqlx::query_scalar("SELECT receipt_json FROM v3_privacy_receipts WHERE receipt_id = ?")
                .bind(receipt_id)
                .fetch_optional(self.pool())
                .await?;
        receipt_json
            .map(|json| {
                let receipt: PrivacyReceipt =
                    serde_json::from_str(&json).map_err(|_| anyhow!("invalid stored receipt"))?;
                receipt
                    .validate()
                    .map_err(|_| anyhow!("invalid stored receipt"))?;
                Ok(receipt)
            })
            .transpose()
    }

    pub async fn upsert_source_policy(&self, policy: &SourcePolicy) -> Result<SourcePolicy> {
        policy
            .validate()
            .map_err(|_| anyhow!("invalid source policy"))?;
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO v3_source_policies (
                source_id, source_class, access, request_limit_per_hour,
                user_review_required, policy_ref, revision,
                restriction_reason_code, reviewed_at, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_id) DO UPDATE SET
                source_class = excluded.source_class,
                access = excluded.access,
                request_limit_per_hour = excluded.request_limit_per_hour,
                user_review_required = excluded.user_review_required,
                policy_ref = excluded.policy_ref,
                revision = excluded.revision,
                restriction_reason_code = excluded.restriction_reason_code,
                reviewed_at = excluded.reviewed_at,
                updated_at = excluded.updated_at
             WHERE excluded.revision > v3_source_policies.revision",
        )
        .bind(&policy.source_id)
        .bind(enum_text(policy.source_class)?)
        .bind(enum_text(policy.access)?)
        .bind(i64::from(policy.request_limit_per_hour))
        .bind(policy.user_review_required)
        .bind(&policy.policy_ref)
        .bind(i64::from(policy.revision))
        .bind(&policy.restriction_reason_code)
        .bind(policy.reviewed_at.to_rfc3339())
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await?;
        let stored = self
            .get_source_policy(&policy.source_id)
            .await?
            .ok_or_else(|| anyhow!("source policy was not stored"))?;
        if result.rows_affected() == 0 && stored != *policy {
            return Err(anyhow!("source policy revision conflict"));
        }
        Ok(stored)
    }

    pub async fn get_source_policy(&self, source_id: &str) -> Result<Option<SourcePolicy>> {
        sqlx::query_as::<_, SourcePolicyRow>(
            "SELECT source_id, source_class, access, request_limit_per_hour,
                    user_review_required, policy_ref, revision,
                    restriction_reason_code, reviewed_at
             FROM v3_source_policies
             WHERE source_id = ?",
        )
        .bind(source_id)
        .fetch_optional(self.pool())
        .await?
        .map(source_policy_from_row)
        .transpose()
    }

    pub(crate) async fn get_source_policy_revision(
        &self,
        source_id: &str,
        revision: i64,
    ) -> Result<SourcePolicy> {
        let row = sqlx::query_as::<_, SourcePolicyRow>(
            "SELECT source_id, source_class, access, request_limit_per_hour,
                    user_review_required, policy_ref, revision,
                    restriction_reason_code, reviewed_at
             FROM v3_source_policy_ledger
             WHERE source_id = ? AND revision = ?",
        )
        .bind(source_id)
        .bind(revision)
        .fetch_one(self.pool())
        .await?;
        source_policy_from_row(row)
    }

    pub async fn read_compatibility_metadata(&self) -> Result<CompatibilityMetadata> {
        let row = sqlx::query(
            "SELECT schema, compatibility_line, database_schema, migration_version
             FROM v3_compatibility_metadata
             WHERE singleton = 1",
        )
        .fetch_one(self.pool())
        .await?;
        let metadata = CompatibilityMetadata {
            schema: parse_enum(row.try_get("schema")?)?,
            compatibility_line: u32::try_from(row.try_get::<i64, _>("compatibility_line")?)
                .map_err(|_| anyhow!("invalid stored compatibility metadata"))?,
            database_schema: u32::try_from(row.try_get::<i64, _>("database_schema")?)
                .map_err(|_| anyhow!("invalid stored compatibility metadata"))?,
            migration_version: row.try_get("migration_version")?,
        };
        metadata
            .validate()
            .map_err(|_| anyhow!("invalid stored compatibility metadata"))?;
        Ok(metadata)
    }
}

fn case_file_from_row(row: CaseFileRow) -> Result<CaseFile> {
    Ok(CaseFile {
        case_file_id: row.case_file_id,
        job_hash: row.job_hash,
        created_at: parse_sqlite_datetime(&row.created_at)?,
    })
}

fn event_from_row(row: EventRow) -> Result<CaseFileEvent> {
    if !row.local_only || !row.sensitive {
        return Err(anyhow!("invalid stored event privacy"));
    }
    Ok(CaseFileEvent {
        event_id: row.event_id,
        case_file_id: row.case_file_id,
        kind: parse_enum(&row.event_kind)?,
        origin: parse_enum(&row.origin)?,
        user_action: row.user_action,
        privacy_labels: [
            jobsentinel_domain::v3_manifests::PrivacyLabel::LocalOnly,
            jobsentinel_domain::v3_manifests::PrivacyLabel::Sensitive,
        ],
        metadata: serde_json::from_str(&row.metadata_json)
            .map_err(|_| anyhow!("invalid stored event metadata"))?,
        created_at: parse_sqlite_datetime(&row.created_at)?,
    })
}

pub(crate) fn source_policy_from_row(row: SourcePolicyRow) -> Result<SourcePolicy> {
    let policy = SourcePolicy {
        source_id: row.source_id,
        source_class: parse_enum(&row.source_class)?,
        access: parse_enum(&row.access)?,
        request_limit_per_hour: u16::try_from(row.request_limit_per_hour)
            .map_err(|_| anyhow!("invalid stored source policy"))?,
        user_review_required: row.user_review_required,
        policy_ref: row.policy_ref,
        revision: u32::try_from(row.revision)
            .map_err(|_| anyhow!("invalid stored source policy"))?,
        restriction_reason_code: row.restriction_reason_code,
        reviewed_at: parse_sqlite_datetime(&row.reviewed_at)?,
    };
    policy
        .validate()
        .map_err(|_| anyhow!("invalid stored source policy"))?;
    Ok(policy)
}

pub(crate) fn enum_text(value: impl Serialize) -> Result<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or_else(|| anyhow!("invalid typed storage value"))
}

pub(crate) fn parse_enum<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| anyhow!("invalid stored enum"))
}

mod case_evidence;
pub use case_evidence::CaseRequirementEvidenceContext;
mod saved_match_evidence;
pub use saved_match_evidence::{SavedMatchConfirmedEvidence, SavedMatchEvidenceConfirmation};
mod evidence_packets;
pub use evidence_packets::{
    EvidenceBoundPacketClaimRecord, EvidenceBoundPacketClaimsRead, EvidencePacketBoundary,
    NewEvidenceBoundPacketClaim,
};
mod opportunity_case;
pub use opportunity_case::{
    EmployerHistoryRead, OpportunityCaseRead, OpportunityCaseTimelineRecord,
};

#[cfg(test)]
#[path = "v3_foundation/case_evidence_tests.rs"]
mod case_evidence_tests;

#[cfg(test)]
#[path = "v3_foundation/evidence_packet_tests.rs"]
mod evidence_packet_tests;

#[cfg(test)]
#[path = "v3_foundation/saved_match_evidence_tests.rs"]
mod saved_match_evidence_tests;

#[cfg(test)]
#[path = "v3_foundation/opportunity_case_tests.rs"]
mod opportunity_case_tests;

#[cfg(test)]
mod tests;
