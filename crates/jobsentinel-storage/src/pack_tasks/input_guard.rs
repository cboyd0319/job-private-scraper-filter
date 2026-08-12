//! Hashes every local source row that can affect one reviewed draft application packet.

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, Transaction};

use crate::Database;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftPacketInputGuard {
    job_hash: String,
    resume_id: i64,
    digest: String,
}

impl DraftPacketInputGuard {
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

impl Database {
    pub async fn read_draft_packet_input_guard(
        &self,
        job_hash: &str,
        resume_id: i64,
    ) -> Result<DraftPacketInputGuard> {
        let mut transaction = self.pool().begin().await?;
        let guard = read_guard(&mut transaction, job_hash, resume_id).await?;
        transaction.commit().await?;
        Ok(guard)
    }
}

pub(super) async fn guard_matches_current(
    transaction: &mut Transaction<'_, Sqlite>,
    guard: &DraftPacketInputGuard,
    job_hash: &str,
) -> Result<bool> {
    if guard.job_hash != job_hash {
        return Ok(false);
    }
    Ok(read_guard(transaction, &guard.job_hash, guard.resume_id).await? == *guard)
}

async fn read_guard(
    transaction: &mut Transaction<'_, Sqlite>,
    job_hash: &str,
    resume_id: i64,
) -> Result<DraftPacketInputGuard> {
    validate_context(job_hash, resume_id)?;
    let header = sqlx::query_scalar::<_, String>(
        "SELECT json_object(
            'job_hash', j.hash, 'title', j.title, 'company', j.company,
            'description', j.description, 'job_revision', j.updated_at,
            'case_file_id', c.case_file_id, 'application_id', a.id,
            'application_status', a.status, 'offer_id', o.id,
            'offer_accepted', o.accepted, 'resume_id', r.id,
            'resume_name', r.name, 'resume_path', r.file_path,
            'resume_text', r.parsed_text, 'resume_revision', r.updated_at,
            'saved_match_id', m.id
         )
         FROM jobs AS j
         JOIN opportunity_case_files AS c ON c.job_hash = j.hash
         JOIN resume_job_matches AS m ON m.job_hash = j.hash AND m.resume_id = ?
         JOIN resumes AS r ON r.id = m.resume_id
         LEFT JOIN applications AS a ON a.job_hash = j.hash
         LEFT JOIN offers AS o ON o.application_id = a.id
         WHERE j.hash = ?",
    )
    .bind(resume_id)
    .bind(job_hash)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(|| anyhow!("draft packet source is no longer available"))?;
    let skills = sqlx::query_scalar::<_, String>(
        "SELECT json_object('name', skill_name, 'confidence', confidence_score)
         FROM user_skills
         WHERE resume_id = ?
         ORDER BY confidence_score DESC, skill_name ASC",
    )
    .bind(resume_id)
    .fetch_all(&mut **transaction)
    .await?;
    let confirmed_evidence = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT json_object('evidence_id', link.object_id)
         FROM career_graph_links AS link
         JOIN opportunity_case_files AS c ON c.case_file_id = link.subject_id
         WHERE c.job_hash = ?
           AND link.relation = 'evidence'
           AND link.provenance = 'user_confirmed'
           AND link.provenance_ref IS NULL
           AND length(CAST(link.object_id AS BLOB)) = 64
           AND link.object_id NOT GLOB '*[^0-9a-f]*'
           AND EXISTS(
               SELECT 1 FROM v3_job_events AS event
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
    .bind(job_hash)
    .fetch_all(&mut **transaction)
    .await?;
    let claims = sqlx::query_scalar::<_, String>(
        "SELECT json_object(
            'packet_id', packet.packet_id, 'resume_revision', packet.resume_revision,
            'job_revision', packet.job_revision, 'claim_id', packet.claim_id,
            'reviewed_text', packet.reviewed_text, 'local_only', packet.local_only,
            'sensitive', packet.sensitive, 'created_at', packet.created_at
         )
         FROM v3_evidence_packets AS packet
         JOIN opportunity_case_files AS c ON c.case_file_id = packet.case_file_id
         WHERE c.job_hash = ? AND packet.resume_id = ?
         ORDER BY packet.created_at, packet.packet_id",
    )
    .bind(job_hash)
    .bind(resume_id)
    .fetch_all(&mut **transaction)
    .await?;
    let claim_evidence = sqlx::query_scalar::<_, String>(
        "SELECT json_object(
            'packet_id', evidence.packet_id, 'ordinal', evidence.ordinal,
            'evidence_id', evidence.evidence_id
         )
         FROM v3_evidence_packet_evidence AS evidence
         JOIN v3_evidence_packets AS packet ON packet.packet_id = evidence.packet_id
         JOIN opportunity_case_files AS c ON c.case_file_id = packet.case_file_id
         WHERE c.job_hash = ? AND packet.resume_id = ?
         ORDER BY packet.created_at, packet.packet_id, evidence.ordinal",
    )
    .bind(job_hash)
    .bind(resume_id)
    .fetch_all(&mut **transaction)
    .await?;
    let claim_boundaries = sqlx::query_scalar::<_, String>(
        "SELECT json_object(
            'packet_id', boundary.packet_id, 'ordinal', boundary.ordinal,
            'boundary', boundary.boundary
         )
         FROM v3_evidence_packet_boundaries AS boundary
         JOIN v3_evidence_packets AS packet ON packet.packet_id = boundary.packet_id
         JOIN opportunity_case_files AS c ON c.case_file_id = packet.case_file_id
         WHERE c.job_hash = ? AND packet.resume_id = ?
         ORDER BY packet.created_at, packet.packet_id, boundary.ordinal",
    )
    .bind(job_hash)
    .bind(resume_id)
    .fetch_all(&mut **transaction)
    .await?;

    let mut digest = Sha256::new();
    digest.update(b"jobsentinel.pack-task.draft-packet-input.v1\0");
    hash_group(&mut digest, "header", std::slice::from_ref(&header));
    hash_group(&mut digest, "skills", &skills);
    hash_group(&mut digest, "confirmed-evidence", &confirmed_evidence);
    hash_group(&mut digest, "claims", &claims);
    hash_group(&mut digest, "claim-evidence", &claim_evidence);
    hash_group(&mut digest, "claim-boundaries", &claim_boundaries);
    Ok(DraftPacketInputGuard {
        job_hash: job_hash.to_string(),
        resume_id,
        digest: hex::encode(digest.finalize()),
    })
}

fn hash_group(digest: &mut Sha256, label: &str, values: &[String]) {
    hash_value(digest, label.as_bytes());
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        hash_value(digest, value.as_bytes());
    }
}

fn hash_value(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_le_bytes());
    digest.update(value);
}

fn validate_context(job_hash: &str, resume_id: i64) -> Result<()> {
    if job_hash.is_empty()
        || job_hash.len() > 128
        || job_hash.chars().any(char::is_control)
        || resume_id <= 0
    {
        return Err(anyhow!("invalid draft packet context"));
    }
    Ok(())
}
