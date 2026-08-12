//! Strict, non-executing self-tests for signed v3 pack payloads.

use chrono::{DateTime, NaiveDate, Utc};
use jobsentinel_security::{
    contains_prompt_injection_phrase, contains_review_required_invisible_control,
};
use serde::Deserialize;

use crate::{
    v3_evaluations::{
        parse_ats_resume_requirement_evaluation_set, parse_v3_evaluation_set,
        AtsResumeRequirementEvaluationScorer, EvaluationScorer,
        EvidenceReviewerResumeRequirementEvaluationScorer,
    },
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::{
        AgentTask, AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass,
        PackType, PrivacyLabel, SourceClass,
    },
    v3_signed_packs::VerifiedPackRelease,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{parse_source_manifest, SourceManifest, SourceStopCondition},
};

const PACK_PAYLOAD_SCHEMA: &str = "jobsentinel.v3.pack-payload.v1";
const MAX_SOURCE_FIXTURES: usize = 32;
const MAX_FIXTURE_BYTES: usize = 512 * 1024;
const MAX_PLAN_STEPS: usize = 8;

mod static_skill;

use static_skill::{self_test_static_skill, StaticSkillPayloadV1};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedTaskPlanStep {
    pub action: PackAction,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillHandoff {
    pub task_kind: AgentTaskKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticSkillResource {
    pub path: String,
    pub content: String,
}

#[derive(Debug)]
pub enum SelfTestedPackPayload {
    Source {
        policy: SourcePolicy,
        manifest: Box<SourceManifest>,
        fixture_count: usize,
        initial_stop: SourceStopCondition,
    },
    ReviewedWorkflow {
        task: AgentTask,
        plan: Vec<ReviewedTaskPlanStep>,
        failure_message: String,
    },
    StaticSkill {
        skill_name: String,
        skill_md: String,
        resources: Vec<StaticSkillResource>,
        handoff: Option<SkillHandoff>,
    },
    Evaluation {
        scorer: EvaluationScorer,
    },
    AtsResumeRequirementEvaluation {
        scorer: AtsResumeRequirementEvaluationScorer,
    },
    EvidenceReviewerResumeRequirementEvaluation {
        scorer: EvidenceReviewerResumeRequirementEvaluationScorer,
    },
}

/// A self-test result inseparable from the signed release that produced it.
#[derive(Debug)]
pub struct SelfTestedPackRelease {
    publisher_key_id: String,
    pack_id: String,
    release_sequence: u64,
    signed_release_sha256: String,
    payload: SelfTestedPackPayload,
}

impl SelfTestedPackRelease {
    pub fn publisher_key_id(&self) -> &str {
        &self.publisher_key_id
    }

    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    pub const fn release_sequence(&self) -> u64 {
        self.release_sequence
    }

    pub fn signed_release_sha256(&self) -> &str {
        &self.signed_release_sha256
    }

    pub const fn payload(&self) -> &SelfTestedPackPayload {
        &self.payload
    }

    pub fn into_payload(self) -> SelfTestedPackPayload {
        self.payload
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "pack_type", rename_all = "snake_case", deny_unknown_fields)]
enum PackPayloadV1 {
    Source {
        schema: String,
        policy: SourcePolicyPayloadV1,
        manifest_json: String,
        fixtures: Vec<SourceFixturePayloadV1>,
    },
    Agent(ReviewedWorkflowPayloadV1),
    Workflow(ReviewedWorkflowPayloadV1),
    Skill(StaticSkillPayloadV1),
    Evaluation(EvaluationPayloadV1),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationPayloadV1 {
    schema: String,
    #[serde(default)]
    target: Option<EvaluationTargetV1>,
    evaluation_set_json: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvaluationTargetV1 {
    AtsPlainTextResumeRequirementV1,
    EvidenceReviewerResumeRequirementV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedWorkflowPayloadV1 {
    schema: String,
    task: AgentTask,
    plan: Vec<ReviewedTaskPlanStepV1>,
    failure_message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewedTaskPlanStepV1 {
    action: PackAction,
    label: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcePolicyPayloadV1 {
    source_id: String,
    source_class: SourceClass,
    access: SourceAccess,
    request_limit_per_hour: u16,
    user_review_required: bool,
    policy_ref: String,
    revision: u32,
    restriction_reason_code: Option<String>,
    reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFixturePayloadV1 {
    path: String,
    content: String,
}

/// Parse and self-test a signature-verified payload without installing or executing it.
pub fn parse_and_self_test_pack_payload(
    release: &VerifiedPackRelease,
    today: NaiveDate,
) -> Result<SelfTestedPackRelease, String> {
    release
        .manifest
        .verify_payload(release.payload.as_bytes())
        .map_err(|_| "pack payload self-test failed".to_string())?;
    let payload: PackPayloadV1 = serde_json::from_str(&release.payload)
        .map_err(|_| "pack payload self-test failed".to_string())?;
    let payload = match payload {
        PackPayloadV1::Source {
            schema,
            policy,
            manifest_json,
            fixtures,
        } => self_test_source(release, &schema, policy, &manifest_json, fixtures, today),
        PackPayloadV1::Agent(payload) => {
            self_test_reviewed_workflow(release, PackType::Agent, payload)
        }
        PackPayloadV1::Workflow(payload) => {
            self_test_reviewed_workflow(release, PackType::Workflow, payload)
        }
        PackPayloadV1::Skill(payload) => self_test_static_skill(release, payload),
        PackPayloadV1::Evaluation(payload) => self_test_evaluation(release, payload),
    }?;
    Ok(SelfTestedPackRelease {
        publisher_key_id: release.publisher_key_id.clone(),
        pack_id: release.manifest.pack_id.clone(),
        release_sequence: release.release_sequence,
        signed_release_sha256: release.signed_release_sha256.clone(),
        payload,
    })
}

fn self_test_evaluation(
    release: &VerifiedPackRelease,
    payload: EvaluationPayloadV1,
) -> Result<SelfTestedPackPayload, String> {
    if payload.schema != PACK_PAYLOAD_SCHEMA
        || release.manifest.pack_type != PackType::Evaluation
        || release.manifest.execution_class != PackExecutionClass::StaticContent
        || release.manifest.privacy_labels != [PrivacyLabel::LocalOnly]
        || !release.manifest.allowed_data_categories.is_empty()
        || !release.external_destinations.is_empty()
    {
        return Err("evaluation pack self-test failed".to_string());
    }
    match payload.target {
        None => {
            let evaluation = parse_v3_evaluation_set(&payload.evaluation_set_json)
                .map_err(|_| "evaluation pack self-test failed".to_string())?;
            Ok(SelfTestedPackPayload::Evaluation {
                scorer: EvaluationScorer::new(evaluation),
            })
        }
        Some(EvaluationTargetV1::AtsPlainTextResumeRequirementV1) => {
            let evaluation =
                parse_ats_resume_requirement_evaluation_set(&payload.evaluation_set_json)
                    .map_err(|_| "evaluation pack self-test failed".to_string())?;
            Ok(SelfTestedPackPayload::AtsResumeRequirementEvaluation {
                scorer: AtsResumeRequirementEvaluationScorer::new(evaluation),
            })
        }
        Some(EvaluationTargetV1::EvidenceReviewerResumeRequirementV1) => {
            let evaluation =
                parse_ats_resume_requirement_evaluation_set(&payload.evaluation_set_json)
                    .map_err(|_| "evaluation pack self-test failed".to_string())?;
            Ok(
                SelfTestedPackPayload::EvidenceReviewerResumeRequirementEvaluation {
                    scorer: EvidenceReviewerResumeRequirementEvaluationScorer::new(evaluation),
                },
            )
        }
    }
}

fn self_test_reviewed_workflow(
    release: &VerifiedPackRelease,
    pack_type: PackType,
    payload: ReviewedWorkflowPayloadV1,
) -> Result<SelfTestedPackPayload, String> {
    payload
        .task
        .validate()
        .map_err(|_| "reviewed workflow self-test failed".to_string())?;
    let actions = payload
        .plan
        .iter()
        .map(|step| step.action)
        .collect::<Vec<_>>();
    if payload.schema != PACK_PAYLOAD_SCHEMA
        || release.manifest.pack_type != pack_type
        || release.manifest.execution_class != PackExecutionClass::ReviewedTypedWorkflow
        || release.manifest.privacy_labels != payload.task.privacy_labels
        || release.manifest.allowed_data_categories != payload.task.data_categories
        || release.manifest.allowed_task_kinds != [payload.task.kind]
        || release.manifest.allowed_actions != actions
        || release.manifest.approval_gates != [ApprovalGate::PerExecutionReview]
        || release.manifest.gateway_policy_id.is_some()
        || !release.external_destinations.is_empty()
        || payload.task.pack_id.as_deref() != Some(release.manifest.pack_id.as_str())
        || !is_identifier(&payload.task.task_id)
        || !(1..=MAX_PLAN_STEPS).contains(&payload.plan.len())
        || has_duplicate_actions(&payload.plan)
        || payload
            .plan
            .iter()
            .any(|step| !is_safe_display_text(&step.label, 120))
        || !is_safe_display_text(&payload.failure_message, 256)
        || !matches_compiled_workflow(&payload.task, &actions)
    {
        return Err("reviewed workflow self-test failed".to_string());
    }
    Ok(SelfTestedPackPayload::ReviewedWorkflow {
        task: payload.task,
        plan: payload
            .plan
            .into_iter()
            .map(|step| ReviewedTaskPlanStep {
                action: step.action,
                label: step.label,
            })
            .collect(),
        failure_message: payload.failure_message,
    })
}

fn matches_compiled_workflow(task: &AgentTask, actions: &[PackAction]) -> bool {
    match task.kind {
        AgentTaskKind::EvidenceReview => {
            task.privacy_labels == [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive]
                && task.data_categories == [DataCategory::ResumeEvidence]
                && task.max_duration_seconds <= 30
                && task.max_output_bytes <= 256 * 1024
                && task.max_attempts == 1
                && actions
                    == [
                        PackAction::ReadSelectedResumeEvidence,
                        PackAction::WriteLocalEvent,
                    ]
        }
        AgentTaskKind::DraftPacket => {
            task.privacy_labels == [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive]
                && task.data_categories
                    == [DataCategory::PublicJobPosting, DataCategory::ResumeEvidence]
                && task.max_duration_seconds <= 60
                && task.max_output_bytes <= 512 * 1024
                && task.max_attempts == 1
                && actions
                    == [
                        PackAction::ReadSelectedCaseFile,
                        PackAction::ReadSelectedResumeEvidence,
                        PackAction::ReadPublicJobPosting,
                        PackAction::CreateDraftApplicationPacket,
                        PackAction::WriteLocalEvent,
                    ]
        }
        AgentTaskKind::SourceCheck
        | AgentTaskKind::Backup
        | AgentTaskKind::Export
        | AgentTaskKind::PackInstall => false,
    }
}

fn self_test_source(
    release: &VerifiedPackRelease,
    schema: &str,
    policy: SourcePolicyPayloadV1,
    manifest_json: &str,
    fixtures: Vec<SourceFixturePayloadV1>,
    today: NaiveDate,
) -> Result<SelfTestedPackPayload, String> {
    if schema != PACK_PAYLOAD_SCHEMA
        || release.manifest.pack_type != PackType::Source
        || release.manifest.execution_class != PackExecutionClass::StaticContent
        || release.manifest.privacy_labels != [PrivacyLabel::LocalOnly]
        || !release.manifest.allowed_data_categories.is_empty()
        || !release.external_destinations.is_empty()
        || !(1..=MAX_SOURCE_FIXTURES).contains(&fixtures.len())
        || fixtures.iter().any(|fixture| {
            fixture.content.is_empty()
                || fixture.content.len() > MAX_FIXTURE_BYTES
                || fixture.path.is_empty()
                || fixture.path.len() > 256
        })
        || has_duplicate_paths(&fixtures)
    {
        return Err("source pack payload self-test failed".to_string());
    }
    let policy = policy.into_policy();
    if policy.access != SourceAccess::Disabled || policy.reviewed_at.date_naive() > today {
        return Err("source pack must install disabled".to_string());
    }
    let manifest = parse_source_manifest(manifest_json, &policy)
        .map_err(|_| "source pack payload self-test failed".to_string())?;
    let fixture_refs = fixtures
        .iter()
        .map(|fixture| (fixture.path.as_str(), fixture.content.as_bytes()))
        .collect::<Vec<_>>();
    for action in &manifest.actions {
        let report = manifest
            .simulate(
                &policy,
                action.operation,
                today,
                SourceGrantState::NotRequired,
                &fixture_refs,
            )
            .map_err(|_| "source pack payload self-test failed".to_string())?;
        if report.decision != SourceActionDecision::Blocked(SourceStopCondition::PolicyDisabled) {
            return Err("source pack payload self-test failed".to_string());
        }
    }
    Ok(SelfTestedPackPayload::Source {
        policy,
        manifest: Box::new(manifest),
        fixture_count: fixtures.len(),
        initial_stop: SourceStopCondition::PolicyDisabled,
    })
}

fn has_duplicate_paths(fixtures: &[SourceFixturePayloadV1]) -> bool {
    fixtures.iter().enumerate().any(|(index, fixture)| {
        fixtures[..index]
            .iter()
            .any(|other| other.path == fixture.path)
    })
}

fn has_duplicate_actions(steps: &[ReviewedTaskPlanStepV1]) -> bool {
    steps.iter().enumerate().any(|(index, step)| {
        steps[..index]
            .iter()
            .any(|other| other.action == step.action)
    })
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_safe_display_text(value: &str, max_bytes: usize) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.len() <= max_bytes
        && !value.chars().any(char::is_control)
        && !contains_review_required_invisible_control(value)
        && !contains_prompt_injection_phrase(value)
}

impl SourcePolicyPayloadV1 {
    fn into_policy(self) -> SourcePolicy {
        SourcePolicy {
            source_id: self.source_id,
            source_class: self.source_class,
            access: self.access,
            request_limit_per_hour: self.request_limit_per_hour,
            user_review_required: self.user_review_required,
            policy_ref: self.policy_ref,
            revision: self.revision,
            restriction_reason_code: self.restriction_reason_code,
            reviewed_at: self.reviewed_at,
        }
    }
}
