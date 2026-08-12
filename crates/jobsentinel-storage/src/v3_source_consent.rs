use std::{error::Error, fmt};

use anyhow::{anyhow, Result};
use chrono::Utc;
use jobsentinel_domain::v3_source_consent::{
    SourceConsentContext, SourceConsentDecision, SourceConsentEvent, SourceConsentOperation,
    SourceConsentReviewReason, SourceConsentStatus, SourcePolicyLedgerEntry,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    sqlite_time::parse_sqlite_datetime,
    v3_foundation::{enum_text, parse_enum, source_policy_from_row, SourcePolicyRow},
    Database,
};

const MAX_HISTORY_RESULTS: u16 = 100;

#[derive(Debug)]
enum ConsentStorageError {
    Invalid,
    Conflict,
    Corrupt,
}

impl fmt::Display for ConsentStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "invalid source consent",
            Self::Conflict => "source consent state changed",
            Self::Corrupt => "stored source consent is invalid",
        })
    }
}

impl Error for ConsentStorageError {}

pub fn error_kind(error: &anyhow::Error) -> Option<&'static str> {
    match error.downcast_ref::<ConsentStorageError>()? {
        ConsentStorageError::Invalid => Some("invalid"),
        ConsentStorageError::Conflict => Some("conflict"),
        ConsentStorageError::Corrupt => Some("corrupt"),
    }
}

#[derive(FromRow)]
struct SourcePolicyLedgerRow {
    sequence: i64,
    #[sqlx(flatten)]
    policy: SourcePolicyRow,
    recorded_at: String,
}

#[derive(FromRow)]
struct SourceConsentEventRow {
    sequence: i64,
    event_id: String,
    previous_event_id: Option<String>,
    source_id: String,
    operation: String,
    warning_version: i64,
    behavior_revision: i64,
    policy_ref: String,
    policy_revision: i64,
    source_class: String,
    data_categories_json: String,
    destination_sha256: String,
    request_sha256: String,
    decision: String,
    recorded_at: String,
}

impl Database {
    pub async fn source_policy_history(
        &self,
        source_id: &str,
        limit: u16,
    ) -> Result<Vec<SourcePolicyLedgerEntry>> {
        validate_history_query(source_id, limit)?;
        let rows = sqlx::query_as::<_, SourcePolicyLedgerRow>(
            "SELECT sequence, source_id, source_class, access,
                    request_limit_per_hour, user_review_required, policy_ref,
                    revision, restriction_reason_code, reviewed_at, recorded_at
             FROM v3_source_policy_ledger
             WHERE source_id = ?
             ORDER BY sequence DESC
             LIMIT ?",
        )
        .bind(source_id)
        .bind(i64::from(limit))
        .fetch_all(self.pool())
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(SourcePolicyLedgerEntry {
                    sequence: row.sequence,
                    policy: source_policy_from_row(row.policy).map_err(|_| corrupt_consent())?,
                    recorded_at: parse_sqlite_datetime(&row.recorded_at)
                        .map_err(|_| corrupt_consent())?,
                })
            })
            .collect()
    }

    pub async fn grant_source_consent(
        &self,
        context: &SourceConsentContext,
        expected_latest_event_id: Option<&str>,
    ) -> Result<SourceConsentEvent> {
        context.validate().map_err(|_| invalid_consent())?;
        let policy = self
            .get_source_policy(&context.source_id)
            .await
            .map_err(map_policy_read_error)?
            .ok_or_else(invalid_consent)?;
        if !context.matches_policy(&policy) {
            return Err(invalid_consent());
        }
        let operation = enum_text(context.operation)?;
        let latest_event_id = self
            .latest_source_consent_id(&context.source_id, &operation)
            .await?;
        if latest_event_id.as_deref() != expected_latest_event_id {
            return Err(conflicting_consent());
        }
        let event_id = Uuid::new_v4().to_string();
        let recorded_at = Utc::now();
        let data_categories_json =
            serde_json::to_string(&context.data_categories).map_err(|_| invalid_consent())?;
        let sequence = sqlx::query_scalar(
            "INSERT INTO v3_source_consent_events (
                event_id, previous_event_id, source_id, operation,
                warning_version, behavior_revision, policy_ref, policy_revision,
                source_class, data_categories_json, destination_sha256,
                request_sha256, decision, recorded_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'granted', ?)
             RETURNING sequence",
        )
        .bind(&event_id)
        .bind(expected_latest_event_id)
        .bind(&context.source_id)
        .bind(&operation)
        .bind(i64::from(context.warning_version))
        .bind(i64::from(context.behavior_revision))
        .bind(&context.policy_ref)
        .bind(i64::from(context.policy_revision))
        .bind(enum_text(context.source_class)?)
        .bind(data_categories_json)
        .bind(&context.destination_sha256)
        .bind(&context.request_sha256)
        .bind(recorded_at.to_rfc3339())
        .fetch_one(self.pool())
        .await
        .map_err(map_consent_write_error)?;
        Ok(SourceConsentEvent {
            sequence,
            event_id,
            previous_event_id: expected_latest_event_id.map(str::to_string),
            context: context.clone(),
            decision: SourceConsentDecision::Granted,
            recorded_at,
        })
    }

    pub async fn revoke_source_consent(
        &self,
        source_id: &str,
        operation: SourceConsentOperation,
    ) -> Result<bool> {
        validate_history_query(source_id, 1)?;
        let operation = enum_text(operation)?;
        let result = sqlx::query(
            "INSERT INTO v3_source_consent_events (
                event_id, previous_event_id, source_id, operation,
                warning_version, behavior_revision, policy_ref, policy_revision,
                source_class, data_categories_json, destination_sha256,
                request_sha256, decision, recorded_at
             )
             SELECT ?, event_id, source_id, operation, warning_version,
                    behavior_revision, policy_ref, policy_revision, source_class,
                    data_categories_json, destination_sha256, request_sha256,
                    'revoked', ?
             FROM v3_source_consent_events
             WHERE sequence = (
                SELECT sequence
                FROM v3_source_consent_events
                WHERE source_id = ? AND operation = ?
                ORDER BY sequence DESC
                LIMIT 1
             )
             AND decision = 'granted'",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(Utc::now().to_rfc3339())
        .bind(source_id)
        .bind(operation)
        .execute(self.pool())
        .await
        .map_err(map_consent_write_error)?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn source_consent_status(
        &self,
        current: &SourceConsentContext,
    ) -> Result<SourceConsentStatus> {
        current.validate().map_err(|_| invalid_consent())?;
        let latest = self
            .latest_source_consent(&current.source_id, current.operation)
            .await?;
        let latest_event_id = latest.as_ref().map(|event| event.event_id.clone());
        let policy_matches = self
            .get_source_policy(&current.source_id)
            .await
            .map_err(map_policy_read_error)?
            .is_some_and(|policy| current.matches_policy(&policy));
        Ok(match latest {
            _ if !policy_matches => {
                review_required(SourceConsentReviewReason::ContextChanged, latest_event_id)
            }
            None => review_required(SourceConsentReviewReason::Missing, None),
            Some(event) if event.decision == SourceConsentDecision::Revoked => {
                review_required(SourceConsentReviewReason::Revoked, latest_event_id)
            }
            Some(event) if event.context.matches(current) => SourceConsentStatus::Remembered {
                event_id: event.event_id,
            },
            Some(_) => review_required(SourceConsentReviewReason::ContextChanged, latest_event_id),
        })
    }

    pub async fn source_consent_history(
        &self,
        source_id: &str,
        limit: u16,
    ) -> Result<Vec<SourceConsentEvent>> {
        validate_history_query(source_id, limit)?;
        sqlx::query_as::<_, SourceConsentEventRow>(
            "SELECT sequence, event_id, previous_event_id, source_id, operation,
                    warning_version, behavior_revision, policy_ref,
                    policy_revision, source_class, data_categories_json,
                    destination_sha256, request_sha256, decision, recorded_at
             FROM v3_source_consent_events
             WHERE source_id = ?
             ORDER BY sequence DESC
             LIMIT ?",
        )
        .bind(source_id)
        .bind(i64::from(limit))
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(source_consent_from_row)
        .collect()
    }

    async fn latest_source_consent(
        &self,
        source_id: &str,
        operation: SourceConsentOperation,
    ) -> Result<Option<SourceConsentEvent>> {
        sqlx::query_as::<_, SourceConsentEventRow>(
            "SELECT sequence, event_id, previous_event_id, source_id, operation,
                    warning_version, behavior_revision, policy_ref,
                    policy_revision, source_class, data_categories_json,
                    destination_sha256, request_sha256, decision, recorded_at
             FROM v3_source_consent_events
             WHERE source_id = ? AND operation = ?
             ORDER BY sequence DESC
             LIMIT 1",
        )
        .bind(source_id)
        .bind(enum_text(operation)?)
        .fetch_optional(self.pool())
        .await?
        .map(source_consent_from_row)
        .transpose()
    }

    async fn latest_source_consent_id(
        &self,
        source_id: &str,
        operation: &str,
    ) -> Result<Option<String>> {
        sqlx::query_scalar(
            "SELECT event_id
             FROM v3_source_consent_events
             WHERE source_id = ? AND operation = ?
             ORDER BY sequence DESC
             LIMIT 1",
        )
        .bind(source_id)
        .bind(operation)
        .fetch_optional(self.pool())
        .await
        .map_err(Into::into)
    }
}

fn review_required(
    reason: SourceConsentReviewReason,
    latest_event_id: Option<String>,
) -> SourceConsentStatus {
    SourceConsentStatus::ReviewRequired {
        reason,
        latest_event_id,
    }
}

fn validate_history_query(source_id: &str, limit: u16) -> Result<()> {
    if source_id.is_empty()
        || source_id.len() > 128
        || !source_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
        || !(1..=MAX_HISTORY_RESULTS).contains(&limit)
    {
        return Err(invalid_consent());
    }
    Ok(())
}

fn source_consent_from_row(row: SourceConsentEventRow) -> Result<SourceConsentEvent> {
    let context = SourceConsentContext {
        source_id: row.source_id,
        operation: parse_enum(&row.operation).map_err(|_| corrupt_consent())?,
        warning_version: u32::try_from(row.warning_version).map_err(|_| corrupt_consent())?,
        behavior_revision: u32::try_from(row.behavior_revision).map_err(|_| corrupt_consent())?,
        policy_ref: row.policy_ref,
        policy_revision: u32::try_from(row.policy_revision).map_err(|_| corrupt_consent())?,
        source_class: parse_enum(&row.source_class).map_err(|_| corrupt_consent())?,
        data_categories: serde_json::from_str(&row.data_categories_json)
            .map_err(|_| corrupt_consent())?,
        destination_sha256: row.destination_sha256,
        request_sha256: row.request_sha256,
    };
    context.validate().map_err(|_| corrupt_consent())?;
    Ok(SourceConsentEvent {
        sequence: row.sequence,
        event_id: row.event_id,
        previous_event_id: row.previous_event_id,
        context,
        decision: parse_enum(&row.decision).map_err(|_| corrupt_consent())?,
        recorded_at: parse_sqlite_datetime(&row.recorded_at).map_err(|_| corrupt_consent())?,
    })
}

fn invalid_consent() -> anyhow::Error {
    anyhow!(ConsentStorageError::Invalid)
}

fn conflicting_consent() -> anyhow::Error {
    anyhow!(ConsentStorageError::Conflict)
}

fn corrupt_consent() -> anyhow::Error {
    anyhow!(ConsentStorageError::Corrupt)
}

fn map_policy_read_error(error: anyhow::Error) -> anyhow::Error {
    if error.downcast_ref::<sqlx::Error>().is_some() {
        error
    } else {
        corrupt_consent()
    }
}

fn map_consent_write_error(error: sqlx::Error) -> anyhow::Error {
    if error.as_database_error().is_some_and(|error| {
        let message = error.message();
        message.contains("source consent state changed")
            || message.contains("source consent must match current policy")
    }) {
        conflicting_consent()
    } else {
        error.into()
    }
}

#[cfg(test)]
#[path = "v3_source_consent_tests.rs"]
mod tests;
