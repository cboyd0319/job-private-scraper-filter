//! Deep Link Generation Commands
//!
//! Tauri commands for generating job search deep links.

use crate::desktop::sanitize_url_for_logging;
use crate::desktop::validate_external_https_url_for_fetch;
use crate::desktop::{
    generate_all_links, generate_link_for_site, get_all_sites, DeepLink, SearchCriteria,
    SiteCategory, SiteInfo,
};
use crate::ipc::errors::user_friendly_error;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Url};

/// Generate deep links for all supported sites
#[tauri::command]
#[tracing::instrument(skip(_app, criteria))]
pub(crate) async fn generate_deep_links(
    _app: tauri::AppHandle,
    criteria: SearchCriteria,
) -> Result<Vec<DeepLink>, String> {
    tracing::info!("Generating deep links for all sites");

    generate_all_links(&criteria).map_err(|e| {
        let message = user_friendly_error("Failed to generate deep links", &e);
        tracing::error!(error = %message, "Failed to generate deep links");
        message
    })
}

/// Generate deep link for a specific site
#[tauri::command]
#[tracing::instrument(skip(_app, criteria), fields(site_id = %site_id))]
pub(crate) async fn generate_deep_link(
    _app: tauri::AppHandle,
    site_id: String,
    criteria: SearchCriteria,
) -> Result<DeepLink, String> {
    tracing::info!("Generating deep link for site: {}", site_id);

    generate_link_for_site(&site_id, &criteria).map_err(|e| {
        let message = user_friendly_error("Failed to generate deep link", &e);
        tracing::error!(error = %message, site_id = %site_id, "Failed to generate deep link");
        message
    })
}

/// Get all supported job sites
#[tauri::command]
#[tracing::instrument(skip(_app))]
pub(crate) async fn get_supported_sites(_app: tauri::AppHandle) -> Result<Vec<SiteInfo>, String> {
    tracing::debug!("Fetching all supported sites");
    Ok(get_all_sites())
}

/// Get sites by category
#[tauri::command]
#[tracing::instrument(skip(_app))]
pub(crate) async fn get_sites_by_category_cmd(
    _app: tauri::AppHandle,
    category: SiteCategory,
) -> Result<Vec<SiteInfo>, String> {
    tracing::debug!("Fetching sites for category: {}", category);
    Ok(crate::desktop::get_sites_by_category(category))
}

/// Validate that a URL is safe to open in the user's browser.
/// Allows external HTTPS URLs while blocking localhost, private networks, and unsafe schemes.
async fn validate_deep_link_url(url: &str) -> Result<(), String> {
    validate_external_https_url_for_fetch(url).await.map(|_| ())
}

/// Open a deep link URL in the default browser
#[tauri::command]
#[tracing::instrument(skip(app))]
pub(crate) async fn open_deep_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;

    validate_deep_link_url(&url).await?;
    if requires_linkedin_workbench_review(&url) && !confirm_native_linkedin_open(&app).await? {
        return Err("LinkedIn was kept closed.".to_string());
    }

    let url_label = sanitize_url_for_logging(&url);
    tracing::info!(url = %url_label, "Opening deep link in browser");

    // Use tauri-plugin-shell to open URL
    let shell = app.shell();
    #[allow(deprecated)]
    shell.open(&url, None).map_err(|e| {
        let message = user_friendly_error("Failed to open URL", &e);
        tracing::error!(error = %message, url = %url_label, "Failed to open URL");
        message
    })?;

    // Emit event for analytics/tracking
    app.emit("deep-link-opened", DeepLinkOpenedEvent { url: url_label })
        .map_err(|e| user_friendly_error("Failed to emit event", e))?;

    Ok(())
}

async fn confirm_native_linkedin_open(app: &tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
    use tokio::sync::oneshot;

    let (decision, received) = oneshot::channel();
    app.dialog()
        .message(
            "Open LinkedIn yourself. JobSentinel will not automate LinkedIn, read the page, or access browser session state.",
        )
        .title("Open LinkedIn")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Open LinkedIn".to_string(),
            "Keep Closed".to_string(),
        ))
        .show(move |approved| {
            let _ = decision.send(approved);
        });
    received
        .await
        .map_err(|_| "LinkedIn confirmation could not be completed.".to_string())
}

fn requires_linkedin_workbench_review(value: &str) -> bool {
    Url::parse(value).ok().and_then(|url| {
        url.host_str().map(|host| {
            let host = host.trim_end_matches('.').to_ascii_lowercase();
            host == "linkedin.com" || host.ends_with(".linkedin.com")
        })
    }) == Some(true)
}

/// Event emitted when a deep link is opened
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepLinkOpenedEvent {
    url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::RemoteType;

    #[test]
    fn linkedin_deep_links_require_the_restricted_workbench_review() {
        for restricted in [
            "https://linkedin.com/jobs/",
            "https://www.linkedin.com/jobs/search/?keywords=security",
            "https://jobs.linkedin.com/view/1",
        ] {
            assert!(
                requires_linkedin_workbench_review(restricted),
                "{restricted}"
            );
        }
        for allowed in [
            "https://example.com/jobs",
            "https://linkedin.com.example/jobs",
            "https://notlinkedin.com/jobs",
        ] {
            assert!(!requires_linkedin_workbench_review(allowed), "{allowed}");
        }
    }

    #[test]
    fn test_generate_deep_links_basic() {
        let criteria = SearchCriteria {
            query: "Customer Support Lead".to_string(),
            location: Some("Chicago, IL".to_string()),
            experience_level: None,
            job_type: None,
            remote_type: None,
        };

        let links = generate_all_links(&criteria).unwrap();
        assert!(!links.is_empty());
        assert!(links
            .iter()
            .any(|link| link.url.contains("Customer") && link.url.contains("Support")));
    }

    #[test]
    fn test_search_criteria_serialization() {
        let criteria = SearchCriteria {
            query: "Care Coordinator".to_string(),
            location: Some("Remote".to_string()),
            experience_level: None,
            job_type: None,
            remote_type: Some(RemoteType::Remote),
        };

        let json = serde_json::to_string(&criteria).unwrap();
        assert!(json.contains("Care Coordinator"));
        assert!(json.contains("Remote"));

        let deserialized: SearchCriteria = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, "Care Coordinator");
    }

    // ========================================================================
    // Security: deep link URL validation (CWE-601 Open Redirect)
    // ========================================================================

    #[tokio::test]
    async fn test_deep_link_allows_https_public_ip() {
        assert!(validate_deep_link_url("https://93.184.216.34/jobs?q=rust")
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_http_public_ip() {
        assert!(validate_deep_link_url("http://93.184.216.34/search")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_file_scheme() {
        assert!(validate_deep_link_url("file:///etc/passwd").await.is_err());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_localhost() {
        assert!(validate_deep_link_url("http://localhost:3000/jobs")
            .await
            .is_err());
        assert!(validate_deep_link_url("http://127.0.0.1/jobs")
            .await
            .is_err());
        assert!(validate_deep_link_url("http://[::1]/jobs").await.is_err());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_private_network_urls() {
        assert!(validate_deep_link_url("http://10.0.0.5/jobs")
            .await
            .is_err());
        assert!(validate_deep_link_url("http://172.20.0.5/jobs")
            .await
            .is_err());
        assert!(validate_deep_link_url("http://192.168.1.5/jobs")
            .await
            .is_err());
        assert!(validate_deep_link_url("http://169.254.1.5/jobs")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_javascript_scheme() {
        assert!(validate_deep_link_url("javascript:alert(1)").await.is_err());
    }

    #[tokio::test]
    async fn test_deep_link_blocks_data_scheme() {
        assert!(
            validate_deep_link_url("data:text/html,<script>alert(1)</script>")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_deep_link_rejects_invalid_url() {
        assert!(validate_deep_link_url("not a url").await.is_err());
    }
}
