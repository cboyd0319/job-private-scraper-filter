//! Reads one bounded, privacy-safe local opportunity case and employer-history summary.

use super::*;
use chrono::{DateTime, Utc};

#[path = "opportunity_case/employer_history.rs"]
mod employer_history;

#[derive(Debug, Clone, PartialEq)]
pub struct OpportunityCaseRead {
    pub job_hash: String,
    pub title: String,
    pub company: String,
    pub job_url: String,
    pub location: Option<String>,
    pub remote: Option<bool>,
    pub times_seen: i64,
    pub first_seen_at: DateTime<Utc>,
    pub source_name: String,
    pub last_seen_at: DateTime<Utc>,
    pub repost_count: i64,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub currency: Option<String>,
    pub employer_history: EmployerHistoryRead,
    pub posting_risk_score: Option<f64>,
    pub posting_risk_reasons: Vec<String>,
    pub application_status: Option<String>,
    pub has_contact: bool,
    pub upcoming_interview_count: u32,
    pub completed_interview_count: u32,
    pub offer_status: Option<String>,
    pub outcome_status: Option<String>,
    pub confirmed_evidence_count: u32,
    pub current_packet_count: u32,
    pub stale_packet_count: u32,
    pub timeline: Vec<OpportunityCaseTimelineRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmployerHistoryRead {
    pub saved_job_count: u32,
    pub application_count: u32,
    pub interview_count: u32,
    pub offer_count: u32,
    pub terminal_outcome_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityCaseTimelineRecord {
    pub at: DateTime<Utc>,
    pub kind: String,
}

#[derive(FromRow)]
struct CaseRow {
    case_file_id: String,
    job_hash: String,
    case_created_at: String,
    title: String,
    company: String,
    job_url: String,
    location: Option<String>,
    remote: Option<bool>,
    times_seen: i64,
    first_seen_at: String,
    source_name: String,
    last_seen_at: String,
    repost_count: i64,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
    currency: Option<String>,
    posting_risk_score: Option<f64>,
    posting_risk_reasons: Option<String>,
    job_revision: String,
}

#[derive(FromRow)]
struct ApplicationRow {
    id: i64,
    status: String,
    has_contact: bool,
}

#[derive(FromRow)]
struct CountRow {
    count: i64,
}

#[derive(FromRow)]
struct OfferRow {
    accepted: Option<bool>,
}

#[derive(FromRow)]
struct PacketRow {
    resume_revision: String,
    job_revision: String,
    current_resume_revision: Option<String>,
}

#[derive(FromRow)]
struct CaseEventRow {
    event_kind: String,
    metadata_json: String,
    created_at: String,
}

#[derive(FromRow)]
struct ApplicationEventRow {
    event_type: String,
    created_at: String,
}

impl Database {
    /// Reads one local, privacy-safe opportunity case against a single SQLite snapshot.
    pub async fn read_opportunity_case(&self, job_hash: &str) -> Result<OpportunityCaseRead> {
        if !valid_job_hash(job_hash) {
            return Err(anyhow!("invalid job reference"));
        }
        let mut transaction = self.pool().begin().await?;
        let case = sqlx::query_as::<_, CaseRow>(
            "SELECT c.case_file_id, c.job_hash, c.created_at AS case_created_at,
                    j.title, j.company, j.url AS job_url, j.location, j.remote, j.times_seen,
                    COALESCE(j.first_seen, j.created_at) AS first_seen_at,
                    j.source AS source_name, j.last_seen AS last_seen_at, j.repost_count,
                    j.salary_min, j.salary_max, j.currency,
                    j.ghost_score AS posting_risk_score,
                    j.ghost_reasons AS posting_risk_reasons, j.updated_at AS job_revision
             FROM opportunity_case_files AS c
             JOIN jobs AS j ON j.hash = c.job_hash
             WHERE c.job_hash = ?",
        )
        .bind(job_hash)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| anyhow!("job reference does not exist"))?;

        let application = sqlx::query_as::<_, ApplicationRow>(
            "SELECT id, status,
                    (last_contact IS NOT NULL OR recruiter_name IS NOT NULL
                     OR recruiter_email IS NOT NULL OR recruiter_phone IS NOT NULL) AS has_contact
             FROM applications WHERE job_hash = ?",
        )
        .bind(job_hash)
        .fetch_optional(&mut *transaction)
        .await?;
        let (upcoming_interview_count, completed_interview_count, offer_status) = if let Some(
            application,
        ) =
            &application
        {
            let upcoming = count_for_application(
                    &mut transaction,
                    "SELECT COUNT(*) AS count FROM interviews WHERE application_id = ? AND completed = 0",
                    application.id,
                )
                .await?;
            let completed = count_for_application(
                    &mut transaction,
                    "SELECT COUNT(*) AS count FROM interviews WHERE application_id = ? AND completed = 1",
                    application.id,
                )
                .await?;
            let offer = sqlx::query_as::<_, OfferRow>(
                "SELECT accepted FROM offers WHERE application_id = ?",
            )
            .bind(application.id)
            .fetch_optional(&mut *transaction)
            .await?
            .map(|offer| match offer.accepted {
                Some(true) => "accepted".to_string(),
                Some(false) => "declined".to_string(),
                None => "pending".to_string(),
            })
            .or_else(|| match application.status.as_str() {
                "offer_received" => Some("pending".to_string()),
                "offer_accepted" => Some("accepted".to_string()),
                "offer_rejected" => Some("declined".to_string()),
                _ => None,
            });
            (upcoming, completed, offer)
        } else {
            (0, 0, None)
        };

        let confirmed_evidence_count =
            count_case_evidence(&mut transaction, &case.case_file_id).await?;
        let (current_packet_count, stale_packet_count) =
            packet_counts(&mut transaction, &case.case_file_id, &case.job_revision).await?;
        let employer_history =
            employer_history::read_employer_history(&mut transaction, &case.company).await?;
        let mut timeline = vec![OpportunityCaseTimelineRecord {
            at: parse_sqlite_datetime(&case.case_created_at)?,
            kind: "case_created".to_string(),
        }];
        timeline.extend(case_timeline(&mut transaction, &case.case_file_id).await?);
        if let Some(application) = &application {
            timeline.extend(application_timeline(&mut transaction, application.id).await?);
        }
        timeline.sort_by_key(|event| event.at);
        transaction.commit().await?;

        Ok(OpportunityCaseRead {
            job_hash: case.job_hash,
            title: case.title,
            company: case.company,
            job_url: case.job_url,
            location: case.location,
            remote: case.remote,
            times_seen: case.times_seen,
            first_seen_at: parse_sqlite_datetime(&case.first_seen_at)?,
            source_name: case.source_name,
            last_seen_at: parse_sqlite_datetime(&case.last_seen_at)?,
            repost_count: case.repost_count,
            salary_min: case.salary_min,
            salary_max: case.salary_max,
            currency: case.currency,
            employer_history,
            posting_risk_score: case.posting_risk_score,
            posting_risk_reasons: sanitized_risk_reasons(case.posting_risk_reasons.as_deref()),
            application_status: application
                .as_ref()
                .map(|application| application.status.clone()),
            has_contact: application
                .as_ref()
                .is_some_and(|application| application.has_contact),
            upcoming_interview_count,
            completed_interview_count,
            offer_status,
            outcome_status: application
                .as_ref()
                .and_then(|application| terminal_outcome(&application.status).map(str::to_string)),
            confirmed_evidence_count,
            current_packet_count,
            stale_packet_count,
            timeline,
        })
    }
}

async fn count_for_application(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    query: &'static str,
    application_id: i64,
) -> Result<u32> {
    let row = sqlx::query_as::<_, CountRow>(query)
        .bind(application_id)
        .fetch_one(&mut **transaction)
        .await?;
    u32::try_from(row.count).map_err(|_| anyhow!("invalid case count"))
}

async fn count_case_evidence(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    case_file_id: &str,
) -> Result<u32> {
    let row = sqlx::query_as::<_, CountRow>(
        "SELECT COUNT(*) AS count FROM career_graph_links
         WHERE subject_id = ? AND relation = 'evidence'",
    )
    .bind(case_file_id)
    .fetch_one(&mut **transaction)
    .await?;
    u32::try_from(row.count).map_err(|_| anyhow!("invalid case count"))
}

async fn packet_counts(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    case_file_id: &str,
    job_revision: &str,
) -> Result<(u32, u32)> {
    let current_job_revision = parse_sqlite_datetime(job_revision)?.to_rfc3339();
    let rows = sqlx::query_as::<_, PacketRow>(
        "SELECT packet.resume_revision, packet.job_revision,
                resume.updated_at AS current_resume_revision
         FROM v3_evidence_packets AS packet
         LEFT JOIN resumes AS resume ON resume.id = packet.resume_id
         WHERE packet.case_file_id = ?",
    )
    .bind(case_file_id)
    .fetch_all(&mut **transaction)
    .await?;
    let mut current = 0_u32;
    let mut stale = 0_u32;
    for row in rows {
        let current_resume_revision = row
            .current_resume_revision
            .as_deref()
            .and_then(|revision| parse_sqlite_datetime(revision).ok())
            .map(|revision| revision.to_rfc3339());
        if row.job_revision == current_job_revision
            && current_resume_revision.as_deref() == Some(row.resume_revision.as_str())
        {
            current = current
                .checked_add(1)
                .ok_or_else(|| anyhow!("invalid packet count"))?;
        } else {
            stale = stale
                .checked_add(1)
                .ok_or_else(|| anyhow!("invalid packet count"))?;
        }
    }
    Ok((current, stale))
}

async fn case_timeline(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    case_file_id: &str,
) -> Result<Vec<OpportunityCaseTimelineRecord>> {
    let rows = sqlx::query_as::<_, CaseEventRow>(
        "SELECT event_kind, metadata_json, created_at FROM v3_job_events
         WHERE case_file_id = ? AND event_kind <> 'case_created'
         ORDER BY created_at, event_id",
    )
    .bind(case_file_id)
    .fetch_all(&mut **transaction)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(OpportunityCaseTimelineRecord {
                at: parse_sqlite_datetime(&row.created_at)?,
                kind: sanitized_case_event_kind(&row.event_kind, &row.metadata_json)?,
            })
        })
        .collect()
}

async fn application_timeline(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    application_id: i64,
) -> Result<Vec<OpportunityCaseTimelineRecord>> {
    let rows = sqlx::query_as::<_, ApplicationEventRow>(
        "SELECT event_type, created_at FROM application_events
         WHERE application_id = ? ORDER BY created_at, id",
    )
    .bind(application_id)
    .fetch_all(&mut **transaction)
    .await?;
    rows.into_iter()
        .map(|row| {
            let kind = match row.event_type.as_str() {
                "status_change" => "application_status_changed",
                "email_received" => "application_email_received",
                "email_sent" => "application_email_sent",
                "phone_call" => "application_phone_called",
                "interview_scheduled" => "application_interview_scheduled",
                "note_added" => "application_note_added",
                "reminder_set" => "application_reminder_set",
                _ => return Err(anyhow!("invalid application event")),
            };
            Ok(OpportunityCaseTimelineRecord {
                at: parse_sqlite_datetime(&row.created_at)?,
                kind: kind.to_string(),
            })
        })
        .collect()
}

fn sanitized_case_event_kind(event_kind: &str, metadata_json: &str) -> Result<String> {
    let metadata: jobsentinel_domain::v3_foundation::EventMetadata =
        serde_json::from_str(metadata_json).map_err(|_| anyhow!("invalid case event"))?;
    match (event_kind, metadata) {
        ("status_changed", _) => Ok("status_changed".to_string()),
        ("evidence_linked", _) => Ok("evidence_linked".to_string()),
        ("privacy_receipt_recorded", _) => Ok("privacy_receipt_recorded".to_string()),
        ("source_policy_changed", _) => Ok("source_policy_changed".to_string()),
        (
            "source_checked",
            jobsentinel_domain::v3_foundation::EventMetadata::SourceOutcome { outcome, .. },
        ) => Ok(match outcome {
            jobsentinel_domain::v3_foundation::SourceOutcome::Success => "source_checked_succeeded",
            jobsentinel_domain::v3_foundation::SourceOutcome::Failure => "source_checked_failed",
            jobsentinel_domain::v3_foundation::SourceOutcome::Timeout => "source_checked_timed_out",
            jobsentinel_domain::v3_foundation::SourceOutcome::Cancelled => {
                "source_checked_cancelled"
            }
        }
        .to_string()),
        (
            "recovery_recorded",
            jobsentinel_domain::v3_foundation::EventMetadata::RecoveryOutcome { outcome },
        ) => Ok(format!("recovery_{}", enum_text(outcome)?)),
        _ => Err(anyhow!("invalid case event")),
    }
}

fn sanitized_risk_reasons(raw: Option<&str>) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw.unwrap_or("[]"))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|reason| {
            let trimmed = reason.trim();
            (!trimmed.is_empty() && !trimmed.chars().any(char::is_control))
                .then(|| trimmed.chars().take(160).collect::<String>())
        })
        .take(8)
        .collect()
}

fn terminal_outcome(status: &str) -> Option<&str> {
    matches!(
        status,
        "offer_accepted" | "offer_rejected" | "rejected" | "ghosted" | "withdrawn"
    )
    .then_some(status)
}

fn valid_job_hash(job_hash: &str) -> bool {
    !job_hash.is_empty() && job_hash.len() <= 128 && !job_hash.chars().any(char::is_control)
}
