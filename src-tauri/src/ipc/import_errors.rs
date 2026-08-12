use crate::ipc::errors::user_friendly_error;
use jobsentinel_application::ImportError;

pub(super) fn format_import_error(error: &ImportError) -> String {
    match error {
        ImportError::NoSchemaOrgData => {
            "Could not read this page as a single job posting. Open one job posting, copy its browser address, or save the job with the details JobSentinel can find.".to_string()
        }
        ImportError::MultipleJobPostings(count) => {
            format!(
                "Found {} job postings on this page. Please use a more specific URL that links to a single job.",
                count
            )
        }
        ImportError::MissingRequiredField { field } => {
            format!("Missing required information: {field}. This job posting may be incomplete.")
        }
        ImportError::Timeout => {
            "This took too long. Check your internet connection and try again.".to_string()
        }
        ImportError::InvalidUrl(message) if message == "Blocked insecure URL: https required" => {
            "Paste an https job posting link from your browser address bar.".to_string()
        }
        ImportError::InvalidUrl(_) => {
            "Paste the full job link from your browser address bar.".to_string()
        }
        ImportError::RedirectBlocked { .. } => {
            "The job link redirects to another page. Paste the final public job posting link from your browser address bar.".to_string()
        }
        ImportError::HttpStatus(status) => format!("The website returned an error: {status}"),
        ImportError::HttpRequest => {
            "Failed to fetch the page. Please check the URL and try again.".to_string()
        }
        ImportError::HttpBodyRead(crate::desktop::HttpBodyReadError::ResponseTooLarge {
            max_bytes,
            ..
        }) => format!(
            "The job page response is too large to import safely. Maximum size is {} MiB.",
            max_bytes / (1024 * 1024)
        ),
        ImportError::HttpBodyRead(error) => {
            user_friendly_error("Failed to read the job page response", error)
        }
        ImportError::HtmlParseError(_) | ImportError::InvalidJsonLd(_) => {
            "Could not read this as one job posting. Open one job posting and copy its browser address.".to_string()
        }
        ImportError::DatabaseError(details) => {
            user_friendly_error("Database operation failed", details)
        }
        ImportError::AlreadyExists => "This job is already in your saved jobs.".to_string(),
        ImportError::PendingImportNotFound => {
            "This job preview expired. Check the job link again before saving.".to_string()
        }
        ImportError::SmartPasteTooLarge => {
            "Paste no more than 50,000 characters of job details at a time.".to_string()
        }
        ImportError::SmartPasteFieldTooLong { field } => {
            format!("Shorten the pasted {field} before reviewing this draft.")
        }
        ImportError::SmartPasteCredentialMaterial => {
            "Remove passwords, tokens, cookies, and authorization details before creating this draft.".to_string()
        }
        ImportError::SourcePolicyBlocked {
            visible_capture_allowed: true,
        } => "JobSentinel cannot fetch this pasted link. Open it in your browser and use visible Browser Import or manual entry.".to_string(),
        ImportError::SourcePolicyBlocked {
            visible_capture_allowed: false,
        } => "JobSentinel cannot fetch or capture this source. Open it in your browser or use manual entry.".to_string(),
        ImportError::SourceReviewRequired => {
            "Review the destination before JobSentinel checks this page.".to_string()
        }
        ImportError::SourceAuthorizationUnavailable => {
            "Job link checks are paused because the reviewed source policy changed. Restart JobSentinel and try again.".to_string()
        }
    }
}

pub(super) fn format_smart_paste_error(error: &ImportError) -> String {
    match error {
        ImportError::PendingImportNotFound => {
            "This Smart Paste draft expired. Create it again before saving.".to_string()
        }
        ImportError::SourceReviewRequired => {
            "Create and review the Smart Paste draft before saving.".to_string()
        }
        ImportError::SourceAuthorizationUnavailable => {
            "Smart Paste is paused because the reviewed source policy changed. Restart JobSentinel and try again.".to_string()
        }
        _ => format_import_error(error),
    }
}
