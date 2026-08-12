use std::collections::BTreeSet;

use chrono::NaiveDate;
use jobsentinel_security::validate_credential_free_external_https_url;
use serde::{Deserialize, Serialize};

use crate::{
    v3_contracts::{parse_contract, require_nonempty, require_schema, SchemaId},
    v3_foundation::validate_identifier,
};

const INDEX: &str = include_str!("fixtures/v3_veteran_public_service_index_v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VeteranResourceAuthority {
    Government,
    GovernmentSponsored,
    Community,
    Commercial,
    ResearchOrganization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VeteranResourceUse {
    OfficialJobSearch,
    OccupationCrosswalk,
    CredentialResearch,
    VeteranHiringGuidance,
    ProtectedStatusGuidance,
    EmployerDiscoverySeed,
    ResumePromptEvaluation,
    OccupationComparison,
    RegionalWorkforceResearch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VeteranResourceAccess {
    CredentialedApi,
    ManualReviewOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityBoundary {
    NotApplicable,
    GeneralGuidanceOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VeteranPublicServiceResource {
    pub resource_id: String,
    pub display_name: String,
    pub authority: VeteranResourceAuthority,
    pub url: String,
    pub intended_use: VeteranResourceUse,
    pub runtime_access: VeteranResourceAccess,
    pub reviewed_on: NaiveDate,
    pub source_release: Option<String>,
    pub jurisdiction: Option<String>,
    pub policy_note: String,
    pub eligibility_boundary: EligibilityBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VeteranPublicServiceIndex {
    pub schema: SchemaId,
    pub reviewed_on: NaiveDate,
    pub incomplete_coverage: bool,
    pub resources: Vec<VeteranPublicServiceResource>,
}

pub fn parse_veteran_public_service_index(
    input: &str,
    today: NaiveDate,
) -> Result<VeteranPublicServiceIndex, String> {
    let index: VeteranPublicServiceIndex =
        parse_contract(input, SchemaId::VeteranPublicServiceIndexV1)?;
    index.validate(today)?;
    Ok(index)
}

pub fn reviewed_veteran_public_service_resource(
    resource_id: &str,
    today: NaiveDate,
) -> Result<VeteranPublicServiceResource, String> {
    parse_veteran_public_service_index(INDEX, today)?
        .resources
        .into_iter()
        .find(|resource| resource.resource_id == resource_id)
        .ok_or_else(|| "veteran resource is not in the reviewed index".to_string())
}

impl VeteranPublicServiceIndex {
    fn validate(&self, today: NaiveDate) -> Result<(), String> {
        require_schema(self.schema, SchemaId::VeteranPublicServiceIndexV1)?;
        if self.reviewed_on > today {
            return Err("veteran resource review date cannot be in the future".to_string());
        }
        if !self.incomplete_coverage || self.resources.is_empty() || self.resources.len() > 64 {
            return Err(
                "veteran resource coverage must remain explicitly incomplete and bounded"
                    .to_string(),
            );
        }
        let mut ids = BTreeSet::new();
        for resource in &self.resources {
            if !ids.insert(resource.resource_id.as_str()) {
                return Err("veteran resource identifiers must be unique".to_string());
            }
            resource.validate(self.reviewed_on)?;
        }
        Ok(())
    }
}

impl VeteranPublicServiceResource {
    fn validate(&self, index_reviewed_on: NaiveDate) -> Result<(), String> {
        validate_identifier("veteran resource", &self.resource_id)?;
        require_bounded_text("veteran resource name", &self.display_name, 128)?;
        require_bounded_text("veteran resource policy note", &self.policy_note, 512)?;
        if self.reviewed_on > index_reviewed_on {
            return Err("veteran resource review date exceeds its index review".to_string());
        }
        if let Some(release) = self.source_release.as_deref() {
            require_bounded_text("veteran resource release", release, 128)?;
        }
        if let Some(jurisdiction) = self.jurisdiction.as_deref() {
            if jurisdiction.is_empty()
                || jurisdiction.len() > 16
                || !jurisdiction
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte == b'-')
            {
                return Err("veteran resource jurisdiction is invalid".to_string());
            }
        }

        let url = validate_credential_free_external_https_url(&self.url)
            .map_err(|_| "veteran resource URL must be credential-free external HTTPS")?;
        let host = url
            .host_str()
            .ok_or_else(|| "veteran resource URL requires a host".to_string())?;
        validate_authority_host(self.authority, host)?;
        validate_reviewed_identity(self, host)?;
        validate_use_boundaries(self)?;
        Ok(())
    }
}

impl VeteranResourceUse {
    pub const fn requires_user_confirmed_evidence(self) -> bool {
        matches!(
            self,
            Self::OccupationCrosswalk
                | Self::CredentialResearch
                | Self::OccupationComparison
                | Self::RegionalWorkforceResearch
        )
    }
}

fn validate_authority_host(authority: VeteranResourceAuthority, host: &str) -> Result<(), String> {
    let host_matches = match authority {
        VeteranResourceAuthority::Government => matches!(
            host.rsplit_once('.').map(|(_, suffix)| suffix),
            Some("gov" | "mil")
        ),
        VeteranResourceAuthority::GovernmentSponsored => {
            host == "onetcenter.org" || host == "www.onetcenter.org"
        }
        VeteranResourceAuthority::Community => {
            host == "github.com" || host == "mos.directory" || host == "www.mos.directory"
        }
        VeteranResourceAuthority::Commercial => {
            matches!(
                host,
                "militarymoney.com"
                    | "www.militarymoney.com"
                    | "bestmilitaryresume.com"
                    | "www.bestmilitaryresume.com"
                    | "militarytransitiontoolkit.com"
                    | "www.militarytransitiontoolkit.com"
            )
        }
        VeteranResourceAuthority::ResearchOrganization => {
            host == "coeccc.net" || host == "www.coeccc.net"
        }
    };
    if host_matches {
        Ok(())
    } else {
        Err("veteran resource authority does not match its reviewed host".to_string())
    }
}

fn validate_reviewed_identity(
    resource: &VeteranPublicServiceResource,
    host: &str,
) -> Result<(), String> {
    let approved = match resource.resource_id.as_str() {
        "usajobs-api" => matches!(
            (
                host,
                resource.authority,
                resource.intended_use,
                resource.runtime_access,
                resource.eligibility_boundary,
            ),
            (
                "developer.usajobs.gov",
                VeteranResourceAuthority::Government,
                VeteranResourceUse::OfficialJobSearch,
                VeteranResourceAccess::CredentialedApi,
                EligibilityBoundary::GeneralGuidanceOnly,
            )
        ),
        "onet-military-crosswalk" => {
            resource.url == "https://www.onetcenter.org/crosswalks.html"
                && matches!(
                    (
                        host,
                        resource.authority,
                        resource.intended_use,
                        resource.runtime_access,
                        resource.eligibility_boundary,
                    ),
                    (
                        "www.onetcenter.org",
                        VeteranResourceAuthority::GovernmentSponsored,
                        VeteranResourceUse::OccupationCrosswalk,
                        VeteranResourceAccess::ManualReviewOnly,
                        EligibilityBoundary::NotApplicable,
                    )
                )
        }
        "opm-veteran-job-seekers" => matches!(
            (
                host,
                resource.authority,
                resource.intended_use,
                resource.runtime_access,
                resource.eligibility_boundary,
            ),
            (
                "www.opm.gov",
                VeteranResourceAuthority::Government,
                VeteranResourceUse::VeteranHiringGuidance,
                VeteranResourceAccess::ManualReviewOnly,
                EligibilityBoundary::GeneralGuidanceOnly,
            )
        ),
        "dol-vevraa-guidance" => matches!(
            (
                host,
                resource.authority,
                resource.intended_use,
                resource.runtime_access,
                resource.eligibility_boundary,
            ),
            (
                "www.dol.gov",
                VeteranResourceAuthority::Government,
                VeteranResourceUse::ProtectedStatusGuidance,
                VeteranResourceAccess::ManualReviewOnly,
                EligibilityBoundary::GeneralGuidanceOnly,
            )
        ),
        _ => {
            matches!(
                resource.runtime_access,
                VeteranResourceAccess::ManualReviewOnly
            ) && !matches!(
                resource.intended_use,
                VeteranResourceUse::OfficialJobSearch
                    | VeteranResourceUse::OccupationCrosswalk
                    | VeteranResourceUse::VeteranHiringGuidance
                    | VeteranResourceUse::ProtectedStatusGuidance
            )
        }
    };
    if approved {
        Ok(())
    } else {
        Err("veteran resource capability does not match its reviewed identity".to_string())
    }
}

fn validate_use_boundaries(resource: &VeteranPublicServiceResource) -> Result<(), String> {
    if matches!(
        resource.eligibility_boundary,
        EligibilityBoundary::GeneralGuidanceOnly
    ) && (!matches!(resource.authority, VeteranResourceAuthority::Government)
        || !matches!(
            resource.intended_use,
            VeteranResourceUse::OfficialJobSearch
                | VeteranResourceUse::VeteranHiringGuidance
                | VeteranResourceUse::ProtectedStatusGuidance
        ))
    {
        return Err("secondary sources cannot provide eligibility guidance".to_string());
    }
    Ok(())
}

fn require_bounded_text(label: &str, value: &str, maximum: usize) -> Result<(), String> {
    require_nonempty(label, value)?;
    if value.chars().count() > maximum {
        Err(format!("{label} exceeds its maximum length"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 19).unwrap()
    }

    #[test]
    fn reviewed_index_preserves_authority_and_user_evidence_boundaries() {
        let index = parse_veteran_public_service_index(INDEX, today()).unwrap();

        assert!(index.incomplete_coverage);
        assert_eq!(
            index
                .resources
                .iter()
                .map(|resource| resource.resource_id.as_str())
                .collect::<BTreeSet<_>>(),
            [
                "best-military-resume-mos-chart",
                "coeccc-service-to-sector-mapping",
                "dod-cool-military-occupations",
                "dol-vevraa-guidance",
                "military-money-mos-lists",
                "military-transition-toolkit",
                "mos-directory",
                "onet-military-crosswalk",
                "opm-veteran-job-seekers",
                "usajobs-api",
                "vetsec-remote-security-employers",
                "vetsec-resume-prompt-examples",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            index
                .resources
                .iter()
                .find(|resource| resource.resource_id == "usajobs-api")
                .unwrap()
                .runtime_access,
            VeteranResourceAccess::CredentialedApi
        );
        let onet = index
            .resources
            .iter()
            .find(|resource| resource.resource_id == "onet-military-crosswalk")
            .unwrap();
        assert_eq!(onet.runtime_access, VeteranResourceAccess::ManualReviewOnly);
        assert!(onet.intended_use.requires_user_confirmed_evidence());
    }

    #[test]
    fn authority_escalation_and_unsafe_urls_fail_closed() {
        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        let vetsec = value["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|resource| resource["resource_id"] == "vetsec-remote-security-employers")
            .unwrap();
        vetsec["authority"] = serde_json::json!("government");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        value["resources"][0]["url"] =
            serde_json::json!("https://user:secret@developer.usajobs.gov/");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());
    }

    #[test]
    fn future_reviews_false_completeness_and_unknown_fields_fail_closed() {
        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        value["reviewed_on"] = serde_json::json!("2026-07-20");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        value["resources"][0]["infers_eligibility"] = serde_json::json!(false);
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());
    }

    #[test]
    fn secondary_sources_cannot_become_eligibility_authorities() {
        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        let toolkit = value["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|resource| resource["resource_id"] == "military-transition-toolkit")
            .unwrap();
        toolkit["eligibility_boundary"] = serde_json::json!("general_guidance_only");

        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());
    }

    #[test]
    fn elevated_access_and_guidance_roles_are_bound_to_reviewed_identities() {
        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        let usajobs = resource_mut(&mut value, "usajobs-api");
        usajobs["url"] = serde_json::json!("https://www.dol.gov/jobs");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        resource_mut(&mut value, "onet-military-crosswalk")["runtime_access"] =
            serde_json::json!("packaged_public_download");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        resource_mut(&mut value, "onet-military-crosswalk")["url"] =
            serde_json::json!("https://www.onetcenter.org/technology.html");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        resource_mut(&mut value, "onet-military-crosswalk")["resource_id"] =
            serde_json::json!("unreviewed-crosswalk");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        let dol = resource_mut(&mut value, "dol-vevraa-guidance");
        dol["intended_use"] = serde_json::json!("official_job_search");
        dol["runtime_access"] = serde_json::json!("credentialed_api");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());

        let mut value: serde_json::Value = serde_json::from_str(INDEX).unwrap();
        resource_mut(&mut value, "opm-veteran-job-seekers")["intended_use"] =
            serde_json::json!("protected_status_guidance");
        assert!(parse_veteran_public_service_index(&value.to_string(), today()).is_err());
    }

    #[test]
    fn mapping_resources_require_user_confirmed_evidence() {
        let index = parse_veteran_public_service_index(INDEX, today()).unwrap();
        assert_eq!(
            index
                .resources
                .iter()
                .filter(|resource| resource.intended_use.requires_user_confirmed_evidence())
                .map(|resource| resource.resource_id.as_str())
                .collect::<BTreeSet<_>>(),
            [
                "best-military-resume-mos-chart",
                "coeccc-service-to-sector-mapping",
                "dod-cool-military-occupations",
                "military-money-mos-lists",
                "military-transition-toolkit",
                "mos-directory",
                "onet-military-crosswalk",
            ]
            .into_iter()
            .collect()
        );
    }

    fn resource_mut<'a>(
        value: &'a mut serde_json::Value,
        resource_id: &str,
    ) -> &'a mut serde_json::Value {
        value["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|resource| resource["resource_id"] == resource_id)
            .unwrap()
    }
}
