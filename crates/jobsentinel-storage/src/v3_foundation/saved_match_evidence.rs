// Persists and reads revision-bound saved match evidence for opportunity cases.

use super::case_evidence::insert_case_file_evidence_in_transaction;
use super::*;
use chrono::{DateTime, Utc};
use jobsentinel_domain::{
    v3_foundation::{
        CareerRelation, CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin,
        GraphProvenance,
    },
    v3_manifests::PrivacyLabel,
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};
use std::collections::HashSet;

const MAX_SAVED_MATCH_SKILLS: usize = 256;
const MAX_SAVED_MATCH_SKILL_BYTES: usize = 256;

#[derive(FromRow)]
struct SavedMatchContextRow {
    job_revision: String,
    resume_revision: String,
}

/// Expected current inputs for one explicit saved-match evidence confirmation.
/// They are verified atomically and never persisted as a second copy.
pub struct SavedMatchEvidenceConfirmation {
    pub job_hash: String,
    pub resume_id: i64,
    pub saved_match_id: i64,
    pub expected_job_revision: String,
    pub expected_resume_snapshot: ResumeEvidenceSnapshot,
    pub expected_skills: Vec<String>,
    pub citation: ResumeEvidenceCitation,
}

/// Internal durable confirmation state for one saved resume/job match.
pub struct SavedMatchConfirmedEvidence {
    case_file_id: Option<String>,
    evidence_ids: Vec<String>,
}

impl SavedMatchConfirmedEvidence {
    #[must_use]
    pub fn case_file_id(&self) -> Option<&str> {
        self.case_file_id.as_deref()
    }

    #[must_use]
    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }
}

impl Database {
    pub async fn confirm_saved_match_evidence(
        &self,
        input: &SavedMatchEvidenceConfirmation,
    ) -> Result<bool> {
        validate_saved_match_confirmation(input)?;
        let mut transaction = self.pool().begin().await?;
        let current = sqlx::query_as::<_, SavedMatchContextRow>(
            "SELECT j.updated_at AS job_revision, r.updated_at AS resume_revision
             FROM resume_job_matches AS saved_match
             JOIN jobs AS j ON j.hash = saved_match.job_hash
             JOIN resumes AS r ON r.id = saved_match.resume_id
             WHERE saved_match.id = ?
               AND saved_match.resume_id = ?
               AND saved_match.job_hash = ?",
        )
        .bind(input.saved_match_id)
        .bind(input.resume_id)
        .bind(&input.job_hash)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| anyhow!("saved match no longer exists"))?;
        let current_snapshot = ResumeEvidenceSnapshot {
            source_id: format!("resume:{}", input.resume_id),
            revision: parse_sqlite_datetime(&current.resume_revision)?.to_rfc3339(),
        };
        let current_job_revision = parse_sqlite_datetime(&current.job_revision)?.to_rfc3339();
        if current_snapshot != input.expected_resume_snapshot
            || current_job_revision != input.expected_job_revision
        {
            return Err(anyhow!("saved match source revision is stale"));
        }
        let current_skills = sqlx::query_scalar::<_, String>(
            "SELECT skill_name
             FROM user_skills
             WHERE resume_id = ?
             ORDER BY confidence_score DESC, skill_name ASC",
        )
        .bind(input.resume_id)
        .fetch_all(&mut *transaction)
        .await?;
        if current_skills != input.expected_skills {
            return Err(anyhow!("saved match skills are stale"));
        }
        let case_file_id =
            create_case_file_in_transaction(&mut transaction, &input.job_hash).await?;
        let link = CareerGraphLink {
            link_id: Uuid::new_v4().to_string(),
            subject_id: case_file_id.clone(),
            relation: CareerRelation::Evidence,
            object_id: input.citation.evidence_id.clone(),
            provenance: GraphProvenance::UserConfirmed,
            provenance_ref: None,
        };
        let confirmation = CaseFileEventInput {
            case_file_id,
            kind: CaseFileEventKind::EvidenceLinked,
            origin: EventOrigin::User,
            user_action: true,
            privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
            metadata: EventMetadata::LocalReference {
                reference_id: input.citation.evidence_id.clone(),
            },
        };
        let inserted =
            insert_case_file_evidence_in_transaction(&mut transaction, &link, &confirmation)
                .await?;
        transaction.commit().await?;
        Ok(inserted)
    }

    pub async fn list_saved_match_evidence_packet_claims(
        &self,
        job_hash: &str,
        resume_id: i64,
    ) -> Result<EvidenceBoundPacketClaimsRead> {
        let mut transaction = self.pool().begin().await?;
        let case_file_id =
            require_saved_match_case_file(&mut transaction, job_hash, resume_id).await?;
        transaction.commit().await?;
        let Some(case_file_id) = case_file_id else {
            return Ok(EvidenceBoundPacketClaimsRead::Current(Vec::new()));
        };
        self.list_current_evidence_bound_packet_claims(&case_file_id, resume_id)
            .await
    }

    pub async fn read_saved_match_confirmed_evidence(
        &self,
        job_hash: &str,
        resume_id: i64,
    ) -> Result<SavedMatchConfirmedEvidence> {
        let mut transaction = self.pool().begin().await?;
        let case_file_id =
            require_saved_match_case_file(&mut transaction, job_hash, resume_id).await?;
        let Some(case_file_id) = case_file_id else {
            transaction.commit().await?;
            return Ok(SavedMatchConfirmedEvidence {
                case_file_id: None,
                evidence_ids: Vec::new(),
            });
        };
        let evidence_ids = sqlx::query_scalar(
            "SELECT DISTINCT link.object_id
             FROM career_graph_links AS link
             WHERE link.subject_id = ?
               AND link.relation = 'evidence'
               AND link.provenance = 'user_confirmed'
               AND link.provenance_ref IS NULL
               AND length(CAST(link.object_id AS BLOB)) = 64
               AND link.object_id NOT GLOB '*[^0-9a-f]*'
               AND EXISTS(
                   SELECT 1
                   FROM v3_job_events AS event
                   WHERE event.case_file_id = link.subject_id
                     AND event.event_kind = 'evidence_linked'
                     AND event.origin = 'user'
                     AND event.user_action = 1
                     AND event.local_only = 1
                     AND event.sensitive = 1
                     AND json_extract(event.metadata_json, '$.kind') = 'local_reference'
                     AND json_extract(event.metadata_json, '$.reference_id') = link.object_id
               )
             ORDER BY link.object_id",
        )
        .bind(&case_file_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(SavedMatchConfirmedEvidence {
            case_file_id: Some(case_file_id),
            evidence_ids,
        })
    }
}

async fn require_saved_match_case_file(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    job_hash: &str,
    resume_id: i64,
) -> Result<Option<String>> {
    validate_saved_match_context(job_hash, resume_id)?;
    let saved_match_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1
            FROM resume_job_matches AS saved_match
            JOIN jobs AS j ON j.hash = saved_match.job_hash
            JOIN resumes AS r ON r.id = saved_match.resume_id
            WHERE saved_match.resume_id = ? AND saved_match.job_hash = ?
        )",
    )
    .bind(resume_id)
    .bind(job_hash)
    .fetch_one(&mut **transaction)
    .await?;
    if !saved_match_exists {
        return Err(anyhow!("saved match no longer exists"));
    }
    sqlx::query_scalar(
        "SELECT case_file_id
         FROM opportunity_case_files
         WHERE job_hash = ?",
    )
    .bind(job_hash)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(Into::into)
}

async fn create_case_file_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    job_hash: &str,
) -> Result<String> {
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
    .execute(&mut **transaction)
    .await?;
    sqlx::query_scalar(
        "SELECT case_file_id
         FROM opportunity_case_files
         WHERE job_hash = ?",
    )
    .bind(job_hash)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(|| anyhow!("saved match job no longer exists"))
}

fn validate_saved_match_confirmation(input: &SavedMatchEvidenceConfirmation) -> Result<()> {
    validate_saved_match_context(&input.job_hash, input.resume_id)?;
    if input.saved_match_id <= 0
        || input.expected_skills.len() > MAX_SAVED_MATCH_SKILLS
        || input.expected_skills.iter().any(|skill| {
            skill.trim().is_empty()
                || skill.len() > MAX_SAVED_MATCH_SKILL_BYTES
                || skill.chars().any(char::is_control)
        })
    {
        return Err(anyhow!("invalid saved match confirmation"));
    }
    let source_id = format!("resume:{}", input.resume_id);
    if input.expected_resume_snapshot.source_id != source_id
        || canonical_revision(&input.expected_resume_snapshot.revision).is_none()
        || canonical_revision(&input.expected_job_revision).is_none()
        || ResumeEvidenceCitation::for_field(
            &input.expected_resume_snapshot,
            &input.citation.field_path,
        )
        .as_ref()
            != Some(&input.citation)
    {
        return Err(anyhow!("invalid saved match evidence citation"));
    }
    let mut seen = HashSet::with_capacity(input.expected_skills.len());
    if input
        .expected_skills
        .iter()
        .any(|skill| !seen.insert(skill.as_str()))
    {
        return Err(anyhow!("duplicate saved match skill"));
    }
    Ok(())
}

fn validate_saved_match_context(job_hash: &str, resume_id: i64) -> Result<()> {
    if job_hash.is_empty()
        || job_hash.len() > 128
        || job_hash.chars().any(char::is_control)
        || resume_id <= 0
    {
        return Err(anyhow!("invalid saved match context"));
    }
    Ok(())
}

fn canonical_revision(value: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc).to_rfc3339())
        .filter(|canonical| canonical == value)
}
