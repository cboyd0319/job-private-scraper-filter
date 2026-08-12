// Opens and validates portable restore bundles without following unsafe filesystem links.

use super::portable_restore::{restore_error, sibling_path};
use crate::encryption::sqlite_sidecar_path;
use jobsentinel_platform::opened_file_matches_path;
use std::{
    io,
    path::{Path, PathBuf},
};

pub(super) fn require_regular_file(path: &Path) -> Result<(), sqlx::Error> {
    let metadata = std::fs::symlink_metadata(path).map_err(sqlx::Error::Io)?;
    if metadata.file_type().is_file() {
        Ok(())
    } else {
        Err(restore_error(
            "Portable restore rollback must be a regular file",
        ))
    }
}

pub(super) fn preserve_opaque_bundle(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    copy_regular_bundle(source, destination)
}

pub(super) fn publish_opaque_bundle(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    copy_regular_bundle(source, destination)
}

fn remove_linked_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

fn copy_regular_bundle(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    let pairs = opaque_bundle_pairs(source, destination);
    let mut copied = Vec::new();
    for (index, (source, destination)) in pairs.iter().enumerate() {
        let metadata = match std::fs::symlink_metadata(source) {
            Ok(metadata) => metadata,
            Err(error) if index > 0 && error.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(error) => {
                remove_linked_files(&copied);
                return Err(sqlx::Error::Io(error));
            }
        };
        if !metadata.file_type().is_file() {
            remove_linked_files(&copied);
            return Err(restore_error(
                "Portable restore quarantine must contain regular files",
            ));
        }
        let mut input = std::fs::File::open(source).map_err(sqlx::Error::Io)?;
        if !opened_file_matches_path(&input, source) {
            remove_linked_files(&copied);
            return Err(restore_error(
                "Portable restore quarantine changed while it was checked",
            ));
        }
        let mut output = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
        {
            Ok(output) => output,
            Err(error) => {
                remove_linked_files(&copied);
                return Err(sqlx::Error::Io(error));
            }
        };
        copied.push(destination.clone());
        let result = io::copy(&mut input, &mut output)
            .and_then(|_| set_private_permissions(&output))
            .and_then(|()| output.sync_all());
        if let Err(error) = result {
            drop(output);
            remove_linked_files(&copied);
            return Err(sqlx::Error::Io(error));
        }
    }
    if let Err(error) = sync_parent_checked(destination) {
        remove_linked_files(&copied);
        return Err(error);
    }
    Ok(())
}

fn set_private_permissions(file: &std::fs::File) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub(super) fn move_database_bundle(source: &Path, destination: &Path) -> Result<(), sqlx::Error> {
    let pairs = bundle_pairs(source, destination);
    let mut linked = Vec::new();
    for (source, destination) in &pairs {
        if source.exists() {
            if let Err(error) = std::fs::hard_link(source, destination) {
                for path in linked {
                    let _ = std::fs::remove_file(path);
                }
                return Err(sqlx::Error::Io(error));
            }
            linked.push(destination.clone());
        }
    }
    for (source, _) in pairs {
        remove_if_exists(&source)?;
    }
    Ok(())
}

fn bundle_pairs(source: &Path, destination: &Path) -> Vec<(PathBuf, PathBuf)> {
    ["", "-wal", "-journal"]
        .into_iter()
        .map(|suffix| {
            if suffix.is_empty() {
                (source.to_path_buf(), destination.to_path_buf())
            } else {
                (
                    sqlite_sidecar_path(source, suffix),
                    sqlite_sidecar_path(destination, suffix),
                )
            }
        })
        .collect()
}

fn opaque_bundle_pairs(source: &Path, destination: &Path) -> Vec<(PathBuf, PathBuf)> {
    ["", "-wal", "-shm", "-journal"]
        .into_iter()
        .map(|suffix| {
            if suffix.is_empty() {
                (source.to_path_buf(), destination.to_path_buf())
            } else {
                (
                    sqlite_sidecar_path(source, suffix),
                    sqlite_sidecar_path(destination, suffix),
                )
            }
        })
        .collect()
}

pub(super) fn remove_database_bundle(path: &Path) -> Result<(), sqlx::Error> {
    for suffix in ["-wal", "-shm", "-journal"] {
        remove_if_exists(&sqlite_sidecar_path(path, suffix))?;
    }
    remove_if_exists(path)
}

pub(super) fn remove_if_exists(path: &Path) -> Result<(), sqlx::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(sqlx::Error::Io(error)),
    }
}

pub(super) fn previous_path(database_path: &Path, restore_id: &str) -> PathBuf {
    sibling_path(database_path, &format!(".restore-{restore_id}.previous"))
}

pub(super) fn rollback_path(
    database_path: &Path,
    restore_id: &str,
) -> Result<PathBuf, sqlx::Error> {
    recovery_backup_path(database_path, &format!("restore_rollback_{restore_id}.db"))
}

pub(super) fn quarantine_path(
    database_path: &Path,
    restore_id: &str,
) -> Result<PathBuf, sqlx::Error> {
    recovery_backup_path(
        database_path,
        &format!("restore_quarantine_{restore_id}.db"),
    )
}

fn recovery_backup_path(database_path: &Path, name: &str) -> Result<PathBuf, sqlx::Error> {
    let parent = database_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| restore_error("Portable restore requires a database directory"))?;
    let backup_dir = parent.join("backups");
    let metadata = match std::fs::symlink_metadata(&backup_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match std::fs::create_dir(&backup_dir) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(sqlx::Error::Io(error)),
            }
            std::fs::symlink_metadata(&backup_dir).map_err(sqlx::Error::Io)?
        }
        Err(error) => return Err(sqlx::Error::Io(error)),
    };
    if !metadata.file_type().is_dir() {
        return Err(restore_error(
            "Portable restore backup directory is invalid",
        ));
    }
    jobsentinel_platform::ensure_private_dir(&backup_dir).map_err(sqlx::Error::Io)?;
    if !std::fs::symlink_metadata(&backup_dir)
        .map_err(sqlx::Error::Io)?
        .file_type()
        .is_dir()
    {
        return Err(restore_error(
            "Portable restore backup directory is invalid",
        ));
    }
    Ok(backup_dir.join(name))
}

fn sync_parent_checked(path: &Path) -> Result<(), sqlx::Error> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(sqlx::Error::Io)?;
    }
    Ok(())
}

pub(super) fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        let _ = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
    }
}
