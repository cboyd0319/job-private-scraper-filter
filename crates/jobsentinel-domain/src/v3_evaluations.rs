//! Validates synthetic evaluation sets and runs their exact local assertion oracles.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::Deserialize;

use crate::v3_evaluation_assertions::{
    exact_assertions, ACCESSIBILITY_FORBIDDEN, ACCESSIBILITY_REQUIRED,
    COMMERCIAL_COMPARISON_FORBIDDEN, COMMERCIAL_COMPARISON_REQUIRED, EMPLOYER_CONTEXT_FORBIDDEN,
    EMPLOYER_CONTEXT_REQUIRED, EMPLOYER_CONTEXT_STALE_CONFLICT_REQUIRED, MILITARY_FORBIDDEN,
    MILITARY_REQUIRED, MODEST_HARDWARE_FORBIDDEN, MODEST_HARDWARE_REQUIRED, PAY_CLARITY_FORBIDDEN,
    PAY_CLARITY_REQUIRED, POSTING_RISK_FORBIDDEN, POSTING_RISK_REQUIRED,
    PROTECTED_SELECTED_FORBIDDEN, PROTECTED_SELECTED_REQUIRED, PROTECTED_UNANSWERED_FORBIDDEN,
    PROTECTED_UNANSWERED_REQUIRED, RECOVERY_FORBIDDEN, RECOVERY_REQUIRED,
    RESUME_EVIDENCE_FORBIDDEN, RESUME_EVIDENCE_REQUIRED, SOURCE_TRUTH_FORBIDDEN,
    SOURCE_TRUTH_REQUIRED,
};
pub use crate::v3_evaluation_assertions::{EvaluationAssertion, EvaluationOracle};
pub use crate::v3_evaluation_inputs::EvaluationCategory;
use crate::v3_evaluation_inputs::EvaluationInput;

mod resume_requirement;
pub use resume_requirement::{
    AtsResumeRequirementEvaluationScorer, EvidenceReviewerResumeRequirementEvaluationScorer,
};

const SCHEMA: &str = "jobsentinel.v3.evaluation-set";
const SCHEMA_VERSION: u32 = 1;
const MAX_CASE_ID_BYTES: usize = 128;
const MAX_REVISION_BYTES: usize = 128;
const MAX_REQUIREMENT_BYTES: usize = 512;
const MAX_EVIDENCE_ITEMS: usize = 16;
const MAX_EVIDENCE_ITEM_BYTES: usize = 512;
const MAX_SHARED_KEYWORDS: usize = 32;
const MAX_SHARED_KEYWORD_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationVerificationTarget {
    SourceGraph,
    ResumeEvidence,
    PayIntelligence,
    PostingRisk,
    EmployerIntelligence,
    Accessibility,
    Recovery,
    Editions,
    MilitaryTransition,
    ApplicationReview,
    ReleaseBenchmark,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationEvidenceTarget {
    StructuredTestResult,
    AccessibilityReport,
    RecoveryReport,
    PerformanceReport,
    ComparisonScorecard,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationPrivacyBoundary {
    pub local_only: bool,
    pub external_ai_allowed: bool,
    pub contains_personal_data: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidenceCapture {
    pub target: EvaluationEvidenceTarget,
    pub captures_fixture_content: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MilitaryServiceBasis {
    NotApplicable,
    UserConfirmed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedVeteranAnswerBasis {
    NotApplicable,
    UserSelected,
    Unanswered,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V3EvaluationCase {
    pub id: String,
    pub category: EvaluationCategory,
    pub verification_target: EvaluationVerificationTarget,
    pub privacy_boundary: EvaluationPrivacyBoundary,
    pub evidence_capture: EvaluationEvidenceCapture,
    pub input: EvaluationInput,
    pub oracle: EvaluationOracle,
    pub military_service_basis: MilitaryServiceBasis,
    pub protected_veteran_answer_basis: ProtectedVeteranAnswerBasis,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct V3EvaluationSet {
    pub schema: String,
    pub schema_version: u32,
    pub revision: String,
    pub data_origin: String,
    pub contains_personal_data: bool,
    pub cases: Vec<V3EvaluationCase>,
}

pub struct EvaluationScorer {
    evaluation: V3EvaluationSet,
}

impl fmt::Debug for EvaluationScorer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EvaluationScorer")
            .field("revision", &self.evaluation.revision)
            .field("case_count", &self.evaluation.cases.len())
            .finish()
    }
}

pub fn parse_v3_evaluation_set(input: &str) -> Result<V3EvaluationSet, String> {
    let set: V3EvaluationSet =
        serde_json::from_str(input).map_err(|error| format!("invalid evaluation set: {error}"))?;
    set.validate()?;
    Ok(set)
}

pub(crate) fn parse_ats_resume_requirement_evaluation_set(
    input: &str,
) -> Result<V3EvaluationSet, String> {
    let set: V3EvaluationSet =
        serde_json::from_str(input).map_err(|error| format!("invalid evaluation set: {error}"))?;
    set.validate_common()?;
    if set.cases.len() != 1 || set.cases[0].category != EvaluationCategory::ResumeEvidence {
        return Err(
            "resume evidence evaluation sets require exactly one matching case".to_string(),
        );
    }
    Ok(set)
}

impl V3EvaluationSet {
    fn validate(&self) -> Result<(), String> {
        self.validate_common()?;
        let categories = self
            .cases
            .iter()
            .map(|case| case.category)
            .collect::<BTreeSet<_>>();
        if categories != EvaluationCategory::ALL.into_iter().collect() {
            return Err("evaluation set must cover all 11 v3 categories".to_string());
        }
        let protected_bases = self
            .cases
            .iter()
            .filter(|case| case.category == EvaluationCategory::ProtectedVeteranAnswers)
            .map(|case| case.protected_veteran_answer_basis)
            .collect::<BTreeSet<_>>();
        if protected_bases
            != [
                ProtectedVeteranAnswerBasis::Unanswered,
                ProtectedVeteranAnswerBasis::UserSelected,
            ]
            .into_iter()
            .collect()
        {
            return Err(
                "protected veteran evaluations require Unanswered and UserSelected cases"
                    .to_string(),
            );
        }
        Ok(())
    }

    fn validate_common(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err("unsupported evaluation schema".to_string());
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "unsupported evaluation schema version {}",
                self.schema_version
            ));
        }
        if !is_revision(&self.revision) || self.data_origin != "synthetic" {
            return Err("M1 evaluation data must be revisioned and synthetic".to_string());
        }
        if self.contains_personal_data {
            return Err("evaluation sets must not contain personal data".to_string());
        }

        let mut ids = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if !ids.insert(case.id.as_str()) {
                return Err("evaluation case ids must be unique".to_string());
            }
        }
        Ok(())
    }
}

fn is_revision(value: &str) -> bool {
    is_identifier(value, MAX_REVISION_BYTES)
}

fn is_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b':'))
}

impl EvaluationScorer {
    pub(crate) const fn new(evaluation: V3EvaluationSet) -> Self {
        Self { evaluation }
    }

    pub fn revision(&self) -> &str {
        &self.evaluation.revision
    }

    pub fn case_count(&self) -> usize {
        self.evaluation.cases.len()
    }

    /// Score one exact observation set per case without exposing fixture content.
    pub fn score_observations(
        &self,
        observations: &BTreeMap<String, Vec<EvaluationAssertion>>,
    ) -> Result<BTreeMap<String, bool>, String> {
        self.evaluation.validate()?;
        if observations.len() != self.evaluation.cases.len()
            || self
                .evaluation
                .cases
                .iter()
                .any(|case| !observations.contains_key(&case.id))
        {
            return Err("evaluation observations must cover every case exactly".to_string());
        }
        Ok(self
            .evaluation
            .cases
            .iter()
            .map(|case| (case.id.clone(), case.evaluate(&observations[&case.id])))
            .collect())
    }
}

impl V3EvaluationCase {
    fn validate(&self) -> Result<(), String> {
        if !is_identifier(&self.id, MAX_CASE_ID_BYTES) {
            return Err("evaluation case id is required".to_string());
        }
        if self.category != self.input.category()
            || self.verification_target != self.category.verification_target()
        {
            return Err(
                "evaluation input and verification owner must match its category".to_string(),
            );
        }
        if !self.privacy_boundary.local_only
            || self.privacy_boundary.external_ai_allowed
            || self.privacy_boundary.contains_personal_data
        {
            return Err("evaluation privacy boundary must remain local synthetic data".to_string());
        }
        if self.evidence_capture.captures_fixture_content {
            return Err("evaluation evidence must not capture fixture content".to_string());
        }
        if self.evidence_capture.target != self.category.evidence_target() {
            return Err("evaluation evidence target must match its category".to_string());
        }
        self.oracle.validate()?;
        self.validate_assertions()?;
        self.input.validate()?;
        self.validate_execution_bounds()?;
        self.validate_sensitive_bases()
    }

    fn validate_execution_bounds(&self) -> Result<(), String> {
        let EvaluationInput::ResumeEvidence {
            requirement,
            evidence,
            shared_keywords,
        } = &self.input
        else {
            return Ok(());
        };
        if requirement.len() > MAX_REQUIREMENT_BYTES
            || requirement.trim() != requirement
            || requirement.chars().any(char::is_control)
            || evidence.len() > MAX_EVIDENCE_ITEMS
            || evidence.iter().any(|value| {
                value.is_empty()
                    || value.len() > MAX_EVIDENCE_ITEM_BYTES
                    || value.chars().any(char::is_control)
            })
            || shared_keywords.len() > MAX_SHARED_KEYWORDS
            || shared_keywords.iter().any(|value| {
                value.is_empty()
                    || value.len() > MAX_SHARED_KEYWORD_BYTES
                    || value.chars().any(char::is_control)
            })
        {
            return Err("resume evidence evaluation input exceeds execution bounds".to_string());
        }
        Ok(())
    }

    pub fn evaluate(&self, observed: &[EvaluationAssertion]) -> bool {
        self.validate().is_ok() && self.oracle.evaluate(observed)
    }

    fn validate_sensitive_bases(&self) -> Result<(), String> {
        match (&self.input, self.protected_veteran_answer_basis) {
            (
                EvaluationInput::MilitaryToCivilianEvidence { .. },
                ProtectedVeteranAnswerBasis::NotApplicable,
            ) if self.military_service_basis == MilitaryServiceBasis::UserConfirmed => Ok(()),
            (
                EvaluationInput::ProtectedVeteranAnswers {
                    user_selection: None,
                    ..
                },
                ProtectedVeteranAnswerBasis::Unanswered,
            ) if self.military_service_basis == MilitaryServiceBasis::NotApplicable => Ok(()),
            (
                EvaluationInput::ProtectedVeteranAnswers {
                    user_selection: Some(_),
                    ..
                },
                ProtectedVeteranAnswerBasis::UserSelected,
            ) if self.military_service_basis == MilitaryServiceBasis::NotApplicable => Ok(()),
            (EvaluationInput::MilitaryToCivilianEvidence { .. }, _) => {
                Err("military evidence must use user-confirmed duties without a protected answer"
                    .to_string())
            }
            (EvaluationInput::ProtectedVeteranAnswers { .. }, _) => Err(
                "protected veteran UserSelected cases require a selected answer; Unanswered cases require none"
                    .to_string(),
            ),
            _ if self.military_service_basis != MilitaryServiceBasis::NotApplicable
                || self.protected_veteran_answer_basis
                    != ProtectedVeteranAnswerBasis::NotApplicable =>
            {
                Err("unrelated evaluations cannot carry military or protected-status data"
                    .to_string())
            }
            _ => Ok(()),
        }
    }

    fn validate_assertions(&self) -> Result<(), String> {
        let expected = match self.category {
            EvaluationCategory::SourceTruth => (SOURCE_TRUTH_REQUIRED, SOURCE_TRUTH_FORBIDDEN),
            EvaluationCategory::ResumeEvidence => {
                (RESUME_EVIDENCE_REQUIRED, RESUME_EVIDENCE_FORBIDDEN)
            }
            EvaluationCategory::PayClarity => (PAY_CLARITY_REQUIRED, PAY_CLARITY_FORBIDDEN),
            EvaluationCategory::PostingRisk => (POSTING_RISK_REQUIRED, POSTING_RISK_FORBIDDEN),
            EvaluationCategory::EmployerContext => {
                let EvaluationInput::EmployerContext { evidence } = &self.input else {
                    return Err("employer evaluation input is required".to_string());
                };
                let has_stale = evidence.iter().any(|item| {
                    matches!(
                        item.state,
                        crate::v3_evaluation_inputs::EmployerEvidenceState::Stale
                    )
                });
                let has_conflict = evidence.iter().any(|item| {
                    matches!(
                        item.state,
                        crate::v3_evaluation_inputs::EmployerEvidenceState::Conflicting
                    )
                });
                (
                    if has_stale && has_conflict {
                        EMPLOYER_CONTEXT_STALE_CONFLICT_REQUIRED
                    } else {
                        EMPLOYER_CONTEXT_REQUIRED
                    },
                    EMPLOYER_CONTEXT_FORBIDDEN,
                )
            }
            EvaluationCategory::Accessibility => (ACCESSIBILITY_REQUIRED, ACCESSIBILITY_FORBIDDEN),
            EvaluationCategory::Recovery => (RECOVERY_REQUIRED, RECOVERY_FORBIDDEN),
            EvaluationCategory::ModestHardware => {
                (MODEST_HARDWARE_REQUIRED, MODEST_HARDWARE_FORBIDDEN)
            }
            EvaluationCategory::MilitaryToCivilianEvidence => {
                (MILITARY_REQUIRED, MILITARY_FORBIDDEN)
            }
            EvaluationCategory::ProtectedVeteranAnswers => {
                match self.protected_veteran_answer_basis {
                    ProtectedVeteranAnswerBasis::Unanswered => (
                        PROTECTED_UNANSWERED_REQUIRED,
                        PROTECTED_UNANSWERED_FORBIDDEN,
                    ),
                    ProtectedVeteranAnswerBasis::UserSelected => {
                        (PROTECTED_SELECTED_REQUIRED, PROTECTED_SELECTED_FORBIDDEN)
                    }
                    ProtectedVeteranAnswerBasis::NotApplicable => {
                        return Err(
                            "protected veteran assertions require an explicit answer basis"
                                .to_string(),
                        );
                    }
                }
            }
            EvaluationCategory::CommercialComparison => (
                COMMERCIAL_COMPARISON_REQUIRED,
                COMMERCIAL_COMPARISON_FORBIDDEN,
            ),
        };
        if exact_assertions(&self.oracle.required_assertions, expected.0)
            && exact_assertions(&self.oracle.forbidden_assertions, expected.1)
        {
            Ok(())
        } else {
            Err("evaluation safety assertions must match the category schema".to_string())
        }
    }
}

impl EvaluationCategory {
    fn verification_target(self) -> EvaluationVerificationTarget {
        match self {
            Self::SourceTruth => EvaluationVerificationTarget::SourceGraph,
            Self::ResumeEvidence => EvaluationVerificationTarget::ResumeEvidence,
            Self::PayClarity => EvaluationVerificationTarget::PayIntelligence,
            Self::PostingRisk => EvaluationVerificationTarget::PostingRisk,
            Self::EmployerContext => EvaluationVerificationTarget::EmployerIntelligence,
            Self::Accessibility => EvaluationVerificationTarget::Accessibility,
            Self::Recovery => EvaluationVerificationTarget::Recovery,
            Self::ModestHardware => EvaluationVerificationTarget::Editions,
            Self::MilitaryToCivilianEvidence => EvaluationVerificationTarget::MilitaryTransition,
            Self::ProtectedVeteranAnswers => EvaluationVerificationTarget::ApplicationReview,
            Self::CommercialComparison => EvaluationVerificationTarget::ReleaseBenchmark,
        }
    }

    fn evidence_target(self) -> EvaluationEvidenceTarget {
        match self {
            Self::Accessibility => EvaluationEvidenceTarget::AccessibilityReport,
            Self::Recovery => EvaluationEvidenceTarget::RecoveryReport,
            Self::ModestHardware => EvaluationEvidenceTarget::PerformanceReport,
            Self::CommercialComparison => EvaluationEvidenceTarget::ComparisonScorecard,
            _ => EvaluationEvidenceTarget::StructuredTestResult,
        }
    }
}
