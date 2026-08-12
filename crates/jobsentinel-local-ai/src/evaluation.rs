//! Evaluation, feedback, and future training data contracts.

const SEED_EVAL_FIXTURE_JSON: &str = include_str!("eval_fixtures/seed_v1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLabel {
    NoEvidence = 0,
    KeywordOnly = 1,
    RelatedIncomplete = 2,
    StrongDirectEvidence = 3,
    ExceptionalOwnership = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalDatasetKind {
    JobRequirementToResumeEvidence,
    ResumeProfileToJobMatch,
    SkillPhraseToResumeEvidence,
    JobTitleToResumeSeniority,
    GapAnalysis,
    RoleFamilyExpansion,
    SkillGraphConfusable,
    FairnessCounterfactual,
    SelfPreferenceCheck,
    AdversarialPosting,
    GeneratedAdviceSeparation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceLabelExample {
    pub dataset_kind: EvalDatasetKind,
    pub query: String,
    pub candidate: String,
    pub label: EvidenceLabel,
    pub reason: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HardNegativeExample {
    pub dataset_kind: EvalDatasetKind,
    pub query: String,
    pub positive: String,
    pub hard_negative: String,
    pub mining_source: HardNegativeMiningSource,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardNegativeMiningSource {
    SeedFixture,
    DenseFalsePositive,
    Bm25FalsePositive,
    RerankerFalsePositive,
    UserLabeledFalsePositive,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairwisePreferenceExample {
    pub query_context_hash: String,
    pub chosen_id: String,
    pub rejected_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RankingFeatures {
    pub dense_score: Option<f32>,
    pub reranker_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub skill_coverage: Option<f32>,
    pub required_coverage: Option<f32>,
    pub seniority_match: Option<f32>,
    pub salary_fit: Option<bool>,
    pub location_fit: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackAction {
    SavedJob,
    HidJob,
    Applied,
    Bookmarked,
    MarkedIrrelevant,
    MarkedGhostJob,
    EditedMatchReason,
    AcceptedSuggestedResumeEdit,
    RejectedSuggestedResumeEdit,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedbackEvidenceSummary {
    pub requirement_id: String,
    pub candidate_chunk_id: String,
    pub evidence_class: EvidenceLabel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobFeedbackEvent {
    pub user_action: FeedbackAction,
    pub query_profile_hash: String,
    pub job_id: String,
    pub rank: Option<usize>,
    pub features: RankingFeatures,
    pub top_evidence: Vec<FeedbackEvidenceSummary>,
    pub timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceLabelEvent {
    pub dataset_kind: EvalDatasetKind,
    pub requirement_id: String,
    pub candidate_chunk_id: String,
    pub label: EvidenceLabel,
    pub reason: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalFixtureSet {
    pub schema_version: u32,
    pub evidence_labels: Vec<EvidenceLabelExample>,
    pub hard_negatives: Vec<HardNegativeExample>,
    pub pairwise_preferences: Vec<PairwisePreferenceExample>,
}

impl EvalFixtureSet {
    pub fn seed() -> anyhow::Result<Self> {
        serde_json::from_str(SEED_EVAL_FIXTURE_JSON)
            .map_err(|source| anyhow::anyhow!("seed ML eval fixture is invalid: {source}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelImprovementPhase {
    DomainEvalSet,
    HardNegativeMining,
    RerankerFineTune,
    EmbeddingFineTuneIfRecallFails,
    LearningToRank,
    ContextualBanditPersonalization,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetrievalProvenance {
    pub candidate_id: String,
    pub sources: Vec<String>,
    pub dense_rank: Option<usize>,
    pub bm25_rank: Option<usize>,
    pub skill_hits: Vec<String>,
    pub rerank_rank: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchBlocker {
    MissingClearance,
    WrongLocation,
    SalaryBelowFloor,
    MissingRequiredSkill(String),
    SeniorityMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_labels_are_ordered_for_quality_bands() {
        assert!((EvidenceLabel::NoEvidence as u8) < (EvidenceLabel::KeywordOnly as u8));
        assert!(
            (EvidenceLabel::StrongDirectEvidence as u8)
                < (EvidenceLabel::ExceptionalOwnership as u8)
        );
    }

    #[test]
    fn hard_negative_shape_keeps_positive_and_near_miss_separate() {
        let example = HardNegativeExample {
            dataset_kind: EvalDatasetKind::JobRequirementToResumeEvidence,
            query: "Experience building AWS CloudTrail detections".to_string(),
            positive: "Built CloudTrail detections for IAM privilege escalation.".to_string(),
            hard_negative: "Used AWS IAM while deploying application infrastructure.".to_string(),
            mining_source: HardNegativeMiningSource::SeedFixture,
            reason: Some("Shares AWS/IAM vocabulary but lacks detection evidence.".to_string()),
        };

        assert_ne!(example.positive, example.hard_negative);
        assert!(example
            .reason
            .expect("reason should exist")
            .contains("lacks"));
    }

    #[test]
    fn seed_eval_fixture_parses_and_covers_core_tasks() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");

        assert_eq!(fixture.schema_version, 1);
        assert!(!fixture.hard_negatives.is_empty());
        assert!(!fixture.pairwise_preferences.is_empty());

        for kind in [
            EvalDatasetKind::JobRequirementToResumeEvidence,
            EvalDatasetKind::ResumeProfileToJobMatch,
            EvalDatasetKind::SkillPhraseToResumeEvidence,
            EvalDatasetKind::JobTitleToResumeSeniority,
            EvalDatasetKind::GapAnalysis,
        ] {
            assert!(
                fixture
                    .evidence_labels
                    .iter()
                    .any(|example| example.dataset_kind == kind),
                "missing seed example for {kind:?}"
            );
        }
    }

    #[test]
    fn seed_eval_fixture_covers_research_addendum_tasks() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");

        for kind in [
            EvalDatasetKind::RoleFamilyExpansion,
            EvalDatasetKind::SkillGraphConfusable,
            EvalDatasetKind::FairnessCounterfactual,
            EvalDatasetKind::SelfPreferenceCheck,
            EvalDatasetKind::AdversarialPosting,
            EvalDatasetKind::GeneratedAdviceSeparation,
        ] {
            assert!(
                fixture
                    .evidence_labels
                    .iter()
                    .any(|example| example.dataset_kind == kind),
                "missing seed example for {kind:?}"
            );
        }
    }

    #[test]
    fn seed_eval_fixture_freezes_diverse_requirement_hard_negatives() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");
        let hard_negatives = fixture
            .hard_negatives
            .iter()
            .filter(|example| {
                example.dataset_kind == EvalDatasetKind::JobRequirementToResumeEvidence
            })
            .collect::<Vec<_>>();

        assert_eq!(hard_negatives.len(), 10);
        for query in [
            "Experience making product help clear for customers",
            "Experience with patient intake and electronic health records",
            "Experience building cloud audit detections for privilege escalation",
            "Current unrestricted registered nurse (RN) license required",
            "Experience managing at least ten software engineers",
            "Experience owning a GAAP month-end close",
            "Current state journeyman electrician license required",
            "Valid Class A commercial driver's license (CDL-A) required",
            "Current state teaching credential for secondary mathematics required",
            "Experience owning full profit-and-loss responsibility for a retail store",
        ] {
            assert!(
                hard_negatives.iter().any(|example| example.query == query),
                "missing frozen hard negative for {query}"
            );
        }
        let rn_license = hard_negatives
            .iter()
            .find(|example| {
                example.query == "Current unrestricted registered nurse (RN) license required"
            })
            .expect("RN license hard negative should be frozen");

        assert_eq!(
            rn_license.hard_negative,
            "Served as an Army combat medic and provided emergency care during field exercises."
        );
        assert_eq!(
            rn_license.reason.as_deref(),
            Some(
                "Military medical experience can be relevant, but it does not evidence the required civilian registered nurse license."
            )
        );
    }

    #[test]
    fn fairness_counterfactual_seed_examples_keep_label_stable_for_same_evidence() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");
        let examples: Vec<&EvidenceLabelExample> = fixture
            .evidence_labels
            .iter()
            .filter(|example| {
                example
                    .tags
                    .iter()
                    .any(|tag| tag == "fairness_group_cloud_security_same_evidence")
            })
            .collect();

        assert!(examples.len() >= 2);
        assert!(examples
            .iter()
            .all(|example| example.label == EvidenceLabel::StrongDirectEvidence));
    }

    #[test]
    fn self_preference_seed_examples_keep_label_stable_for_same_facts() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");
        let examples: Vec<&EvidenceLabelExample> = fixture
            .evidence_labels
            .iter()
            .filter(|example| {
                example
                    .tags
                    .iter()
                    .any(|tag| tag == "self_preference_group_same_facts")
            })
            .collect();

        assert!(examples.len() >= 2);
        assert!(examples
            .iter()
            .all(|example| example.label == EvidenceLabel::StrongDirectEvidence));
    }

    #[test]
    fn confusable_and_adversarial_seed_examples_do_not_overclaim_fit() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");

        assert!(fixture.evidence_labels.iter().any(|example| {
            example.dataset_kind == EvalDatasetKind::SkillGraphConfusable
                && example.label == EvidenceLabel::KeywordOnly
                && example
                    .tags
                    .iter()
                    .any(|tag| tag == "confusable_kubernetes_admin_vs_security_detection")
        }));
        assert!(fixture.evidence_labels.iter().any(|example| {
            example.dataset_kind == EvalDatasetKind::AdversarialPosting
                && example.label == EvidenceLabel::NoEvidence
                && example
                    .tags
                    .iter()
                    .any(|tag| tag == "adversarial_prompt_injection")
        }));
    }

    #[test]
    fn generated_advice_seed_examples_are_not_real_posting_evidence() {
        let fixture = EvalFixtureSet::seed().expect("fixture should parse");

        assert!(fixture.evidence_labels.iter().any(|example| {
            example.dataset_kind == EvalDatasetKind::GeneratedAdviceSeparation
                && example.label == EvidenceLabel::NoEvidence
                && example
                    .tags
                    .iter()
                    .any(|tag| tag == "generated_advice_not_real_job")
        }));
    }

    #[test]
    fn feedback_event_uses_ids_and_scores_not_raw_resume_text() {
        let event = JobFeedbackEvent {
            user_action: FeedbackAction::MarkedIrrelevant,
            query_profile_hash: "profile_hash_fixture".to_string(),
            job_id: "job_123".to_string(),
            rank: Some(4),
            features: RankingFeatures {
                dense_score: Some(0.73),
                reranker_score: Some(0.81),
                bm25_score: Some(4.2),
                skill_coverage: Some(0.5),
                required_coverage: Some(0.45),
                seniority_match: Some(0.2),
                salary_fit: Some(false),
                location_fit: Some(true),
            },
            top_evidence: vec![FeedbackEvidenceSummary {
                requirement_id: "req_123".to_string(),
                candidate_chunk_id: "chunk_456".to_string(),
                evidence_class: EvidenceLabel::RelatedIncomplete,
            }],
            timestamp: "2026-06-19T00:00:00Z".to_string(),
        };

        let serialized = serde_json::to_value(event).expect("event should serialize");

        assert!(serialized.get("top_evidence").is_some());
        assert!(serialized.get("resume_text").is_none());
        assert!(serialized.get("candidate_text").is_none());
        assert!(serialized.get("notes").is_none());
    }
}
