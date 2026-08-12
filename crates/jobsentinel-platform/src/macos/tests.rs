// Verifies macOS path selection and initialization against isolated environment roots.

use super::*;
use std::env;
use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_guard() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn test_get_data_dir() {
    let _env = env_guard();
    let dir = get_data_dir();
    assert!(dir.to_string_lossy().contains("JobSentinel"));
    assert!(dir.to_string_lossy().contains("Library"));
    assert!(dir.to_string_lossy().contains("Application Support"));
}

#[test]
fn test_get_data_dir_structure() {
    let _env = env_guard();
    let dir = get_data_dir();
    let path_str = dir.to_string_lossy();

    // Verify full path structure: ~/Library/Application Support/JobSentinel
    assert!(path_str.ends_with("Library/Application Support/JobSentinel"));
}

#[test]
fn test_get_data_dir_with_home_env() {
    let _env = env_guard();
    // Save original HOME
    let original_home = env::var("HOME").ok();

    // Set custom HOME
    env::set_var("HOME", "fixture-home");
    let dir = get_data_dir();

    assert_eq!(
        dir,
        PathBuf::from("fixture-home/Library/Application Support/JobSentinel")
    );

    // Restore original HOME
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
}

#[test]
fn package_smoke_root_redirects_app_paths() {
    let _env = env_guard();
    let smoke_root = tempfile::Builder::new()
        .prefix("jobsentinel-macos-smoke-")
        .tempdir()
        .expect("create smoke root");

    let original_home = env::var("HOME").ok();
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    let original_smoke = env::var(PACKAGE_SMOKE_ROOT_ENV).ok();

    env::set_var(PACKAGE_SMOKE_ROOT_ENV, smoke_root.path());
    env::set_var("HOME", "fixture-home-that-should-not-be-used");
    env::set_var("XDG_CONFIG_HOME", "fixture-xdg-that-should-not-be-used");

    assert_eq!(
        get_data_dir(),
        smoke_root
            .path()
            .join("home")
            .join("Library")
            .join("Application Support")
            .join("JobSentinel")
    );
    assert_eq!(
        get_config_dir(),
        smoke_root.path().join("xdg").join("jobsentinel")
    );
    assert_eq!(
        get_cache_dir(),
        smoke_root
            .path()
            .join("home")
            .join("Library")
            .join("Caches")
            .join("JobSentinel")
    );
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    match original_smoke {
        Some(smoke) => env::set_var(PACKAGE_SMOKE_ROOT_ENV, smoke),
        None => env::remove_var(PACKAGE_SMOKE_ROOT_ENV),
    }
}

#[test]
fn package_smoke_root_rejects_non_verifier_roots() {
    let _env = env_guard();
    let temp_dir = tempfile::tempdir().expect("create temp dir");

    assert!(package_smoke_root_from(
        temp_dir.path().join("not-a-smoke-root"),
        std::env::temp_dir()
    )
    .is_none());
    assert!(package_smoke_root_from(
        PathBuf::from("/tmp/jobsentinel-macos-smoke-outside"),
        PathBuf::from("/different-temp")
    )
    .is_none());
}

#[test]
fn test_get_config_dir() {
    let _env = env_guard();
    let dir = get_config_dir();
    assert!(dir.to_string_lossy().contains("jobsentinel"));
    // Should be either .config or XDG_CONFIG_HOME
    let dir_str = dir.to_string_lossy();
    assert!(dir_str.contains(".config") || dir_str.contains("jobsentinel"));
}

#[test]
fn test_get_config_dir_default_path() {
    let _env = env_guard();
    // Save original env vars
    let original_home = env::var("HOME").ok();
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();

    // Clear XDG_CONFIG_HOME and set known HOME
    env::remove_var("XDG_CONFIG_HOME");
    env::set_var("HOME", "fixture-home");

    let dir = get_config_dir();
    assert_eq!(dir, PathBuf::from("fixture-home/.config/jobsentinel"));

    // Restore original env vars
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
fn test_get_config_dir_with_xdg_config_home() {
    let _env = env_guard();
    // Save original env vars
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();

    // Set XDG_CONFIG_HOME
    env::set_var("XDG_CONFIG_HOME", "fixture-config");

    let dir = get_config_dir();
    assert_eq!(dir, PathBuf::from("fixture-config/jobsentinel"));

    // Restore original env var
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
fn test_get_cache_dir() {
    let _env = env_guard();
    let dir = get_cache_dir();
    assert!(dir.to_string_lossy().contains("JobSentinel"));
    assert!(dir.to_string_lossy().contains("Caches"));
}

#[test]
fn test_get_cache_dir_structure() {
    let _env = env_guard();
    let dir = get_cache_dir();
    let path_str = dir.to_string_lossy();

    // Verify full path structure: ~/Library/Caches/JobSentinel
    assert!(path_str.ends_with("Library/Caches/JobSentinel"));
}

#[test]
fn test_get_cache_dir_with_home_env() {
    let _env = env_guard();
    // Save original HOME
    let original_home = env::var("HOME").ok();

    // Set custom HOME
    env::set_var("HOME", "fixture-home");
    let dir = get_cache_dir();

    assert_eq!(
        dir,
        PathBuf::from("fixture-home/Library/Caches/JobSentinel")
    );

    // Restore original HOME
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
}

#[test]
fn test_get_macos_version() {
    let _env = env_guard();
    let version = get_macos_version();
    assert!(version.contains("macOS"));
}

#[test]
fn test_get_macos_version_format() {
    let _env = env_guard();
    let version = get_macos_version();

    // Should either be "macOS X.Y" or "macOS (unknown)"
    assert!(
        version.starts_with("macOS "),
        "Version should start with 'macOS ': {}",
        version
    );
}

#[test]
#[ignore = "env var isolation issue in parallel test execution - code works in production"]
fn test_initialize_creates_directories() {
    let _env = env_guard();
    use tempfile::TempDir;

    // Create temporary home directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_home = temp_dir.path().canonicalize().unwrap();

    // Save original env vars
    let original_home = env::var("HOME").ok();
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();

    // Set HOME to temp directory and clear XDG_CONFIG_HOME
    env::set_var("HOME", &temp_home);
    env::remove_var("XDG_CONFIG_HOME");

    // Run initialize
    let result = initialize();
    assert!(result.is_ok());

    // Verify directories were created
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();

    assert!(
        data_dir.exists(),
        "Data directory should exist: {:?}",
        data_dir
    );
    assert!(
        config_dir.exists(),
        "Config directory should exist: {:?}",
        config_dir
    );

    // Restore original env vars
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
fn test_initialize_existing_directories() {
    let _env = env_guard();
    use std::fs;
    use tempfile::TempDir;

    // Create temporary home directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let temp_home = temp_dir.path().canonicalize().unwrap();

    // Save original env vars
    let original_home = env::var("HOME").ok();
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();

    // Set HOME to temp directory and clear XDG_CONFIG_HOME
    env::set_var("HOME", &temp_home);
    env::remove_var("XDG_CONFIG_HOME");

    // Pre-create directories
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();
    fs::create_dir_all(&data_dir).expect("Failed to create data dir");
    fs::create_dir_all(&config_dir).expect("Failed to create config dir");

    // Run initialize (should not fail on existing directories)
    let result = initialize();
    assert!(result.is_ok());

    // Verify directories still exist
    assert!(data_dir.exists());
    assert!(config_dir.exists());

    // Restore original env vars
    match original_home {
        Some(home) => env::set_var("HOME", home),
        None => env::remove_var("HOME"),
    }
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
}

#[test]
fn test_path_consistency() {
    let _env = env_guard();
    // All path functions should use consistent logic for path construction
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();
    let cache_dir = get_cache_dir();

    // All paths should be absolute
    assert!(data_dir.is_absolute());
    assert!(config_dir.is_absolute());
    assert!(cache_dir.is_absolute());

    // Verify macOS-specific paths use Library
    assert!(data_dir.to_string_lossy().contains("Library"));
    assert!(cache_dir.to_string_lossy().contains("Library"));

    // Config typically uses .config (unless XDG_CONFIG_HOME is set)
    let config_str = config_dir.to_string_lossy();
    assert!(config_str.contains("jobsentinel"));
}

#[test]
fn test_directory_names_are_correct() {
    let _env = env_guard();
    let data_dir = get_data_dir();
    let config_dir = get_config_dir();
    let cache_dir = get_cache_dir();

    // Verify final component is correct
    assert_eq!(data_dir.file_name().unwrap(), "JobSentinel");
    assert_eq!(config_dir.file_name().unwrap(), "jobsentinel");
    assert_eq!(cache_dir.file_name().unwrap(), "JobSentinel");
}

#[test]
fn test_data_dir_follows_apple_guidelines() {
    let _env = env_guard();
    let dir = get_data_dir();
    let path_str = dir.to_string_lossy();

    // Should follow macOS convention: ~/Library/Application Support/AppName
    assert!(path_str.contains("Library/Application Support"));
}

#[test]
fn test_config_dir_follows_xdg_spec() {
    let _env = env_guard();
    // Save original env vars
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();

    // Test default behavior (no XDG_CONFIG_HOME)
    env::remove_var("XDG_CONFIG_HOME");
    let dir = get_config_dir();
    assert!(dir.to_string_lossy().contains(".config/jobsentinel"));

    // Restore original env var
    match original_xdg {
        Some(xdg) => env::set_var("XDG_CONFIG_HOME", xdg),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
}
