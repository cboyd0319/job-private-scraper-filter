use super::{
    portable_backup::export_encrypted_to_encrypted,
    portable_restore::{
        read_restore_request, remove_sqlite_sidecars_checked, restore_error, restore_request_path,
        restore_stage_path, sibling_path, write_restore_request, OwnedDatabaseFile, RestorePhase,
        RestoreRequest,
    },
    portable_restore_bundle::{
        move_database_bundle, preserve_opaque_bundle, previous_path, publish_opaque_bundle,
        quarantine_path, remove_database_bundle, remove_if_exists, require_regular_file,
        rollback_path, sync_parent,
    },
    Database, PortableBackupInfo,
};
use crate::encryption::{
    connect_encrypted_pool, connect_encrypted_read_only_pool, load_or_create_database_key,
    sqlite_sidecar_path,
};
use sqlx::SqlitePool;
use std::path::Path;

impl Database {
    pub async fn connect_with_staged_restore(path: &Path) -> Result<Self, sqlx::Error> {
        let promoted = Self::promote_staged_restore(path).await?;
        let database = match Self::connect(path).await {
            Ok(database) => database,
            Err(_) if promoted => {
                Self::rollback_staged_restore(path).await?;
                let database = Self::connect(path).await?;
                database.migrate().await?;
                return Ok(database);
            }
            Err(error) => return Err(error),
        };
        if let Err(error) = database.migrate().await {
            database.pool.close().await;
            drop(database);
            if promoted {
                Self::rollback_staged_restore(path).await?;
                let database = Self::connect(path).await?;
                database.migrate().await?;
                return Ok(database);
            }
            return Err(error);
        }
        if promoted {
            database.finish_staged_restore().await?;
        }
        Ok(database)
    }

    pub async fn promote_staged_restore(database_path: &Path) -> Result<bool, sqlx::Error> {
        let _owner = Self::acquire_owner_lock(database_path)?;
        let Some(mut request) = read_restore_request(database_path)? else {
            return Ok(false);
        };
        let key = load_or_create_database_key().await?;
        match request.phase {
            RestorePhase::Promoted => {
                let finished = verify_promoted(database_path, &key, &request).await?;
                if finished {
                    cleanup_finished_restore(database_path, &request)?;
                }
                Ok(!finished)
            }
            RestorePhase::Promoting => {
                rollback_from_available_copy(database_path, &key, &request).await?;
                Ok(false)
            }
            RestorePhase::Ready => {
                let stage = restore_stage_path(database_path);
                let info = Self::inspect_portable_backup(&stage, &key).await?;
                if info.backup_id != request.backup_id {
                    return Err(restore_error(
                        "Staged portable restore does not match its request",
                    ));
                }
                verify_restore_database(&stage, &key, &request.restore_id, true).await?;
                ensure_restore_rollback(database_path, &key, &request).await?;
                request.phase = RestorePhase::Promoting;
                write_restore_request(database_path, &request)?;
                if let Err(error) =
                    promote_ready_database(database_path, &stage, &key, &request, &info).await
                {
                    rollback_from_available_copy(database_path, &key, &request).await?;
                    return Err(error);
                }
                request.phase = RestorePhase::Promoted;
                write_restore_request(database_path, &request)?;
                Ok(true)
            }
        }
    }

    pub async fn finish_staged_restore(&self) -> Result<(), sqlx::Error> {
        let db_path = self
            .db_path
            .as_deref()
            .ok_or_else(|| restore_error("Portable restore requires an on-disk database"))?;
        let Some(request) = read_restore_request(db_path)? else {
            return Ok(());
        };
        if request.phase != RestorePhase::Promoted {
            return Err(restore_error("Portable restore is not ready to finish"));
        }
        let mut transaction = self.pool.begin().await?;
        let outcome: String = sqlx::query_scalar(
            "SELECT outcome FROM v3_recovery_operations
             WHERE operation_id = ? AND operation_kind = 'restore'",
        )
        .bind(&request.restore_id)
        .fetch_one(&mut *transaction)
        .await?;
        if outcome == "started" {
            sqlx::query(
                "UPDATE v3_recovery_operations
                 SET outcome = 'succeeded', error_kind = NULL,
                     completed_at = datetime('now')
                 WHERE operation_id = ? AND outcome = 'started'",
            )
            .bind(&request.restore_id)
            .execute(&mut *transaction)
            .await?;
        } else if outcome != "succeeded" {
            return Err(restore_error(
                "Portable restore provenance is not finishable",
            ));
        }
        sqlx::query("DROP TABLE IF EXISTS portable_backup_manifest")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        cleanup_finished_restore(db_path, &request)
    }

    pub async fn rollback_staged_restore(database_path: &Path) -> Result<bool, sqlx::Error> {
        let _owner = Self::acquire_owner_lock(database_path)?;
        let Some(request) = read_restore_request(database_path)? else {
            return Ok(false);
        };
        if request.phase == RestorePhase::Ready {
            return Err(restore_error("Portable restore has not been promoted"));
        }
        let key = load_or_create_database_key().await?;
        rollback_from_available_copy(database_path, &key, &request).await?;
        Ok(true)
    }
}

async fn promote_ready_database(
    database_path: &Path,
    stage: &Path,
    key: &str,
    request: &RestoreRequest,
    info: &PortableBackupInfo,
) -> Result<(), sqlx::Error> {
    let previous = previous_path(database_path, &request.restore_id);
    if previous.exists() {
        return Err(restore_error(
            "Portable restore previous database already exists",
        ));
    }
    remove_if_exists(&sqlite_sidecar_path(database_path, "-shm"))?;
    move_database_bundle(database_path, &previous)?;
    std::fs::hard_link(stage, database_path).map_err(sqlx::Error::Io)?;
    sync_parent(database_path);
    let published = Database::inspect_portable_backup(database_path, key).await?;
    if published != *info {
        return Err(restore_error(
            "Promoted portable restore verification failed",
        ));
    }
    verify_restore_database(database_path, key, &request.restore_id, true).await
}

async fn ensure_restore_rollback(
    database_path: &Path,
    key: &str,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    let rollback_path = rollback_path(database_path, &request.restore_id)?;
    if rollback_path.exists() {
        require_regular_file(&rollback_path)?;
        if verify_restore_database(&rollback_path, key, &request.restore_id, false)
            .await
            .is_ok()
        {
            return Ok(());
        }
        remove_database_bundle(&rollback_path)?;
    }
    let rollback = OwnedDatabaseFile::create(rollback_path.clone())?;
    let prepared = async {
        export_encrypted_to_encrypted(database_path, key, &rollback_path, key).await?;
        verify_restore_database(&rollback_path, key, &request.restore_id, false).await?;
        std::fs::File::open(&rollback_path)
            .and_then(|file| file.sync_all())
            .map_err(sqlx::Error::Io)?;
        Ok::<(), sqlx::Error>(())
    }
    .await;
    if prepared.is_err() {
        drop(rollback);
        remove_database_bundle(&rollback_path)?;
        let quarantine = quarantine_path(database_path, &request.restore_id)?;
        remove_database_bundle(&quarantine)?;
        preserve_opaque_bundle(database_path, &quarantine)?;
        return Ok(());
    }
    sync_parent(&rollback_path);
    rollback.keep();
    Ok(())
}

async fn verify_promoted(
    database_path: &Path,
    key: &str,
    request: &RestoreRequest,
) -> Result<bool, sqlx::Error> {
    let pool = connect_encrypted_pool(database_path, key, false).await?;
    let check: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(&pool)
        .await?;
    let outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM v3_recovery_operations
         WHERE operation_id = ? AND operation_kind = 'restore'",
    )
    .bind(&request.restore_id)
    .fetch_one(&pool)
    .await?;
    let manifest: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name = 'portable_backup_manifest'",
    )
    .fetch_one(&pool)
    .await?;
    let matching_manifest = if manifest == 1 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM portable_backup_manifest
             WHERE backup_id = ?",
        )
        .bind(&request.backup_id)
        .fetch_one(&pool)
        .await?
    } else {
        0
    };
    checkpoint_and_close(pool).await?;
    remove_sqlite_sidecars_checked(database_path).map_err(sqlx::Error::Io)?;
    if !check.trim().eq_ignore_ascii_case("ok") {
        return Err(restore_error(
            "Promoted portable restore verification failed",
        ));
    }
    if outcome == "succeeded" && manifest == 0 {
        return Ok(true);
    }
    if outcome != "started" || manifest != 1 || matching_manifest != 1 {
        return Err(restore_error(
            "Promoted portable restore verification failed",
        ));
    }
    Ok(false)
}

async fn verify_restore_database(
    path: &Path,
    key: &str,
    restore_id: &str,
    expect_manifest: bool,
) -> Result<(), sqlx::Error> {
    let pool = connect_encrypted_read_only_pool(path, key).await?;
    let check: String = sqlx::query_scalar("PRAGMA quick_check")
        .fetch_one(&pool)
        .await?;
    let operation: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v3_recovery_operations
         WHERE operation_id = ? AND operation_kind = 'restore'
           AND outcome IN ('started', 'failed')",
    )
    .bind(restore_id)
    .fetch_one(&pool)
    .await?;
    let manifest: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name = 'portable_backup_manifest'",
    )
    .fetch_one(&pool)
    .await?;
    pool.close().await;
    if check.trim().eq_ignore_ascii_case("ok")
        && operation == 1
        && (manifest == 1) == expect_manifest
    {
        Ok(())
    } else {
        Err(restore_error(
            "Portable restore database verification failed",
        ))
    }
}

async fn rollback_from_available_copy(
    database_path: &Path,
    key: &str,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    let rollback = rollback_path(database_path, &request.restore_id)?;
    if rollback.exists()
        && verify_restore_database(&rollback, key, &request.restore_id, false)
            .await
            .is_ok()
    {
        rollback_from_verified_copy(database_path, key, request).await
    } else {
        rollback_from_opaque_copy(database_path, request)
    }
}

async fn rollback_from_verified_copy(
    database_path: &Path,
    key: &str,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    let rollback = rollback_path(database_path, &request.restore_id)?;
    require_regular_file(&rollback)?;
    verify_restore_database(&rollback, key, &request.restore_id, false).await?;
    if database_path.exists()
        && database_has_manifest(database_path, key)
            .await
            .unwrap_or(true)
    {
        let failed = sibling_path(
            database_path,
            &format!(".restore-{}.failed", request.restore_id),
        );
        if !failed.exists() {
            move_database_bundle(database_path, &failed)?;
        } else {
            remove_database_bundle(database_path)?;
        }
    }
    if !database_path.exists() {
        remove_database_bundle(database_path)?;
        publish_rollback_copy(&rollback, database_path, key, &request.restore_id).await?;
    }
    verify_restore_database(database_path, key, &request.restore_id, false).await?;
    let pool = connect_encrypted_pool(database_path, key, false).await?;
    sqlx::query(
        "UPDATE v3_recovery_operations
         SET outcome = 'failed', error_kind = 'protocol',
             completed_at = datetime('now')
         WHERE operation_id = ? AND operation_kind = 'restore'
           AND outcome IN ('started', 'cancelled')",
    )
    .bind(&request.restore_id)
    .execute(&pool)
    .await?;
    checkpoint_and_close(pool).await?;
    remove_database_bundle(&previous_path(database_path, &request.restore_id))?;
    remove_if_exists(&restore_stage_path(database_path))?;
    std::fs::remove_file(restore_request_path(database_path)).map_err(sqlx::Error::Io)?;
    sync_parent(database_path);
    Ok(())
}

fn rollback_from_opaque_copy(
    database_path: &Path,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    let quarantine = quarantine_path(database_path, &request.restore_id)?;
    require_regular_file(&quarantine)?;
    remove_database_bundle(database_path)?;
    remove_database_bundle(&previous_path(database_path, &request.restore_id))?;
    publish_opaque_bundle(&quarantine, database_path)?;
    remove_if_exists(&restore_stage_path(database_path))?;
    remove_if_exists(&restore_request_path(database_path))?;
    sync_parent(database_path);
    Ok(())
}

async fn publish_rollback_copy(
    rollback: &Path,
    database_path: &Path,
    key: &str,
    restore_id: &str,
) -> Result<(), sqlx::Error> {
    let temporary = sibling_path(
        database_path,
        &format!(".restore-{restore_id}.rollback-publish"),
    );
    remove_database_bundle(&temporary)?;
    let published = OwnedDatabaseFile::create(temporary.clone())?;
    std::fs::copy(rollback, &temporary).map_err(sqlx::Error::Io)?;
    std::fs::File::open(&temporary)
        .and_then(|file| file.sync_all())
        .map_err(sqlx::Error::Io)?;
    verify_restore_database(&temporary, key, restore_id, false).await?;
    std::fs::rename(&temporary, database_path).map_err(sqlx::Error::Io)?;
    sync_parent(database_path);
    drop(published);
    Ok(())
}

fn cleanup_finished_restore(
    database_path: &Path,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    remove_database_bundle(&previous_path(database_path, &request.restore_id))?;
    remove_if_exists(&restore_stage_path(database_path))?;
    remove_if_exists(&restore_request_path(database_path))?;
    sync_parent(database_path);
    Ok(())
}

async fn database_has_manifest(path: &Path, key: &str) -> Result<bool, sqlx::Error> {
    let pool = connect_encrypted_read_only_pool(path, key).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name = 'portable_backup_manifest'",
    )
    .fetch_one(&pool)
    .await?;
    pool.close().await;
    Ok(count == 1)
}

async fn checkpoint_and_close(pool: SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}
