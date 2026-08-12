use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::v3_manifests::{
    AgentTask, EditionManifest, ModelProvenance, PackManifest, PrivacyReceipt, RegionManifest,
    VectorFreshness,
};

const MAX_DATABASE_SCHEMA: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaId {
    #[serde(rename = "jobsentinel.v3.compatibility.v1")]
    CompatibilityV1,
    #[serde(rename = "jobsentinel.v3.artifact-manifest.v1")]
    ArtifactManifestV1,
    #[serde(rename = "jobsentinel.v3.privacy-receipt.v1")]
    PrivacyReceiptV1,
    #[serde(rename = "jobsentinel.v3.agent-task.v1")]
    AgentTaskV1,
    #[serde(rename = "jobsentinel.v3.pack-manifest.v1")]
    PackManifestV1,
    #[serde(rename = "jobsentinel.v3.region-manifest.v1")]
    RegionManifestV1,
    #[serde(rename = "jobsentinel.v3.source-manifest.v1")]
    SourceManifestV1,
    #[serde(rename = "jobsentinel.v3.veteran-public-service-index.v1")]
    VeteranPublicServiceIndexV1,
    #[serde(rename = "jobsentinel.v3.edition-manifest.v1")]
    EditionManifestV1,
    #[serde(rename = "jobsentinel.v3.model-provenance.v1")]
    ModelProvenanceV1,
    #[serde(rename = "jobsentinel.v3.vector-freshness.v1")]
    VectorFreshnessV1,
}

impl SchemaId {
    fn as_str(self) -> &'static str {
        match self {
            Self::CompatibilityV1 => "jobsentinel.v3.compatibility.v1",
            Self::ArtifactManifestV1 => "jobsentinel.v3.artifact-manifest.v1",
            Self::PrivacyReceiptV1 => "jobsentinel.v3.privacy-receipt.v1",
            Self::AgentTaskV1 => "jobsentinel.v3.agent-task.v1",
            Self::PackManifestV1 => "jobsentinel.v3.pack-manifest.v1",
            Self::RegionManifestV1 => "jobsentinel.v3.region-manifest.v1",
            Self::SourceManifestV1 => "jobsentinel.v3.source-manifest.v1",
            Self::VeteranPublicServiceIndexV1 => "jobsentinel.v3.veteran-public-service-index.v1",
            Self::EditionManifestV1 => "jobsentinel.v3.edition-manifest.v1",
            Self::ModelProvenanceV1 => "jobsentinel.v3.model-provenance.v1",
            Self::VectorFreshnessV1 => "jobsentinel.v3.vector-freshness.v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityInputKind {
    PreV3Config,
    V3Manifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveKind {
    Backup,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveProtection {
    Encrypted,
    ReviewedPlaintext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveEnvelope {
    pub kind: ArchiveKind,
    pub format_version: u32,
    pub contains_secrets: bool,
    pub protection: ArchiveProtection,
    pub user_review_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorBackend {
    SqliteBlobV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Annual,
    Contract,
    Stipend,
    NotDisclosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityManifest {
    pub schema: SchemaId,
    pub app_version: String,
    pub compatibility_line: u32,
    pub database_schema: u32,
    pub backup: ArchiveEnvelope,
    pub export: ArchiveEnvelope,
    pub vector_backend: VectorBackend,
    #[serde(default)]
    pub extensions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub schema: SchemaId,
    pub artifact_id: String,
    pub artifact_version: String,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityDecision {
    PreV3MigrationRequired,
    CompatibleV3(CompatibilityManifest),
}

pub fn read_v3_compatibility(
    kind: CompatibilityInputKind,
    input: &str,
) -> Result<CompatibilityDecision, String> {
    match kind {
        CompatibilityInputKind::PreV3Config => {
            let value: serde_json::Value = serde_json::from_str(input)
                .map_err(|error| format!("malformed pre-v3 config: {error}"))?;
            if !value.is_object() {
                return Err("pre-v3 config must be a JSON object".to_string());
            }
            Ok(CompatibilityDecision::PreV3MigrationRequired)
        }
        CompatibilityInputKind::V3Manifest => {
            parse_compatibility_manifest(input).map(CompatibilityDecision::CompatibleV3)
        }
    }
}

pub fn parse_compatibility_manifest(input: &str) -> Result<CompatibilityManifest, String> {
    let manifest: CompatibilityManifest = parse_contract(input, SchemaId::CompatibilityV1)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn parse_artifact_manifest(input: &str) -> Result<ArtifactManifest, String> {
    let manifest: ArtifactManifest = parse_contract(input, SchemaId::ArtifactManifestV1)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn parse_privacy_receipt(input: &str) -> Result<PrivacyReceipt, String> {
    let receipt: PrivacyReceipt = parse_contract(input, SchemaId::PrivacyReceiptV1)?;
    receipt.validate()?;
    Ok(receipt)
}

pub fn parse_agent_task(input: &str) -> Result<AgentTask, String> {
    let task: AgentTask = parse_contract(input, SchemaId::AgentTaskV1)?;
    task.validate()?;
    Ok(task)
}

pub fn parse_pack_manifest(input: &str) -> Result<PackManifest, String> {
    let manifest: PackManifest = parse_contract(input, SchemaId::PackManifestV1)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn parse_region_manifest(input: &str, today: NaiveDate) -> Result<RegionManifest, String> {
    let manifest: RegionManifest = parse_contract(input, SchemaId::RegionManifestV1)?;
    manifest.validate(today)?;
    Ok(manifest)
}

pub fn parse_edition_manifest(input: &str) -> Result<EditionManifest, String> {
    let manifest: EditionManifest = parse_contract(input, SchemaId::EditionManifestV1)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn parse_model_provenance(input: &str) -> Result<ModelProvenance, String> {
    let provenance: ModelProvenance = parse_contract(input, SchemaId::ModelProvenanceV1)?;
    provenance.validate()?;
    Ok(provenance)
}

pub fn parse_vector_freshness(
    input: &str,
    model: &ModelProvenance,
) -> Result<VectorFreshness, String> {
    let freshness: VectorFreshness = parse_contract(input, SchemaId::VectorFreshnessV1)?;
    freshness.validate(model)?;
    Ok(freshness)
}

impl CompatibilityManifest {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::CompatibilityV1)?;
        require_v3_semver("application version", &self.app_version)?;
        if self.compatibility_line != 3 {
            return Err("compatibility line must be v3".to_string());
        }
        if self.database_schema > MAX_DATABASE_SCHEMA {
            return Err("database schema is newer than this application supports".to_string());
        }
        validate_archive(
            &self.backup,
            ArchiveKind::Backup,
            ArchiveProtection::Encrypted,
        )?;
        validate_archive(
            &self.export,
            ArchiveKind::Export,
            ArchiveProtection::ReviewedPlaintext,
        )
    }
}

impl ArtifactManifest {
    pub fn validate(&self) -> Result<(), String> {
        require_schema(self.schema, SchemaId::ArtifactManifestV1)?;
        require_nonempty("artifact id", &self.artifact_id)?;
        require_v3_semver("artifact version", &self.artifact_version)?;
        require_sha256("artifact payload", &self.payload_sha256)
    }
}

pub(crate) fn parse_contract<T: DeserializeOwned>(
    input: &str,
    expected: SchemaId,
) -> Result<T, String> {
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|error| format!("malformed contract: {error}"))?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "contract schema is required".to_string())?;
    let expected_name = expected.as_str();
    if schema != expected_name {
        let prefix = expected_name.trim_end_matches('1');
        return Err(if schema.starts_with(prefix) {
            format!("unsupported newer contract schema {schema}")
        } else {
            format!("unknown contract schema {schema}")
        });
    }
    serde_json::from_value(value).map_err(|error| format!("invalid v3 contract: {error}"))
}

fn validate_archive(
    envelope: &ArchiveEnvelope,
    kind: ArchiveKind,
    protection: ArchiveProtection,
) -> Result<(), String> {
    if envelope.kind != kind
        || envelope.format_version != 1
        || envelope.contains_secrets
        || envelope.protection != protection
        || !envelope.user_review_required
    {
        return Err("archive envelope violates the v3 privacy boundary".to_string());
    }
    Ok(())
}

pub(crate) fn require_schema(actual: SchemaId, expected: SchemaId) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err("contract uses the wrong schema identifier".to_string())
    }
}

pub(crate) fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{name} is required"))
    } else {
        Ok(())
    }
}

pub(crate) fn require_sha256(name: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(format!("{name} SHA-256 must be 64 hexadecimal characters"))
    }
}

pub(crate) fn require_v3_semver(name: &str, value: &str) -> Result<(), String> {
    let mut parts = value.split('.');
    let Some(major) = parts.next() else {
        return Err(format!("{name} must be a v3 semantic version"));
    };
    let components = [
        major,
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
    ];
    if parts.next().is_some()
        || major != "3"
        || !components.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
                && part.parse::<u64>().is_ok()
        })
    {
        return Err(format!("{name} must be a v3 semantic version"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3_manifests::{PackExecutionClass, PrivacyLabel};
    use serde::Deserialize;

    const V3_BASELINE: &str = include_str!("fixtures/v3_contract_bundle_v1.json");

    #[derive(Deserialize)]
    struct ContractFixtures {
        compatibility: serde_json::Value,
        artifact_manifest: serde_json::Value,
        privacy_receipt: serde_json::Value,
        agent_task: serde_json::Value,
        pack_manifest: serde_json::Value,
        region_manifest: serde_json::Value,
        edition_manifest: serde_json::Value,
        model_provenance: serde_json::Value,
        vector_freshness: serde_json::Value,
    }

    fn fixtures() -> ContractFixtures {
        serde_json::from_str(V3_BASELINE).expect("baseline fixtures must be valid JSON")
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 7, 19).unwrap()
    }

    #[test]
    fn explicit_pre_v3_config_requires_safe_forward_migration() {
        assert_eq!(
            read_v3_compatibility(CompatibilityInputKind::PreV3Config, r#"{"any":"object"}"#)
                .unwrap(),
            CompatibilityDecision::PreV3MigrationRequired
        );
        assert!(
            read_v3_compatibility(CompatibilityInputKind::PreV3Config, "[]").is_err(),
            "legacy classification must still require a JSON object"
        );
    }

    #[test]
    fn independent_contracts_parse_without_an_omnibus_runtime_lifecycle() {
        let fixtures = fixtures();

        let decision = read_v3_compatibility(
            CompatibilityInputKind::V3Manifest,
            &fixtures.compatibility.to_string(),
        )
        .unwrap();
        assert!(matches!(decision, CompatibilityDecision::CompatibleV3(_)));
        assert_eq!(
            parse_pack_manifest(&fixtures.pack_manifest.to_string())
                .unwrap()
                .execution_class,
            PackExecutionClass::ReviewedTypedWorkflow
        );
        assert!(parse_privacy_receipt(&fixtures.privacy_receipt.to_string())
            .unwrap()
            .labels
            .contains(&PrivacyLabel::LocalOnly));
        parse_artifact_manifest(&fixtures.artifact_manifest.to_string()).unwrap();
        parse_agent_task(&fixtures.agent_task.to_string()).unwrap();
        parse_region_manifest(&fixtures.region_manifest.to_string(), today()).unwrap();
        parse_edition_manifest(&fixtures.edition_manifest.to_string()).unwrap();
        let model = parse_model_provenance(&fixtures.model_provenance.to_string()).unwrap();
        parse_vector_freshness(&fixtures.vector_freshness.to_string(), &model).unwrap();
    }

    #[test]
    fn malformed_unsupported_and_non_semver_versions_fail_closed() {
        assert!(
            read_v3_compatibility(CompatibilityInputKind::V3Manifest, "{").is_err(),
            "malformed input must fail"
        );
        assert!(read_v3_compatibility(
            CompatibilityInputKind::V3Manifest,
            r#"{"schema":"jobsentinel.v3.compatibility.v2"}"#
        )
        .unwrap_err()
        .contains("unsupported newer"));

        for invalid in ["3.", "3.0", "3.not.0", "03.0.0", "3.0.0.1"] {
            let mut value = fixtures().artifact_manifest;
            value["artifact_version"] = serde_json::json!(invalid);
            assert!(
                parse_artifact_manifest(&value.to_string()).is_err(),
                "{invalid} must not be treated as v3 semver"
            );
        }
    }

    #[test]
    fn explicit_extensions_are_forward_compatible_but_unknown_fields_fail_closed() {
        let fixtures = fixtures();
        let mut compatibility = fixtures.compatibility;
        compatibility["extensions"]["future_reader_hint"] = serde_json::json!("ignored");
        assert!(parse_compatibility_manifest(&compatibility.to_string()).is_ok());

        compatibility["backup"]["hostile_bypass"] = serde_json::json!(true);
        assert!(parse_compatibility_manifest(&compatibility.to_string()).is_err());

        let mut receipt = fixtures.privacy_receipt;
        receipt["hostile_bypass"] = serde_json::json!(true);
        assert!(parse_privacy_receipt(&receipt.to_string()).is_err());
    }

    #[test]
    fn agent_tasks_cannot_run_without_user_review() {
        let mut task = fixtures().agent_task;
        task["user_review_required"] = serde_json::json!(false);

        assert!(parse_agent_task(&task.to_string()).is_err());
    }

    #[test]
    fn external_ai_packs_are_gateway_only_and_public_data_only() {
        let mut pack = fixtures().pack_manifest;
        pack["allowed_actions"] = serde_json::json!(["request_external_ai"]);
        pack["allowed_task_kinds"] = serde_json::json!(["source_check"]);
        pack["privacy_labels"] = serde_json::json!(["external_ai_optional", "public_data_only"]);
        pack["gateway_policy_id"] = serde_json::json!("https://arbitrary.example/upload");
        assert!(
            parse_pack_manifest(&pack.to_string()).is_err(),
            "an arbitrary destination cannot replace the governed gateway"
        );

        pack["gateway_policy_id"] = serde_json::json!("jobsentinel.external-ai-gateway.v1");
        for protected in [
            "resume_evidence",
            "military_service",
            "protected_veteran_answer",
        ] {
            pack["allowed_data_categories"] = serde_json::json!(["public_job_posting", protected]);
            assert!(
                parse_pack_manifest(&pack.to_string()).is_err(),
                "{protected} must remain outside external pack flows"
            );
        }
    }

    #[test]
    fn region_and_edition_require_runtime_compatibility_semantics() {
        let mut region = fixtures().region_manifest;
        region.as_object_mut().unwrap().remove("pay_periods");
        assert!(parse_region_manifest(&region.to_string(), today()).is_err());

        let mut edition = fixtures().edition_manifest;
        edition.as_object_mut().unwrap().remove("platform");
        assert!(parse_edition_manifest(&edition.to_string()).is_err());
        edition = fixtures().edition_manifest;
        edition["artifact_version"] = serde_json::json!("3.not-semver");
        assert!(parse_edition_manifest(&edition.to_string()).is_err());
    }
}
