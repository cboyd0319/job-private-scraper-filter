//! Scores one synthetic resume-evidence case against closed runtime targets.

use std::{fmt, future::Future};

use super::{EvaluationAssertion, EvaluationInput, V3EvaluationSet};

pub struct AtsResumeRequirementEvaluationScorer {
    evaluation: V3EvaluationSet,
}

pub struct EvidenceReviewerResumeRequirementEvaluationScorer {
    evaluation: V3EvaluationSet,
}

impl fmt::Debug for AtsResumeRequirementEvaluationScorer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AtsResumeRequirementEvaluationScorer")
            .field("revision", &self.evaluation.revision)
            .field("case_count", &self.evaluation.cases.len())
            .finish()
    }
}

impl fmt::Debug for EvidenceReviewerResumeRequirementEvaluationScorer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EvidenceReviewerResumeRequirementEvaluationScorer")
            .field("revision", &self.evaluation.revision)
            .field("case_count", &self.evaluation.cases.len())
            .finish()
    }
}

impl AtsResumeRequirementEvaluationScorer {
    pub(crate) fn new(evaluation: V3EvaluationSet) -> Self {
        Self { evaluation }
    }

    pub fn revision(&self) -> &str {
        &self.evaluation.revision
    }

    /// Run the fixed ATS component while keeping its one synthetic fixture inside the scorer.
    pub fn score_target(
        &self,
        target: impl FnOnce(&str, &[String], &[String]) -> Result<Vec<EvaluationAssertion>, String>,
    ) -> Result<bool, String> {
        let (case, requirement, evidence, shared_keywords) = self.case_input()?;
        let observed = target(requirement, evidence, shared_keywords)?;
        Ok(case.evaluate(&observed))
    }

    fn case_input(&self) -> Result<(&super::V3EvaluationCase, &str, &[String], &[String]), String> {
        resume_evidence_case_input(&self.evaluation)
    }
}

impl EvidenceReviewerResumeRequirementEvaluationScorer {
    pub(crate) fn new(evaluation: V3EvaluationSet) -> Self {
        Self { evaluation }
    }

    /// Run the compiled Evidence Reviewer while keeping its synthetic fixture callback-scoped.
    pub async fn score_target<Target, Observation>(&self, target: Target) -> Result<bool, String>
    where
        Target: FnOnce(String, Vec<String>, Vec<String>) -> Observation,
        Observation: Future<Output = Result<Vec<EvaluationAssertion>, String>>,
    {
        let (case, requirement, evidence, shared_keywords) =
            resume_evidence_case_input(&self.evaluation)?;
        let observed = target(
            requirement.to_string(),
            evidence.to_vec(),
            shared_keywords.to_vec(),
        )
        .await?;
        Ok(case.evaluate(&observed))
    }
}

fn resume_evidence_case_input(
    evaluation: &V3EvaluationSet,
) -> Result<(&super::V3EvaluationCase, &str, &[String], &[String]), String> {
    evaluation.validate_common()?;
    let case = evaluation
        .cases
        .first()
        .filter(|_| evaluation.cases.len() == 1)
        .ok_or_else(|| "resume evidence evaluation case is unavailable".to_string())?;
    let EvaluationInput::ResumeEvidence {
        requirement,
        evidence,
        shared_keywords,
    } = &case.input
    else {
        return Err("resume evidence evaluation input is invalid".to_string());
    };
    Ok((case, requirement, evidence, shared_keywords))
}
