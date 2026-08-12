// Restores or quarantines desktop data during fail-closed startup recovery.

use std::{
    io::{Read, Write},
    path::Path,
};

use jobsentinel_platform::opened_file_matches_path;
use serde::Serialize;

use super::DesktopStartupError;
use crate::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupConfigRepairOutcome {
    PreservedAndReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StartupConfigRepair {
    pub outcome: StartupConfigRepairOutcome,
    pub connectivity_required: bool,
}

pub fn repair_invalid_startup_config() -> Result<StartupConfigRepair, DesktopStartupError> {
    repair_invalid_startup_config_at(&Config::default_path())
}

fn repair_invalid_startup_config_at(
    config_path: &Path,
) -> Result<StartupConfigRepair, DesktopStartupError> {
    let metadata = std::fs::symlink_metadata(config_path)
        .map_err(|_| config_repair_error("Saved settings could not be inspected"))?;
    if !metadata.file_type().is_file() {
        return Err(config_repair_error(
            "Saved settings are not eligible for recovery",
        ));
    }
    let mut source = std::fs::File::open(config_path)
        .map_err(|_| config_repair_error("Saved settings could not be inspected"))?;
    let mut content = String::new();
    source
        .read_to_string(&mut content)
        .map_err(|_| config_repair_error("Saved settings could not be inspected"))?;
    if !opened_file_matches_path(&source, config_path) || Config::from_json(&content).is_ok() {
        return Err(config_repair_error(
            "Saved settings are not eligible for recovery",
        ));
    }
    let parent = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| config_repair_error("Saved settings have no recovery location"))?;
    let preserved = parent.join(format!(
        ".jobsentinel-config-recovery-{}.json",
        uuid::Uuid::new_v4()
    ));
    let mut recovery = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&preserved)
        .map_err(|_| config_repair_error("Saved settings could not be preserved"))?;
    let prepared = recovery
        .write_all(content.as_bytes())
        .and_then(|()| recovery.sync_all())
        .and_then(|()| set_private_recovery_permissions(&recovery));
    if prepared.is_err() || !opened_file_matches_path(&source, config_path) {
        drop(recovery);
        let _ = std::fs::remove_file(&preserved);
        return Err(config_repair_error(
            "Saved settings could not be reset safely",
        ));
    }
    drop(recovery);
    drop(source);
    if std::fs::remove_file(config_path).is_err() {
        let _ = std::fs::remove_file(&preserved);
        return Err(config_repair_error(
            "Saved settings could not be reset safely",
        ));
    }
    sync_parent(config_path);

    Ok(StartupConfigRepair {
        outcome: StartupConfigRepairOutcome::PreservedAndReset,
        connectivity_required: false,
    })
}

fn set_private_recovery_permissions(file: &std::fs::File) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn config_repair_error(message: &'static str) -> DesktopStartupError {
    DesktopStartupError::Configuration(message.to_string())
}

fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        let _ = std::fs::File::open(parent).and_then(|directory| directory.sync_all());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_config_repair_preserves_the_original_without_network() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        std::fs::write(&config_path, b"{ private invalid settings").unwrap();

        let repair = repair_invalid_startup_config_at(&config_path).unwrap();

        assert_eq!(
            repair.outcome,
            StartupConfigRepairOutcome::PreservedAndReset
        );
        assert!(!repair.connectivity_required);
        assert!(!config_path.exists());
        let preserved = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".jobsentinel-config-recovery-")
            })
            .unwrap();
        assert_eq!(
            std::fs::read(preserved.path()).unwrap(),
            b"{ private invalid settings"
        );
    }

    #[test]
    fn valid_config_is_never_repaired_or_replaced() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");
        Config::first_run().save(&config_path).unwrap();

        assert!(repair_invalid_startup_config_at(&config_path).is_err());
        assert!(config_path.exists());
        assert_eq!(std::fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn invalid_hard_linked_config_is_preserved_without_mutating_its_external_alias() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = tempfile::tempdir().unwrap();
        let external = temp_dir.path().join("external.json");
        let config_path = temp_dir.path().join("config.json");
        std::fs::write(&external, b"{ private invalid settings").unwrap();
        std::fs::set_permissions(&external, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::hard_link(&external, &config_path).unwrap();

        repair_invalid_startup_config_at(&config_path).unwrap();

        assert!(!config_path.exists());
        assert_eq!(
            std::fs::metadata(&external).unwrap().permissions().mode() & 0o777,
            0o644
        );
        let preserved = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with(".jobsentinel-config-recovery-")
                })
            })
            .unwrap();
        std::fs::write(&external, b"external changed").unwrap();
        assert_eq!(
            std::fs::read(preserved).unwrap(),
            b"{ private invalid settings"
        );
    }
}
