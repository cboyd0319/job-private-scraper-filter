use std::path::{Component, Path};

use chrono::NaiveDate;
use jobsentinel_security::validate_credential_free_external_https_url;
use serde::{Deserialize, Serialize};

use crate::{
    v3_contracts::{parse_contract, require_nonempty, require_schema, require_sha256, SchemaId},
    v3_foundation::{
        validate_identifier, SourceAccess, SourceGraphLink, SourcePolicy, SourceRelation,
    },
    v3_manifests::SourceClass,
};

pub const USAJOBS_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/usajobs_v1.json");
pub const USAJOBS_REQUEST_LIMIT_PER_HOUR: u16 = 60;
pub const REMOTEOK_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/remoteok_v1.json");
pub const REMOTEOK_REQUEST_LIMIT_PER_HOUR: u16 = 500;
pub const WEWORKREMOTELY_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/weworkremotely_v1.json");
pub const WEWORKREMOTELY_REQUEST_LIMIT_PER_HOUR: u16 = 1;
pub const HN_HIRING_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/hn_hiring_v1.json");
pub const HN_HIRING_REQUEST_LIMIT_PER_HOUR: u16 = 500;
pub const HN_HIRING_SEARCH_ENDPOINT: &str = "https://hn.algolia.com/api/v1/search_by_date";
pub const HN_HIRING_ITEM_ENDPOINT_PREFIX: &str = "https://hn.algolia.com/api/v1/items/";
pub const GREENHOUSE_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/greenhouse_v1.json");
pub const GREENHOUSE_REQUEST_LIMIT_PER_HOUR: u16 = 1_000;
pub const GREENHOUSE_API_ENDPOINT_PREFIX: &str = "https://boards-api.greenhouse.io/v1/boards/";
pub const LEVER_SOURCE_MANIFEST_V1: &str = include_str!("fixtures/source_manifests/lever_v1.json");
pub const LEVER_REQUEST_LIMIT_PER_HOUR: u16 = 1_000;
pub const LEVER_API_ENDPOINT_PREFIX: &str = "https://api.lever.co/v0/postings/";
pub const JOBSWITHGPT_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/jobswithgpt_v1.json");
pub const LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/linkedin_workbench_v1.json");
pub const USER_SOURCE_ACTIONS_MANIFEST_V1: &str =
    include_str!("fixtures/source_manifests/user_source_actions_v1.json");
pub const BUILTIN_SOURCE_MANIFEST_V2: &str =
    include_str!("fixtures/source_manifests/builtin_v2.json");
pub const DICE_SOURCE_MANIFEST_V2: &str = include_str!("fixtures/source_manifests/dice_v2.json");
pub const SIMPLYHIRED_SOURCE_MANIFEST_V2: &str =
    include_str!("fixtures/source_manifests/simplyhired_v2.json");
pub const GLASSDOOR_SOURCE_MANIFEST_V2: &str =
    include_str!("fixtures/source_manifests/glassdoor_v2.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAuthRequirement {
    None,
    LocalCredential,
    UserSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOperation {
    ScheduledCheck,
    ConnectivityCheck,
    EmployerDiscovery,
    RegionalPackCheck,
    RestrictedWorkbench,
    VisiblePageCapture,
    SmartPaste,
    AppliedLogging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePermission {
    None,
    UserReview,
    PairedBrowserGrant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceActionRule {
    pub operation: SourceOperation,
    pub permission: SourcePermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceReviewStatus {
    NotApplicable,
    Reviewed,
    ReviewRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceReview {
    pub status: SourceReviewStatus,
    pub reference: Option<String>,
    pub reviewed_on: Option<NaiveDate>,
}

impl SourceReview {
    fn validate(&self, label: &str) -> Result<(), String> {
        match (self.status, self.reference.as_deref(), self.reviewed_on) {
            (SourceReviewStatus::NotApplicable, None, None) => Ok(()),
            (SourceReviewStatus::Reviewed, Some(reference), Some(_))
            | (SourceReviewStatus::ReviewRequired, Some(reference), None) => {
                validate_identifier(label, reference)
            }
            _ => Err(format!("{label} review evidence is inconsistent")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFixtureKind {
    List,
    Detail,
    VisiblePage,
    Policy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFixtureRef {
    pub fixture_id: String,
    pub kind: SourceFixtureKind,
    pub path: String,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceField {
    Title,
    Company,
    Location,
    Description,
    Salary,
    EmploymentType,
    PostedAt,
    ApplicationUrl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SalaryCoverage {
    None,
    Partial,
    Declared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStopCondition {
    PolicyDisabled,
    PolicyChanged,
    ReviewExpired,
    RateLimitReached,
    AccessBlocked,
    BotChallenge,
    ParserDrift,
    RepeatedFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceManifest {
    pub schema: SchemaId,
    pub source_id: String,
    pub display_name: String,
    pub source_class: SourceClass,
    pub endpoint_patterns: Vec<String>,
    pub documentation_url: Option<String>,
    pub auth_requirement: SourceAuthRequirement,
    pub policy_ref: String,
    pub policy_revision: u32,
    pub actions: Vec<SourceActionRule>,
    pub terms_review: SourceReview,
    pub robots_review: SourceReview,
    pub parser_id: String,
    pub fixtures: Vec<SourceFixtureRef>,
    pub supported_fields: Vec<SourceField>,
    pub salary_coverage: SalaryCoverage,
    pub verified_on: NaiveDate,
    pub max_age_days: u16,
    pub confidence_percent: u8,
    pub lineage: Vec<SourceGraphLink>,
    pub stop_conditions: Vec<SourceStopCondition>,
    pub risk_note_refs: Vec<String>,
    pub incomplete_coverage: bool,
}

impl SourceManifest {
    pub fn validate(&self, policy: &SourcePolicy) -> Result<(), String> {
        require_schema(self.schema, SchemaId::SourceManifestV1)?;
        policy.validate()?;
        validate_identifier("source", &self.source_id)?;
        require_bounded_text("source display name", &self.display_name)?;
        validate_identifier("source policy", &self.policy_ref)?;
        validate_identifier("source parser", &self.parser_id)?;
        if self.source_id != policy.source_id || self.source_class != policy.source_class {
            return Err("source manifest does not match the current policy".to_string());
        }
        if self.policy_ref != policy.policy_ref {
            return Err("source manifest does not match the current policy".to_string());
        }
        if self.policy_revision != policy.revision {
            return Err("source manifest does not match the current policy".to_string());
        }
        if self.policy_revision == 0 {
            return Err("source manifest policy revision must be positive".to_string());
        }

        require_unique_nonempty("source actions", &self.actions, |value| value.operation)?;
        for action in &self.actions {
            validate_action(action, policy)?;
        }

        if self.endpoint_patterns.is_empty()
            && !matches!(self.source_class, SourceClass::UserImport)
            && !matches!(policy.access, SourceAccess::Disabled)
        {
            return Err("active source endpoints are required for non-import sources".to_string());
        }
        if !self.endpoint_patterns.is_empty() {
            require_unique_nonempty("source endpoints", &self.endpoint_patterns, Clone::clone)?;
        }
        for endpoint in &self.endpoint_patterns {
            let parsed = validate_credential_free_external_https_url(endpoint)
                .map_err(|_| "source endpoint pattern must be public credential-free HTTPS")?;
            if endpoint.contains('*') || parsed.path_segments().is_none() {
                return Err("source endpoint pattern is unsafe".to_string());
            }
        }
        if let Some(documentation_url) = self.documentation_url.as_deref() {
            validate_credential_free_external_https_url(documentation_url)
                .map_err(|_| "source documentation URL must be public credential-free HTTPS")?;
        }

        if matches!(self.auth_requirement, SourceAuthRequirement::UserSession)
            != matches!(self.source_class, SourceClass::RestrictedUserOpened)
        {
            return Err("source session requirement does not match its class".to_string());
        }
        self.terms_review.validate("source terms")?;
        self.robots_review.validate("source robots")?;
        if matches!(
            self.source_class,
            SourceClass::PublicAts
                | SourceClass::PublicEmployerPage
                | SourceClass::RegionalBoard
                | SourceClass::RestrictedPublicScheduled
        ) && matches!(self.robots_review.status, SourceReviewStatus::NotApplicable)
        {
            return Err("public page sources require a robots review state".to_string());
        }

        require_unique_nonempty("source fixtures", &self.fixtures, |value| {
            value.fixture_id.clone()
        })?;
        for fixture in &self.fixtures {
            validate_identifier("source fixture", &fixture.fixture_id)?;
            validate_fixture_path(&fixture.path)?;
            require_sha256("source fixture payload", &fixture.payload_sha256)?;
            if fixture
                .payload_sha256
                .bytes()
                .any(|byte| byte.is_ascii_uppercase())
            {
                return Err("source fixture SHA-256 must be lowercase".to_string());
            }
        }
        if [&self.terms_review, &self.robots_review]
            .iter()
            .any(|review| review.status == SourceReviewStatus::Reviewed)
            && !self
                .fixtures
                .iter()
                .any(|fixture| fixture.kind == SourceFixtureKind::Policy)
        {
            return Err("reviewed source policy evidence must be hash-bound".to_string());
        }
        require_unique_nonempty("source fields", &self.supported_fields, |value| *value)?;
        if self.max_age_days == 0
            || self.max_age_days > 3_650
            || self.confidence_percent == 0
            || self.confidence_percent > 100
        {
            return Err("source freshness and confidence must be bounded".to_string());
        }

        if self.lineage.is_empty() && !matches!(self.source_class, SourceClass::UserImport) {
            return Err("source lineage is required for non-import sources".to_string());
        }
        if !self.lineage.is_empty() {
            require_unique_nonempty("source lineage", &self.lineage, |value| {
                value.link_id.clone()
            })?;
        }
        for link in &self.lineage {
            link.validate()?;
            if link.source_id != self.source_id || link.relation != SourceRelation::Lineage {
                return Err("source lineage must belong to the manifest source".to_string());
            }
        }
        require_unique_nonempty("source stop conditions", &self.stop_conditions, |value| {
            *value
        })?;
        for required in [
            SourceStopCondition::PolicyDisabled,
            SourceStopCondition::PolicyChanged,
            SourceStopCondition::ReviewExpired,
            SourceStopCondition::AccessBlocked,
            SourceStopCondition::ParserDrift,
        ] {
            if !self.stop_conditions.contains(&required) {
                return Err("source manifest omits a required stop condition".to_string());
            }
        }
        if !self.risk_note_refs.is_empty() {
            require_unique_nonempty("source risk notes", &self.risk_note_refs, Clone::clone)?;
        }
        for reference in &self.risk_note_refs {
            validate_identifier("source risk note", reference)?;
        }
        if matches!(self.source_class, SourceClass::RegionalBoard) && !self.incomplete_coverage {
            return Err("regional source coverage must be labeled incomplete".to_string());
        }
        Ok(())
    }
}

pub fn parse_source_manifest(input: &str, policy: &SourcePolicy) -> Result<SourceManifest, String> {
    let manifest: SourceManifest = parse_contract(input, SchemaId::SourceManifestV1)?;
    manifest.validate(policy)?;
    Ok(manifest)
}

fn validate_action(action: &SourceActionRule, policy: &SourcePolicy) -> Result<(), String> {
    if matches!(policy.access, SourceAccess::UserOpened)
        && matches!(action.permission, SourcePermission::None)
    {
        return Err("user-opened source actions require explicit permission".to_string());
    }
    if matches!(action.operation, SourceOperation::ScheduledCheck)
        && matches!(policy.access, SourceAccess::UserOpened)
    {
        return Err("user-opened sources cannot schedule checks".to_string());
    }
    if matches!(action.permission, SourcePermission::PairedBrowserGrant)
        && !matches!(
            action.operation,
            SourceOperation::VisiblePageCapture | SourceOperation::AppliedLogging
        )
    {
        return Err("browser grants cannot authorize unrelated source actions".to_string());
    }
    if matches!(
        action.operation,
        SourceOperation::VisiblePageCapture | SourceOperation::AppliedLogging
    ) && !matches!(action.permission, SourcePermission::PairedBrowserGrant)
    {
        return Err("browser source actions require a paired browser grant".to_string());
    }
    if matches!(
        action.operation,
        SourceOperation::RestrictedWorkbench
            | SourceOperation::SmartPaste
            | SourceOperation::AppliedLogging
    ) && matches!(action.permission, SourcePermission::None)
    {
        return Err("explicit user source actions require permission".to_string());
    }
    if matches!(action.operation, SourceOperation::ScheduledCheck)
        && policy.user_review_required != matches!(action.permission, SourcePermission::UserReview)
    {
        return Err("scheduled source review does not match policy".to_string());
    }
    Ok(())
}

fn validate_fixture_path(value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || value.len() > 256
        || value.contains(['\\', ':'])
        || path.is_absolute()
        || !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("json" | "xml")
        )
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("source fixture path must be a repo-relative JSON or XML path".to_string());
    }
    Ok(())
}

fn require_bounded_text(label: &str, value: &str) -> Result<(), String> {
    require_nonempty(label, value)?;
    if value.len() > 128 {
        return Err(format!("{label} exceeds the byte limit"));
    }
    Ok(())
}

fn require_unique_nonempty<T, K: PartialEq>(
    label: &str,
    values: &[T],
    key: impl Fn(&T) -> K,
) -> Result<(), String> {
    if values.is_empty()
        || values.len() > 32
        || values
            .iter()
            .enumerate()
            .any(|(index, value)| values[..index].iter().any(|other| key(other) == key(value)))
    {
        return Err(format!("{label} must be a bounded nonempty set"));
    }
    Ok(())
}
