// Persists atomic active and rollback artifact lifecycle transitions.

use super::*;

impl Database {
    pub async fn quarantine_active_and_rollback_pack_artifacts(
        &self,
        active: &StoredPackRelease,
        active_reason: PackQuarantineReason,
        rollback: &StoredPackRelease,
        rollback_reason: PackQuarantineReason,
        expected_generation: u64,
    ) -> Result<PackStream> {
        if active.publisher_key_id != rollback.publisher_key_id
            || active.pack_id != rollback.pack_id
            || active.release_sequence == rollback.release_sequence
            || !is_sha256(&active.signed_release_sha256)
            || !is_sha256(&rollback.signed_release_sha256)
        {
            return Err(invalid());
        }
        let active_sequence = i64::try_from(active.release_sequence).map_err(|_| invalid())?;
        let rollback_sequence = i64::try_from(rollback.release_sequence).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let active_reason = artifact_reason(active_reason)?;
        let rollback_reason = artifact_reason(rollback_reason)?;
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = NULL,
                 rollback_release_sequence = NULL,
                 availability = 'quarantined',
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability IN ('ready', 'disabled')
               AND active_release_sequence = ?
               AND rollback_release_sequence = ?",
        )
        .bind(&now)
        .bind(&active.publisher_key_id)
        .bind(&active.pack_id)
        .bind(generation)
        .bind(active_sequence)
        .bind(rollback_sequence)
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                &active.publisher_key_id,
                &active.pack_id,
                generation,
            )
            .await?);
        }
        let releases_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined',
                 quarantine_reason = CASE release_sequence
                     WHEN ? THEN ? WHEN ? THEN ?
                 END,
                 updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND lifecycle_state = 'ready'
               AND ((release_sequence = ? AND signed_release_sha256 = ?)
                 OR (release_sequence = ? AND signed_release_sha256 = ?))",
        )
        .bind(active_sequence)
        .bind(active_reason)
        .bind(rollback_sequence)
        .bind(rollback_reason)
        .bind(&now)
        .bind(&active.publisher_key_id)
        .bind(&active.pack_id)
        .bind(active_sequence)
        .bind(&active.signed_release_sha256)
        .bind(rollback_sequence)
        .bind(&rollback.signed_release_sha256)
        .execute(&mut *transaction)
        .await?;
        if releases_updated.rows_affected() != 2 {
            return Err(invalid_state());
        }
        let stream =
            fetch_stream_by_id(&mut transaction, &active.publisher_key_id, &active.pack_id).await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn replace_invalid_active_pack_with_rollback(
        &self,
        invalid_release: &StoredPackRelease,
        rollback: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.replace_active_pack_with_rollback(
            invalid_release,
            rollback,
            publisher,
            expected_generation,
            "integrity_failed",
        )
        .await
    }

    pub async fn replace_unavailable_active_pack_with_rollback(
        &self,
        unavailable_release: &StoredPackRelease,
        rollback: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.replace_active_pack_with_rollback(
            unavailable_release,
            rollback,
            publisher,
            expected_generation,
            "artifact_missing",
        )
        .await
    }

    async fn replace_active_pack_with_rollback(
        &self,
        invalid_release: &StoredPackRelease,
        rollback: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
        reason: &str,
    ) -> Result<PackStream> {
        if publisher.revoked
            || publisher.publisher_key_id != invalid_release.publisher_key_id
            || rollback.publisher_key_id() != invalid_release.publisher_key_id
            || rollback.pack_id() != invalid_release.pack_id
            || rollback.release_sequence() == invalid_release.release_sequence
            || !is_sha256(&invalid_release.signed_release_sha256)
        {
            return Err(if publisher.revoked {
                revoked()
            } else {
                invalid()
            });
        }
        let invalid_sequence =
            i64::try_from(invalid_release.release_sequence).map_err(|_| invalid())?;
        let rollback_sequence =
            i64::try_from(rollback.release_sequence()).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        require_trusted_publisher(
            &mut transaction,
            &publisher.publisher_key_id,
            &public_key_sha256,
        )
        .await?;
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = ?,
                 rollback_release_sequence = NULL,
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability IN ('ready', 'disabled')
               AND active_release_sequence = ?
               AND rollback_release_sequence = ?
               AND EXISTS (
                   SELECT 1 FROM v3_pack_releases AS retained
                   WHERE retained.publisher_key_id = v3_pack_streams.publisher_key_id
                     AND retained.pack_id = v3_pack_streams.pack_id
                     AND retained.release_sequence = ?
                     AND retained.signed_release_sha256 = ?
                     AND retained.lifecycle_state = 'ready'
               )",
        )
        .bind(rollback_sequence)
        .bind(&now)
        .bind(&invalid_release.publisher_key_id)
        .bind(&invalid_release.pack_id)
        .bind(generation)
        .bind(invalid_sequence)
        .bind(rollback_sequence)
        .bind(rollback_sequence)
        .bind(rollback.signed_release_sha256())
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                &invalid_release.publisher_key_id,
                &invalid_release.pack_id,
                generation,
            )
            .await?);
        }
        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined',
                 quarantine_reason = ?, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'ready'",
        )
        .bind(reason)
        .bind(&now)
        .bind(&invalid_release.publisher_key_id)
        .bind(&invalid_release.pack_id)
        .bind(invalid_sequence)
        .bind(&invalid_release.signed_release_sha256)
        .execute(&mut *transaction)
        .await?;
        if release_updated.rows_affected() != 1 {
            return Err(invalid_state());
        }
        let stream = fetch_stream_by_id(
            &mut transaction,
            &invalid_release.publisher_key_id,
            &invalid_release.pack_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn quarantine_unavailable_active_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_active_pack_artifact(release, expected_generation, "artifact_missing")
            .await
    }

    pub async fn quarantine_invalid_active_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_active_pack_artifact(release, expected_generation, "integrity_failed")
            .await
    }

    pub async fn quarantine_trust_revoked_active_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_active_pack_artifact(release, expected_generation, "trust_revoked")
            .await
    }

    async fn quarantine_active_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
        reason: &str,
    ) -> Result<PackStream> {
        let sequence = i64::try_from(release.release_sequence).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        if !is_sha256(&release.signed_release_sha256) {
            return Err(invalid());
        }
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = NULL,
                 rollback_release_sequence = NULL,
                 availability = 'quarantined',
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability IN ('ready', 'disabled')
               AND active_release_sequence = ?",
        )
        .bind(&now)
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(generation)
        .bind(sequence)
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                &release.publisher_key_id,
                &release.pack_id,
                generation,
            )
            .await?);
        }
        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined', quarantine_reason = ?, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'ready'",
        )
        .bind(reason)
        .bind(&now)
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(sequence)
        .bind(&release.signed_release_sha256)
        .execute(&mut *transaction)
        .await?;
        if release_updated.rows_affected() != 1 {
            return Err(invalid_state());
        }
        let stream = fetch_stream_by_id(
            &mut transaction,
            &release.publisher_key_id,
            &release.pack_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn quarantine_unavailable_rollback_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_rollback_pack_artifact(release, expected_generation, "artifact_missing")
            .await
    }

    pub async fn quarantine_invalid_rollback_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_rollback_pack_artifact(release, expected_generation, "integrity_failed")
            .await
    }

    pub async fn quarantine_trust_revoked_rollback_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_rollback_pack_artifact(release, expected_generation, "trust_revoked")
            .await
    }

    async fn quarantine_rollback_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
        reason: &str,
    ) -> Result<PackStream> {
        let sequence = i64::try_from(release.release_sequence).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        if !is_sha256(&release.signed_release_sha256) {
            return Err(invalid());
        }
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET rollback_release_sequence = NULL,
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability IN ('ready', 'disabled')
               AND active_release_sequence IS NOT NULL
               AND rollback_release_sequence = ?",
        )
        .bind(&now)
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(generation)
        .bind(sequence)
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                &release.publisher_key_id,
                &release.pack_id,
                generation,
            )
            .await?);
        }
        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined', quarantine_reason = ?, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'ready'",
        )
        .bind(reason)
        .bind(&now)
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(sequence)
        .bind(&release.signed_release_sha256)
        .execute(&mut *transaction)
        .await?;
        if release_updated.rows_affected() != 1 {
            return Err(invalid_state());
        }
        let stream = fetch_stream_by_id(
            &mut transaction,
            &release.publisher_key_id,
            &release.pack_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }
}

fn artifact_reason(reason: PackQuarantineReason) -> Result<&'static str> {
    match reason {
        PackQuarantineReason::ArtifactMissing => Ok("artifact_missing"),
        PackQuarantineReason::IntegrityFailed => Ok("integrity_failed"),
        PackQuarantineReason::TrustRevoked => Ok("trust_revoked"),
        _ => Err(invalid()),
    }
}
