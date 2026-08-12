//! Defines and validates the typed records stored by the reviewed pack-task ledger.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use jobsentinel_domain::v3_manifests::{AgentTaskKind, DataCategory, PrivacyLabel};
use serde::Serialize;
use sqlx::FromRow;

use crate::v3_foundation::enum_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackTaskStatus {
    Pending,
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTaskContext {
    pub run_id: String,
    pub approval_reference: String,
    pub publisher_key_id: String,
    pub pack_id: String,
    pub release_sequence: u64,
    pub signed_release_sha256: String,
    pub stream_generation: u64,
    pub task_kind: AgentTaskKind,
    pub task_id: String,
    pub input_sha256: String,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub data_categories: Vec<DataCategory>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTaskRun {
    pub context: PackTaskContext,
    pub status: PackTaskStatus,
    pub receipt_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackTaskReconciliation {
    pub expired_pending: u64,
    pub interrupted_started: u64,
}

#[derive(FromRow)]
pub(super) struct PackTaskRow {
    run_id: String,
    approval_reference: String,
    publisher_key_id: String,
    pack_id: String,
    release_sequence: i64,
    signed_release_sha256: String,
    stream_generation: i64,
    task_kind: String,
    task_id: String,
    input_sha256: String,
    privacy_labels_json: String,
    data_categories_json: String,
    status: String,
    receipt_id: Option<String>,
    created_at: String,
    expires_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
}

impl TryFrom<PackTaskRow> for PackTaskRun {
    type Error = anyhow::Error;

    fn try_from(row: PackTaskRow) -> Result<Self> {
        let context = PackTaskContext {
            run_id: row.run_id,
            approval_reference: row.approval_reference,
            publisher_key_id: row.publisher_key_id,
            pack_id: row.pack_id,
            release_sequence: u64::try_from(row.release_sequence).map_err(|_| invalid())?,
            signed_release_sha256: row.signed_release_sha256,
            stream_generation: u64::try_from(row.stream_generation).map_err(|_| invalid())?,
            task_kind: parse_enum(&row.task_kind)?,
            task_id: row.task_id,
            input_sha256: row.input_sha256,
            privacy_labels: parse_json(&row.privacy_labels_json)?,
            data_categories: parse_json(&row.data_categories_json)?,
            created_at: parse_time(&row.created_at)?,
            expires_at: parse_time(&row.expires_at)?,
        };
        validate_context(&context)?;
        Ok(Self {
            context,
            status: parse_status(&row.status)?,
            receipt_id: row.receipt_id,
            started_at: row.started_at.as_deref().map(parse_time).transpose()?,
            completed_at: row.completed_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

pub(super) fn validate_context(context: &PackTaskContext) -> Result<()> {
    for value in [
        &context.run_id,
        &context.approval_reference,
        &context.publisher_key_id,
        &context.pack_id,
        &context.task_id,
    ] {
        validate_identifier(value)?;
    }
    if !is_sha256(&context.signed_release_sha256)
        || !is_sha256(&context.input_sha256)
        || context.release_sequence == 0
        || !matches!(
            context.task_kind,
            AgentTaskKind::EvidenceReview | AgentTaskKind::DraftPacket
        )
        || context.expires_at <= context.created_at
    {
        return Err(invalid());
    }
    validate_canonical_values(&context.privacy_labels)?;
    validate_canonical_values(&context.data_categories)?;
    Ok(())
}

pub(super) fn validate_identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_canonical_values<T: Serialize>(values: &[T]) -> Result<()> {
    let json = canonical_json(values)?;
    let encoded: Vec<String> = serde_json::from_str(&json).map_err(|_| invalid())?;
    if encoded.is_empty() || encoded.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn canonical_json<T: Serialize>(values: &[T]) -> Result<String> {
    serde_json::to_string(values).map_err(|_| invalid())
}

pub(super) fn task_kind_text(kind: AgentTaskKind) -> Result<String> {
    enum_text(kind)
}

fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|_| invalid())
}

fn parse_json<T: serde::de::DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(|_| invalid())
}

fn parse_status(value: &str) -> Result<PackTaskStatus> {
    match value {
        "pending" => Ok(PackTaskStatus::Pending),
        "started" => Ok(PackTaskStatus::Started),
        "succeeded" => Ok(PackTaskStatus::Succeeded),
        "failed" => Ok(PackTaskStatus::Failed),
        "cancelled" => Ok(PackTaskStatus::Cancelled),
        _ => Err(invalid()),
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| invalid())
}

pub(super) fn time_text(time: DateTime<Utc>) -> String {
    time.to_rfc3339()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) fn invalid() -> anyhow::Error {
    anyhow!("pack task input or state is invalid")
}
