use super::{portable_backup::export_encrypted_to_encrypted, Database, PortableBackupInfo};
use crate::encryption::{connect_encrypted_pool, load_or_create_database_key, sqlite_sidecar_path};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const RESTORE_FORMAT_VERSION: i64 = 1;
const MAX_RESTORE_MARKER_BYTES: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RestorePhase {
    Ready,
    Promoting,
    Promoted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestoreRequest {
    pub(super) format_version: i64,
    pub(super) restore_id: String,
    pub(super) backup_id: String,
    pub(super) requested_at: String,
    pub(super) phase: RestorePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortableRestoreStatus {
    None,
    Incomplete,
    Invalid,
    Ready,
    Promoting,
    Promoted,
}

impl Database {
    pub async fn stage_portable_restore(
        &self,
        backup: &Path,
        passphrase: &str,
    ) -> Result<PortableBackupInfo, sqlx::Error> {
        let info = Self::inspect_portable_backup(backup, passphrase).await?;
        let db_path = self
            .db_path
            .as_deref()
            .ok_or_else(|| restore_error("Portable restore requires an on-disk database"))?;
        let request = new_restore_request(&info);
        self.start_restore_operation(&request.restore_id, info.migration_sequence)
            .await?;
        let result =
            Self::stage_portable_restore_files(backup, passphrase, db_path, &request, &info).await;
        if let Err(error) = &result {
            let _ = self
                .finish_restore_operation(
                    &request.restore_id,
                    "failed",
                    Some(crate::database_error_kind(error)),
                )
                .await;
        }
        result.map(|()| info)
    }

    pub async fn stage_portable_restore_at(
        database_path: &Path,
        backup: &Path,
        passphrase: &str,
    ) -> Result<PortableBackupInfo, sqlx::Error> {
        let _owner = Self::acquire_owner_lock(database_path)?;
        let info = Self::inspect_portable_backup(backup, passphrase).await?;
        let request = new_restore_request(&info);
        Self::stage_portable_restore_files(backup, passphrase, database_path, &request, &info)
            .await?;
        Ok(info)
    }

    async fn stage_portable_restore_files(
        backup: &Path,
        passphrase: &str,
        database_path: &Path,
        request: &RestoreRequest,
        info: &PortableBackupInfo,
    ) -> Result<(), sqlx::Error> {
        let stage = restore_stage_path(database_path);
        let marker = restore_request_path(database_path);
        if path_entry_exists(&stage)? || path_entry_exists(&marker)? {
            return Err(restore_error("A portable restore is already staged"));
        }
        let key = load_or_create_database_key().await?;
        let stage_file = OwnedDatabaseFile::create(stage.clone())?;
        export_encrypted_to_encrypted(backup, passphrase, &stage, &key).await?;
        let pool = connect_encrypted_pool(&stage, &key, false).await?;
        insert_restore_operation(
            &pool,
            &request.restore_id,
            info.migration_sequence,
            info.migration_sequence,
        )
        .await?;
        checkpoint_and_close(pool, &stage).await?;
        let staged = Self::inspect_portable_backup(&stage, &key).await?;
        if staged != *info {
            return Err(restore_error("Staged portable restore verification failed"));
        }
        std::fs::File::open(&stage)
            .and_then(|file| file.sync_all())
            .map_err(sqlx::Error::Io)?;
        publish_restore_request(&marker, request)?;
        stage_file.keep();
        Ok(())
    }

    pub async fn cancel_staged_restore(&self) -> Result<bool, sqlx::Error> {
        let db_path = self
            .db_path
            .as_deref()
            .ok_or_else(|| restore_error("Portable restore requires an on-disk database"))?;
        let (cancelled, request) = revoke_staged_restore(db_path)?;
        if let Some(request) = request {
            self.finish_restore_operation(&request.restore_id, "cancelled", None)
                .await?;
        }
        Ok(cancelled)
    }

    pub fn cancel_staged_restore_at(database_path: &Path) -> Result<bool, sqlx::Error> {
        let _owner = Self::acquire_owner_lock(database_path)?;
        revoke_staged_restore(database_path).map(|(cancelled, _)| cancelled)
    }

    pub fn staged_restore_status(
        database_path: &Path,
    ) -> Result<PortableRestoreStatus, sqlx::Error> {
        let request = match read_restore_request(database_path) {
            Ok(request) => request,
            Err(_) if repairable_invalid_restore_request(database_path)? => {
                return Ok(PortableRestoreStatus::Invalid);
            }
            Err(error) => return Err(error),
        };
        Ok(match request {
            Some(request) => match request.phase {
                RestorePhase::Ready => PortableRestoreStatus::Ready,
                RestorePhase::Promoting => PortableRestoreStatus::Promoting,
                RestorePhase::Promoted => PortableRestoreStatus::Promoted,
            },
            None if path_entry_exists(&restore_stage_path(database_path))? => {
                PortableRestoreStatus::Incomplete
            }
            None => PortableRestoreStatus::None,
        })
    }

    async fn start_restore_operation(
        &self,
        restore_id: &str,
        target_migration: i64,
    ) -> Result<(), sqlx::Error> {
        let source_migration: i64 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations")
                .fetch_one(&self.pool)
                .await?;
        insert_restore_operation(&self.pool, restore_id, source_migration, target_migration).await
    }

    async fn finish_restore_operation(
        &self,
        restore_id: &str,
        outcome: &str,
        error_kind: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let updated = sqlx::query(
            "UPDATE v3_recovery_operations
             SET outcome = ?, error_kind = ?, completed_at = datetime('now')
             WHERE operation_id = ? AND operation_kind = 'restore'
               AND outcome = 'started'",
        )
        .bind(outcome)
        .bind(error_kind)
        .bind(restore_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 1 {
            Ok(())
        } else {
            Err(restore_error("Portable restore provenance update failed"))
        }
    }

    pub(super) async fn reconcile_interrupted_recovery_operations(
        &self,
    ) -> Result<(), sqlx::Error> {
        let active_restore = match self.db_path.as_deref() {
            Some(path) => read_restore_request(path)?.map(|request| request.restore_id),
            None => None,
        };
        sqlx::query(
            "UPDATE v3_recovery_operations
             SET outcome = 'cancelled', completed_at = datetime('now')
             WHERE outcome = 'started'
               AND (operation_kind != 'restore' OR ? IS NULL OR operation_id != ?)",
        )
        .bind(active_restore.as_deref())
        .bind(active_restore.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn new_restore_request(info: &PortableBackupInfo) -> RestoreRequest {
    RestoreRequest {
        format_version: RESTORE_FORMAT_VERSION,
        restore_id: Uuid::new_v4().to_string(),
        backup_id: info.backup_id.clone(),
        requested_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        phase: RestorePhase::Ready,
    }
}

fn revoke_staged_restore(
    database_path: &Path,
) -> Result<(bool, Option<RestoreRequest>), sqlx::Error> {
    let request = match read_restore_request(database_path) {
        Ok(request) => request,
        Err(_) if repairable_invalid_restore_request(database_path)? => {
            preserve_invalid_restore(database_path)?;
            return Ok((true, None));
        }
        Err(error) => return Err(error),
    };
    let Some(request) = request else {
        let stage = restore_stage_path(database_path);
        if !path_entry_exists(&stage)? {
            return Ok((false, None));
        }
        remove_owned_database(&stage).map_err(sqlx::Error::Io)?;
        return Ok((true, None));
    };
    if request.phase != RestorePhase::Ready {
        return Err(restore_error("Portable restore can no longer be cancelled"));
    }
    std::fs::remove_file(restore_request_path(database_path)).map_err(sqlx::Error::Io)?;
    remove_owned_database(&restore_stage_path(database_path)).map_err(sqlx::Error::Io)?;
    Ok((true, Some(request)))
}

fn repairable_invalid_restore_request(database_path: &Path) -> Result<bool, sqlx::Error> {
    let marker = restore_request_path(database_path);
    let metadata = std::fs::symlink_metadata(&marker).map_err(sqlx::Error::Io)?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_RESTORE_MARKER_BYTES {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(marker)
        .map_err(sqlx::Error::Io)?
        .take(MAX_RESTORE_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(sqlx::Error::Io)?;
    Ok(bytes.len() as u64 <= MAX_RESTORE_MARKER_BYTES)
}

fn preserve_invalid_restore(database_path: &Path) -> Result<(), sqlx::Error> {
    let repair_id = Uuid::new_v4();
    let marker = restore_request_path(database_path);
    let stage = restore_stage_path(database_path);
    let mut moves = vec![(
        marker,
        sibling_path(
            database_path,
            &format!(".restore-invalid-{repair_id}.request.json"),
        ),
    )];
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let source = if suffix.is_empty() {
            stage.clone()
        } else {
            sqlite_sidecar_path(&stage, suffix)
        };
        match std::fs::symlink_metadata(&source) {
            Ok(metadata) if metadata.file_type().is_file() => moves.push((
                source,
                sibling_path(
                    database_path,
                    &format!(".restore-invalid-{repair_id}.stage{suffix}"),
                ),
            )),
            Ok(_) => {
                return Err(restore_error("Invalid restore files require manual review"));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(sqlx::Error::Io(error)),
        }
    }
    for (source, destination) in &moves {
        let metadata = std::fs::symlink_metadata(source).map_err(sqlx::Error::Io)?;
        if !metadata.file_type().is_file() || path_entry_exists(destination)? {
            return Err(restore_error("Invalid restore files require manual review"));
        }
        jobsentinel_platform::ensure_private_file(source).map_err(sqlx::Error::Io)?;
    }
    let mut preserved = Vec::new();
    for (source, destination) in &moves {
        if let Err(error) = std::fs::rename(source, destination) {
            for (source, destination) in preserved.into_iter().rev() {
                let _ = std::fs::rename(destination, source);
            }
            return Err(sqlx::Error::Io(error));
        }
        preserved.push((source, destination));
    }
    sync_parent(database_path);
    Ok(())
}

fn path_entry_exists(path: &Path) -> Result<bool, sqlx::Error> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(sqlx::Error::Io(error)),
    }
}

async fn insert_restore_operation(
    pool: &SqlitePool,
    restore_id: &str,
    source_migration: i64,
    target_migration: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO v3_recovery_operations(
            operation_id, operation_kind, outcome,
            source_migration_sequence, target_migration_sequence
         ) VALUES (?, 'restore', 'started', ?, ?)",
    )
    .bind(restore_id)
    .bind(source_migration)
    .bind(target_migration)
    .execute(pool)
    .await?;
    Ok(())
}

async fn checkpoint_and_close(pool: SqlitePool, path: &Path) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;
    pool.close().await;
    remove_sqlite_sidecars_checked(path).map_err(sqlx::Error::Io)
}

pub(super) fn read_restore_request(
    database_path: &Path,
) -> Result<Option<RestoreRequest>, sqlx::Error> {
    let marker = restore_request_path(database_path);
    let metadata = match std::fs::symlink_metadata(&marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(sqlx::Error::Io(error)),
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_RESTORE_MARKER_BYTES {
        return Err(restore_error("Portable restore request is invalid"));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(marker)
        .map_err(sqlx::Error::Io)?
        .take(MAX_RESTORE_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(sqlx::Error::Io)?;
    if bytes.len() as u64 > MAX_RESTORE_MARKER_BYTES {
        return Err(restore_error("Portable restore request is invalid"));
    }
    let request: RestoreRequest = serde_json::from_slice(&bytes)
        .map_err(|_| restore_error("Portable restore request is invalid"))?;
    let identifiers_valid = Uuid::parse_str(&request.restore_id)
        .is_ok_and(|id| id.hyphenated().to_string() == request.restore_id)
        && Uuid::parse_str(&request.backup_id)
            .is_ok_and(|id| id.hyphenated().to_string() == request.backup_id);
    if request.format_version != RESTORE_FORMAT_VERSION
        || !identifiers_valid
        || chrono::DateTime::parse_from_rfc3339(&request.requested_at).is_err()
    {
        return Err(restore_error("Portable restore request is invalid"));
    }
    Ok(Some(request))
}

fn publish_restore_request(path: &Path, request: &RestoreRequest) -> Result<(), sqlx::Error> {
    let content = serde_json::to_vec(request)
        .map_err(|_| restore_error("Portable restore request could not be encoded"))?;
    let temporary = path.with_file_name(format!(
        ".jobsentinel_restore_request_{}.tmp",
        Uuid::new_v4()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(sqlx::Error::Io)?;
    file.write_all(&content).map_err(sqlx::Error::Io)?;
    file.sync_all().map_err(sqlx::Error::Io)?;
    jobsentinel_platform::ensure_private_file(&temporary).map_err(sqlx::Error::Io)?;
    let result = std::fs::hard_link(&temporary, path).map_err(sqlx::Error::Io);
    let _ = std::fs::remove_file(temporary);
    sync_parent(path);
    result
}

pub(super) fn write_restore_request(
    database_path: &Path,
    request: &RestoreRequest,
) -> Result<(), sqlx::Error> {
    let content = serde_json::to_string(request)
        .map_err(|_| restore_error("Portable restore request could not be encoded"))?;
    jobsentinel_platform::write_file_atomic_private(&restore_request_path(database_path), &content)
        .map_err(sqlx::Error::Io)
}

pub(super) fn remove_sqlite_sidecars_checked(path: &Path) -> std::io::Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = sqlite_sidecar_path(path, suffix);
        match std::fs::remove_file(sidecar) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(super) fn remove_owned_database(path: &Path) -> std::io::Result<()> {
    remove_sqlite_sidecars_checked(path)?;
    std::fs::remove_file(path)
}

pub(super) fn restore_request_path(database_path: &Path) -> PathBuf {
    sibling_path(database_path, ".restore-request.json")
}

pub(super) fn restore_stage_path(database_path: &Path) -> PathBuf {
    sibling_path(database_path, ".restore-stage")
}

pub(super) fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

pub(super) struct OwnedDatabaseFile {
    pub(super) path: PathBuf,
    keep: bool,
}

impl OwnedDatabaseFile {
    pub(super) fn create(path: PathBuf) -> Result<Self, sqlx::Error> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(sqlx::Error::Io)?;
        if let Err(error) = jobsentinel_platform::ensure_private_file(&path) {
            let _ = std::fs::remove_file(&path);
            return Err(sqlx::Error::Io(error));
        }
        Ok(Self { path, keep: false })
    }

    pub(super) fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for OwnedDatabaseFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = remove_owned_database(&self.path);
        }
    }
}

pub(super) fn restore_error(message: &'static str) -> sqlx::Error {
    sqlx::Error::Protocol(message.into())
}

fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        let _ = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
    }
}
