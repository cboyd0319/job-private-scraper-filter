//! Pure JobSentinel business values and canonical normalization.

mod application_assistance;
mod external_ai;
mod job;
mod job_hash;
pub mod normalization;
mod scoring_config;
#[cfg(test)]
mod v3_contract_tests;
pub mod v3_contracts;
#[cfg(test)]
mod v3_employer_evaluation_tests;
pub mod v3_evaluation_assertions;
pub mod v3_evaluation_inputs;
#[cfg(test)]
mod v3_evaluation_tests;
pub mod v3_evaluations;
pub mod v3_evidence;
#[cfg(test)]
mod v3_evidence_reviewer_evaluation_tests;
pub mod v3_foundation;
pub mod v3_manifests;
#[cfg(test)]
mod v3_pack_payload_tests;
pub mod v3_pack_payloads;
#[cfg(test)]
mod v3_pack_static_skill_tests;
#[cfg(test)]
mod v3_region_manifest_tests;
#[cfg(test)]
mod v3_signed_pack_tests;
pub mod v3_signed_packs;
pub mod v3_source_authorization;
pub mod v3_source_consent;
#[cfg(test)]
mod v3_source_freshness_tests;
pub mod v3_source_manifest;
#[cfg(test)]
mod v3_source_manifest_linkedin_tests;
#[cfg(test)]
mod v3_source_manifest_tests;
#[cfg(test)]
mod v3_source_manifest_user_actions_tests;
#[cfg(test)]
mod v3_source_simulator_tests;
pub mod v3_veteran_public_service;

pub use application_assistance::{
    requires_user_answer, screening_question_matches, AnswerSource, AnswerStatistics,
    AnswerSuggestion, ApplicationAttempt, ApplicationProfile, ApplicationProfileInput, AtsPlatform,
    AutomationStats, AutomationStatus, ModificationExample, ScreeningAnswer,
};
pub use external_ai::{ExternalAiConfig, ExternalAiProvider, ExternalAiRedactionConfig};
pub use job::Job;
pub use job_hash::calculate_job_hash;
pub use normalization::canonicalize_job_url;
pub use scoring_config::ScoringConfig;
pub use v3_contracts::{read_v3_compatibility, CompatibilityDecision, CompatibilityInputKind};
pub use v3_evaluations::{
    parse_v3_evaluation_set, EvaluationAssertion, EvaluationCategory, EvaluationEvidenceTarget,
    EvaluationVerificationTarget, MilitaryServiceBasis, ProtectedVeteranAnswerBasis,
    V3EvaluationCase, V3EvaluationSet,
};
pub use v3_evidence::{ResumeEvidenceCitation, ResumeEvidenceSnapshot};
pub use v3_manifests::{PackExecutionClass, PrivacyLabel};
