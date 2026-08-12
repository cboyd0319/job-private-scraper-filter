//! System Information for Feedback Reports
//!
//! Collects anonymized system info (app version, OS, arch, config summary).

use crate::application::config::Config;
use serde::Serialize;

use super::sanitizer::ConfigSummary;

/// System information for feedback reports (safe to share publicly)
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SystemInfo {
    pub app_version: String,
    pub platform: String,
    pub os_version: String,
    pub architecture: String,
}

impl SystemInfo {
    /// Get current system information
    pub(crate) fn current() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            os_version: Self::get_os_version(),
            architecture: std::env::consts::ARCH.to_string(),
        }
    }

    /// Get OS version string
    fn get_os_version() -> String {
        #[cfg(target_os = "macos")]
        {
            // macOS version detection
            use std::process::Command;
            if let Ok(output) = Command::new("sw_vers").arg("-productVersion").output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return format!("macOS {}", version.trim());
                }
            }
            "macOS (version unknown)".to_string()
        }

        #[cfg(target_os = "windows")]
        {
            // Windows version detection
            use std::process::Command;
            if let Ok(output) = Command::new("cmd").args(["/C", "ver"]).output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return version.trim().to_string();
                }
            }
            "Windows (version unknown)".to_string()
        }

        #[cfg(target_os = "linux")]
        {
            // Linux version detection
            use std::process::Command;
            if let Ok(output) = Command::new("uname").arg("-r").output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return format!("Linux {}", version.trim());
                }
            }
            "Linux (version unknown)".to_string()
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            std::env::consts::OS.to_string()
        }
    }
}

/// Generate anonymized configuration summary
pub(super) fn summarize_config(config: &Config) -> ConfigSummary {
    // Count configured notification channels
    let notifications_configured = {
        let mut count = 0;
        if config.alerts.slack.enabled {
            count += 1;
        }
        if config.alerts.email.enabled {
            count += 1;
        }
        if config.alerts.discord.enabled {
            count += 1;
        }
        if config.alerts.telegram.enabled {
            count += 1;
        }
        if config.alerts.teams.enabled {
            count += 1;
        }
        if config.alerts.desktop.enabled {
            count += 1;
        }
        count
    };

    // Count enabled scrapers
    let scrapers_enabled = {
        let mut count = config.greenhouse_urls.len() + config.lever_urls.len();
        if config.linkedin.enabled {
            count += 1;
        }
        if config.remoteok.enabled {
            count += 1;
        }
        if config.weworkremotely.enabled {
            count += 1;
        }
        if config.hn_hiring.enabled {
            count += 1;
        }
        if config.usajobs.enabled {
            count += 1;
        }
        count
    };

    ConfigSummary {
        scrapers_enabled,
        keywords_count: config.title_allowlist.len() + config.keywords_boost.len(),
        has_location_prefs: !config.location_preferences.cities.is_empty()
            || !config.location_preferences.states.is_empty()
            || !config.location_preferences.allow_remote,
        has_salary_prefs: config.salary_floor_usd > 0,
        has_blocked_companies: !config.blocked_companies.is_empty(),
        has_preferred_companies: !config.preferred_companies.is_empty(),
        notifications_configured,
        has_resume: config.use_resume_matching,
    }
}

// Note: Tauri commands moved to mod.rs to avoid macro visibility issues

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_current() {
        let info = SystemInfo::current();

        // Should have app version from Cargo.toml
        assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));

        // Should have platform
        assert!(!info.platform.is_empty());

        // Should have architecture
        assert!(!info.architecture.is_empty());
    }

    #[test]
    fn test_os_version_not_empty() {
        let version = SystemInfo::get_os_version();
        assert!(!version.is_empty());
        // Should contain OS name
        #[cfg(target_os = "macos")]
        assert!(version.contains("macOS"));
        #[cfg(target_os = "windows")]
        assert!(version.contains("Windows") || version.contains("Microsoft"));
        #[cfg(target_os = "linux")]
        assert!(version.contains("Linux"));
    }
}
