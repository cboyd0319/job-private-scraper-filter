use super::*;
use chrono::Utc;
use jobsentinel_domain::{
    v3_foundation::{
        CareerRelation, CaseFileEventKind, EventMetadata, EventOrigin, GraphProvenance,
    },
    ResumeEvidenceSnapshot,
};

#[derive(FromRow)]
struct CareerGraphRow {
    link_id: String,
    subject_id: String,
    relation: String,
    object_id: String,
    provenance: String,
    provenance_ref: Option<String>,
}

#[derive(FromRow)]
struct RequirementContextRow {
    job_revision: String,
    job_description: String,
    resume_revision: String,
    resume_text: String,
}

/// One read-only snapshot of the inputs needed to reproduce a local requirement review.
pub struct CaseRequirementEvidenceContext {
    snapshot: ResumeEvidenceSnapshot,
    job_revision: String,
    job_description: String,
    resume_text: String,
    skills: Vec<String>,
    links: Vec<CareerGraphLink>,
}

impl CaseRequirementEvidenceContext {
    #[must_use]
    pub fn snapshot(&self) -> &ResumeEvidenceSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn job_revision(&self) -> &str {
        &self.job_revision
    }

    #[must_use]
    pub fn job_description(&self) -> &str {
        &self.job_description
    }

    #[must_use]
    pub fn resume_text(&self) -> &str {
        &self.resume_text
    }

    #[must_use]
    pub fn skills(&self) -> &[String] {
        &self.skills
    }

    #[must_use]
    pub fn links(&self) -> &[CareerGraphLink] {
        &self.links
    }
}

impl Database {
    pub async fn insert_case_file_evidence(
        &self,
        link: &CareerGraphLink,
        confirmation: &CaseFileEventInput,
    ) -> Result<bool> {
        let mut transaction = self.pool().begin().await?;
        let inserted =
            insert_case_file_evidence_in_transaction(&mut transaction, link, confirmation).await?;
        transaction.commit().await?;
        Ok(inserted)
    }

    pub async fn list_case_file_evidence_links(
        &self,
        case_file_id: &str,
    ) -> Result<Vec<CareerGraphLink>> {
        let mut transaction = self.pool().begin().await?;
        let case_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM opportunity_case_files WHERE case_file_id = ?
             )",
        )
        .bind(case_file_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !case_exists {
            return Err(anyhow!("case file does not exist"));
        }
        let rows = sqlx::query_as::<_, CareerGraphRow>(
            "SELECT link_id, subject_id, relation, object_id,
                    provenance, provenance_ref
             FROM career_graph_links
             WHERE subject_id = ? AND relation = 'evidence'
             ORDER BY created_at, link_id",
        )
        .bind(case_file_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        rows.into_iter().map(career_graph_link_from_row).collect()
    }

    pub async fn read_case_file_resume_evidence_context(
        &self,
        case_file_id: &str,
        resume_id: i64,
    ) -> Result<Option<(ResumeEvidenceSnapshot, Option<String>, Vec<CareerGraphLink>)>> {
        let mut transaction = self.pool().begin().await?;
        let case_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM opportunity_case_files WHERE case_file_id = ?
             )",
        )
        .bind(case_file_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !case_exists {
            return Err(anyhow!("case file does not exist"));
        }
        let resume = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT updated_at, parsed_text
             FROM resumes
             WHERE id = ?",
        )
        .bind(resume_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((revision, parsed_text)) = resume else {
            transaction.commit().await?;
            return Ok(None);
        };
        let rows = sqlx::query_as::<_, CareerGraphRow>(
            "SELECT link_id, subject_id, relation, object_id,
                    provenance, provenance_ref
             FROM career_graph_links
             WHERE subject_id = ? AND relation = 'evidence'
             ORDER BY created_at, link_id",
        )
        .bind(case_file_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;

        Ok(Some((
            ResumeEvidenceSnapshot {
                source_id: format!("resume:{resume_id}"),
                revision: parse_sqlite_datetime(&revision)?.to_rfc3339(),
            },
            parsed_text,
            rows.into_iter()
                .map(career_graph_link_from_row)
                .collect::<Result<_>>()?,
        )))
    }

    pub async fn read_case_requirement_evidence_context(
        &self,
        case_file_id: &str,
        resume_id: i64,
    ) -> Result<Option<CaseRequirementEvidenceContext>> {
        let mut transaction = self.pool().begin().await?;
        let row = sqlx::query_as::<_, RequirementContextRow>(
            "SELECT j.updated_at AS job_revision,
                    j.description AS job_description,
                    r.updated_at AS resume_revision,
                    r.parsed_text AS resume_text
             FROM opportunity_case_files AS c
             JOIN jobs AS j ON j.hash = c.job_hash
             JOIN resumes AS r ON r.id = ?
             WHERE c.case_file_id = ?
               AND j.description IS NOT NULL
               AND r.parsed_text IS NOT NULL",
        )
        .bind(resume_id)
        .bind(case_file_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let skills = sqlx::query_scalar::<_, String>(
            "SELECT skill_name
             FROM user_skills
             WHERE resume_id = ?
             ORDER BY confidence_score DESC, skill_name ASC",
        )
        .bind(resume_id)
        .fetch_all(&mut *transaction)
        .await?;
        let links = sqlx::query_as::<_, CareerGraphRow>(
            "SELECT link_id, subject_id, relation, object_id,
                    provenance, provenance_ref
             FROM career_graph_links
             WHERE subject_id = ? AND relation = 'evidence'
             ORDER BY created_at, link_id",
        )
        .bind(case_file_id)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;

        Ok(Some(CaseRequirementEvidenceContext {
            snapshot: ResumeEvidenceSnapshot {
                source_id: format!("resume:{resume_id}"),
                revision: parse_sqlite_datetime(&row.resume_revision)?.to_rfc3339(),
            },
            job_revision: parse_sqlite_datetime(&row.job_revision)?.to_rfc3339(),
            job_description: row.job_description,
            resume_text: row.resume_text,
            skills,
            links: links
                .into_iter()
                .map(career_graph_link_from_row)
                .collect::<Result<_>>()?,
        }))
    }
}

fn validate_case_evidence(link: &CareerGraphLink, confirmation: &CaseFileEventInput) -> Result<()> {
    link.validate()
        .map_err(|_| anyhow!("invalid case evidence link"))?;
    confirmation
        .validate()
        .map_err(|_| anyhow!("invalid case evidence confirmation"))?;
    let reference_matches = matches!(
        &confirmation.metadata,
        EventMetadata::LocalReference { reference_id } if reference_id == &link.object_id
    );
    let opaque_evidence_id = link.object_id.len() == 64
        && link
            .object_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if Uuid::parse_str(&link.link_id).is_err()
        || !opaque_evidence_id
        || link.relation != CareerRelation::Evidence
        || link.provenance != GraphProvenance::UserConfirmed
        || confirmation.kind != CaseFileEventKind::EvidenceLinked
        || confirmation.origin != EventOrigin::User
        || !confirmation.user_action
        || confirmation.case_file_id != link.subject_id
        || !reference_matches
    {
        return Err(anyhow!("invalid case evidence confirmation"));
    }
    Ok(())
}

pub(super) async fn insert_case_file_evidence_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    link: &CareerGraphLink,
    confirmation: &CaseFileEventInput,
) -> Result<bool> {
    validate_case_evidence(link, confirmation)?;
    let metadata_json = confirmation
        .metadata
        .to_json()
        .map_err(|_| anyhow!("invalid case evidence confirmation"))?;
    let created_at = Utc::now().to_rfc3339();
    let inserted = sqlx::query(
        "INSERT INTO career_graph_links (
            link_id, subject_id, relation, object_id,
            provenance, provenance_ref, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(subject_id, relation, object_id) DO NOTHING",
    )
    .bind(&link.link_id)
    .bind(&link.subject_id)
    .bind(enum_text(link.relation)?)
    .bind(&link.object_id)
    .bind(enum_text(link.provenance)?)
    .bind(&link.provenance_ref)
    .bind(&created_at)
    .execute(&mut **transaction)
    .await?
    .rows_affected()
        == 1;
    if inserted {
        sqlx::query(
            "INSERT INTO v3_job_events (
                event_id, case_file_id, event_kind, origin, user_action,
                local_only, sensitive, metadata_json, created_at
             ) VALUES (?, ?, ?, ?, ?, 1, 1, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&confirmation.case_file_id)
        .bind(enum_text(confirmation.kind)?)
        .bind(enum_text(confirmation.origin)?)
        .bind(confirmation.user_action)
        .bind(&metadata_json)
        .bind(&created_at)
        .execute(&mut **transaction)
        .await?;
    } else {
        let case_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM opportunity_case_files WHERE case_file_id = ?
             )",
        )
        .bind(&confirmation.case_file_id)
        .fetch_one(&mut **transaction)
        .await?;
        if !case_exists {
            return Err(anyhow!("case file does not exist"));
        }
    }
    Ok(inserted)
}

fn career_graph_link_from_row(row: CareerGraphRow) -> Result<CareerGraphLink> {
    let link = CareerGraphLink {
        link_id: row.link_id,
        subject_id: row.subject_id,
        relation: parse_enum(&row.relation)?,
        object_id: row.object_id,
        provenance: parse_enum(&row.provenance)?,
        provenance_ref: row.provenance_ref,
    };
    link.validate()
        .map_err(|_| anyhow!("invalid stored case evidence link"))?;
    Ok(link)
}
