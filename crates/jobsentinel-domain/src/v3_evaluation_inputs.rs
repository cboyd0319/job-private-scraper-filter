//! Defines and validates synthetic v3 evaluation inputs.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationCategory {
    SourceTruth,
    ResumeEvidence,
    PayClarity,
    PostingRisk,
    EmployerContext,
    Accessibility,
    Recovery,
    ModestHardware,
    MilitaryToCivilianEvidence,
    ProtectedVeteranAnswers,
    CommercialComparison,
}

impl EvaluationCategory {
    pub const ALL: [Self; 11] = [
        Self::SourceTruth,
        Self::ResumeEvidence,
        Self::PayClarity,
        Self::PostingRisk,
        Self::EmployerContext,
        Self::Accessibility,
        Self::Recovery,
        Self::ModestHardware,
        Self::MilitaryToCivilianEvidence,
        Self::ProtectedVeteranAnswers,
        Self::CommercialComparison,
    ];
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayPeriod {
    Hourly,
    Annual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EmployerSourceClass {
    OfficialDomain,
    PublicRegistry,
    UserOwnedLocalObservation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EmployerEvidenceState {
    Current,
    Stale,
    Conflicting,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployerEvidence {
    pub source_class: EmployerSourceClass,
    pub observed_on: String,
    pub state: EmployerEvidenceState,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MilitaryBranch {
    Army,
    MarineCorps,
    Navy,
    AirForce,
    SpaceForce,
    CoastGuard,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedVeteranSelection {
    ProtectedVeteran,
    NotProtectedVeteran,
    DeclineToAnswer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonDimensionName {
    AccountRequirements,
    ExternalDataTransfers,
    UserReviewGates,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonDirection {
    Lower,
    Higher,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonDimension {
    pub name: ComparisonDimensionName,
    pub baseline_value: u32,
    pub jobsentinel_target_value: u32,
    pub better_when: ComparisonDirection,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvaluationInput {
    SourceTruth {
        aggregator_listed: bool,
        official_source_listed: bool,
    },
    ResumeEvidence {
        requirement: String,
        evidence: Vec<String>,
        shared_keywords: Vec<String>,
    },
    PayClarity {
        currency: String,
        minimum: u32,
        maximum: u32,
        period: PayPeriod,
        contract: bool,
        benefits_stated: bool,
    },
    PostingRisk {
        recruiter_domain_matches: bool,
        pre_interview_payment_requested: bool,
    },
    EmployerContext {
        evidence: Vec<EmployerEvidence>,
    },
    Accessibility {
        keyboard_only: bool,
        zoom_percent: u16,
        reduced_motion: bool,
    },
    Recovery {
        upgrade_interrupted: bool,
        backup_created: bool,
        newer_data_accepted: bool,
    },
    ModestHardware {
        memory_gib: u8,
        local_model_downloaded: bool,
    },
    MilitaryToCivilianEvidence {
        branch: MilitaryBranch,
        occupation_code: String,
        user_confirmed_duties: Vec<String>,
        evidence_refs: Vec<String>,
    },
    ProtectedVeteranAnswers {
        user_selection: Option<ProtectedVeteranSelection>,
        military_evidence_present: bool,
    },
    CommercialComparison {
        baseline_product_class: String,
        dimensions: Vec<ComparisonDimension>,
    },
}

impl EvaluationInput {
    pub(crate) fn category(&self) -> EvaluationCategory {
        match self {
            Self::SourceTruth { .. } => EvaluationCategory::SourceTruth,
            Self::ResumeEvidence { .. } => EvaluationCategory::ResumeEvidence,
            Self::PayClarity { .. } => EvaluationCategory::PayClarity,
            Self::PostingRisk { .. } => EvaluationCategory::PostingRisk,
            Self::EmployerContext { .. } => EvaluationCategory::EmployerContext,
            Self::Accessibility { .. } => EvaluationCategory::Accessibility,
            Self::Recovery { .. } => EvaluationCategory::Recovery,
            Self::ModestHardware { .. } => EvaluationCategory::ModestHardware,
            Self::MilitaryToCivilianEvidence { .. } => {
                EvaluationCategory::MilitaryToCivilianEvidence
            }
            Self::ProtectedVeteranAnswers { .. } => EvaluationCategory::ProtectedVeteranAnswers,
            Self::CommercialComparison { .. } => EvaluationCategory::CommercialComparison,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        match self {
            Self::ResumeEvidence {
                requirement,
                shared_keywords,
                ..
            } if requirement.trim().is_empty() || !nonempty_strings(shared_keywords) => Err(
                "resume evidence fixtures require a requirement and shared keywords".to_string(),
            ),
            Self::PayClarity {
                currency,
                minimum,
                maximum,
                ..
            } if currency.len() != 3 || maximum < minimum || *maximum == 0 => {
                Err("pay fixtures require a currency and valid range".to_string())
            }
            Self::EmployerContext { evidence }
                if evidence.is_empty()
                    || evidence
                        .iter()
                        .map(|item| item.source_class)
                        .collect::<BTreeSet<_>>()
                        .len()
                        != evidence.len()
                    || evidence.iter().any(|item| {
                        NaiveDate::parse_from_str(&item.observed_on, "%Y-%m-%d").is_err()
                    }) =>
            {
                Err("employer fixtures require uniquely dated source evidence".to_string())
            }
            Self::Accessibility { zoom_percent, .. } if !(100..=400).contains(zoom_percent) => {
                Err("accessibility zoom must be between 100 and 400 percent".to_string())
            }
            Self::ModestHardware { memory_gib: 0, .. } => {
                Err("modest hardware memory must be measurable".to_string())
            }
            Self::MilitaryToCivilianEvidence {
                occupation_code,
                user_confirmed_duties,
                ..
            } if occupation_code.trim().is_empty() || !nonempty_strings(user_confirmed_duties) => {
                Err("military mappings require user-confirmed duties".to_string())
            }
            Self::MilitaryToCivilianEvidence { evidence_refs, .. }
                if !nonempty_strings(evidence_refs) =>
            {
                Err("military mappings require evidence references".to_string())
            }
            Self::CommercialComparison {
                baseline_product_class,
                dimensions,
            } => validate_comparison(baseline_product_class, dimensions),
            _ => Ok(()),
        }
    }
}

fn validate_comparison(
    baseline_product_class: &str,
    dimensions: &[ComparisonDimension],
) -> Result<(), String> {
    if baseline_product_class.trim().is_empty() || dimensions.is_empty() {
        return Err("commercial fixtures require baseline comparison dimensions".to_string());
    }
    let mut names = BTreeSet::new();
    for dimension in dimensions {
        if !names.insert(dimension.name) {
            return Err("commercial comparison dimensions must be unique".to_string());
        }
        let improves = match dimension.better_when {
            ComparisonDirection::Lower => {
                dimension.jobsentinel_target_value < dimension.baseline_value
            }
            ComparisonDirection::Higher => {
                dimension.jobsentinel_target_value > dimension.baseline_value
            }
        };
        if !improves {
            return Err("commercial targets must define a measurable improvement".to_string());
        }
    }
    Ok(())
}

fn nonempty_strings(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| !value.trim().is_empty())
}
