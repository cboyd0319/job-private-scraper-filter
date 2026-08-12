// Implements durable pack lifecycle transitions with generation checks.

use super::*;

impl Database {
    pub async fn record_pack_self_test(
        &self,
        tested: &SelfTestedPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.complete_pack_self_test(
            tested.publisher_key_id(),
            tested.pack_id(),
            tested.release_sequence(),
            tested.signed_release_sha256(),
            expected_generation,
            "self_tested",
            None,
        )
        .await
    }

    pub async fn quarantine_failed_pack_self_test(
        &self,
        release: &VerifiedPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.complete_pack_self_test(
            release.publisher_key_id(),
            &release.manifest().pack_id,
            release.release_sequence(),
            release.signed_release_sha256(),
            expected_generation,
            "quarantined",
            Some("self_test_failed"),
        )
        .await
    }

    pub async fn quarantine_missing_pack_artifact(
        &self,
        tested: &SelfTestedPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.complete_pack_self_test(
            tested.publisher_key_id(),
            tested.pack_id(),
            tested.release_sequence(),
            tested.signed_release_sha256(),
            expected_generation,
            "quarantined",
            Some("artifact_missing"),
        )
        .await
    }

    pub async fn quarantine_unavailable_stored_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_stored_pack_artifact(release, expected_generation, "artifact_missing")
            .await
    }

    pub async fn quarantine_invalid_stored_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_stored_pack_artifact(release, expected_generation, "integrity_failed")
            .await
    }

    pub async fn quarantine_trust_revoked_stored_pack_artifact(
        &self,
        release: &StoredPackRelease,
        expected_generation: u64,
    ) -> Result<PackStream> {
        self.quarantine_stored_pack_artifact(release, expected_generation, "trust_revoked")
            .await
    }

    async fn quarantine_stored_pack_artifact(
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
        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'quarantined', quarantine_reason = ?, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'self_tested'",
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
        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND high_water_sequence = ?
               AND high_water_signed_release_sha256 = ?
               AND availability = 'quarantined'
               AND active_release_sequence IS NULL",
        )
        .bind(&now)
        .bind(&release.publisher_key_id)
        .bind(&release.pack_id)
        .bind(generation)
        .bind(sequence)
        .bind(&release.signed_release_sha256)
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
        let stream = fetch_stream_by_id(
            &mut transaction,
            &release.publisher_key_id,
            &release.pack_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }

    async fn complete_pack_self_test(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
        release_sequence: u64,
        signed_release_sha256: &str,
        expected_generation: u64,
        lifecycle_state: &str,
        quarantine_reason: Option<&str>,
    ) -> Result<PackStream> {
        let sequence = i64::try_from(release_sequence).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        if !is_sha256(signed_release_sha256) {
            return Err(invalid());
        }
        let now = Utc::now().to_rfc3339();
        let verified_at = (lifecycle_state == "self_tested").then_some(now.as_str());
        let mut transaction = self.pool().begin().await?;

        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = ?, quarantine_reason = ?,
                 self_tested_at = ?, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'staged'",
        )
        .bind(lifecycle_state)
        .bind(quarantine_reason)
        .bind(verified_at)
        .bind(&now)
        .bind(publisher_key_id)
        .bind(pack_id)
        .bind(sequence)
        .bind(signed_release_sha256)
        .execute(&mut *transaction)
        .await?;
        if release_updated.rows_affected() != 1 {
            return Err(invalid_state());
        }

        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND high_water_sequence = ?
               AND high_water_signed_release_sha256 = ?",
        )
        .bind(&now)
        .bind(publisher_key_id)
        .bind(pack_id)
        .bind(generation)
        .bind(sequence)
        .bind(signed_release_sha256)
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

        let stream = fetch_stream_by_id(&mut transaction, publisher_key_id, pack_id).await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn activate_self_tested_pack(
        &self,
        tested: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
    ) -> Result<PackStream> {
        if publisher.revoked {
            return Err(revoked());
        }
        if publisher.publisher_key_id != tested.publisher_key_id() {
            return Err(invalid());
        }
        let sequence = i64::try_from(tested.release_sequence()).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        if !is_sha256(tested.signed_release_sha256()) {
            return Err(invalid());
        }
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        require_trusted_publisher(
            &mut transaction,
            tested.publisher_key_id(),
            &public_key_sha256,
        )
        .await?;
        require_current_candidate(
            &mut transaction,
            tested.publisher_key_id(),
            tested.pack_id(),
            sequence,
            tested.signed_release_sha256(),
            generation,
        )
        .await?;

        let release_updated = sqlx::query(
            "UPDATE v3_pack_releases
             SET lifecycle_state = 'ready', updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ?
               AND release_sequence = ? AND signed_release_sha256 = ?
               AND lifecycle_state = 'self_tested'",
        )
        .bind(&now)
        .bind(tested.publisher_key_id())
        .bind(tested.pack_id())
        .bind(sequence)
        .bind(tested.signed_release_sha256())
        .execute(&mut *transaction)
        .await?;
        if release_updated.rows_affected() != 1 {
            return Err(invalid_state());
        }

        let stream_updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET rollback_release_sequence = active_release_sequence,
                 active_release_sequence = ?,
                 availability = CASE
                     WHEN availability = 'disabled' THEN 'disabled'
                     ELSE 'ready'
                 END,
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND high_water_sequence = ?
               AND high_water_signed_release_sha256 = ?",
        )
        .bind(sequence)
        .bind(&now)
        .bind(tested.publisher_key_id())
        .bind(tested.pack_id())
        .bind(generation)
        .bind(sequence)
        .bind(tested.signed_release_sha256())
        .execute(&mut *transaction)
        .await?;
        if stream_updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                tested.publisher_key_id(),
                tested.pack_id(),
                generation,
            )
            .await?);
        }

        let stream = fetch_stream_by_id(
            &mut transaction,
            tested.publisher_key_id(),
            tested.pack_id(),
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn disable_pack(
        &self,
        publisher_key_id: &str,
        pack_id: &str,
        expected_generation: u64,
    ) -> Result<PackStream> {
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        let updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET availability = 'disabled', generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability = 'ready' AND active_release_sequence IS NOT NULL",
        )
        .bind(&now)
        .bind(publisher_key_id)
        .bind(pack_id)
        .bind(generation)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                publisher_key_id,
                pack_id,
                generation,
            )
            .await?);
        }
        let stream = fetch_stream_by_id(&mut transaction, publisher_key_id, pack_id).await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn enable_pack(
        &self,
        tested: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
    ) -> Result<PackStream> {
        if publisher.revoked || publisher.publisher_key_id != tested.publisher_key_id() {
            return Err(if publisher.revoked {
                revoked()
            } else {
                invalid()
            });
        }
        let sequence = i64::try_from(tested.release_sequence()).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        require_trusted_publisher(
            &mut transaction,
            tested.publisher_key_id(),
            &public_key_sha256,
        )
        .await?;
        let updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET availability = 'ready', generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability = 'disabled' AND active_release_sequence = ?
               AND EXISTS (
                   SELECT 1 FROM v3_pack_releases AS active
                   WHERE active.publisher_key_id = v3_pack_streams.publisher_key_id
                     AND active.pack_id = v3_pack_streams.pack_id
                     AND active.release_sequence = v3_pack_streams.active_release_sequence
                     AND active.signed_release_sha256 = ?
                     AND active.lifecycle_state = 'ready'
               )",
        )
        .bind(&now)
        .bind(tested.publisher_key_id())
        .bind(tested.pack_id())
        .bind(generation)
        .bind(sequence)
        .bind(tested.signed_release_sha256())
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                tested.publisher_key_id(),
                tested.pack_id(),
                generation,
            )
            .await?);
        }
        let stream = fetch_stream_by_id(
            &mut transaction,
            tested.publisher_key_id(),
            tested.pack_id(),
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }

    pub async fn rollback_pack(
        &self,
        tested: &SelfTestedPackRelease,
        publisher: &TrustedPublisherKey,
        expected_generation: u64,
    ) -> Result<PackStream> {
        if publisher.revoked || publisher.publisher_key_id != tested.publisher_key_id() {
            return Err(if publisher.revoked {
                revoked()
            } else {
                invalid()
            });
        }
        let sequence = i64::try_from(tested.release_sequence()).map_err(|_| invalid())?;
        let generation = i64::try_from(expected_generation).map_err(|_| invalid())?;
        let public_key_sha256 = hex::encode(Sha256::digest(publisher.public_key));
        let now = Utc::now().to_rfc3339();
        let mut transaction = self.pool().begin().await?;
        require_trusted_publisher(
            &mut transaction,
            tested.publisher_key_id(),
            &public_key_sha256,
        )
        .await?;
        let updated = sqlx::query(
            "UPDATE v3_pack_streams
             SET active_release_sequence = rollback_release_sequence,
                 rollback_release_sequence = active_release_sequence,
                 generation = generation + 1, updated_at = ?
             WHERE publisher_key_id = ? AND pack_id = ? AND generation = ?
               AND availability IN ('ready', 'disabled')
               AND active_release_sequence IS NOT NULL
               AND rollback_release_sequence = ?
               AND EXISTS (
                   SELECT 1 FROM v3_pack_releases AS active
                   WHERE active.publisher_key_id = v3_pack_streams.publisher_key_id
                     AND active.pack_id = v3_pack_streams.pack_id
                     AND active.release_sequence = v3_pack_streams.active_release_sequence
                     AND active.lifecycle_state = 'ready'
               )
               AND EXISTS (
                   SELECT 1 FROM v3_pack_releases AS retained
                   WHERE retained.publisher_key_id = v3_pack_streams.publisher_key_id
                     AND retained.pack_id = v3_pack_streams.pack_id
                     AND retained.release_sequence = v3_pack_streams.rollback_release_sequence
                     AND retained.signed_release_sha256 = ?
                     AND retained.lifecycle_state = 'ready'
               )",
        )
        .bind(&now)
        .bind(tested.publisher_key_id())
        .bind(tested.pack_id())
        .bind(generation)
        .bind(sequence)
        .bind(tested.signed_release_sha256())
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(stream_guard_error(
                &mut transaction,
                tested.publisher_key_id(),
                tested.pack_id(),
                generation,
            )
            .await?);
        }
        let stream = fetch_stream_by_id(
            &mut transaction,
            tested.publisher_key_id(),
            tested.pack_id(),
        )
        .await?;
        transaction.commit().await?;
        Ok(stream)
    }
}
