use chrono::{DateTime, Utc};
use jobsentinel_domain::{
    v3_manifests::{ModelProvenance, VectorFreshness},
    ResumeEvidenceSnapshot,
};

use crate::Database;

const MAX_SUBJECT_ID_BYTES: usize = 128;
const MAX_FRESHNESS_BYTES: usize = 4_096;
const MAX_VECTOR_DIMENSION: usize = 4_096;
type StoredVectorRow = (Vec<u8>, i64, Vec<u8>, i64);

#[derive(Debug, PartialEq)]
pub enum StoredVectorRead {
    Missing,
    Ready(Vec<f32>),
    RebuildNeeded,
}

pub fn resume_vector_subject(resume_id: i64) -> Result<String, sqlx::Error> {
    if resume_id <= 0 {
        return Err(protocol_error("Resume id is invalid"));
    }
    Ok(format!("resume:{resume_id}"))
}

impl Database {
    pub async fn store_v3_vector(
        &self,
        subject_id: &str,
        freshness: &VectorFreshness,
        model: &ModelProvenance,
        values: &[f32],
    ) -> Result<(), sqlx::Error> {
        let (freshness_json, dimension, vector_blob) =
            prepare_vector(subject_id, freshness, model, values)?;

        sqlx::query(
            "INSERT INTO v3_local_vectors
             (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
             VALUES (?, ?, ?, ?, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
             ON CONFLICT(subject_id) DO UPDATE SET
                 freshness_json = excluded.freshness_json,
                 dimension = excluded.dimension,
                 vector_blob = excluded.vector_blob,
                 revision = v3_local_vectors.revision + 1,
                 updated_at = excluded.updated_at",
        )
        .bind(subject_id)
        .bind(freshness_json)
        .bind(dimension)
        .bind(vector_blob)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn store_v3_resume_vector_if_current(
        &self,
        resume_id: i64,
        snapshot: &ResumeEvidenceSnapshot,
        freshness: &VectorFreshness,
        model: &ModelProvenance,
        values: &[f32],
    ) -> Result<(), sqlx::Error> {
        let subject_id = resume_vector_subject(resume_id)?;
        if snapshot.source_id != subject_id {
            return Err(protocol_error("Resume vector snapshot is invalid"));
        }
        let revision = DateTime::parse_from_rfc3339(&snapshot.revision)
            .map_err(|_| protocol_error("Resume vector snapshot is invalid"))?
            .with_timezone(&Utc)
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let (freshness_json, dimension, vector_blob) =
            prepare_vector(&subject_id, freshness, model, values)?;

        sqlx::query(
            "INSERT INTO v3_local_vectors
             (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
             SELECT ?, ?, ?, ?, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE EXISTS (
                 SELECT 1
                 FROM resumes
                 WHERE id = ?
                   AND strftime('%Y-%m-%dT%H:%M:%fZ', updated_at) = ?
             )
             ON CONFLICT(subject_id) DO UPDATE SET
                 freshness_json = excluded.freshness_json,
                 dimension = excluded.dimension,
                 vector_blob = excluded.vector_blob,
                 revision = v3_local_vectors.revision + 1,
                 updated_at = excluded.updated_at",
        )
        .bind(subject_id)
        .bind(freshness_json)
        .bind(dimension)
        .bind(vector_blob)
        .bind(resume_id)
        .bind(revision)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn read_v3_vector(
        &self,
        subject_id: &str,
        current: &VectorFreshness,
        model: Option<&ModelProvenance>,
    ) -> Result<StoredVectorRead, sqlx::Error> {
        validate_subject_id(subject_id)?;
        let Some(model) = model else {
            return purge_vector(self, subject_id).await;
        };
        if validate_freshness(current, model).is_err()
            || usize::try_from(current.dimension)
                .map_or(true, |dimension| dimension > MAX_VECTOR_DIMENSION)
        {
            return purge_vector(self, subject_id).await;
        }

        let row: Option<StoredVectorRow> = sqlx::query_as(
            "SELECT CAST(freshness_json AS BLOB), dimension, vector_blob, revision
             FROM v3_local_vectors
             WHERE subject_id = ?",
        )
        .bind(subject_id)
        .fetch_optional(self.pool())
        .await?;
        let Some(row) = row else {
            return Ok(StoredVectorRead::Missing);
        };

        if let Some(values) = decode_current_vector(&row.0, row.1, &row.2, current, model) {
            return Ok(StoredVectorRead::Ready(values));
        }

        delete_if_unchanged(self, subject_id, &row).await?;
        Ok(StoredVectorRead::RebuildNeeded)
    }
}

fn prepare_vector(
    subject_id: &str,
    freshness: &VectorFreshness,
    model: &ModelProvenance,
    values: &[f32],
) -> Result<(String, i64, Vec<u8>), sqlx::Error> {
    validate_subject_id(subject_id)?;
    validate_freshness(freshness, model)?;
    let dimension = usize::try_from(freshness.dimension)
        .map_err(|_| protocol_error("Vector dimension is invalid"))?;
    if dimension == 0
        || dimension > MAX_VECTOR_DIMENSION
        || values.len() != dimension
        || values.iter().any(|value| !value.is_finite())
    {
        return Err(protocol_error("Vector values are invalid"));
    }
    let freshness_json = serde_json::to_string(freshness)
        .map_err(|_| protocol_error("Vector freshness is invalid"))?;
    if freshness_json.len() > MAX_FRESHNESS_BYTES {
        return Err(protocol_error("Vector freshness is too large"));
    }
    let vector_blob = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    Ok((freshness_json, i64::from(freshness.dimension), vector_blob))
}

fn validate_subject_id(subject_id: &str) -> Result<(), sqlx::Error> {
    if subject_id.is_empty()
        || subject_id.len() > MAX_SUBJECT_ID_BYTES
        || subject_id.chars().any(char::is_control)
    {
        return Err(protocol_error("Vector subject id is invalid"));
    }
    Ok(())
}

fn validate_freshness(
    freshness: &VectorFreshness,
    model: &ModelProvenance,
) -> Result<(), sqlx::Error> {
    model
        .validate()
        .and_then(|()| freshness.validate(model))
        .map_err(|_| protocol_error("Vector provenance is invalid"))
}

fn decode_current_vector(
    freshness_json: &[u8],
    dimension: i64,
    vector_blob: &[u8],
    current: &VectorFreshness,
    model: &ModelProvenance,
) -> Option<Vec<f32>> {
    if freshness_json.len() > MAX_FRESHNESS_BYTES {
        return None;
    }
    let stored: VectorFreshness =
        serde_json::from_str(std::str::from_utf8(freshness_json).ok()?).ok()?;
    stored.validate(model).ok()?;
    if &stored != current {
        return None;
    }
    let dimension = usize::try_from(dimension).ok()?;
    if dimension == 0
        || dimension > MAX_VECTOR_DIMENSION
        || dimension != usize::try_from(stored.dimension).ok()?
        || vector_blob.len() != dimension.checked_mul(size_of::<f32>())?
    {
        return None;
    }

    vector_blob
        .chunks_exact(size_of::<f32>())
        .map(|bytes| {
            let value = f32::from_le_bytes(bytes.try_into().ok()?);
            value.is_finite().then_some(value)
        })
        .collect()
}

async fn delete_if_unchanged(
    database: &Database,
    subject_id: &str,
    row: &StoredVectorRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM v3_local_vectors
         WHERE subject_id = ?
           AND CAST(freshness_json AS BLOB) = ?
           AND dimension = ?
           AND vector_blob = ?
           AND revision = ?",
    )
    .bind(subject_id)
    .bind(&row.0)
    .bind(row.1)
    .bind(&row.2)
    .bind(row.3)
    .execute(database.pool())
    .await?;
    Ok(())
}

async fn purge_vector(
    database: &Database,
    subject_id: &str,
) -> Result<StoredVectorRead, sqlx::Error> {
    let result = sqlx::query("DELETE FROM v3_local_vectors WHERE subject_id = ?")
        .bind(subject_id)
        .execute(database.pool())
        .await?;
    Ok(if result.rows_affected() == 0 {
        StoredVectorRead::Missing
    } else {
        StoredVectorRead::RebuildNeeded
    })
}

fn protocol_error(message: &'static str) -> sqlx::Error {
    sqlx::Error::Protocol(message.to_string())
}

#[cfg(test)]
#[path = "v3_vectors_tests.rs"]
mod tests;
