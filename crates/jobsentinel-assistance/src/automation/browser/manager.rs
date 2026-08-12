//! Manages isolated browser profiles, browser lifecycle, and guided page creation.

use anyhow::{Context, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;
use jobsentinel_platform as platforms;
use jobsentinel_security::sanitize_url_for_logging;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::page::AutomationPage;

const BROWSER_CONFIG_ERROR: &str = "Could not prepare browser settings for guided form filling";
const AUTOMATION_BROWSER_PROFILE_DIR: &str = "automation-browser-profiles";
const AUTOMATION_BROWSER_PROFILE_PREFIX: &str = "profile-";
const BROWSER_PAGE_CLOSE_TIMEOUT: Duration = Duration::from_secs(2);
const BROWSER_PROCESS_CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

fn automation_browser_launch_args() -> &'static [&'static str] {
    &[]
}

fn automation_browser_profile_root() -> PathBuf {
    platforms::get_cache_dir().join(AUTOMATION_BROWSER_PROFILE_DIR)
}

fn create_automation_browser_profile_dir() -> Result<PathBuf> {
    create_automation_browser_profile_dir_in(&automation_browser_profile_root())
}

fn create_automation_browser_profile_dir_in(root: &Path) -> Result<PathBuf> {
    platforms::ensure_private_dir_tree(root).map_err(|_| anyhow::anyhow!(BROWSER_CONFIG_ERROR))?;

    let profile_dir = root.join(format!(
        "{}{}",
        AUTOMATION_BROWSER_PROFILE_PREFIX,
        Uuid::new_v4()
    ));
    platforms::ensure_private_dir_tree(&profile_dir)
        .map_err(|_| anyhow::anyhow!(BROWSER_CONFIG_ERROR))?;

    Ok(profile_dir)
}

fn cleanup_automation_browser_profile_dir(profile_dir: &Path) {
    match std::fs::remove_dir_all(profile_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_error) => {
            tracing::debug!("Could not remove automation browser profile directory");
        }
    }
}

fn automation_browser_config(profile_dir: &Path) -> Result<BrowserConfig> {
    let mut config_builder = BrowserConfig::builder()
        .window_size(1280, 900)
        .user_data_dir(profile_dir)
        // Visible mode - user can see form being filled
        .with_head();

    for arg in automation_browser_launch_args() {
        config_builder = config_builder.arg(*arg);
    }

    config_builder
        .build()
        .map_err(|_| anyhow::anyhow!(BROWSER_CONFIG_ERROR))
}

struct BrowserSession {
    browser: Browser,
    profile_dir: PathBuf,
}

async fn close_browser_session(session: &mut BrowserSession) {
    if let Ok(pages) = session.browser.pages().await {
        for page in pages {
            let _ = tokio::time::timeout(BROWSER_PAGE_CLOSE_TIMEOUT, page.close()).await;
        }
    }

    if tokio::time::timeout(BROWSER_PROCESS_CLOSE_TIMEOUT, session.browser.close())
        .await
        .is_err()
    {
        tracing::debug!("Browser close timed out; killing browser process");
        let _ = session.browser.kill().await;
        return;
    }

    if tokio::time::timeout(BROWSER_PROCESS_CLOSE_TIMEOUT, session.browser.wait())
        .await
        .is_err()
    {
        tracing::debug!("Browser wait timed out; killing browser process");
        let _ = session.browser.kill().await;
    }
}

/// Browser manager for automation
pub struct BrowserManager {
    browser: Arc<Mutex<Option<BrowserSession>>>,
}

impl BrowserManager {
    /// Create a new browser manager (browser not yet launched)
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
        }
    }

    /// Launch the browser
    ///
    /// Launches Chrome in visible mode so user can watch form filling.
    #[tracing::instrument(skip(self), level = "info")]
    pub async fn launch(&self) -> Result<()> {
        use std::time::Instant;

        let start = Instant::now();
        tracing::info!("Launching browser in visible mode");
        let mut browser_guard = self.browser.lock().await;

        if browser_guard.is_some() {
            tracing::debug!("Browser already launched, skipping");
            return Ok(()); // Already launched
        }

        let profile_dir = create_automation_browser_profile_dir()?;
        let config = match automation_browser_config(&profile_dir) {
            Ok(config) => config,
            Err(error) => {
                cleanup_automation_browser_profile_dir(&profile_dir);
                return Err(error);
            }
        };

        // Launch browser
        let (browser, mut handler) = match Browser::launch(config).await {
            Ok(result) => result,
            Err(error) => {
                cleanup_automation_browser_profile_dir(&profile_dir);
                return Err(error)
                    .context("Failed to launch browser. Is Chrome/Chromium installed?");
            }
        };

        // Spawn handler task to process browser events
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(_e) = event {
                    tracing::debug!("Browser handler event error");
                }
            }
            tracing::debug!("Browser handler task terminated");
        });

        let duration = start.elapsed();
        tracing::info!(
            elapsed_ms = duration.as_millis(),
            "Browser launched successfully (visible mode, 1280x900)"
        );
        *browser_guard = Some(BrowserSession {
            browser,
            profile_dir,
        });

        Ok(())
    }

    /// Create a new page for automation
    ///
    /// Returns an AutomationPage that provides form filling methods.
    #[tracing::instrument(skip(self, url), fields(url = %sanitize_url_for_logging(url)), level = "info")]
    pub async fn new_page(&self, url: &str) -> Result<AutomationPage> {
        use std::time::Instant;

        let start = Instant::now();
        tracing::debug!("Creating new browser page");
        let browser_guard = self.browser.lock().await;
        let browser = browser_guard
            .as_ref()
            .context("Browser not launched. Call launch() first.")?;

        let page = browser
            .browser
            .new_page(url)
            .await
            .context("Failed to create new page")?;

        tracing::debug!("Waiting for page navigation to complete");
        // Wait for page to load
        page.wait_for_navigation()
            .await
            .context("Failed to wait for navigation")?;

        let duration = start.elapsed();
        tracing::info!(
            elapsed_ms = duration.as_millis(),
            "Browser page created and loaded"
        );

        Ok(AutomationPage::new(page))
    }

    /// Close the browser
    pub async fn close(&self) -> Result<()> {
        let mut browser_guard = self.browser.lock().await;

        if let Some(mut session) = browser_guard.take() {
            close_browser_session(&mut session).await;
            cleanup_automation_browser_profile_dir(&session.profile_dir);
            tracing::info!("Browser closed");
        }

        Ok(())
    }

    /// Check if browser is running
    pub async fn is_running(&self) -> bool {
        self.browser.lock().await.is_some()
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_manager_new() {
        let manager = BrowserManager::new();
        assert!(!manager.is_running().await);
        drop(manager);
    }

    #[test]
    fn browser_config_error_does_not_echo_raw_detail() {
        assert!(!BROWSER_CONFIG_ERROR.contains("Chrome"));
        assert!(!BROWSER_CONFIG_ERROR.contains("--"));
        assert!(!BROWSER_CONFIG_ERROR.contains(&format!("/{}/", "Users")));
    }

    #[test]
    fn browser_launch_args_keep_sandbox_and_automation_visibility() {
        let args = automation_browser_launch_args();
        assert!(!args.contains(&"--no-sandbox"));
        assert!(!args.contains(&"--disable-setuid-sandbox"));
        assert!(!args.contains(&"--disable-blink-features=AutomationControlled"));
    }

    #[test]
    fn browser_config_uses_isolated_profile_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let profile_dir = temp_dir.path().join("profile");
        std::fs::create_dir_all(&profile_dir).unwrap();

        let config = automation_browser_config(&profile_dir).unwrap();

        assert_eq!(config.user_data_dir.as_deref(), Some(profile_dir.as_path()));
    }

    #[test]
    fn browser_profile_dirs_are_unique_private_and_cleaned_up() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path().canonicalize().unwrap().join("profiles");

        let first = create_automation_browser_profile_dir_in(&root).unwrap();
        let second = create_automation_browser_profile_dir_in(&root).unwrap();

        assert_ne!(first, second);
        assert!(first.starts_with(&root));
        assert!(second.starts_with(&root));
        assert!(first.is_dir());
        assert!(second.is_dir());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(&first).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }

        cleanup_automation_browser_profile_dir(&first);

        assert!(!first.exists());
        assert!(second.exists());
    }

    #[tokio::test]
    #[ignore = "Requires Chrome installed - run manually"]
    async fn test_browser_launch() {
        let manager = BrowserManager::new();
        assert!(!manager.is_running().await);

        manager.launch().await.expect("Failed to launch browser");
        assert!(manager.is_running().await);

        manager.close().await.expect("Failed to close browser");
        assert!(!manager.is_running().await);
    }
}
