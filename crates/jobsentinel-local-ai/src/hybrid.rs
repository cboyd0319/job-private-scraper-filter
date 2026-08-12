//! Hybrid local ranking for resume and job matching.

use std::cmp::Ordering;

use crate::{MatchBlocker, RankingFeatures, RetrievalProvenance};

#[derive(Debug, Clone)]
pub struct HybridCandidate {
    pub id: String,
    pub dense_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub skill_coverage: Option<f32>,
    pub required_coverage: Option<f32>,
    pub seniority_match: Option<f32>,
    pub reranker_score: Option<f32>,
    pub skill_hits: Vec<String>,
    pub blockers: Vec<MatchBlocker>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HybridScore {
    pub candidate_id: String,
    pub score: f32,
    pub rank: usize,
    pub features: RankingFeatures,
    pub blockers: Vec<MatchBlocker>,
    pub provenance: RetrievalProvenance,
}

#[derive(Debug, Clone)]
pub struct HybridWeights {
    pub dense: f32,
    pub bm25: f32,
    pub skill_coverage: f32,
    pub required_coverage: f32,
    pub seniority: f32,
    pub reranker: f32,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            dense: 0.20,
            bm25: 0.10,
            skill_coverage: 0.20,
            required_coverage: 0.15,
            seniority: 0.05,
            reranker: 0.30,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HybridScorer {
    weights: HybridWeights,
}

impl HybridScorer {
    pub fn new(weights: HybridWeights) -> Self {
        Self { weights }
    }

    pub fn score(&self, candidates: &[HybridCandidate]) -> Vec<HybridScore> {
        let bm25_max = candidates
            .iter()
            .filter_map(|candidate| candidate.bm25_score)
            .filter(|score| score.is_finite() && *score > 0.0)
            .fold(0.0_f32, f32::max);
        let dense_ranks = ranks_by_score(candidates, |candidate| candidate.dense_score);
        let bm25_ranks = ranks_by_score(candidates, |candidate| candidate.bm25_score);

        let mut results: Vec<HybridScore> = candidates
            .iter()
            .map(|candidate| {
                let normalized_bm25 = candidate
                    .bm25_score
                    .filter(|score| bm25_max > 0.0 && score.is_finite())
                    .map(|score| clamp01(score / bm25_max));
                let features = RankingFeatures {
                    dense_score: candidate.dense_score.map(clamp01),
                    reranker_score: candidate.reranker_score.map(clamp01),
                    bm25_score: normalized_bm25,
                    skill_coverage: candidate.skill_coverage.map(clamp01),
                    required_coverage: candidate.required_coverage.map(clamp01),
                    seniority_match: candidate.seniority_match.map(clamp01),
                    salary_fit: Some(
                        !candidate
                            .blockers
                            .iter()
                            .any(|blocker| matches!(blocker, MatchBlocker::SalaryBelowFloor)),
                    ),
                    location_fit: Some(
                        !candidate
                            .blockers
                            .iter()
                            .any(|blocker| matches!(blocker, MatchBlocker::WrongLocation)),
                    ),
                };
                let weighted_score = self.weighted_score(&features);
                let score = weighted_score.min(blocker_cap(&candidate.blockers));
                let provenance = RetrievalProvenance {
                    candidate_id: candidate.id.clone(),
                    sources: retrieval_sources(&features, &candidate.skill_hits),
                    dense_rank: dense_ranks
                        .iter()
                        .position(|id| id == &candidate.id)
                        .map(|index| index + 1),
                    bm25_rank: bm25_ranks
                        .iter()
                        .position(|id| id == &candidate.id)
                        .map(|index| index + 1),
                    skill_hits: candidate.skill_hits.clone(),
                    rerank_rank: None,
                };

                HybridScore {
                    candidate_id: candidate.id.clone(),
                    score,
                    rank: 0,
                    features,
                    blockers: candidate.blockers.clone(),
                    provenance,
                }
            })
            .collect();

        results.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });

        for (index, result) in results.iter_mut().enumerate() {
            result.rank = index + 1;
            if result.features.reranker_score.is_some() {
                result.provenance.rerank_rank = Some(index + 1);
            }
        }

        results
    }

    fn weighted_score(&self, features: &RankingFeatures) -> f32 {
        let signals = [
            (features.dense_score, self.weights.dense),
            (features.bm25_score, self.weights.bm25),
            (features.skill_coverage, self.weights.skill_coverage),
            (features.required_coverage, self.weights.required_coverage),
            (features.seniority_match, self.weights.seniority),
            (features.reranker_score, self.weights.reranker),
        ];
        let (weighted_sum, active_weight) = signals.into_iter().fold(
            (0.0_f32, 0.0_f32),
            |(weighted_sum, active_weight), (score, weight)| {
                if weight <= 0.0 {
                    return (weighted_sum, active_weight);
                }
                match score {
                    Some(score) => (
                        weighted_sum + clamp01(score) * weight,
                        active_weight + weight,
                    ),
                    None => (weighted_sum, active_weight),
                }
            },
        );

        if active_weight > 0.0 {
            weighted_sum / active_weight
        } else {
            0.0
        }
    }
}

fn retrieval_sources(features: &RankingFeatures, skill_hits: &[String]) -> Vec<String> {
    let mut sources = Vec::new();
    if features.dense_score.is_some() {
        sources.push("dense".to_string());
    }
    if features.bm25_score.is_some() {
        sources.push("bm25".to_string());
    }
    if !skill_hits.is_empty() || features.skill_coverage.is_some() {
        sources.push("skill_exact".to_string());
    }
    if features.required_coverage.is_some() {
        sources.push("required_coverage".to_string());
    }
    if features.seniority_match.is_some() {
        sources.push("seniority".to_string());
    }
    if features.reranker_score.is_some() {
        sources.push("reranker".to_string());
    }
    sources
}

fn ranks_by_score<F>(candidates: &[HybridCandidate], score: F) -> Vec<String>
where
    F: Fn(&HybridCandidate) -> Option<f32>,
{
    let mut scored: Vec<(String, f32)> = candidates
        .iter()
        .filter_map(|candidate| {
            score(candidate)
                .filter(|score| score.is_finite())
                .map(|score| (candidate.id.clone(), score))
        })
        .collect();
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
    scored.into_iter().map(|(id, _)| id).collect()
}

fn blocker_cap(blockers: &[MatchBlocker]) -> f32 {
    blockers.iter().fold(1.0_f32, |cap, blocker| {
        cap.min(match blocker {
            MatchBlocker::MissingClearance | MatchBlocker::MissingRequiredSkill(_) => 0.45,
            MatchBlocker::WrongLocation | MatchBlocker::SalaryBelowFloor => 0.55,
            MatchBlocker::SeniorityMismatch => 0.65,
        })
    })
}

fn clamp01(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str) -> HybridCandidate {
        HybridCandidate {
            id: id.to_string(),
            dense_score: None,
            bm25_score: None,
            skill_coverage: None,
            required_coverage: None,
            seniority_match: None,
            reranker_score: None,
            skill_hits: Vec::new(),
            blockers: Vec::new(),
        }
    }

    #[test]
    fn reranker_and_required_coverage_beat_keyword_only_near_miss() {
        let mut direct = candidate("direct");
        direct.dense_score = Some(0.82);
        direct.bm25_score = Some(1.2);
        direct.skill_coverage = Some(0.80);
        direct.required_coverage = Some(0.85);
        direct.seniority_match = Some(0.75);
        direct.reranker_score = Some(0.91);
        direct.skill_hits = vec!["Kubernetes".to_string(), "audit logs".to_string()];

        let mut near_miss = candidate("near_miss");
        near_miss.dense_score = Some(0.86);
        near_miss.bm25_score = Some(3.0);
        near_miss.skill_coverage = Some(0.25);
        near_miss.required_coverage = Some(0.10);
        near_miss.seniority_match = Some(0.40);
        near_miss.reranker_score = Some(0.31);
        near_miss.skill_hits = vec!["Kubernetes".to_string()];

        let ranked = HybridScorer::default().score(&[near_miss, direct]);

        assert_eq!(ranked[0].candidate_id, "direct");
        assert!(ranked[0].score > ranked[1].score);
        assert!(ranked[0]
            .provenance
            .sources
            .contains(&"reranker".to_string()));
    }

    #[test]
    fn hard_blockers_cap_otherwise_strong_scores() {
        let mut blocked = candidate("blocked");
        blocked.dense_score = Some(0.95);
        blocked.bm25_score = Some(10.0);
        blocked.skill_coverage = Some(1.0);
        blocked.required_coverage = Some(1.0);
        blocked.seniority_match = Some(1.0);
        blocked.reranker_score = Some(0.97);
        blocked.blockers = vec![MatchBlocker::MissingClearance];

        let ranked = HybridScorer::default().score(&[blocked]);

        assert_eq!(ranked[0].candidate_id, "blocked");
        assert!(ranked[0].score <= 0.45);
        assert!(ranked[0].blockers.contains(&MatchBlocker::MissingClearance));
    }

    #[test]
    fn provenance_tracks_dense_bm25_skill_and_reranker_sources() {
        let mut result = candidate("result");
        result.dense_score = Some(0.60);
        result.bm25_score = Some(2.0);
        result.skill_coverage = Some(0.50);
        result.required_coverage = Some(0.40);
        result.seniority_match = Some(0.60);
        result.reranker_score = Some(0.70);
        result.skill_hits = vec!["CloudTrail".to_string()];

        let ranked = HybridScorer::default().score(&[result]);
        let provenance = &ranked[0].provenance;

        assert_eq!(provenance.dense_rank, Some(1));
        assert_eq!(provenance.bm25_rank, Some(1));
        assert_eq!(provenance.rerank_rank, Some(1));
        assert_eq!(provenance.skill_hits, vec!["CloudTrail".to_string()]);
        for source in [
            "dense",
            "bm25",
            "skill_exact",
            "required_coverage",
            "seniority",
            "reranker",
        ] {
            assert!(provenance.sources.contains(&source.to_string()));
        }
    }
}
