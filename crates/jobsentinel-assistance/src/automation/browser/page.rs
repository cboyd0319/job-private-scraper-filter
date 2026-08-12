//! Page Automation
//!
//! Provides form filling and interaction methods for a browser page.

use anyhow::{Context, Result};
use chromiumoxide::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, CaptureScreenshotParams,
};
use chromiumoxide::Page;
use jobsentinel_security::path_label_for_logging;
use serde::{Deserialize, Serialize};
use std::path::Path;

const FILE_UPLOAD_UNAVAILABLE: &str = "Could not attach the selected resume file";
const FILE_UPLOAD_SETUP_ERROR: &str = "Could not prepare the resume upload field";
const MANUAL_REVIEW_TOPIC: &str = "voluntary or sensitive personal questions";

fn javascript_string_literal(value: &str) -> Result<String> {
    serde_json::to_string(value).context("Failed to encode JavaScript string literal")
}

fn dropdown_select_script(selector: &str, value: &str) -> Result<String> {
    let selector = javascript_string_literal(selector)?;
    let value = javascript_string_literal(value)?;

    Ok(format!(
        r#"
                    const select = document.querySelector({selector});
                    if (select) {{
                        select.value = {value};
                        select.dispatchEvent(new Event('change', {{ bubbles: true }}));
                        true;
                    }} else {{
                        false;
                    }}
                    "#
    ))
}

/// Automation page wrapper
pub struct AutomationPage {
    page: Page,
}

/// Result of form filling attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillResult {
    /// Fields that were successfully filled
    pub filled_fields: Vec<String>,
    /// Review topics for saved screening answers that were prepared
    pub screening_answer_topics: Vec<String>,
    /// Bounded categories of questions that require the user's direct answer.
    pub manual_review_topics: Vec<String>,
    /// Fields that could not be filled (not found or error)
    pub unfilled_fields: Vec<String>,
    /// Whether a CAPTCHA was detected
    pub captcha_detected: bool,
    /// Whether the form is ready for user to review and submit
    pub ready_for_review: bool,
    /// Error message if any
    pub error_message: Option<String>,
}

impl FillResult {
    pub fn new() -> Self {
        Self {
            filled_fields: Vec::new(),
            screening_answer_topics: Vec::new(),
            manual_review_topics: Vec::new(),
            unfilled_fields: Vec::new(),
            captcha_detected: false,
            ready_for_review: false,
            error_message: None,
        }
    }

    pub fn success(filled: Vec<String>) -> Self {
        Self {
            filled_fields: filled,
            screening_answer_topics: Vec::new(),
            manual_review_topics: Vec::new(),
            unfilled_fields: Vec::new(),
            captcha_detected: false,
            ready_for_review: true,
            error_message: None,
        }
    }

    pub fn partial(filled: Vec<String>, unfilled: Vec<String>) -> Self {
        Self {
            filled_fields: filled,
            screening_answer_topics: Vec::new(),
            manual_review_topics: Vec::new(),
            unfilled_fields: unfilled,
            captcha_detected: false,
            ready_for_review: true,
            error_message: None,
        }
    }

    pub fn with_captcha(mut self) -> Self {
        self.captcha_detected = true;
        self.error_message = Some("CAPTCHA detected - please solve manually".to_string());
        self
    }

    pub fn add_screening_answer_topic(&mut self, topic: Option<&str>) {
        let Some(topic) = topic else {
            return;
        };

        if !self
            .screening_answer_topics
            .iter()
            .any(|existing| existing == topic)
        {
            self.screening_answer_topics.push(topic.to_string());
        }
    }

    pub fn add_manual_review_topic(&mut self) {
        if !self
            .manual_review_topics
            .iter()
            .any(|existing| existing == MANUAL_REVIEW_TOPIC)
        {
            self.manual_review_topics
                .push(MANUAL_REVIEW_TOPIC.to_string());
        }
    }
}

impl Default for FillResult {
    fn default() -> Self {
        Self::new()
    }
}

impl AutomationPage {
    /// Create a new automation page wrapper
    pub fn new(page: Page) -> Self {
        Self { page }
    }

    /// Return the browser's current page URL after navigation.
    pub async fn current_url(&self) -> Result<Option<String>> {
        self.page
            .url()
            .await
            .context("Failed to read current page URL")
    }

    /// Navigate to a URL
    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.page
            .goto(url)
            .await
            .context("Failed to navigate to URL")?;

        self.page
            .wait_for_navigation()
            .await
            .context("Failed to wait for navigation")?;

        Ok(())
    }

    /// Fill a text field by selector
    pub async fn fill(&self, selector: &str, value: &str) -> Result<bool> {
        match self.page.find_element(selector).await {
            Ok(element) => {
                // Click to focus
                element.click().await.context("Failed to click element")?;

                // Clear existing value
                element
                    .press_key("Control+a")
                    .await
                    .context("Failed to select all")?;

                // Type new value
                element
                    .type_str(value)
                    .await
                    .context("Failed to type value")?;

                tracing::debug!("Filled field '{}' with value", selector);
                Ok(true)
            }
            Err(_) => {
                tracing::debug!("Field not found: {}", selector);
                Ok(false)
            }
        }
    }

    /// Click an element by selector
    pub async fn click(&self, selector: &str) -> Result<bool> {
        match self.page.find_element(selector).await {
            Ok(element) => {
                element.click().await.context("Failed to click element")?;
                tracing::debug!("Clicked element: {}", selector);
                Ok(true)
            }
            Err(_) => {
                tracing::debug!("Element not found for click: {}", selector);
                Ok(false)
            }
        }
    }

    /// Select an option from a dropdown
    pub async fn select(&self, selector: &str, value: &str) -> Result<bool> {
        match self.page.find_element(selector).await {
            Ok(element) => {
                // Use JavaScript to set select value
                let script = dropdown_select_script(selector, value)?;

                let _ = element;
                self.page
                    .evaluate(script)
                    .await
                    .context("Failed to select option")?;

                tracing::debug!("Selected dropdown option");
                Ok(true)
            }
            Err(_) => {
                tracing::debug!("Dropdown element not found");
                Ok(false)
            }
        }
    }

    /// Upload a file to a file input
    ///
    /// Uses CDP's DOM.setFileInputFiles command to set file on input element.
    pub async fn upload_file(&self, selector: &str, file_path: &Path) -> Result<bool> {
        use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;

        if !file_path.exists() {
            return Err(anyhow::anyhow!(FILE_UPLOAD_UNAVAILABLE));
        }

        match self.page.find_element(selector).await {
            Ok(element) => {
                let path_str = file_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!(FILE_UPLOAD_UNAVAILABLE))?;

                // Use CDP to set files on input element
                let params = SetFileInputFilesParams::builder()
                    .files(vec![path_str.to_string()])
                    .node_id(element.node_id)
                    .build()
                    .map_err(|_| anyhow::anyhow!(FILE_UPLOAD_SETUP_ERROR))?;

                self.page
                    .execute(params)
                    .await
                    .context("Failed to upload file via CDP")?;

                tracing::debug!("Uploaded file to: {}", selector);
                Ok(true)
            }
            Err(_) => {
                tracing::debug!("File input not found: {}", selector);
                Ok(false)
            }
        }
    }

    /// Take a screenshot and save to file
    pub async fn screenshot(&self, path: &Path) -> Result<()> {
        let params = CaptureScreenshotParams::builder()
            .format(CaptureScreenshotFormat::Png)
            .build();

        let screenshot_data = self
            .page
            .screenshot(params)
            .await
            .context("Failed to capture screenshot")?;

        tokio::fs::write(path, screenshot_data)
            .await
            .context("Failed to save screenshot")?;

        tracing::info!(path = %path_label_for_logging(path), "Screenshot saved");
        Ok(())
    }

    /// Get the page HTML content
    pub async fn get_html(&self) -> Result<String> {
        let html = self
            .page
            .content()
            .await
            .context("Failed to get page content")?;
        Ok(html)
    }

    /// Check if a CAPTCHA is present on the page
    pub async fn has_captcha(&self) -> bool {
        // Common CAPTCHA indicators
        let captcha_selectors = [
            // reCAPTCHA
            ".g-recaptcha",
            "#recaptcha",
            "iframe[src*='recaptcha']",
            // hCaptcha
            ".h-captcha",
            "iframe[src*='hcaptcha']",
            // Generic
            "[data-captcha]",
            ".captcha",
            "#captcha",
        ];

        for selector in captcha_selectors {
            if self.page.find_element(selector).await.is_ok() {
                tracing::warn!("CAPTCHA detected with selector: {}", selector);
                return true;
            }
        }

        // Also check HTML content for CAPTCHA mentions
        if let Ok(html) = self.get_html().await {
            let html_lower = html.to_lowercase();
            if html_lower.contains("recaptcha") || html_lower.contains("hcaptcha") {
                tracing::warn!("CAPTCHA detected in page HTML");
                return true;
            }
        }

        false
    }

    /// Get the underlying page (for advanced operations)
    pub fn inner(&self) -> &Page {
        &self.page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_result_new() {
        let result = FillResult::new();
        assert!(result.filled_fields.is_empty());
        assert!(result.manual_review_topics.is_empty());
        assert!(result.unfilled_fields.is_empty());
        assert!(!result.captcha_detected);
        assert!(!result.ready_for_review);
    }

    #[test]
    fn test_fill_result_success() {
        let result = FillResult::success(vec!["email".to_string(), "name".to_string()]);
        assert_eq!(result.filled_fields.len(), 2);
        assert!(result.unfilled_fields.is_empty());
        assert!(result.ready_for_review);
    }

    #[test]
    fn test_fill_result_partial() {
        let result = FillResult::partial(vec!["email".to_string()], vec!["phone".to_string()]);
        assert_eq!(result.filled_fields.len(), 1);
        assert_eq!(result.unfilled_fields.len(), 1);
        assert!(result.ready_for_review);
    }

    #[test]
    fn test_fill_result_with_captcha() {
        let result = FillResult::new().with_captcha();
        assert!(result.captcha_detected);
        assert!(result.error_message.is_some());
    }

    #[test]
    fn fill_result_manual_review_topics_are_bounded_and_deduplicated() {
        let mut result = FillResult::new();
        result.add_manual_review_topic();
        result.add_manual_review_topic();

        assert_eq!(
            result.manual_review_topics,
            vec!["voluntary or sensitive personal questions"]
        );
    }

    #[test]
    fn dropdown_select_script_json_encodes_untrusted_values() -> Result<()> {
        let script = dropdown_select_script(
            "select[name='state']",
            r#"CO\';window.__jobsentinelInjected=true;//"#,
        )?;

        assert!(script.contains(r#"document.querySelector("select[name='state']")"#));
        assert!(script.contains(r#"select.value = "CO\\';window.__jobsentinelInjected=true;//";"#));
        assert!(!script.contains("select.value = 'CO"));
        assert!(!script.contains("querySelector('select"));
        Ok(())
    }

    #[test]
    fn file_upload_errors_do_not_echo_local_paths() {
        for message in [FILE_UPLOAD_UNAVAILABLE, FILE_UPLOAD_SETUP_ERROR] {
            assert!(!message.contains(&format!("/{}/", "Users")));
            assert!(!message.contains("resume.pdf"));
            assert!(!message.contains("CDP"));
        }
    }
}
