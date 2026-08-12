//! Sanitizer - Anonymize all user-identifiable information
//!
//! CRITICAL: This is a PUBLIC repository. Every log, every debug event, every error message
//! goes through this sanitizer before being shown to users or included in feedback reports.

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

// Unix home paths are reduced to /[USER_PATH]/...
#[allow(clippy::expect_used)]
static PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"/(Users|home)/[^/\s]+")
        .expect("Unix path regex pattern is valid and should compile")
});

// Windows home paths are reduced to C:\[USER_PATH]\...
#[allow(clippy::expect_used)]
static WINDOWS_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z]:\\Users\\[^\\]+")
        .expect("Windows path regex pattern is valid and should compile")
});

// Non-home local paths can still contain user-specific temp, container, or
// mounted-volume details.
#[allow(clippy::expect_used)]
static LOCAL_UNIX_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"/(?:private/var|var/folders|tmp|var/tmp|run/user|Volumes)/[^\s"'<>\\)]+"#)
        .expect("Local Unix path regex pattern is valid and should compile")
});

// Non-home Windows paths can include temp folders, alternate drives, or
// user-created folders with private job-search file names.
#[allow(clippy::expect_used)]
static LOCAL_WINDOWS_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"[A-Za-z]:\\(?:(?:Temp|Windows\\Temp|ProgramData)(?:\\[A-Za-z0-9._ -]+)*|(?:[A-Za-z0-9._ -]+\\)+[A-Za-z0-9._ -]+)"#,
    )
    .expect("Local Windows path regex pattern is valid and should compile")
});

// Emails: john@example.com → [EMAIL]
#[allow(clippy::expect_used)]
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        .expect("Email address regex pattern is valid and should compile")
});

// Phone numbers: common North American formats → [PHONE]
#[allow(clippy::expect_used)]
static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:\+?1[\s.-]?)?(?:\([2-9][0-9]{2}\)|[2-9][0-9]{2})[\s.-]?[2-9][0-9]{2}[\s.-]?[0-9]{4}\b",
    )
    .expect("Phone number regex pattern is valid and should compile")
});

// Webhooks: provider URL → [WEBHOOK_CONFIGURED]
#[allow(clippy::expect_used)]
static WEBHOOK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"https://(?:hooks\.slack\.com|discord(?:app)?\.com/api/webhooks|outlook\.office(?:365)?\.com/webhook|(?:[a-z0-9-]+\.)+webhook\.office\.com|(?:[a-z0-9-]+\.)+logic\.azure\.com|hooks\.discord\.com/api/webhooks|hooks\.teams\.com/workflows)[^\s"'<>\\)]*"#,
    )
        .expect("Webhook URL regex pattern is valid and should compile")
});

// LinkedIn cookies: li_at=AQEDARa... → li_at=[REDACTED]
#[allow(clippy::expect_used)]
static LINKEDIN_COOKIE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"li_at=[^\s;]+").expect("LinkedIn cookie regex pattern is valid and should compile")
});

// API tokens: Bearer eyJ..., token=..., access_token=..., or JSON/header token fields → [TOKEN]
#[allow(clippy::expect_used)]
static TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(Bearer\s+[^\s"'<>]+|(?:access_token|refresh_token|api[_-]?key|token|secret|password|x-jobsentinel-token)=[^\s&"'<>\\)]+|["']?(?:access_token|refresh_token|api[_-]?key|token|secret|password|x-jobsentinel-token)["']?\s*:\s*["'][^"']+["']|(?:token|secret|password)\s+[^\s"'<>]+)"#,
    )
        .expect("API token regex pattern is valid and should compile")
});

// Generic URLs can contain job-search details or query secrets → [URL]
#[allow(clippy::expect_used)]
static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://[^\s"'<>\\)]+"#).expect("URL regex pattern is valid and should compile")
});

// IP addresses: 192.168.1.1 → [IP_ADDRESS]
#[allow(clippy::expect_used)]
static IP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b")
        .expect("IP address regex pattern is valid and should compile")
});

// Sensitive job-search context entered as free text. Support reports should
// keep the label but remove the user's private content.
#[allow(clippy::expect_used)]
static JOB_SEARCH_LABELED_CONTEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)\b((?:salary|compensation|pay)[ _-]?(?:floor|expectation|target|range|requirement)|expected salary|desired salary|resume(?:[ _-]?(?:text|data|content|summary|excerpt))?|cover[ _-]?letter(?:[ _-]?(?:text|data|content|summary|excerpt))?|private[ _-]?notes?|application[ _-]?(?:history|notes?)|screening[ _-]?(?:questions?|answers?)|question[ _-]?text|answer[ _-]?text|location[ _-]?preferences?|career[ _-]?goals?|personal[ _-]?circumstances?|(?:full|candidate|applicant|user|your)[ _-]?name)\s*[:=]\s*[^\r\n]+",
    )
    .expect("Sensitive job-search labeled context regex pattern is valid and should compile")
});

#[allow(clippy::expect_used)]
static JOB_SEARCH_STATEMENT_CONTEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)\b((?:my\s+)?(?:salary|compensation|pay)\s+(?:floor|expectation|target|range|requirement)|expected salary|desired salary|private note|application note|screening answer|location preference|career goal|personal circumstance)\s+(?:is|are|was|were)\s+[^\r\n]+",
    )
    .expect("Sensitive job-search statement context regex pattern is valid and should compile")
});

#[allow(clippy::expect_used)]
static PERSON_NAME_STATEMENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)\b((?:my|candidate|applicant|user)\s+name)\s+(?:is|was)\s+[^\r\n]+")
        .expect("Person-name statement regex pattern is valid and should compile")
});

#[allow(clippy::expect_used)]
static JOB_SEARCH_NARRATIVE_CONTEXT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?im)\b((?:while\s+)?(?:applying|applied|interviewing|interviewed|negotiating|rejected|offer(?:ed)?|laid off|layoff|unemployed|employment gap|resume gap|job search urgency)\b[^\r\n]*)",
    )
    .expect("Sensitive job-search narrative regex pattern is valid and should compile")
});

// Quoted strings (might be job titles or company names)
#[allow(clippy::expect_used)]
static QUOTED_STRING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""[^"]+""#)
        .expect("Double-quoted string regex pattern is valid and should compile")
});
#[allow(clippy::expect_used)]
static SINGLE_QUOTED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"'[^']+'").expect("Single-quoted string regex pattern is valid and should compile")
});

/// Sanitizer removes all user-identifiable information from text
pub(super) struct Sanitizer;

impl Sanitizer {
    /// Sanitize all user-identifiable information from text
    ///
    /// This function MUST be called on ALL output before showing to users or
    /// including in feedback reports.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use jobsentinel::ipc::feedback::sanitizer::Sanitizer;
    ///
    /// let dirty = format!("Error reading /{}/person/Documents/jobs.db", "Users");
    /// let clean = Sanitizer::sanitize(&dirty);
    /// assert_eq!(clean, "Error reading /[USER_PATH]/Documents/jobs.db");
    /// ```
    pub(super) fn sanitize(text: &str) -> String {
        let mut result = text.to_string();

        // Unix home paths are reduced to /[USER_PATH].
        result = PATH_REGEX.replace_all(&result, "/[USER_PATH]").to_string();

        // Windows home paths are reduced to C:\[USER_PATH].
        result = WINDOWS_PATH_REGEX
            .replace_all(&result, "C:\\[USER_PATH]")
            .to_string();

        // Non-home local paths are reduced to /[LOCAL_PATH].
        result = LOCAL_UNIX_PATH_REGEX
            .replace_all(&result, "/[LOCAL_PATH]")
            .to_string();
        result = LOCAL_WINDOWS_PATH_REGEX
            .replace_all(&result, "C:\\[LOCAL_PATH]")
            .to_string();

        // Emails: john@example.com → [EMAIL]
        result = EMAIL_REGEX.replace_all(&result, "[EMAIL]").to_string();

        // Phone numbers: +1 (303) 555-1212 → [PHONE]
        result = PHONE_REGEX.replace_all(&result, "[PHONE]").to_string();

        // Webhooks: full URL → [WEBHOOK_CONFIGURED]
        result = WEBHOOK_REGEX
            .replace_all(&result, "[WEBHOOK_CONFIGURED]")
            .to_string();

        // LinkedIn cookies
        result = LINKEDIN_COOKIE_REGEX
            .replace_all(&result, "li_at=[REDACTED]")
            .to_string();

        // API tokens
        result = TOKEN_REGEX.replace_all(&result, "[TOKEN]").to_string();

        // Generic URLs
        result = URL_REGEX.replace_all(&result, "[URL]").to_string();

        // IP addresses (optional - some may want to keep local IPs)
        result = IP_REGEX.replace_all(&result, "[IP_ADDRESS]").to_string();

        // Sensitive job-search context from user-entered free text
        result = JOB_SEARCH_LABELED_CONTEXT_REGEX
            .replace_all(&result, "$1: [JOB_SEARCH_DETAIL_REDACTED]")
            .to_string();
        result = JOB_SEARCH_STATEMENT_CONTEXT_REGEX
            .replace_all(&result, "$1 [JOB_SEARCH_DETAIL_REDACTED]")
            .to_string();
        result = PERSON_NAME_STATEMENT_REGEX
            .replace_all(&result, "$1 [PERSON_NAME_REDACTED]")
            .to_string();

        result
    }

    /// Sanitize error messages specifically
    ///
    /// More aggressive than `sanitize()` - also removes quoted strings that might
    /// contain job titles or company names.
    pub(super) fn sanitize_error(error: &str) -> String {
        let mut result = Self::sanitize(error);

        // Remove double-quoted strings (might be job titles/companies)
        result = QUOTED_STRING_REGEX
            .replace_all(&result, "\"[REDACTED]\"")
            .to_string();

        // Remove single-quoted strings too
        result = SINGLE_QUOTED_REGEX
            .replace_all(&result, "'[REDACTED]'")
            .to_string();

        result
    }

    pub(super) fn sanitize_support_report_text(text: &str) -> String {
        let result = remove_support_report_path_suffixes(&Self::sanitize_error(text));

        JOB_SEARCH_NARRATIVE_CONTEXT_REGEX
            .replace_all(&result, "[JOB_SEARCH_DETAIL_REDACTED]")
            .to_string()
    }
}

fn remove_support_report_path_suffixes(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());

    for chunk in text.split_inclusive('\n') {
        let (line, ending) = if let Some(line) = chunk.strip_suffix("\r\n") {
            (line, "\r\n")
        } else if let Some(line) = chunk.strip_suffix('\n') {
            (line, "\n")
        } else {
            (chunk, "")
        };
        if let Some(start) = support_report_path_start(line) {
            sanitized.push_str(&line[..start]);
            sanitized.push_str("[LOCAL_PATH]");
            sanitized.push_str(ending);
        } else {
            sanitized.push_str(chunk);
        }
    }

    sanitized
}

fn support_report_path_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    (0..bytes.len()).find(|&index| {
        let boundary =
            index == 0 || (!bytes[index - 1].is_ascii_alphanumeric() && bytes[index - 1] != b'_');
        boundary
            && (bytes[index] == b'/'
                || (bytes[index] == b'\\' && bytes.get(index + 1) == Some(&b'\\'))
                || (bytes[index].is_ascii_alphabetic()
                    && bytes.get(index + 1) == Some(&b':')
                    && bytes
                        .get(index + 2)
                        .is_some_and(|separator| matches!(separator, b'/' | b'\\'))))
    })
}

/// Anonymized configuration summary (counts only, no values)
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConfigSummary {
    pub scrapers_enabled: usize,
    pub keywords_count: usize,
    pub has_location_prefs: bool,
    pub has_salary_prefs: bool,
    pub has_blocked_companies: bool,
    pub has_preferred_companies: bool,
    pub notifications_configured: usize,
    pub has_resume: bool,
}

#[cfg(test)]
mod tests;
