//! Builds renderer-safe local opportunity-case snapshots and decisions.

use super::FoundationError;
use crate::ats::ApplicationStatus;
use chrono::{DateTime, Utc};
use jobsentinel_storage::{
    v3_foundation::{OpportunityCaseRead, OpportunityCaseTimelineRecord},
    Database,
};
use serde::Serialize;

#[path = "opportunity_case/dossier.rs"]
pub mod dossier;
use dossier::EmployerDossier;
#[path = "opportunity_case/evidence.rs"]
mod evidence;
use evidence::{
    build_opportunity_case_decision, load_opportunity_case_evidence, OpportunityCaseDecision,
    OpportunityCaseEvidenceRequirement, OpportunityCaseEvidenceReviewStatus,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpportunityCaseSnapshot {
    pub job: OpportunityCaseJob,
    pub source: OpportunityCaseSource,
    pub posting_risk: OpportunityCasePostingRisk,
    pub application: Option<OpportunityCaseApplication>,
    pub interviews: Option<OpportunityCaseInterviewSummary>,
    pub offer: Option<OpportunityCaseOffer>,
    pub outcome: Option<OpportunityCaseOutcome>,
    pub evidence: OpportunityCaseEvidence,
    pub decision: OpportunityCaseDecision,
    pub employer_dossier: EmployerDossier,
    pub timeline: Vec<OpportunityCaseTimelineItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpportunityCaseJob {
    pub job_hash: String,
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub remote: Option<bool>,
    pub times_seen: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpportunityCaseSource {
    pub name: String,
    pub last_seen_at: DateTime<Utc>,
    pub stale: bool,
    pub connectivity_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OpportunityCasePostingRisk {
    pub score: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseApplication {
    pub status: ApplicationStatus,
    pub has_contact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseInterviewSummary {
    pub upcoming_count: u32,
    pub completed_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct OpportunityCaseOffer {
    pub status: OpportunityCaseOfferStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityCaseOfferStatus {
    Pending,
    Accepted,
    Declined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseOutcome {
    pub status: OpportunityCaseOutcomeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityCaseOutcomeStatus {
    OfferAccepted,
    OfferRejected,
    Rejected,
    Ghosted,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseEvidence {
    pub confirmed_count: u32,
    pub current_packet_count: u32,
    pub stale_packet_count: u32,
    pub review_status: OpportunityCaseEvidenceReviewStatus,
    pub requirements: Vec<OpportunityCaseEvidenceRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpportunityCaseTimelineItem {
    pub at: DateTime<Utc>,
    pub kind: OpportunityCaseTimelineKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityCaseTimelineKind {
    CaseCreated,
    StatusChanged,
    EvidenceLinked,
    SourceCheckedSucceeded,
    SourceCheckedFailed,
    SourceCheckedTimedOut,
    SourceCheckedCancelled,
    PrivacyReceiptRecorded,
    SourcePolicyChanged,
    RecoveryRetried,
    RecoveryRestored,
    RecoveryFailed,
    ApplicationStatusChanged,
    ApplicationEmailReceived,
    ApplicationEmailSent,
    ApplicationPhoneCalled,
    ApplicationInterviewScheduled,
    ApplicationNoteAdded,
    ApplicationReminderSet,
}

impl OpportunityCaseTimelineKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CaseCreated => "case_created",
            Self::StatusChanged => "status_changed",
            Self::EvidenceLinked => "evidence_linked",
            Self::SourceCheckedSucceeded => "source_checked_succeeded",
            Self::SourceCheckedFailed => "source_checked_failed",
            Self::SourceCheckedTimedOut => "source_checked_timed_out",
            Self::SourceCheckedCancelled => "source_checked_cancelled",
            Self::PrivacyReceiptRecorded => "privacy_receipt_recorded",
            Self::SourcePolicyChanged => "source_policy_changed",
            Self::RecoveryRetried => "recovery_retried",
            Self::RecoveryRestored => "recovery_restored",
            Self::RecoveryFailed => "recovery_failed",
            Self::ApplicationStatusChanged => "application_status_changed",
            Self::ApplicationEmailReceived => "application_email_received",
            Self::ApplicationEmailSent => "application_email_sent",
            Self::ApplicationPhoneCalled => "application_phone_called",
            Self::ApplicationInterviewScheduled => "application_interview_scheduled",
            Self::ApplicationNoteAdded => "application_note_added",
            Self::ApplicationReminderSet => "application_reminder_set",
        }
    }
}

/// Explicitly creates or reuses the local case for one saved job, then reads its safe snapshot.
pub async fn open_opportunity_case(
    database: &Database,
    job_hash: &str,
    stale_threshold_days: i64,
) -> Result<OpportunityCaseSnapshot, FoundationError> {
    open_opportunity_case_at(
        database,
        job_hash,
        stale_threshold_days,
        Utc::now().date_naive(),
    )
    .await
}

async fn open_opportunity_case_at(
    database: &Database,
    job_hash: &str,
    stale_threshold_days: i64,
    today: chrono::NaiveDate,
) -> Result<OpportunityCaseSnapshot, FoundationError> {
    if !valid_job_hash(job_hash) || stale_threshold_days < 1 {
        return Err(FoundationError::InvalidInput);
    }
    super::create_or_reuse_case_file(database, job_hash).await?;
    let read = database
        .read_opportunity_case(job_hash)
        .await
        .map_err(super::map_error)?;
    let employer_dossier = dossier::build_employer_dossier(database, &read, today).await?;
    let evidence_review = load_opportunity_case_evidence(database, job_hash).await?;
    renderer_snapshot(
        read,
        stale_threshold_days,
        evidence_review,
        employer_dossier,
    )
}

fn renderer_snapshot(
    read: OpportunityCaseRead,
    stale_threshold_days: i64,
    evidence_review: evidence::OpportunityCaseEvidenceReview,
    employer_dossier: EmployerDossier,
) -> Result<OpportunityCaseSnapshot, FoundationError> {
    let application = read
        .application_status
        .as_deref()
        .map(application_status)
        .transpose()?
        .map(|status| OpportunityCaseApplication {
            status,
            has_contact: read.has_contact,
        });
    let offer = read.offer_status.as_deref().map(offer_status).transpose()?;
    let outcome = read
        .outcome_status
        .as_deref()
        .map(outcome_status)
        .transpose()?
        .map(|status| OpportunityCaseOutcome { status });
    let source_stale = Utc::now()
        .signed_duration_since(read.last_seen_at)
        .num_days()
        >= stale_threshold_days;
    let decision = build_opportunity_case_decision(&read, source_stale, &evidence_review);
    let timeline = read
        .timeline
        .into_iter()
        .map(timeline_item)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OpportunityCaseSnapshot {
        job: OpportunityCaseJob {
            job_hash: read.job_hash,
            title: read.title,
            company: read.company,
            location: read.location,
            remote: read.remote,
            times_seen: read.times_seen,
        },
        source: OpportunityCaseSource {
            name: read.source_name,
            last_seen_at: read.last_seen_at,
            stale: source_stale,
            connectivity_required: true,
        },
        posting_risk: OpportunityCasePostingRisk {
            score: read.posting_risk_score,
            reasons: read.posting_risk_reasons,
        },
        application,
        interviews: (read.upcoming_interview_count > 0 || read.completed_interview_count > 0)
            .then_some(OpportunityCaseInterviewSummary {
                upcoming_count: read.upcoming_interview_count,
                completed_count: read.completed_interview_count,
            }),
        offer,
        outcome,
        evidence: OpportunityCaseEvidence {
            confirmed_count: read.confirmed_evidence_count,
            current_packet_count: read.current_packet_count,
            stale_packet_count: read.stale_packet_count,
            review_status: evidence_review.status,
            requirements: evidence_review.requirements,
        },
        decision,
        employer_dossier,
        timeline,
    })
}

fn timeline_item(
    record: OpportunityCaseTimelineRecord,
) -> Result<OpportunityCaseTimelineItem, FoundationError> {
    Ok(OpportunityCaseTimelineItem {
        at: record.at,
        kind: match record.kind.as_str() {
            "case_created" => OpportunityCaseTimelineKind::CaseCreated,
            "status_changed" => OpportunityCaseTimelineKind::StatusChanged,
            "evidence_linked" => OpportunityCaseTimelineKind::EvidenceLinked,
            "source_checked_succeeded" => OpportunityCaseTimelineKind::SourceCheckedSucceeded,
            "source_checked_failed" => OpportunityCaseTimelineKind::SourceCheckedFailed,
            "source_checked_timed_out" => OpportunityCaseTimelineKind::SourceCheckedTimedOut,
            "source_checked_cancelled" => OpportunityCaseTimelineKind::SourceCheckedCancelled,
            "privacy_receipt_recorded" => OpportunityCaseTimelineKind::PrivacyReceiptRecorded,
            "source_policy_changed" => OpportunityCaseTimelineKind::SourcePolicyChanged,
            "recovery_retried" => OpportunityCaseTimelineKind::RecoveryRetried,
            "recovery_restored" => OpportunityCaseTimelineKind::RecoveryRestored,
            "recovery_failed" => OpportunityCaseTimelineKind::RecoveryFailed,
            "application_status_changed" => OpportunityCaseTimelineKind::ApplicationStatusChanged,
            "application_email_received" => OpportunityCaseTimelineKind::ApplicationEmailReceived,
            "application_email_sent" => OpportunityCaseTimelineKind::ApplicationEmailSent,
            "application_phone_called" => OpportunityCaseTimelineKind::ApplicationPhoneCalled,
            "application_interview_scheduled" => {
                OpportunityCaseTimelineKind::ApplicationInterviewScheduled
            }
            "application_note_added" => OpportunityCaseTimelineKind::ApplicationNoteAdded,
            "application_reminder_set" => OpportunityCaseTimelineKind::ApplicationReminderSet,
            _ => return Err(FoundationError::Storage("invalid")),
        },
    })
}

fn application_status(status: &str) -> Result<ApplicationStatus, FoundationError> {
    status
        .parse()
        .map_err(|_| FoundationError::Storage("invalid"))
}

fn offer_status(status: &str) -> Result<OpportunityCaseOffer, FoundationError> {
    match status {
        "pending" => Ok(OpportunityCaseOffer {
            status: OpportunityCaseOfferStatus::Pending,
        }),
        "accepted" => Ok(OpportunityCaseOffer {
            status: OpportunityCaseOfferStatus::Accepted,
        }),
        "declined" => Ok(OpportunityCaseOffer {
            status: OpportunityCaseOfferStatus::Declined,
        }),
        _ => Err(FoundationError::Storage("invalid")),
    }
}

fn outcome_status(status: &str) -> Result<OpportunityCaseOutcomeStatus, FoundationError> {
    Ok(match status {
        "offer_accepted" => OpportunityCaseOutcomeStatus::OfferAccepted,
        "offer_rejected" => OpportunityCaseOutcomeStatus::OfferRejected,
        "rejected" => OpportunityCaseOutcomeStatus::Rejected,
        "ghosted" => OpportunityCaseOutcomeStatus::Ghosted,
        "withdrawn" => OpportunityCaseOutcomeStatus::Withdrawn,
        _ => return Err(FoundationError::Storage("invalid")),
    })
}

fn valid_job_hash(job_hash: &str) -> bool {
    !job_hash.is_empty() && job_hash.len() <= 128 && !job_hash.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "opportunity_case/dossier_tests.rs"]
mod dossier_tests;

#[cfg(test)]
mod tests {
    use super::open_opportunity_case;
    use crate::test_support::test_job;
    use jobsentinel_storage::Database;

    #[tokio::test]
    async fn opening_a_case_creates_one_safe_local_snapshot_for_an_existing_job() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let mut job = test_job(
            "case-office-assistant",
            "Office Assistant",
            "Community Care",
        );
        job.ghost_score = Some(0.7);
        job.times_seen = 3;
        job.ghost_reasons = Some(r#"["Reposted frequently"]"#.to_string());
        database.insert_job_if_new(&job).await.unwrap();

        let first = open_opportunity_case(&database, &job.hash, 60)
            .await
            .unwrap();
        let second = open_opportunity_case(&database, &job.hash, 60)
            .await
            .unwrap();
        let serialized = serde_json::to_string(&first).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.job.job_hash, job.hash);
        assert_eq!(first.job.title, "Office Assistant");
        assert_eq!(first.job.times_seen, 3);
        assert!(first.source.connectivity_required);
        assert_eq!(first.posting_risk.score, Some(0.7));
        assert_eq!(first.posting_risk.reasons, ["Reposted frequently"]);
        assert_eq!(first.timeline.len(), 1);
        assert_eq!(first.timeline[0].kind.as_str(), "case_created");
        for private_field in [
            "case_file_id",
            "description",
            "resume",
            "notes",
            "credential",
            "metadata_json",
        ] {
            assert!(
                !serialized.contains(&format!("\"{private_field}\":")),
                "leaked {private_field}"
            );
        }
    }

    #[tokio::test]
    async fn opening_a_case_rejects_missing_or_invalid_job_references() {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();

        assert!(open_opportunity_case(&database, "", 60).await.is_err());
        assert!(open_opportunity_case(&database, "missing-job", 60)
            .await
            .is_err());
        assert!(open_opportunity_case(&database, &"a".repeat(129), 60)
            .await
            .is_err());
        assert!(open_opportunity_case(&database, "missing-job", 0)
            .await
            .is_err());
    }
}
