//! Runs verified evaluation packs against closed deterministic product components.

use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{NaiveDate, TimeZone, Utc};
use jobsentinel_documents::{AtsAnalyzer, RequirementMatchState};
use jobsentinel_domain::{
    v3_evaluations::EvaluationAssertion, v3_pack_payloads::SelfTestedPackPayload,
    v3_signed_packs::TrustedPublisherKey, Job,
};
use jobsentinel_storage::Database;
use serde::Serialize;

use super::execution::load_active_pack_payload;
use crate::v3_foundation::prepare_saved_match_debugger;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AtsResumeRequirementEvaluation {
    pub target: &'static str,
    pub passed_cases: u8,
    pub failed_cases: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceReviewerResumeRequirementEvaluation {
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

#[allow(clippy::too_many_arguments)]
pub async fn evaluate_active_evidence_reviewer_resume_requirement_pack(
    database: &Database,
    artifact_root: &Path,
    publisher_key_id: &str,
    pack_id: &str,
    expected_generation: u64,
    trusted_publishers: &[TrustedPublisherKey],
    today: NaiveDate,
) -> Result<EvidenceReviewerResumeRequirementEvaluation> {
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
    let SelfTestedPackPayload::EvidenceReviewerResumeRequirementEvaluation { scorer } =
        active.payload
    else {
        return Err(anyhow!(
            "pack does not contain a runnable Evidence Reviewer resume requirement evaluation"
        ));
    };
    let passed = scorer
        .score_target(observe_evidence_reviewer)
        .await
        .map_err(|_| anyhow!("pack evaluation failed"))?;
    Ok(EvidenceReviewerResumeRequirementEvaluation {
        target: "evidence_reviewer_resume_requirement_v1",
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

async fn observe_evidence_reviewer(
    requirement: String,
    evidence: Vec<String>,
    shared_keywords: Vec<String>,
) -> Result<Vec<EvaluationAssertion>, String> {
    let database = Database::connect_memory()
        .await
        .map_err(|_| "synthetic evaluation storage is unavailable".to_string())?;
    database
        .migrate()
        .await
        .map_err(|_| "synthetic evaluation storage is unavailable".to_string())?;
    let observed_at = Utc
        .with_ymd_and_hms(2026, 7, 20, 12, 0, 0)
        .single()
        .ok_or_else(|| "synthetic evaluation time is invalid".to_string())?;
    let mut job = Job::newly_discovered(
        "Synthetic Evidence Reviewer Evaluation",
        "JobSentinel",
        "https://example.invalid/evidence-reviewer-evaluation",
        Some("Remote".to_string()),
        "synthetic-evaluation",
        observed_at,
    );
    job.description = Some(format!("Required: {requirement}"));
    database
        .insert_job_if_new(&job)
        .await
        .map_err(|_| "synthetic evaluation setup failed".to_string())?;

    let directory =
        tempfile::tempdir().map_err(|_| "synthetic evaluation setup failed".to_string())?;
    let resume_path = directory.path().join("synthetic-resume.txt");
    let mut resume_lines = vec!["Skills".to_string()];
    resume_lines.extend(shared_keywords);
    resume_lines.push("Experience".to_string());
    if evidence.is_empty() {
        resume_lines.push("Supported an unrelated synthetic operations team.".to_string());
    } else {
        resume_lines.extend(evidence.iter().cloned());
    }
    std::fs::write(&resume_path, resume_lines.join("\n"))
        .map_err(|_| "synthetic evaluation setup failed".to_string())?;
    let resume_id = database
        .resume_matcher()
        .upload_resume("Synthetic Resume", resume_path.to_string_lossy().as_ref())
        .await
        .map_err(|_| "synthetic evaluation setup failed".to_string())?;
    database
        .resume_matcher()
        .match_resume_to_job(resume_id, &job.hash)
        .await
        .map_err(|_| "synthetic evaluation setup failed".to_string())?;

    let debugger = prepare_saved_match_debugger(&database, &job.hash, resume_id)
        .await
        .map_err(|_| "Evidence Reviewer evaluation failed".to_string())?;
    let [review] = debugger.requirements() else {
        return Err("Evidence Reviewer evaluation requires exactly one requirement".to_string());
    };
    let direct = matches!(
        review.match_state(),
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
