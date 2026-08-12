use anyhow::{anyhow, Result};
use chrono::Utc;
use jobsentinel_domain::{
    v3_foundation::{GraphProvenance, SourceGraphLink, SourceRelation},
    v3_source_manifest::{parse_source_manifest, SourceManifest},
};
use sqlx::FromRow;

use crate::{
    v3_foundation::{enum_text, parse_enum},
    Database,
};

const MAX_SOURCE_MANIFEST_BYTES: usize = 64 * 1024;

#[derive(FromRow)]
struct SourceManifestRow {
    source_id: String,
    policy_ref: String,
    policy_revision: i64,
    manifest_json: String,
}

#[derive(FromRow)]
struct SourceGraphLinkRow {
    link_id: String,
    source_id: String,
    relation: String,
    related_id: String,
    provenance: String,
    provenance_ref: Option<String>,
}

impl Database {
    pub async fn store_source_manifest(&self, manifest: &SourceManifest) -> Result<()> {
        let policy = self
            .get_source_policy(&manifest.source_id)
            .await?
            .ok_or_else(|| anyhow!("source policy is required"))?;
        manifest
            .validate(&policy)
            .map_err(|_| anyhow!("invalid source manifest"))?;
        let manifest_json =
            serde_json::to_string(manifest).map_err(|_| anyhow!("invalid source manifest"))?;
        if manifest_json.len() > MAX_SOURCE_MANIFEST_BYTES {
            return Err(anyhow!("source manifest exceeds the local limit"));
        }
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let previous = sqlx::query_scalar::<_, String>(
            "SELECT manifest_json
             FROM v3_source_manifests
             WHERE source_id = ?",
        )
        .bind(&manifest.source_id)
        .fetch_optional(&mut *transaction)
        .await?
        .map(|value| {
            serde_json::from_str::<SourceManifest>(&value)
                .map_err(|_| anyhow!("stored source manifest is invalid"))
        })
        .transpose()?;
        let result = sqlx::query(
            "INSERT INTO v3_source_manifests (
                source_id, policy_ref, policy_revision,
                manifest_json, created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_id) DO UPDATE SET
                policy_ref = excluded.policy_ref,
                policy_revision = excluded.policy_revision,
                manifest_json = excluded.manifest_json,
                updated_at = excluded.updated_at
             WHERE excluded.policy_revision > v3_source_manifests.policy_revision",
        )
        .bind(&manifest.source_id)
        .bind(&manifest.policy_ref)
        .bind(i64::from(manifest.policy_revision))
        .bind(manifest_json)
        .bind(&now)
        .bind(&now)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() > 0 {
            if let Some(previous) = previous {
                for link in previous.lineage {
                    sqlx::query(
                        "DELETE FROM source_graph_links
                         WHERE link_id = ? AND source_id = ? AND relation = 'lineage'",
                    )
                    .bind(link.link_id)
                    .bind(&manifest.source_id)
                    .execute(&mut *transaction)
                    .await?;
                }
            }
            for link in &manifest.lineage {
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
                .bind(&now)
                .execute(&mut *transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        let stored = self
            .get_source_manifest(&manifest.source_id)
            .await?
            .ok_or_else(|| anyhow!("source manifest was not stored"))?;
        if result.rows_affected() == 0 && stored != *manifest {
            return Err(anyhow!("source manifest revision conflict"));
        }
        Ok(())
    }

    pub async fn get_source_manifest(&self, source_id: &str) -> Result<Option<SourceManifest>> {
        let Some(row) = sqlx::query_as::<_, SourceManifestRow>(
            "SELECT source_id, policy_ref, policy_revision, manifest_json
             FROM v3_source_manifests
             WHERE source_id = ?",
        )
        .bind(source_id)
        .fetch_optional(self.pool())
        .await?
        else {
            return Ok(None);
        };
        let policy = self
            .get_source_policy_revision(&row.source_id, row.policy_revision)
            .await?;
        if row.policy_ref != policy.policy_ref {
            return Err(anyhow!("stored source manifest policy is invalid"));
        }
        parse_source_manifest(&row.manifest_json, &policy)
            .map(Some)
            .map_err(|_| anyhow!("stored source manifest is invalid"))
    }

    pub async fn list_source_graph_links(&self, source_id: &str) -> Result<Vec<SourceGraphLink>> {
        let rows = sqlx::query_as::<_, SourceGraphLinkRow>(
            "SELECT link_id, source_id, relation, related_id, provenance, provenance_ref
             FROM source_graph_links
             WHERE source_id = ?
             ORDER BY link_id",
        )
        .bind(source_id)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(source_graph_link_from_row).collect()
    }
}

fn source_graph_link_from_row(row: SourceGraphLinkRow) -> Result<SourceGraphLink> {
    let link = SourceGraphLink {
        link_id: row.link_id,
        source_id: row.source_id,
        relation: parse_enum::<SourceRelation>(&row.relation)?,
        related_id: row.related_id,
        provenance: parse_enum::<GraphProvenance>(&row.provenance)?,
        provenance_ref: row.provenance_ref,
    };
    link.validate()
        .map_err(|_| anyhow!("stored source graph link is invalid"))?;
    Ok(link)
}
