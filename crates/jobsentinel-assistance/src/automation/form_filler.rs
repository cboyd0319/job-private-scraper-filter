//! Form Auto-Fill Logic
//!
//! Fills job application forms with user profile data.
//! Platform-specific selectors for each ATS type.
//! Also handles screening questions using stored answer patterns.

use super::browser::{AutomationPage, FillResult};
use super::AtsPlatform;
use anyhow::Result;
use jobsentinel_domain::{
    requires_user_answer, screening_question_matches, ApplicationProfile, ScreeningAnswer,
};
use jobsentinel_security::path_label_for_logging;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod selectors;

const SCREENING_FIELD_LABEL: &str = "screening:saved_answer";
const QUESTION_DISCOVERY_ERROR: &str = "Could not inspect screening questions on this page";

fn screening_answer_review_topic(pattern: &str) -> Option<&'static str> {
    let normalized = pattern.to_ascii_lowercase();

    if normalized.contains("citizen") {
        Some("citizenship")
    } else if normalized.contains("work authorization")
        || normalized.contains("authorized to work")
        || normalized.contains("sponsorship")
        || normalized.contains("visa")
    {
        Some("work authorization")
    } else if normalized.contains("transportation") || normalized.contains("vehicle") {
        Some("transportation")
    } else if normalized.contains("relocat") || normalized.contains("travel") {
        Some("travel or relocation")
    } else if normalized.contains("education")
        || normalized.contains("degree")
        || normalized.contains("diploma")
        || normalized.contains("bachelor")
        || normalized.contains("ged")
    {
        Some("education")
    } else if normalized.contains("salary")
        || normalized.contains("compensation")
        || normalized.contains("pay")
    {
        Some("salary")
    } else if normalized.contains("start date") || normalized.contains("notice period") {
        Some("start date")
    } else if normalized.contains("availability")
        || normalized.contains("schedule")
        || normalized.contains("shift")
        || normalized.contains("weekend")
        || normalized.contains("overtime")
        || normalized.contains("holiday")
    {
        Some("schedule or availability")
    } else if normalized.contains("managed a team")
        || normalized.contains("management")
        || normalized.contains("supervis")
    {
        Some("management experience")
    } else if normalized.contains("bilingual")
        || normalized.contains("multilingual")
        || normalized.contains("language")
        || normalized.contains("fluenc")
    {
        Some("language fluency")
    } else if normalized.contains("background") || normalized.contains("drug") {
        Some("background or drug screen")
    } else if normalized.contains("physical")
        || normalized.contains("lift")
        || normalized.contains("standing")
        || normalized.contains("stand for")
    {
        Some("physical requirements")
    } else if normalized.contains("18 years")
        || normalized.contains("minimum age")
        || normalized.contains("age requirement")
    {
        Some("age requirement")
    } else if normalized.contains("driver")
        || normalized.contains("license")
        || normalized.contains("certification")
        || normalized.contains("clearance")
    {
        Some("license, certification, or clearance")
    } else {
        None
    }
}

/// Form filler - fills application forms based on ATS platform
pub struct FormFiller {
    profile: ApplicationProfile,
    resume_path: Option<PathBuf>,
    screening_answers: Vec<ScreeningAnswer>,
}

impl FormFiller {
    /// Create a new form filler with user profile
    pub fn new(profile: ApplicationProfile, resume_path: Option<PathBuf>) -> Self {
        Self {
            profile,
            resume_path,
            screening_answers: Vec::new(),
        }
    }

    /// Create form filler with screening answers for auto-filling questions
    pub fn with_screening_answers(mut self, answers: Vec<ScreeningAnswer>) -> Self {
        self.screening_answers = answers;
        self
    }

    /// Fill the application form on the page
    ///
    /// Returns what was filled and what needs manual attention.
    /// Does NOT submit - user must click submit manually.
    #[tracing::instrument(
        skip(self, page),
        fields(
            platform = ?platform,
            has_resume = self.resume_path.is_some(),
            screening_answer_count = self.screening_answers.len()
        ),
        level = "info"
    )]
    pub async fn fill_page(
        &self,
        page: &AutomationPage,
        platform: &AtsPlatform,
    ) -> Result<FillResult> {
        use std::time::Instant;

        let start = Instant::now();
        tracing::info!("Starting form auto-fill");
        let mut result = FillResult::new();

        if *platform == AtsPlatform::Unknown {
            result.error_message = Some(
                "Prepare Form only works on recognized application sites. Open this page yourself, or apply manually."
                    .to_string(),
            );
            return Ok(result);
        }

        // Check for CAPTCHA first
        if page.has_captcha().await {
            tracing::warn!("CAPTCHA detected before filling, aborting auto-fill");
            return Ok(result.with_captcha());
        }

        // Get platform-specific selectors
        let selectors = Self::get_field_selectors(platform);

        // Fill basic contact fields
        let contact_start = Instant::now();
        tracing::debug!("Filling contact fields");
        self.fill_contact_fields(page, &selectors, &mut result)
            .await;
        tracing::debug!(
            elapsed_ms = contact_start.elapsed().as_millis(),
            "Contact fields filled"
        );

        // Fill URLs (LinkedIn, GitHub, etc.)
        tracing::debug!("Filling URL fields");
        self.fill_url_fields(page, &selectors, &mut result).await;

        // Fill work authorization
        tracing::debug!("Filling work authorization fields");
        self.fill_work_auth_fields(page, &selectors, &mut result)
            .await;

        // Upload resume if available
        if let Some(ref resume_path) = self.resume_path {
            tracing::debug!(
                resume_path = %path_label_for_logging(resume_path),
                "Uploading resume"
            );
            self.fill_resume(page, &selectors, resume_path, &mut result)
                .await;
        }

        // Fill screening questions using stored answers
        if !self.screening_answers.is_empty() {
            let answer_count = self.screening_answers.len();
            tracing::debug!(answer_count, "Filling screening questions");
            self.fill_screening_questions(page, &mut result).await;
        }

        // Check for CAPTCHA again after filling (some appear after form interaction)
        if page.has_captcha().await {
            tracing::warn!("CAPTCHA appeared after form interaction");
            return Ok(result.with_captcha());
        }

        result.ready_for_review = true;
        let duration = start.elapsed();
        tracing::info!(
            fields_filled = result.filled_fields.len(),
            elapsed_ms = duration.as_millis(),
            "Form auto-fill complete"
        );
        Ok(result)
    }

    /// Fill contact information fields
    async fn fill_contact_fields(
        &self,
        page: &AutomationPage,
        selectors: &HashMap<FieldType, Vec<&str>>,
        result: &mut FillResult,
    ) {
        // First name
        if let Some(sel_list) = selectors.get(&FieldType::FirstName) {
            let first_name = self
                .profile
                .full_name
                .split_whitespace()
                .next()
                .unwrap_or("");
            for selector in sel_list {
                if let Ok(true) = page.fill(selector, first_name).await {
                    result.filled_fields.push("first_name".to_string());
                    break;
                }
            }
        }

        // Last name
        if let Some(sel_list) = selectors.get(&FieldType::LastName) {
            let last_name = self
                .profile
                .full_name
                .split_whitespace()
                .last()
                .unwrap_or("");
            for selector in sel_list {
                if let Ok(true) = page.fill(selector, last_name).await {
                    result.filled_fields.push("last_name".to_string());
                    break;
                }
            }
        }

        // Full name (some forms use this instead of first/last)
        if let Some(sel_list) = selectors.get(&FieldType::FullName) {
            for selector in sel_list {
                if let Ok(true) = page.fill(selector, &self.profile.full_name).await {
                    result.filled_fields.push("full_name".to_string());
                    break;
                }
            }
        }

        // Email
        if let Some(sel_list) = selectors.get(&FieldType::Email) {
            for selector in sel_list {
                if let Ok(true) = page.fill(selector, &self.profile.email).await {
                    result.filled_fields.push("email".to_string());
                    break;
                }
            }
        }

        // Phone
        if let Some(phone) = &self.profile.phone {
            if let Some(sel_list) = selectors.get(&FieldType::Phone) {
                for selector in sel_list {
                    if let Ok(true) = page.fill(selector, phone).await {
                        result.filled_fields.push("phone".to_string());
                        break;
                    }
                }
            }
        }
    }

    /// Fill URL fields (LinkedIn, GitHub, portfolio, etc.)
    async fn fill_url_fields(
        &self,
        page: &AutomationPage,
        selectors: &HashMap<FieldType, Vec<&str>>,
        result: &mut FillResult,
    ) {
        // LinkedIn
        if let Some(url) = &self.profile.linkedin_url {
            if let Some(sel_list) = selectors.get(&FieldType::LinkedIn) {
                for selector in sel_list {
                    if let Ok(true) = page.fill(selector, url).await {
                        result.filled_fields.push("linkedin".to_string());
                        break;
                    }
                }
            }
        }

        // GitHub
        if let Some(url) = &self.profile.github_url {
            if let Some(sel_list) = selectors.get(&FieldType::GitHub) {
                for selector in sel_list {
                    if let Ok(true) = page.fill(selector, url).await {
                        result.filled_fields.push("github".to_string());
                        break;
                    }
                }
            }
        }

        // Portfolio
        if let Some(url) = &self.profile.portfolio_url {
            if let Some(sel_list) = selectors.get(&FieldType::Portfolio) {
                for selector in sel_list {
                    if let Ok(true) = page.fill(selector, url).await {
                        result.filled_fields.push("portfolio".to_string());
                        break;
                    }
                }
            }
        }

        // Website
        if let Some(url) = &self.profile.website_url {
            if let Some(sel_list) = selectors.get(&FieldType::Website) {
                for selector in sel_list {
                    if let Ok(true) = page.fill(selector, url).await {
                        result.filled_fields.push("website".to_string());
                        break;
                    }
                }
            }
        }
    }

    /// Fill work authorization fields
    async fn fill_work_auth_fields(
        &self,
        page: &AutomationPage,
        selectors: &HashMap<FieldType, Vec<&str>>,
        result: &mut FillResult,
    ) {
        // Work authorized
        if let Some(sel_list) = selectors.get(&FieldType::WorkAuthorized) {
            let value = if self.profile.us_work_authorized {
                "Yes"
            } else {
                "No"
            };
            for selector in sel_list {
                // Try both fill (for text input) and select (for dropdown)
                if let Ok(true) = page.fill(selector, value).await {
                    result.filled_fields.push("work_authorized".to_string());
                    break;
                }
                if let Ok(true) = page.select(selector, value).await {
                    result.filled_fields.push("work_authorized".to_string());
                    break;
                }
            }
        }

        // Requires sponsorship
        if let Some(sel_list) = selectors.get(&FieldType::RequiresSponsorship) {
            let value = if self.profile.requires_sponsorship {
                "Yes"
            } else {
                "No"
            };
            for selector in sel_list {
                if let Ok(true) = page.fill(selector, value).await {
                    result
                        .filled_fields
                        .push("requires_sponsorship".to_string());
                    break;
                }
                if let Ok(true) = page.select(selector, value).await {
                    result
                        .filled_fields
                        .push("requires_sponsorship".to_string());
                    break;
                }
            }
        }
    }

    /// Upload resume file
    async fn fill_resume(
        &self,
        page: &AutomationPage,
        selectors: &HashMap<FieldType, Vec<&str>>,
        resume_path: &Path,
        result: &mut FillResult,
    ) {
        if let Some(sel_list) = selectors.get(&FieldType::Resume) {
            for selector in sel_list {
                if let Ok(true) = page.upload_file(selector, resume_path).await {
                    result.filled_fields.push("resume".to_string());
                    break;
                }
            }
        }
    }

    /// Get field selectors for each ATS platform
    fn get_field_selectors(platform: &AtsPlatform) -> HashMap<FieldType, Vec<&'static str>> {
        selectors::get_field_selectors(platform)
    }
}

mod questions;

#[cfg(test)]
use questions::question_discovery_error;

/// Field types for form filling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FieldType {
    FirstName,
    LastName,
    FullName,
    Email,
    Phone,
    LinkedIn,
    GitHub,
    Portfolio,
    Website,
    Resume,
    CoverLetter,
    WorkAuthorized,
    RequiresSponsorship,
}

#[cfg(test)]
mod tests;
