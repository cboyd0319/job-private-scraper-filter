//! Enforces source policy, permission, freshness, and restricted-route boundaries.

use chrono::{Days, NaiveDate};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_source_manifest::{
        SourceManifest, SourceOperation, SourcePermission, SourceReview, SourceReviewStatus,
        SourceStopCondition,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceGrantState {
    NotRequired,
    Missing,
    Granted {
        source_id: String,
        policy_ref: String,
        permission: SourcePermission,
        operation: SourceOperation,
        policy_revision: u32,
    },
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceActionDecision {
    Allowed {
        request_limit_per_hour: u16,
        connectivity_required: bool,
    },
    ReviewRequired,
    Blocked(SourceStopCondition),
    Stale,
    Revoked,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFreshnessStatus {
    Current,
    Stale,
    ReviewRequired,
    Blocked(SourceStopCondition),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFreshnessReport {
    pub status: SourceFreshnessStatus,
    pub verified_on: NaiveDate,
    pub expires_on: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSimulationReport {
    pub decision: SourceActionDecision,
    pub manifest_expires_on: NaiveDate,
    pub review_expires_on: Option<NaiveDate>,
    pub risk_note_refs: Vec<String>,
}

const AUTOMATION_BLOCKED_SOURCE_DOMAINS: &[&str] = &[
    "builtin.com",
    "builtincolorado.com",
    "dice.com",
    "glassdoor.com",
    "linkedin.com",
    "simplyhired.com",
    "ycombinator.com",
];
const VISIBLE_CAPTURE_BLOCKED_SOURCE_DOMAINS: &[&str] = &["linkedin.com", "ycombinator.com"];

#[must_use]
pub fn automated_url_fetch_is_blocked(value: &str) -> bool {
    url_matches_source_domain(value, AUTOMATION_BLOCKED_SOURCE_DOMAINS)
}

#[must_use]
pub fn visible_page_capture_is_blocked(value: &str) -> bool {
    url_matches_source_domain(value, VISIBLE_CAPTURE_BLOCKED_SOURCE_DOMAINS)
}

fn url_matches_source_domain(value: &str, domains: &[&str]) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    url.host_str().is_some_and(|host| {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        domains.iter().any(|domain| {
            host == *domain
                || host
                    .strip_suffix(domain)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
    })
}

impl SourceManifest {
    pub fn freshness(
        &self,
        policy: &SourcePolicy,
        today: NaiveDate,
    ) -> Result<SourceFreshnessReport, String> {
        policy.validate()?;
        if self.source_id != policy.source_id || self.source_class != policy.source_class {
            return Err("source manifest does not match the current policy".to_string());
        }
        let mut report = self.validated_freshness(today)?;
        let policy_changed =
            (&self.policy_ref, self.policy_revision) != (&policy.policy_ref, policy.revision);
        if policy_changed {
            report.status = SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyChanged);
            return Ok(report);
        }
        if matches!(policy.access, SourceAccess::Disabled) {
            report.status = SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyDisabled);
            return Ok(report);
        }
        self.validate(policy)?;
        if report.status == SourceFreshnessStatus::Stale {
            return Ok(report);
        }
        for review in [&self.terms_review, &self.robots_review] {
            report.status = match review.action_decision(today, self.max_age_days)? {
                Some(SourceActionDecision::ReviewRequired) => SourceFreshnessStatus::ReviewRequired,
                Some(SourceActionDecision::Blocked(reason)) => {
                    SourceFreshnessStatus::Blocked(reason)
                }
                Some(_) => return Err("source review state is invalid".to_string()),
                None => continue,
            };
            return Ok(report);
        }
        Ok(report)
    }

    pub fn authorize(
        &self,
        policy: &SourcePolicy,
        operation: SourceOperation,
        today: NaiveDate,
        grant: SourceGrantState,
    ) -> Result<SourceActionDecision, String> {
        policy.validate()?;
        if self.source_id != policy.source_id || self.source_class != policy.source_class {
            return Err("source manifest does not match the current policy".to_string());
        }
        if self.policy_ref != policy.policy_ref {
            return Ok(SourceActionDecision::Blocked(
                SourceStopCondition::PolicyChanged,
            ));
        }
        if self.policy_revision != policy.revision {
            return Ok(SourceActionDecision::Blocked(
                SourceStopCondition::PolicyChanged,
            ));
        }
        self.validate(policy)?;
        if matches!(policy.access, SourceAccess::Disabled) {
            return Ok(SourceActionDecision::Blocked(
                SourceStopCondition::PolicyDisabled,
            ));
        }
        let Some(rule) = self.actions.iter().find(|rule| rule.operation == operation) else {
            return Ok(SourceActionDecision::Unsupported);
        };
        if self.validated_freshness(today)?.status == SourceFreshnessStatus::Stale {
            return Ok(SourceActionDecision::Stale);
        }
        for review in [&self.terms_review, &self.robots_review] {
            if let Some(decision) = review.action_decision(today, self.max_age_days)? {
                return Ok(decision);
            }
        }

        let grant_matches = matches!(
            &grant,
            SourceGrantState::Granted {
                source_id,
                policy_ref,
                permission,
                operation: granted_operation,
                policy_revision,
            } if source_id == &self.source_id
                && policy_ref == &self.policy_ref
                && *permission == rule.permission
                && *granted_operation == operation
                && *policy_revision == policy.revision
        );
        match (rule.permission, &grant) {
            (SourcePermission::None, SourceGrantState::NotRequired) => {}
            (
                SourcePermission::UserReview | SourcePermission::PairedBrowserGrant,
                SourceGrantState::Missing | SourceGrantState::NotRequired,
            ) => return Ok(SourceActionDecision::ReviewRequired),
            (SourcePermission::UserReview | SourcePermission::PairedBrowserGrant, _)
                if grant_matches => {}
            _ => return Ok(SourceActionDecision::Revoked),
        }
        Ok(SourceActionDecision::Allowed {
            request_limit_per_hour: policy.request_limit_per_hour,
            connectivity_required: operation.connectivity_required(),
        })
    }

    pub fn simulate(
        &self,
        policy: &SourcePolicy,
        operation: SourceOperation,
        today: NaiveDate,
        grant: SourceGrantState,
        fixtures: &[(&str, &[u8])],
    ) -> Result<SourceSimulationReport, String> {
        let authorized = self.authorize(policy, operation, today, grant)?;
        let decision = if self.fixtures_are_current(fixtures) {
            authorized
        } else {
            SourceActionDecision::Blocked(SourceStopCondition::ParserDrift)
        };

        let manifest_expires_on = checked_expiry(self.verified_on, self.max_age_days)?;
        let review_expires_on = [&self.terms_review, &self.robots_review]
            .into_iter()
            .filter_map(|review| {
                (review.status == SourceReviewStatus::Reviewed)
                    .then_some(review.reviewed_on)
                    .flatten()
            })
            .map(|reviewed_on| checked_expiry(reviewed_on, self.max_age_days))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min();

        Ok(SourceSimulationReport {
            decision,
            manifest_expires_on,
            review_expires_on,
            risk_note_refs: self.risk_note_refs.clone(),
        })
    }

    fn fixtures_are_current(&self, fixtures: &[(&str, &[u8])]) -> bool {
        self.fixtures.len() == fixtures.len()
            && self.fixtures.iter().all(|expected| {
                let mut matching = fixtures.iter().filter(|(path, _)| *path == expected.path);
                let Some((_, payload)) = matching.next() else {
                    return false;
                };
                matching.next().is_none()
                    && hex::encode(Sha256::digest(payload)) == expected.payload_sha256
            })
            && fixtures.iter().all(|(path, _)| {
                self.fixtures
                    .iter()
                    .filter(|expected| expected.path == *path)
                    .count()
                    == 1
            })
    }

    fn validated_freshness(&self, today: NaiveDate) -> Result<SourceFreshnessReport, String> {
        if today < self.verified_on {
            return Err("source verification date cannot be in the future".to_string());
        }
        let expires_on = checked_expiry(self.verified_on, self.max_age_days)?;
        Ok(SourceFreshnessReport {
            status: if today > expires_on {
                SourceFreshnessStatus::Stale
            } else {
                SourceFreshnessStatus::Current
            },
            verified_on: self.verified_on,
            expires_on,
        })
    }
}

fn checked_expiry(reviewed_on: NaiveDate, max_age_days: u16) -> Result<NaiveDate, String> {
    reviewed_on
        .checked_add_days(Days::new(max_age_days.into()))
        .ok_or_else(|| "source freshness window is invalid".to_string())
}

impl SourceReview {
    fn action_decision(
        &self,
        today: NaiveDate,
        max_age_days: u16,
    ) -> Result<Option<SourceActionDecision>, String> {
        match (self.status, self.reviewed_on) {
            (SourceReviewStatus::ReviewRequired, _) => {
                Ok(Some(SourceActionDecision::ReviewRequired))
            }
            (SourceReviewStatus::Reviewed, Some(reviewed_on)) if reviewed_on > today => {
                Err("source review date cannot be in the future".to_string())
            }
            (SourceReviewStatus::Reviewed, Some(reviewed_on)) => {
                let expires_on = checked_expiry(reviewed_on, max_age_days)?;
                Ok(
                    (today > expires_on).then_some(SourceActionDecision::Blocked(
                        SourceStopCondition::ReviewExpired,
                    )),
                )
            }
            _ => Ok(None),
        }
    }
}

impl SourceOperation {
    const fn connectivity_required(self) -> bool {
        matches!(
            self,
            Self::ScheduledCheck
                | Self::ConnectivityCheck
                | Self::EmployerDiscovery
                | Self::RegionalPackCheck
                | Self::RestrictedWorkbench
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{automated_url_fetch_is_blocked, visible_page_capture_is_blocked};

    #[test]
    fn policy_blocked_automation_matches_domain_boundaries() {
        for blocked in [
            "https://builtin.com/jobs",
            "https://jobs.builtin.com/role/1",
            "https://builtincolorado.com/jobs",
            "https://www.builtincolorado.com/jobs",
            "https://dice.com/jobs",
            "https://www.dice.com/job-detail/1",
            "https://glassdoor.com/jobs",
            "https://jobs.glassdoor.com/role/1",
            "https://linkedin.com/jobs",
            "https://www.linkedin.com/jobs/view/1",
            "https://simplyhired.com/search",
            "https://www.simplyhired.com/job/1",
            "https://ycombinator.com/jobs",
            "https://www.ycombinator.com/jobs",
            "https://jobs.ycombinator.com/role/1",
        ] {
            assert!(automated_url_fetch_is_blocked(blocked), "{blocked}");
        }

        for allowed in [
            "https://example.com/jobs",
            "https://dice.com.example/jobs",
            "https://notglassdoor.com/jobs",
            "https://linkedin.com.example/jobs",
            "https://notlinkedin.com/jobs",
            "https://ycombinator.com.example/jobs",
            "https://notycombinator.com/jobs",
        ] {
            assert!(!automated_url_fetch_is_blocked(allowed), "{allowed}");
        }
    }

    #[test]
    fn visible_capture_blocks_only_policy_prohibited_domains() {
        for blocked in [
            "https://www.linkedin.com/jobs/view/1",
            "https://www.ycombinator.com/jobs/1",
        ] {
            assert!(visible_page_capture_is_blocked(blocked), "{blocked}");
        }

        for allowed in [
            "https://builtin.com/jobs/1",
            "https://www.dice.com/jobs/1",
            "https://www.simplyhired.com/job/1",
            "https://jobs.glassdoor.com/jobs/1",
        ] {
            assert!(!visible_page_capture_is_blocked(allowed), "{allowed}");
        }
    }
}
