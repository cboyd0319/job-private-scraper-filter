//! Applies generation-guarded pack state changes and release cleanup transitions.

use super::*;

impl Database {
    pub async fn list_runnable_pack_streams(&self) -> Result<Vec<PackStream>> {
        sqlx::query_as::<_, PackStreamRow>(
            "SELECT publisher_key_id, pack_id, high_water_sequence,
                    active_release_sequence, rollback_release_sequence,
                    availability, generation
             FROM v3_pack_streams AS stream
             WHERE availability IN ('ready', 'disabled')
               AND EXISTS (
                   SELECT 1 FROM pack_release_reviews AS review
                   WHERE review.publisher_key_id = stream.publisher_key_id
                     AND review.pack_id = stream.pack_id
                     AND review.release_sequence = stream.active_release_sequence
               )
             ORDER BY publisher_key_id, pack_id",
        )
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(stream_from_row)
        .collect()
    }

    pub async fn list_stored_pack_releases(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
    ) -> Result<Vec<StoredPackRelease>> {
        sqlx::query_as::<_, StoredPackReleaseRow>(
            "SELECT publisher_key_id, pack_id, release_sequence,
                    signed_release_sha256, lifecycle_state, quarantine_reason,
                    artifact_cleanup_pending
             FROM v3_pack_releases
             WHERE publisher_key_id = ? AND pack_id = ?
             ORDER BY release_sequence",
        )
        .bind(publisher_key_id)
        .bind(pack_id)
        .fetch_all(self.pool())
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    pub async fn revoke_pack_publisher(&self, publisher: &TrustedPublisherKey) -> Result<u64> {
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        require_trusted_publisher(
            &mut transaction,
            &publisher.publisher_key_id,
            &public_key_sha256,
        )
        .await?;

        let streams = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = NULL,
                 rollback_release_sequence = NULL,
                 availability = 'quarantined',
                 generation = generation + 1,
                 updated_at = ?
             WHERE publisher_key_id = ?",
        )
        .bind(&now)
        .bind(&publisher.publisher_key_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined',
                 quarantine_reason = 'trust_revoked',
                 updated_at = ?
             WHERE publisher_key_id = ? AND lifecycle_state <> 'removed'",
        )
        .bind(&now)
        .bind(&publisher.publisher_key_id)
        .execute(&mut *transaction)
        .await?;
        let publisher_updated = sqlx::query(
            "UPDATE v3_pack_publishers
             SET trust_state = 'revoked', revoked_at = ?, updated_at = ?
             WHERE publisher_key_id = ? AND trust_state = 'trusted'",
        )
        .bind(&now)
        .bind(&now)
        .bind(&publisher.publisher_key_id)
        .execute(&mut *transaction)
        .await?;
        if publisher_updated.rows_affected() != 1 {
            return Err(revoked());
        }
        transaction.commit().await?;
        Ok(streams)
    }

    pub async fn uninstall_pack(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
        expected_generation: u64,
    ) -> Result<PackStream> {
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = NULL,
                 rollback_release_sequence = NULL,
                 availability = 'removed',
                 generation = generation + 1,
                 updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability <> 'removed'",
        )
        .bind(&now)
        .bind(publisher_key_id)
        .bind(pack_id)
        .bind(generation)
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                publisher_key_id,
                pack_id,
                generation,
            )
            .await?);
        }
        sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'removed', artifact_cleanup_pending = 1,
                 updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND lifecycle_state <> 'removed'",
        )
        .bind(&now)
        .bind(publisher_key_id)
        .bind(pack_id)
        .execute(&mut *transaction)
        .await?;
        let stream = fetch_stream_by_id(&mut transaction, publisher_key_id, pack_id).await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn complete_pack_release_cleanup(&self, release: &StoredPackRelease) -> Result<()> {
        let sequence = i64::try_from(release.release_sequence).map_err(|_| invalid())?;
        let updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET artifact_cleanup_pending = 0, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND release_sequence = ?
               AND signed_release_sha256 = ? AND lifecycle_state = 'removed'
               AND artifact_cleanup_pending = 1",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(sequence)
        .bind(&release.signed_release_sha256)
        .execute(self.pool())
        .await?;
        if updated.rows_affected() == 1
            || sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1 FROM v3_pack_releases
                    WHERE publisher_key_id = ? AND pack_id = ?
                      AND release_sequence = ? AND signed_release_sha256 = ?
                      AND lifecycle_state = 'removed'
                      AND artifact_cleanup_pending = 0
                )",
            )
            .bind(&release.publisher_key_id)
            .bind(&release.pack_id)
            .bind(sequence)
            .bind(&release.signed_release_sha256)
            .fetch_one(self.pool())
            .await?
        {
            Ok(())
        } else {
            Err(invalid_state())
        }
    }

    pub async fn reconcile_interrupted_pack_lifecycle(&self) -> Result<u64> {
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let streams = sqlx::query(
            "UPDATE v3_pack_streams
             SET generation = generation + 1, updated_at = ?
             WHERE EXISTS (
                 SELECT 1 FROM v3_pack_releases AS interrupted
                 WHERE interrupted.publisher_key_id = v3_pack_streams.publisher_key_id
                   AND interrupted.pack_id = v3_pack_streams.pack_id
                   AND interrupted.lifecycle_state IN ('staged', 'self_tested')
             )",
        )
        .bind(&now)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined',
                 quarantine_reason = 'interrupted',
                 updated_at = ?
             WHERE lifecycle_state IN ('staged', 'self_tested')",
        )
        .bind(&now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(streams)
    }
}
