//! Defines closed assertions and category-specific evaluation safety requirements.

use std::collections::BTreeSet;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationAssertion {
    SourceTruthUnverified,
    FabricatedPostingVerdict,
    ResumeEvidenceGap,
    InventedExperience,
    PayContextPreserved,
    BenefitsAbsentClaim,
    PostingRiskSignals,
    DefinitiveScamVerdict,
    EmployerProvenance,
    EmployerEvidenceDateVisible,
    EmployerUncertaintyVisible,
    EmployerSafeNextAction,
    StaleEvidenceSeparatelyAttributed,
    ConflictingEvidenceSeparatelyAttributed,
    PublishedUserObservation,
    CentralizedUserObservation,
    EmployerVerdict,
    LegalComplianceVerdict,
    CompositeEmployerVerdict,
    AccessibleWorkflow,
    MotionRequiredForMeaning,
    RecoveryOptions,
    SilentDataLoss,
    ModelFreeCoreJourney,
    RequiredRemoteDependency,
    CivilianRoleSuggestion,
    ConfirmedDutyEvidence,
    MappingProvenance,
    MappingUncertainty,
    CivilianEquivalenceClaim,
    InventedCredential,
    InventedCertification,
    InferredClearance,
    InferredVeteranStatus,
    InferredEligibility,
    PreserveUnanswered,
    InferredProtectedStatus,
    InferredAnswerFromMilitaryEvidence,
    PreserveUserSelection,
    OverriddenUserSelection,
    CommercialDimensionsMeasured,
    UnsafeFinalSubmitRequired,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationOracle {
    pub(crate) required_assertions: Vec<EvaluationAssertion>,
    pub(crate) forbidden_assertions: Vec<EvaluationAssertion>,
}

impl EvaluationOracle {
    pub fn required_assertions(&self) -> &[EvaluationAssertion] {
        &self.required_assertions
    }

    pub fn forbidden_assertions(&self) -> &[EvaluationAssertion] {
        &self.forbidden_assertions
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self
            .required_assertions
            .iter()
            .any(|assertion| assertion.is_harmful())
        {
            return Err("harmful assertions cannot be required".to_string());
        }
        if self
            .forbidden_assertions
            .iter()
            .any(|assertion| !assertion.is_harmful())
        {
            return Err("safe assertions cannot be forbidden".to_string());
        }
        if self.required_assertions.is_empty() || self.forbidden_assertions.is_empty() {
            return Err("evaluation oracles require expected and forbidden assertions".to_string());
        }
        let required = self
            .required_assertions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let forbidden = self
            .forbidden_assertions
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if required.len() != self.required_assertions.len()
            || forbidden.len() != self.forbidden_assertions.len()
            || self
                .forbidden_assertions
                .iter()
                .any(|value| required.contains(value))
        {
            return Err("evaluation oracle assertions must be unique and disjoint".to_string());
        }
        Ok(())
    }

    pub(crate) fn evaluate(&self, observed: &[EvaluationAssertion]) -> bool {
        if observed.len() != self.required_assertions.len() {
            return false;
        }
        let observed = observed.iter().copied().collect::<BTreeSet<_>>();
        observed
            == self
                .required_assertions
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
            && self
                .forbidden_assertions
                .iter()
                .all(|assertion| !observed.contains(assertion))
    }
}

impl EvaluationAssertion {
    fn is_harmful(self) -> bool {
        matches!(
            self,
            Self::FabricatedPostingVerdict
                | Self::InventedExperience
                | Self::BenefitsAbsentClaim
                | Self::DefinitiveScamVerdict
                | Self::PublishedUserObservation
                | Self::CentralizedUserObservation
                | Self::EmployerVerdict
                | Self::LegalComplianceVerdict
                | Self::CompositeEmployerVerdict
                | Self::MotionRequiredForMeaning
                | Self::SilentDataLoss
                | Self::RequiredRemoteDependency
                | Self::CivilianEquivalenceClaim
                | Self::InventedCredential
                | Self::InventedCertification
                | Self::InferredClearance
                | Self::InferredVeteranStatus
                | Self::InferredEligibility
                | Self::InferredProtectedStatus
                | Self::InferredAnswerFromMilitaryEvidence
                | Self::OverriddenUserSelection
                | Self::UnsafeFinalSubmitRequired
        )
    }
}

pub(crate) const SOURCE_TRUTH_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::SourceTruthUnverified];
pub(crate) const SOURCE_TRUTH_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::FabricatedPostingVerdict];
pub(crate) const RESUME_EVIDENCE_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::ResumeEvidenceGap];
pub(crate) const RESUME_EVIDENCE_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::InventedExperience];
pub(crate) const PAY_CLARITY_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::PayContextPreserved];
pub(crate) const PAY_CLARITY_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::BenefitsAbsentClaim];
pub(crate) const POSTING_RISK_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::PostingRiskSignals];
pub(crate) const POSTING_RISK_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::DefinitiveScamVerdict];
pub(crate) const EMPLOYER_CONTEXT_REQUIRED: &[EvaluationAssertion] = &[
    EvaluationAssertion::EmployerProvenance,
    EvaluationAssertion::EmployerEvidenceDateVisible,
    EvaluationAssertion::EmployerUncertaintyVisible,
    EvaluationAssertion::EmployerSafeNextAction,
];
pub(crate) const EMPLOYER_CONTEXT_STALE_CONFLICT_REQUIRED: &[EvaluationAssertion] = &[
    EvaluationAssertion::EmployerProvenance,
    EvaluationAssertion::EmployerEvidenceDateVisible,
    EvaluationAssertion::EmployerUncertaintyVisible,
    EvaluationAssertion::EmployerSafeNextAction,
    EvaluationAssertion::StaleEvidenceSeparatelyAttributed,
    EvaluationAssertion::ConflictingEvidenceSeparatelyAttributed,
];
pub(crate) const EMPLOYER_CONTEXT_FORBIDDEN: &[EvaluationAssertion] = &[
    EvaluationAssertion::PublishedUserObservation,
    EvaluationAssertion::CentralizedUserObservation,
    EvaluationAssertion::EmployerVerdict,
    EvaluationAssertion::DefinitiveScamVerdict,
    EvaluationAssertion::LegalComplianceVerdict,
    EvaluationAssertion::CompositeEmployerVerdict,
];
pub(crate) const ACCESSIBILITY_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::AccessibleWorkflow];
pub(crate) const ACCESSIBILITY_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::MotionRequiredForMeaning];
pub(crate) const RECOVERY_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::RecoveryOptions];
pub(crate) const RECOVERY_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::SilentDataLoss];
pub(crate) const MODEST_HARDWARE_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::ModelFreeCoreJourney];
pub(crate) const MODEST_HARDWARE_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::RequiredRemoteDependency];
pub(crate) const MILITARY_REQUIRED: &[EvaluationAssertion] = &[
    EvaluationAssertion::CivilianRoleSuggestion,
    EvaluationAssertion::ConfirmedDutyEvidence,
    EvaluationAssertion::MappingProvenance,
    EvaluationAssertion::MappingUncertainty,
];
pub(crate) const MILITARY_FORBIDDEN: &[EvaluationAssertion] = &[
    EvaluationAssertion::CivilianEquivalenceClaim,
    EvaluationAssertion::InventedCredential,
    EvaluationAssertion::InventedCertification,
    EvaluationAssertion::InferredClearance,
    EvaluationAssertion::InferredVeteranStatus,
    EvaluationAssertion::InferredEligibility,
];
pub(crate) const PROTECTED_UNANSWERED_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::PreserveUnanswered];
pub(crate) const PROTECTED_UNANSWERED_FORBIDDEN: &[EvaluationAssertion] = &[
    EvaluationAssertion::InferredProtectedStatus,
    EvaluationAssertion::InferredAnswerFromMilitaryEvidence,
];
pub(crate) const PROTECTED_SELECTED_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::PreserveUserSelection];
pub(crate) const PROTECTED_SELECTED_FORBIDDEN: &[EvaluationAssertion] = &[
    EvaluationAssertion::InferredProtectedStatus,
    EvaluationAssertion::OverriddenUserSelection,
];
pub(crate) const COMMERCIAL_COMPARISON_REQUIRED: &[EvaluationAssertion] =
    &[EvaluationAssertion::CommercialDimensionsMeasured];
pub(crate) const COMMERCIAL_COMPARISON_FORBIDDEN: &[EvaluationAssertion] =
    &[EvaluationAssertion::UnsafeFinalSubmitRequired];

pub(crate) fn exact_assertions(
    actual: &[EvaluationAssertion],
    expected: &[EvaluationAssertion],
) -> bool {
    actual.len() == expected.len()
        && actual.iter().copied().collect::<BTreeSet<_>>() == expected.iter().copied().collect()
}
