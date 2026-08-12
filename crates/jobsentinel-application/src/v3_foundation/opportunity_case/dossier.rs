//! Builds a renderer-safe employer dossier from reviewed source metadata and count-only local history.

use chrono::{DateTime, NaiveDate, Utc};
use jobsentinel_domain::{
    v3_manifests::SourceClass,
    v3_source_authorization::{SourceFreshnessReport, SourceFreshnessStatus},
    v3_source_manifest::{SalaryCoverage, SourceField, SourceStopCondition},
};
use jobsentinel_security::validate_credential_free_external_https_url;
use jobsentinel_storage::{
    v3_foundation::{EmployerHistoryRead, OpportunityCaseRead},
    Database,
};
use serde::Serialize;

use crate::v3_foundation::FoundationError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerDossier {
    pub employer: EmployerIdentityEvidence,
    pub role: EmployerRoleEvidence,
    pub source: EmployerSourceEvidence,
    pub pay: EmployerPayEvidence,
    pub local_history: EmployerLocalHistory,
    pub application_channel: EmployerApplicationChannel,
    pub uncertainty: Vec<EmployerDossierUncertainty>,
    pub next_action: EmployerDossierNextAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerIdentityEvidence {
    pub name: String,
    pub identity_status: EmployerIdentityStatus,
    pub official_domain: Option<String>,
    pub posting_domain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerIdentityStatus {
    UnverifiedSavedName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerRoleEvidence {
    pub title: String,
    pub status: EmployerRoleStatus,
    pub posting_url: Option<String>,
    pub first_observed_at: DateTime<Utc>,
    pub last_observed_at: DateTime<Utc>,
    pub times_seen: i64,
    pub repost_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerRoleStatus {
    LastObserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerSourceEvidence {
    pub source_id: String,
    pub display_name: Option<String>,
    pub source_class: Option<SourceClass>,
    pub status: EmployerSourceStatus,
    pub documentation_url: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub retrieved_at: Option<DateTime<Utc>>,
    pub verified_on: Option<NaiveDate>,
    pub expires_on: Option<NaiveDate>,
    pub jurisdiction: Option<String>,
    pub confidence_percent: Option<u8>,
    pub policy_ref: Option<String>,
    pub policy_revision: Option<u32>,
    pub terms_review_ref: Option<String>,
    pub robots_review_ref: Option<String>,
    pub parser_version: Option<String>,
    pub salary_coverage: Option<SalaryCoverage>,
    pub incomplete_coverage: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerSourceStatus {
    Current,
    Stale,
    ReviewRequired,
    PolicyDisabled,
    PolicyChanged,
    ReviewExpired,
    Unavailable,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerPayEvidence {
    pub clarity: EmployerPayClarity,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
    pub currency: Option<String>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerPayClarity {
    RangeListed,
    MinimumOnly,
    MaximumOnly,
    NotListed,
    Unusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmployerLocalHistory {
    pub basis: EmployerLocalHistoryBasis,
    pub saved_job_count: u32,
    pub application_count: u32,
    pub interview_count: u32,
    pub offer_count: u32,
    pub terminal_outcome_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerLocalHistoryBasis {
    ExactSavedName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerApplicationChannel {
    PublicAts,
    OfficialPublicSource,
    EmployerPage,
    UserProvided,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerDossierUncertainty {
    EmployerIdentityNotCanonical,
    OfficialDomainUnknown,
    RoleNotLiveChecked,
    RetrievalDateUnavailable,
    JurisdictionUnknown,
    ExactNameHistoryOnly,
    SourceMissing,
    SourceNotCurrent,
    PayNotListed,
    PayUnusable,
    PayProvenanceIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerDossierNextAction {
    OpenSavedPosting,
    VerifyOnEmployerCareersPage,
}

pub(super) async fn build_employer_dossier(
    database: &Database,
    read: &OpportunityCaseRead,
    today: NaiveDate,
) -> Result<EmployerDossier, FoundationError> {
    let manifest = database
        .get_source_manifest(&read.source_name)
        .await
        .map_err(super::super::map_error)?;
    let policy = database
        .get_source_policy(&read.source_name)
        .await
        .map_err(super::super::map_error)?;
    let (source, supports_application_url) = match (manifest, policy) {
        (Some(manifest), Some(policy)) => {
            let freshness = manifest
                .freshness(&policy, today)
                .map_err(|_| FoundationError::InvalidInput)?;
            let supports_application_url = manifest
                .supported_fields
                .contains(&SourceField::ApplicationUrl);
            (
                EmployerSourceEvidence {
                    source_id: manifest.source_id,
                    display_name: Some(manifest.display_name),
                    source_class: Some(manifest.source_class),
                    status: source_status(freshness),
                    documentation_url: manifest.documentation_url,
                    observed_at: read.last_seen_at,
                    retrieved_at: None,
                    verified_on: Some(freshness.verified_on),
                    expires_on: Some(freshness.expires_on),
                    jurisdiction: None,
                    confidence_percent: Some(manifest.confidence_percent),
                    policy_ref: Some(manifest.policy_ref),
                    policy_revision: Some(manifest.policy_revision),
                    terms_review_ref: manifest.terms_review.reference,
                    robots_review_ref: manifest.robots_review.reference,
                    parser_version: Some(manifest.parser_id),
                    salary_coverage: Some(manifest.salary_coverage),
                    incomplete_coverage: Some(manifest.incomplete_coverage),
                },
                supports_application_url,
            )
        }
        _ => (missing_source(read), false),
    };
    let posting = validated_posting(&read.source_name, &read.job_url);
    let pay = pay_evidence(read);
    let application_channel = application_channel(&source, supports_application_url);
    let mut uncertainty = vec![
        EmployerDossierUncertainty::EmployerIdentityNotCanonical,
        EmployerDossierUncertainty::OfficialDomainUnknown,
        EmployerDossierUncertainty::RoleNotLiveChecked,
        EmployerDossierUncertainty::RetrievalDateUnavailable,
        EmployerDossierUncertainty::JurisdictionUnknown,
        EmployerDossierUncertainty::ExactNameHistoryOnly,
    ];
    if source.status == EmployerSourceStatus::Missing {
        uncertainty.push(EmployerDossierUncertainty::SourceMissing);
    } else if source.status != EmployerSourceStatus::Current {
        uncertainty.push(EmployerDossierUncertainty::SourceNotCurrent);
    }
    match pay.clarity {
        EmployerPayClarity::NotListed => uncertainty.push(EmployerDossierUncertainty::PayNotListed),
        EmployerPayClarity::Unusable => uncertainty.push(EmployerDossierUncertainty::PayUnusable),
        _ if source.salary_coverage != Some(SalaryCoverage::Declared) => {
            uncertainty.push(EmployerDossierUncertainty::PayProvenanceIncomplete);
        }
        _ => {}
    }
    let next_action = if source.status == EmployerSourceStatus::Current && posting.is_some() {
        EmployerDossierNextAction::OpenSavedPosting
    } else {
        EmployerDossierNextAction::VerifyOnEmployerCareersPage
    };
    Ok(EmployerDossier {
        employer: EmployerIdentityEvidence {
            name: read.company.clone(),
            identity_status: EmployerIdentityStatus::UnverifiedSavedName,
            official_domain: None,
            posting_domain: posting.as_ref().map(|(_, domain)| domain.clone()),
        },
        role: EmployerRoleEvidence {
            title: read.title.clone(),
            status: EmployerRoleStatus::LastObserved,
            posting_url: posting.map(|(url, _)| url),
            first_observed_at: read.first_seen_at,
            last_observed_at: read.last_seen_at,
            times_seen: read.times_seen,
            repost_count: read.repost_count,
        },
        source,
        pay,
        local_history: local_history(&read.employer_history),
        application_channel,
        uncertainty,
        next_action,
    })
}

fn missing_source(read: &OpportunityCaseRead) -> EmployerSourceEvidence {
    EmployerSourceEvidence {
        source_id: read.source_name.clone(),
        display_name: None,
        source_class: None,
        status: EmployerSourceStatus::Missing,
        documentation_url: None,
        observed_at: read.last_seen_at,
        retrieved_at: None,
        verified_on: None,
        expires_on: None,
        jurisdiction: None,
        confidence_percent: None,
        policy_ref: None,
        policy_revision: None,
        terms_review_ref: None,
        robots_review_ref: None,
        parser_version: None,
        salary_coverage: None,
        incomplete_coverage: None,
    }
}

fn source_status(report: SourceFreshnessReport) -> EmployerSourceStatus {
    match report.status {
        SourceFreshnessStatus::Current => EmployerSourceStatus::Current,
        SourceFreshnessStatus::Stale => EmployerSourceStatus::Stale,
        SourceFreshnessStatus::ReviewRequired => EmployerSourceStatus::ReviewRequired,
        SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyDisabled) => {
            EmployerSourceStatus::PolicyDisabled
        }
        SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyChanged) => {
            EmployerSourceStatus::PolicyChanged
        }
        SourceFreshnessStatus::Blocked(SourceStopCondition::ReviewExpired) => {
            EmployerSourceStatus::ReviewExpired
        }
        SourceFreshnessStatus::Blocked(_) => EmployerSourceStatus::Unavailable,
    }
}

fn application_channel(
    source: &EmployerSourceEvidence,
    supports_url: bool,
) -> EmployerApplicationChannel {
    if !supports_url {
        return EmployerApplicationChannel::Unknown;
    }
    match source.source_class {
        Some(SourceClass::PublicAts) => EmployerApplicationChannel::PublicAts,
        Some(SourceClass::OfficialPublicApi) => EmployerApplicationChannel::OfficialPublicSource,
        Some(SourceClass::PublicEmployerPage) => EmployerApplicationChannel::EmployerPage,
        Some(SourceClass::UserImport) => EmployerApplicationChannel::UserProvided,
        _ => EmployerApplicationChannel::Unknown,
    }
}

fn validated_posting(source_id: &str, raw: &str) -> Option<(String, String)> {
    let parsed = validate_credential_free_external_https_url(raw).ok()?;
    let domain = parsed
        .host_str()?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let allowed = match source_id {
        "greenhouse" => matches!(
            domain.as_str(),
            "boards.greenhouse.io" | "job-boards.greenhouse.io"
        ),
        "lever" => domain == "jobs.lever.co",
        _ => false,
    };
    (allowed && parsed.port().is_none() && parsed.query().is_none() && parsed.fragment().is_none())
        .then_some((parsed.to_string(), domain))
}

fn pay_evidence(read: &OpportunityCaseRead) -> EmployerPayEvidence {
    let currency = read
        .currency
        .as_deref()
        .filter(|value| value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase()));
    let (clarity, minimum, maximum, currency) = match (read.salary_min, read.salary_max, currency) {
        (None, None, _) => (EmployerPayClarity::NotListed, None, None, None),
        (Some(minimum), Some(maximum), Some(currency)) if minimum > 0 && maximum >= minimum => (
            EmployerPayClarity::RangeListed,
            Some(minimum),
            Some(maximum),
            Some(currency.to_string()),
        ),
        (Some(minimum), None, Some(currency)) if minimum > 0 => (
            EmployerPayClarity::MinimumOnly,
            Some(minimum),
            None,
            Some(currency.to_string()),
        ),
        (None, Some(maximum), Some(currency)) if maximum > 0 => (
            EmployerPayClarity::MaximumOnly,
            None,
            Some(maximum),
            Some(currency.to_string()),
        ),
        _ => (EmployerPayClarity::Unusable, None, None, None),
    };
    EmployerPayEvidence {
        clarity,
        minimum,
        maximum,
        currency,
        observed_at: read.last_seen_at,
    }
}

fn local_history(read: &EmployerHistoryRead) -> EmployerLocalHistory {
    EmployerLocalHistory {
        basis: EmployerLocalHistoryBasis::ExactSavedName,
        saved_job_count: read.saved_job_count,
        application_count: read.application_count,
        interview_count: read.interview_count,
        offer_count: read.offer_count,
        terminal_outcome_count: read.terminal_outcome_count,
    }
}
