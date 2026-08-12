//! Owns JobSentinel platform paths, private local storage, and native security adapters.
/*
This module contains platform-specific implementations and utilities.
Code is conditionally compiled based on the target OS using #[cfg(...)] attributes.

Supported platforms are Windows 11+, macOS, and Linux. This owner supplies
native paths, credential storage adapters, permission enforcement, and health
reporting used across the application.
*/

mod credential_vault_key;
mod database_key;
mod platform_health;
mod private_files;
mod secure_storage;

pub use credential_vault_key::{
    credential_vault_key_storage_policy, decode_credential_vault_key, delete_credential_vault_key,
    load_credential_vault_key, store_credential_vault_key, CredentialVaultKeyBackend,
    CredentialVaultKeyStoragePolicy, SECURE_STORAGE_UNAVAILABLE_MESSAGE,
};
pub use database_key::{load_or_create_database_key, DatabaseKeyError};
pub use platform_health::{
    inspect_platform_health, repair_platform_permissions, PackageRepairAction,
    PackageRepairActionId, PackageRepairGuidance, PackageRepairMode, PlatformHealthReport,
    PlatformPermissionAction, PlatformPermissionCheck, PlatformPermissionRepair,
    PlatformPermissionRepairOutcome, PlatformPermissionState, PlatformStorageArea,
    PLATFORM_HEALTH_SCHEMA_VERSION,
};
pub use private_files::write_file_atomic_private;
pub use secure_storage::{
    delete_device_secret, retrieve_device_secret, store_device_secret, SecureStorageError,
};

/// Service namespace for JobSentinel device secure-storage entries.
pub const SECURE_STORAGE_SERVICE: &str = "JobSentinel";

#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

use std::path::{Path, PathBuf};

/// Get the platform-specific application data directory
///
/// - Windows: %LOCALAPPDATA%\JobSentinel
/// - macOS: ~/Library/Application Support/JobSentinel
/// - Linux: ~/.local/share/jobsentinel
pub fn get_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        windows::get_data_dir()
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_data_dir()
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_data_dir()
    }
}

/// Get the platform-specific configuration directory
///
/// - Windows: %APPDATA%\JobSentinel
/// - macOS: ~/.config/jobsentinel
/// - Linux: ~/.config/jobsentinel
pub fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        windows::get_config_dir()
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_config_dir()
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_config_dir()
    }
}

/// Get the platform-specific application cache directory.
///
/// - Windows: %LOCALAPPDATA%\JobSentinel\Cache
/// - macOS: ~/Library/Caches/JobSentinel
/// - Linux: ~/.cache/jobsentinel
pub fn get_cache_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        windows::get_cache_dir()
    }

    #[cfg(target_os = "macos")]
    {
        macos::get_cache_dir()
    }

    #[cfg(target_os = "linux")]
    {
        linux::get_cache_dir()
    }
}

/// Initialize platform-specific features
///
/// This should be called once during application startup.
pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        windows::initialize()?;
    }

    #[cfg(target_os = "macos")]
    {
        macos::initialize()?;
    }

    #[cfg(target_os = "linux")]
    {
        linux::initialize()?;
    }

    Ok(())
}

/// Return the isolated macOS package-smoke root when the verifier configured a valid one.
#[cfg(target_os = "macos")]
pub fn package_smoke_root() -> Option<PathBuf> {
    macos::package_smoke_root()
}

/// Create an app-owned directory and keep it private on Unix platforms.
pub fn ensure_private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    set_private_dir_permissions(path)?;
    Ok(())
}

/// Create an app-owned directory and keep every existing child private.
///
/// Symlinks are ignored so a user-controlled link inside app storage cannot
/// make startup chmod files outside the app-owned tree.
pub fn ensure_private_dir_tree(path: &Path) -> std::io::Result<()> {
    #[cfg(not(unix))]
    {
        return ensure_private_dir(path);
    }

    #[cfg(unix)]
    {
        private_files::ensure_private_dir_tree_unix(path)
    }
}

/// Keep an app-owned file private on Unix platforms.
pub fn ensure_private_file(path: &Path) -> std::io::Result<()> {
    if path.exists() {
        set_private_file_permissions(path)?;
    }
    Ok(())
}

/// Apply private file modes to SQLite sidecar files when they exist.
pub fn ensure_private_sqlite_files(db_path: &Path) -> std::io::Result<()> {
    ensure_private_file(db_path)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", db_path.display(), suffix));
        ensure_private_file(&sidecar)?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::*;

    #[cfg(unix)]
    fn resolved_temp_root(temp_dir: &tempfile::TempDir) -> PathBuf {
        temp_dir.path().canonicalize().unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn ensure_private_dir_tree_tightens_existing_children() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let root = resolved_temp_root(&temp_dir).join("JobSentinel");
        let nested = root.join("backups");
        let db_path = nested.join("backup.db");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(&db_path, b"backup").unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        ensure_private_dir_tree(&root).unwrap();

        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&nested).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&db_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_private_dir_tree_does_not_follow_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = resolved_temp_root(&temp_dir);
        let root = temp_root.join("JobSentinel");
        let external = temp_root.join("external.txt");
        let link = root.join("linked.txt");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&external, b"external").unwrap();
        std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o644)).unwrap();
        symlink(&external, &link).unwrap();

        ensure_private_dir_tree(&root).unwrap();

        assert_eq!(
            std::fs::metadata(&external).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn ensure_private_dir_tree_rejects_a_symlinked_root_without_changing_external_modes() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = resolved_temp_root(&temp_dir);
        let external = temp_root.join("external");
        let root = temp_root.join("JobSentinel");
        std::fs::create_dir(&external).unwrap();
        std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o755)).unwrap();
        symlink(&external, &root).unwrap();

        let error = ensure_private_dir_tree(&root).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(&external).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }

    #[cfg(unix)]
    #[test]
    fn ensure_private_dir_tree_rejects_a_symlinked_ancestor_without_writing_outside() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = resolved_temp_root(&temp_dir);
        let external = temp_root.join("external");
        let linked_parent = temp_root.join("linked-parent");
        std::fs::create_dir(&external).unwrap();
        symlink(&external, &linked_parent).unwrap();
        let root = linked_parent.join("JobSentinel");

        let error = ensure_private_dir_tree(&root).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(!external.join("JobSentinel").exists());
    }

    #[cfg(unix)]
    #[test]
    fn ensure_private_dir_tree_rejects_hard_linked_children_without_changing_external_modes() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let temp_root = resolved_temp_root(&temp_dir);
        let root = temp_root.join("JobSentinel");
        let external = temp_root.join("shared-tool");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(&external, b"shared").unwrap();
        std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::hard_link(&external, root.join("linked-tool")).unwrap();

        let error = ensure_private_dir_tree(&root).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(&external).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
}
