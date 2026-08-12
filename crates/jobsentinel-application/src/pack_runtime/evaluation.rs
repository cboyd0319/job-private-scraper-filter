//! Runs verified evaluation packs against one deterministic plain-text ATS component.

use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use jobsentinel_documents::{AtsAnalyzer, RequirementMatchState};
use jobsentinel_domain::{
    v3_evaluations::EvaluationAssertion, v3_pack_payloads::SelfTestedPackPayload,
    v3_signed_packs::TrustedPublisherKey,
};
use jobsentinel_storage::Database;
use serde::Serialize;

use super::execution::load_active_pack_payload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtsResumeRequirementEvaluation {
    pub target: &'static str,
    pub passed_cases: u8,
    pub failed_cases: u8,
}

#[allow(clippy::too_many_arguments)]
pub async fn evaluate_active_ats_resume_requirement_pack(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<AtsResumeRequirementEvaluation> {
    let active = load_active_pack_payload(
        database,
        artifact_root,
        publisher_key_id,
        pack_id,
        expected_generation,
        trusted_publishers,
        today,
    )
    .await?;
    let SelfTestedPackPayload::AtsResumeRequirementEvaluation { scorer } = active.payload else {
        return Err(anyhow!(
            "pack does not contain a runnable ATS resume requirement evaluation"
        ));
    };
    let passed = scorer
        .score_target(observe_resume_evidence)
        .map_err(|_| anyhow!("pack evaluation failed"))?;
    Ok(AtsResumeRequirementEvaluation {
        target: "ats_plain_text_resume_requirement_v1",
        passed_cases: u8::from(passed),
        failed_cases: u8::from(!passed),
    })
}

fn observe_resume_evidence(
    requirement: &str,
    evidence: &[String],
    shared_keywords: &[String],
) -> Result<Vec<EvaluationAssertion>, String> {
    let analysis = AtsAnalyzer::analyze_text_for_job(
        &evidence.join("\n"),
        shared_keywords,
        &format!("Required: {requirement}"),
    );
    let [review] = analysis.requirement_reviews.as_slice() else {
        return Err("ATS evaluation requires exactly one extracted requirement".to_string());
    };
    let direct = matches!(
        review.match_state,
        RequirementMatchState::Direct | RequirementMatchState::Strong
    );
    if !evidence.is_empty() && direct {
        Ok(Vec::new())
    } else if evidence.is_empty() && direct {
        Ok(vec![EvaluationAssertion::InventedExperience])
    } else {
        Ok(vec![EvaluationAssertion::ResumeEvidenceGap])
    }
}
