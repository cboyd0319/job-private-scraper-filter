use super::*;
use chrono::{DateTime, Utc};
use jobsentinel_domain::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};
use std::collections::HashSet;

const MAX_CLAIM_BYTES: usize = 8_192;
const MAX_EVIDENCE_IDS: usize = 32;
const MAX_BOUNDARIES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePacketBoundary {
    ClearanceCurrentnessUnverified,
    MilitaryCivilianEquivalenceUnverified,
}

impl EvidencePacketBoundary {
    fn as_str(self) -> &'static str {
        match self {
            Self::ClearanceCurrentnessUnverified => "clearance_currentness_unverified",
            Self::MilitaryCivilianEquivalenceUnverified => {
                "military_civilian_equivalence_unverified"
            }
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "clearance_currentness_unverified" => Some(Self::ClearanceCurrentnessUnverified),
            "military_civilian_equivalence_unverified" => {
                Some(Self::MilitaryCivilianEquivalenceUnverified)
            }
            _ => None,
        }
    }
}

/// One reviewed claim and its write-time citations. Field paths are validated,
/// then discarded before the claim reaches durable storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvidenceBoundPacketClaim {
    pub case_file_id: String,
    pub claim_id: String,
    pub reviewed_text: String,
    pub resume_snapshot: ResumeEvidenceSnapshot,
    pub job_revision: String,
    pub evidence_ids: Vec<String>,
    pub citations: Vec<ResumeEvidenceCitation>,
    pub boundaries: Vec<EvidencePacketBoundary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBoundPacketClaimRecord {
    packet_id: String,
    case_file_id: String,
    claim_id: String,
    reviewed_text: String,
    evidence_ids: Vec<String>,
    boundaries: Vec<EvidencePacketBoundary>,
}

impl EvidenceBoundPacketClaimRecord {
    #[must_use]
    pub fn packet_id(&self) -> &str {
        &self.packet_id
    }

    #[must_use]
    pub fn case_file_id(&self) -> &str {
        &self.case_file_id
    }

    #[must_use]
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    #[must_use]
    pub fn reviewed_text(&self) -> &str {
        &self.reviewed_text
    }

    #[must_use]
    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }

    #[must_use]
    pub fn boundaries(&self) -> &[EvidencePacketBoundary] {
        &self.boundaries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceBoundPacketClaimsRead {
    Current(Vec<EvidenceBoundPacketClaimRecord>),
    Invalid,
}

#[derive(FromRow)]
struct PacketRow {
    packet_id: String,
    case_file_id: String,
    resume_id: i64,
    resume_revision: String,
    job_revision: String,
    claim_id: String,
    reviewed_text: String,
    local_only: bool,
    sensitive: bool,
}

#[derive(FromRow)]
struct OrdinalTextRow {
    ordinal: i64,
    value: String,
}

#[derive(FromRow)]
struct RevisionRow {
    resume_revision: String,
    job_revision: String,
}

impl Database {
    pub async fn persist_evidence_bound_packet_claim(
        &self,
        input: &NewEvidenceBoundPacketClaim,
    ) -> Result<EvidenceBoundPacketClaimRecord> {
        let validated = validate_new_claim(input)?;
        let packet_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let current = current_revisions(&mut transaction, &input.case_file_id, validated.resume_id)
            .await?
            .ok_or_else(|| anyhow!("packet source does not exist"))?;
        if current.resume_revision != validated.resume_revision
            || current.job_revision != validated.job_revision
        {
            return Err(anyhow!("packet source revision is stale"));
        }
        for evidence_id in &validated.evidence_ids {
            if !confirmed_case_evidence(&mut transaction, &input.case_file_id, evidence_id).await? {
                return Err(anyhow!("packet evidence is not confirmed for this case"));
            }
        }

        sqlx::query(
            "INSERT INTO v3_evidence_packets (
                packet_id, case_file_id, resume_id, resume_revision, job_revision,
                claim_id, reviewed_text, local_only, sensitive, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 1, ?)",
        )
        .bind(&packet_id)
        .bind(&input.case_file_id)
        .bind(validated.resume_id)
        .bind(&validated.resume_revision)
        .bind(&validated.job_revision)
        .bind(&input.claim_id)
        .bind(&input.reviewed_text)
        .bind(&created_at)
        .execute(&mut *transaction)
        .await?;
        for (ordinal, evidence_id) in validated.evidence_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO v3_evidence_packet_evidence (packet_id, ordinal, evidence_id)
                 VALUES (?, ?, ?)",
            )
            .bind(&packet_id)
            .bind(i64::try_from(ordinal).map_err(|_| anyhow!("invalid evidence order"))?)
            .bind(evidence_id)
            .execute(&mut *transaction)
            .await?;
        }
        for (ordinal, boundary) in input.boundaries.iter().enumerate() {
            sqlx::query(
                "INSERT INTO v3_evidence_packet_boundaries (packet_id, ordinal, boundary)
                 VALUES (?, ?, ?)",
            )
            .bind(&packet_id)
            .bind(i64::try_from(ordinal).map_err(|_| anyhow!("invalid boundary order"))?)
            .bind(boundary.as_str())
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;

        Ok(EvidenceBoundPacketClaimRecord {
            packet_id,
            case_file_id: input.case_file_id.clone(),
            claim_id: input.claim_id.clone(),
            reviewed_text: input.reviewed_text.clone(),
            evidence_ids: validated.evidence_ids,
            boundaries: input.boundaries.clone(),
        })
    }

    pub async fn list_current_evidence_bound_packet_claims(
        &self,
        case_file_id: &str,
        resume_id: i64,
    ) -> Result<EvidenceBoundPacketClaimsRead> {
        if !canonical_uuid(case_file_id) || resume_id <= 0 {
            return Err(anyhow!("invalid packet context"));
        }
        let mut transaction = self.pool().begin().await?;
        let Some(current) = current_revisions(&mut transaction, case_file_id, resume_id).await?
        else {
            transaction.commit().await?;
            return Ok(EvidenceBoundPacketClaimsRead::Invalid);
        };
        let rows = sqlx::query_as::<_, PacketRow>(
            "SELECT packet_id, case_file_id, resume_id, resume_revision, job_revision,
                    claim_id, reviewed_text, local_only, sensitive
             FROM v3_evidence_packets
             WHERE case_file_id = ? AND resume_id = ?
             ORDER BY created_at, packet_id",
        )
        .bind(case_file_id)
        .bind(resume_id)
        .fetch_all(&mut *transaction)
        .await?;
        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(record) = read_current_packet(&mut transaction, row, &current).await? else {
                transaction.commit().await?;
                return Ok(EvidenceBoundPacketClaimsRead::Invalid);
            };
            records.push(record);
        }
        transaction.commit().await?;
        Ok(EvidenceBoundPacketClaimsRead::Current(records))
    }
}

async fn read_current_packet(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    row: PacketRow,
    current: &RevisionRow,
) -> Result<Option<EvidenceBoundPacketClaimRecord>> {
    if !valid_stored_packet(&row)
        || current.resume_revision != row.resume_revision
        || current.job_revision != row.job_revision
    {
        return Ok(None);
    }
    let evidence = sqlx::query_as::<_, OrdinalTextRow>(
        "SELECT ordinal, evidence_id AS value
             FROM v3_evidence_packet_evidence
             WHERE packet_id = ?
             ORDER BY ordinal",
    )
    .bind(&row.packet_id)
    .fetch_all(&mut **transaction)
    .await?;
    let boundaries = sqlx::query_as::<_, OrdinalTextRow>(
        "SELECT ordinal, boundary AS value
             FROM v3_evidence_packet_boundaries
             WHERE packet_id = ?
             ORDER BY ordinal",
    )
    .bind(&row.packet_id)
    .fetch_all(&mut **transaction)
    .await?;
    let Some(evidence_ids) = validated_ordered_evidence(&evidence) else {
        return Ok(None);
    };
    let Some(boundaries) = validated_ordered_boundaries(&boundaries) else {
        return Ok(None);
    };
    for evidence_id in &evidence_ids {
        if !confirmed_case_evidence(transaction, &row.case_file_id, evidence_id).await? {
            return Ok(None);
        }
    }
    Ok(Some(EvidenceBoundPacketClaimRecord {
        packet_id: row.packet_id,
        case_file_id: row.case_file_id,
        claim_id: row.claim_id,
        reviewed_text: row.reviewed_text,
        evidence_ids,
        boundaries,
    }))
}

struct ValidatedClaim {
    resume_id: i64,
    resume_revision: String,
    job_revision: String,
    evidence_ids: Vec<String>,
}

fn validate_new_claim(input: &NewEvidenceBoundPacketClaim) -> Result<ValidatedClaim> {
    if !canonical_uuid(&input.case_file_id)
        || !canonical_uuid(&input.claim_id)
        || input.reviewed_text.trim().is_empty()
        || input.reviewed_text.len() > MAX_CLAIM_BYTES
        || input.reviewed_text.chars().any(char::is_control)
        || input.citations.is_empty()
        || input.citations.len() > MAX_EVIDENCE_IDS
        || input.evidence_ids.len() != input.citations.len()
        || input.boundaries.len() > MAX_BOUNDARIES
    {
        return Err(anyhow!("invalid packet claim"));
    }
    let resume_id = input
        .resume_snapshot
        .source_id
        .strip_prefix("resume:")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .filter(|value| input.resume_snapshot.source_id == format!("resume:{value}"))
        .ok_or_else(|| anyhow!("invalid packet resume reference"))?;
    let resume_revision = canonical_revision(&input.resume_snapshot.revision)?;
    let job_revision = canonical_revision(&input.job_revision)?;
    let mut evidence_seen = HashSet::with_capacity(input.citations.len());
    let mut evidence_ids = Vec::with_capacity(input.citations.len());
    for citation in &input.citations {
        if ResumeEvidenceCitation::for_field(&input.resume_snapshot, &citation.field_path).as_ref()
            != Some(citation)
            || !opaque_evidence_id(&citation.evidence_id)
            || !evidence_seen.insert(citation.evidence_id.as_str())
        {
            return Err(anyhow!("invalid packet evidence citation"));
        }
        evidence_ids.push(citation.evidence_id.clone());
    }
    if input.evidence_ids != evidence_ids {
        return Err(anyhow!("packet evidence does not match reviewed claim"));
    }
    let mut boundary_seen = HashSet::with_capacity(input.boundaries.len());
    if input
        .boundaries
        .iter()
        .any(|boundary| !boundary_seen.insert(boundary.as_str()))
    {
        return Err(anyhow!("duplicate packet boundary"));
    }
    Ok(ValidatedClaim {
        resume_id,
        resume_revision,
        job_revision,
        evidence_ids,
    })
}

async fn current_revisions(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    case_file_id: &str,
    resume_id: i64,
) -> Result<Option<RevisionRow>> {
    let row = sqlx::query_as::<_, RevisionRow>(
        "SELECT r.updated_at AS resume_revision, j.updated_at AS job_revision
         FROM opportunity_case_files AS c
         JOIN jobs AS j ON j.hash = c.job_hash
         JOIN resumes AS r ON r.id = ?
         WHERE c.case_file_id = ?",
    )
    .bind(resume_id)
    .bind(case_file_id)
    .fetch_optional(&mut **transaction)
    .await?;
    row.map(|row| {
        Ok(RevisionRow {
            resume_revision: parse_sqlite_datetime(&row.resume_revision)?.to_rfc3339(),
            job_revision: parse_sqlite_datetime(&row.job_revision)?.to_rfc3339(),
        })
    })
    .transpose()
}

async fn confirmed_case_evidence(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    case_file_id: &str,
    evidence_id: &str,
) -> Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1
            FROM career_graph_links AS link
            WHERE link.subject_id = ?
              AND link.relation = 'evidence'
              AND link.object_id = ?
              AND link.provenance = 'user_confirmed'
              AND link.provenance_ref IS NULL
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
        )",
    )
    .bind(case_file_id)
    .bind(evidence_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(Into::into)
}

fn valid_stored_packet(row: &PacketRow) -> bool {
    canonical_uuid(&row.packet_id)
        && canonical_uuid(&row.case_file_id)
        && canonical_uuid(&row.claim_id)
        && row.resume_id > 0
        && canonical_revision(&row.resume_revision).is_ok_and(|value| value == row.resume_revision)
        && canonical_revision(&row.job_revision).is_ok_and(|value| value == row.job_revision)
        && row.local_only
        && row.sensitive
        && !row.reviewed_text.trim().is_empty()
        && row.reviewed_text.len() <= MAX_CLAIM_BYTES
        && !row.reviewed_text.chars().any(char::is_control)
}

fn validated_ordered_evidence(rows: &[OrdinalTextRow]) -> Option<Vec<String>> {
    if rows.is_empty() || rows.len() > MAX_EVIDENCE_IDS {
        return None;
    }
    let mut seen = HashSet::with_capacity(rows.len());
    rows.iter()
        .enumerate()
        .map(|(ordinal, row)| {
            (row.ordinal == i64::try_from(ordinal).ok()?
                && opaque_evidence_id(&row.value)
                && seen.insert(row.value.as_str()))
            .then(|| row.value.clone())
        })
        .collect()
}

fn validated_ordered_boundaries(rows: &[OrdinalTextRow]) -> Option<Vec<EvidencePacketBoundary>> {
    if rows.len() > MAX_BOUNDARIES {
        return None;
    }
    let mut seen = HashSet::with_capacity(rows.len());
    rows.iter()
        .enumerate()
        .map(|(ordinal, row)| {
            let boundary = EvidencePacketBoundary::parse(&row.value)?;
            (row.ordinal == i64::try_from(ordinal).ok()? && seen.insert(boundary.as_str()))
                .then_some(boundary)
        })
        .collect()
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|parsed| parsed.hyphenated().to_string() == value)
}

fn canonical_revision(value: &str) -> Result<String> {
    if value.len() > 128 || value.chars().any(char::is_control) {
        return Err(anyhow!("invalid packet revision"));
    }
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| anyhow!("invalid packet revision"))
}

fn opaque_evidence_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
