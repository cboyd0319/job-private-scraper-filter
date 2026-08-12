use chrono::NaiveDate;
use jobsentinel_security::validate_credential_free_external_https_url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::v3_contracts::{
    require_nonempty, require_schema, require_sha256, require_v3_semver, Architecture, PayPeriod,
    Platform, SchemaId,
};

mod privacy_validation;

pub const EXTERNAL_AI_GATEWAY_POLICY: &str = "jobsentinel.external-ai-gateway.v1";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLabel {
    LocalOnly,
    ExternalAiOptional,
    Sensitive,
    PublicDataOnly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    PublicJobPosting,
    ResumeEvidence,
    ApplicationHistory,
    CareerGoals,
    PayPreferences,
    LocationPreferences,
    MilitaryService,
    ClearanceClaim,
    ProtectedVeteranAnswer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyReceipt {
    pub schema: SchemaId,
    pub receipt_id: String,
    pub task_id: String,
    pub pack_id: Option<String>,
    pub labels: Vec<PrivacyLabel>,
    pub data_categories: Vec<DataCategory>,
    pub stored_locally: bool,
    pub data_left_device: bool,
    pub external_destination: Option<String>,
    pub gateway_policy_id: Option<String>,
    pub approval_reference: Option<String>,
    pub delete_or_revoke_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskKind {
    SourceCheck,
    EvidenceReview,
    DraftPacket,
    Backup,
    Export,
    PackInstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTask {
    pub schema: SchemaId,
    pub task_id: String,
    pub pack_id: Option<String>,
    pub kind: AgentTaskKind,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub data_categories: Vec<DataCategory>,
    pub max_duration_seconds: u32,
    pub max_output_bytes: u64,
    pub max_attempts: u8,
    pub user_review_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackType {
    Skill,
    Agent,
    Workflow,
    Role,
    Region,
    Source,
    Rubric,
    Evaluation,
    Template,
    OsHelper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackExecutionClass {
    StaticContent,
    ReviewedTypedWorkflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackAction {
    ReadSelectedCaseFile,
    ReadSelectedResumeEvidence,
    ReadPublicJobPosting,
    CreateDraftLocalNote,
    CreateDraftApplicationPacket,
    CreateReminder,
    OpenBrowserLink,
    RequestSourceCheck,
    RequestExternalAi,
    WriteLocalEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalGate {
    PerExecutionReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub schema: SchemaId,
    pub pack_id: String,
    pub pack_type: PackType,
    pub execution_class: PackExecutionClass,
    pub publisher_key_id: String,
    pub payload_sha256: String,
    pub privacy_labels: Vec<PrivacyLabel>,
    pub allowed_data_categories: Vec<DataCategory>,
    pub allowed_task_kinds: Vec<AgentTaskKind>,
    pub allowed_actions: Vec<PackAction>,
    pub approval_gates: Vec<ApprovalGate>,
    pub gateway_policy_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    OfficialPublicApi,
    PublicAts,
    PublicEmployerPage,
    RegionalBoard,
    RestrictedPublicScheduled,
    UserImport,
    RestrictedUserOpened,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationRules {
    pub remote_labels: Vec<String>,
    pub subdivision_required: bool,
    pub locality_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionManifest {
    pub schema: SchemaId,
    pub region_id: String,
    pub reviewed_on: NaiveDate,
    pub country_codes: Vec<String>,
    pub languages: Vec<String>,
    pub currencies: Vec<String>,
    pub pay_periods: Vec<PayPeriod>,
    pub location_rules: LocationRules,
    pub work_authorization_labels: Vec<String>,
    pub source_classes: Vec<SourceClass>,
    pub cv_profiles: Vec<String>,
    pub taxonomy_ids: Vec<String>,
    pub policy_note_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub evaluation_fixture_ids: Vec<String>,
    pub incomplete_coverage: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    Essentials,
    Standard,
    ProLocal,
    Developer,
    Portable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProfile {
    Balanced,
    Lighter,
    StrongerLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditionComponent {
    DeterministicMatching,
    PublicSources,
    SafeSupport,
    GovernedModelSetup,
    BrowserCompanion,
    PackDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditionManifest {
    pub schema: SchemaId,
    pub edition: Edition,
    pub profile: ModelProfile,
    pub artifact_version: String,
    pub platform: Platform,
    pub architecture: Architecture,
    pub compatibility_line: u32,
    pub rollback_supported: bool,
    pub default_components: Vec<EditionComponent>,
    pub automatic_model_downloads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProvenance {
    pub schema: SchemaId,
    pub model_id: String,
    pub revision: String,
    pub backend: String,
    pub dimension: u32,
    pub tokenizer_sha256: String,
    pub manifest_sha256: String,
    pub instruction_profile: String,
    pub pooling: String,
    pub normalization: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorFreshness {
    pub schema: SchemaId,
    pub model_id: String,
    pub model_revision: String,
    pub backend: String,
    pub dimension: u32,
    pub instruction_profile: String,
    pub chunker_version: String,
    pub normalizer_version: String,
    pub pooling: String,
    pub normalization: String,
    pub model_manifest_sha256: String,
    pub content_sha256: String,
}

impl PackManifest {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::PackManifestV1)?;
        require_nonempty("pack id", &self.pack_id)?;
        require_nonempty("publisher key id", &self.publisher_key_id)?;
        require_sha256("pack payload", &self.payload_sha256)?;
        if self.privacy_labels.is_empty() {
            return Err("pack privacy labels are required".to_string());
        }
        match self.execution_class {
            PackExecutionClass::StaticContent
                if !self.allowed_actions.is_empty()
                    || !self.allowed_task_kinds.is_empty()
                    || !self.approval_gates.is_empty() =>
            {
                return Err("static content cannot request executable capabilities".to_string());
            }
            PackExecutionClass::ReviewedTypedWorkflow
                if self.allowed_actions.is_empty()
                    || self.allowed_task_kinds.is_empty()
                    || self.allowed_data_categories.is_empty()
                    || !self
                        .approval_gates
                        .contains(&ApprovalGate::PerExecutionReview) =>
            {
                return Err(
                    "typed workflow must declare reviewed tasks, data, and actions".to_string(),
                );
            }
            _ => {}
        }

        let requests_external_ai = self
            .allowed_actions
            .contains(&PackAction::RequestExternalAi);
        if requests_external_ai {
            if self.gateway_policy_id.as_deref() != Some(EXTERNAL_AI_GATEWAY_POLICY)
                || !self
                    .privacy_labels
                    .contains(&PrivacyLabel::ExternalAiOptional)
                || !self.privacy_labels.contains(&PrivacyLabel::PublicDataOnly)
                || self.privacy_labels.contains(&PrivacyLabel::LocalOnly)
                || self.privacy_labels.contains(&PrivacyLabel::Sensitive)
                || self.allowed_data_categories.is_empty()
                || !self
                    .allowed_data_categories
                    .iter()
                    .all(|category| *category == DataCategory::PublicJobPosting)
                || !self
                    .allowed_task_kinds
                    .iter()
                    .all(|kind| *kind == AgentTaskKind::SourceCheck)
                || !self.allowed_actions.iter().all(|action| {
                    matches!(
                        action,
                        PackAction::ReadPublicJobPosting
                            | PackAction::CreateDraftLocalNote
                            | PackAction::RequestSourceCheck
                            | PackAction::RequestExternalAi
                            | PackAction::WriteLocalEvent
                    )
                })
            {
                return Err(
                    "external AI pack actions are gateway-bound and public-data-only".to_string(),
                );
            }
        } else if self.gateway_policy_id.is_some() {
            return Err("pack cannot declare an unused external AI gateway".to_string());
        }
        Ok(())
    }

    pub fn verify_payload(&self, payload: &[u8]) -> Result<(), String> {
        self.validate()?;
        let expected = hex::decode(&self.payload_sha256)
            .map_err(|_| "pack payload integrity check failed".to_string())?;
        let actual: [u8; 32] = Sha256::digest(payload).into();
        if actual.as_slice() == expected {
            Ok(())
        } else {
            Err("pack payload integrity check failed".to_string())
        }
    }
}

impl RegionManifest {
    pub fn validate(&self, today: NaiveDate) -> Result<(), String> {
        require_schema(self.schema, SchemaId::RegionManifestV1)?;
        require_nonempty("region id", &self.region_id)?;
        if self.reviewed_on > today
            || !self.incomplete_coverage
            || self.country_codes.is_empty()
            || !self
                .country_codes
                .iter()
                .all(|code| is_upper_ascii_code(code, 2))
            || self.currencies.is_empty()
            || !self
                .currencies
                .iter()
                .all(|code| is_upper_ascii_code(code, 3))
            || self.languages.is_empty()
            || self.pay_periods.is_empty()
            || self.location_rules.remote_labels.is_empty()
            || self.work_authorization_labels.is_empty()
            || self.source_classes.is_empty()
            || self.cv_profiles.is_empty()
            || self.taxonomy_ids.is_empty()
            || self.policy_note_refs.is_empty()
            || !self
                .policy_note_refs
                .iter()
                .all(|reference| is_reference(reference))
            || self.provenance_refs.is_empty()
            || !self
                .provenance_refs
                .iter()
                .all(|reference| is_reference(reference))
            || self.evaluation_fixture_ids.is_empty()
        {
            return Err("region manifest must declare reviewed starter coverage".to_string());
        }
        Ok(())
    }
}

impl EditionManifest {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::EditionManifestV1)?;
        require_v3_semver("edition artifact version", &self.artifact_version)?;
        if self.compatibility_line != 3
            || !self.rollback_supported
            || self.default_components.is_empty()
        {
            return Err("edition must remain rollback-compatible with the v3 line".to_string());
        }
        if matches!(self.edition, Edition::Essentials)
            && (self.automatic_model_downloads || !matches!(self.profile, ModelProfile::Lighter))
        {
            return Err(
                "Essentials must remain lighter with no automatic model download".to_string(),
            );
        }
        Ok(())
    }
}

impl ModelProvenance {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::ModelProvenanceV1)?;
        require_nonempty("model id", &self.model_id)?;
        require_nonempty("model revision", &self.revision)?;
        require_nonempty("model backend", &self.backend)?;
        require_nonempty("instruction profile", &self.instruction_profile)?;
        require_nonempty("pooling", &self.pooling)?;
        require_nonempty("normalization", &self.normalization)?;
        if self.dimension == 0 {
            return Err("model dimension must be positive".to_string());
        }
        require_sha256("tokenizer", &self.tokenizer_sha256)?;
        require_sha256("model manifest", &self.manifest_sha256)
    }
}

impl VectorFreshness {
    pub fn validate(&self, model: &ModelProvenance) -> Result<(), String> {
        require_schema(self.schema, SchemaId::VectorFreshnessV1)?;
        require_nonempty("chunker version", &self.chunker_version)?;
        require_nonempty("normalizer version", &self.normalizer_version)?;
        require_sha256("vector content", &self.content_sha256)?;
        let stored_identity = (
            &self.model_id,
            &self.model_revision,
            &self.backend,
            self.dimension,
            &self.instruction_profile,
            &self.pooling,
            &self.normalization,
            &self.model_manifest_sha256,
        );
        let current_identity = (
            &model.model_id,
            &model.revision,
            &model.backend,
            model.dimension,
            &model.instruction_profile,
            &model.pooling,
            &model.normalization,
            &model.manifest_sha256,
        );
        if stored_identity != current_identity {
            return Err("vector freshness does not match model provenance".to_string());
        }
        Ok(())
    }
}

fn is_upper_ascii_code(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn is_reference(value: &str) -> bool {
    value.strip_prefix("docs/").is_some_and(|path| {
        !path.is_empty() && !path.contains("..") && !path.chars().any(char::is_whitespace)
    }) || (value
        .strip_prefix("https://")
        .is_some_and(|authority| !authority.starts_with('/'))
        && validate_credential_free_external_https_url(value).is_ok())
}
