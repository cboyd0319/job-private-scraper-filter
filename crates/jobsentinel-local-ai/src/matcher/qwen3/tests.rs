use super::*;
use crate::RerankScore;

mod calibration;

#[test]
fn reranker_selection_preserves_dense_score_and_typed_provenance() {
    let selected = select_reranked_match(
        &[(2, 0.72), (5, 0.88)],
        &[score("5", 3.4, 1), score("2", -1.2, 2)],
    )
    .unwrap()
    .unwrap();

    assert_eq!(selected.user_index, 5);
    assert_eq!(selected.dense_score, 0.88);
    assert_eq!(selected.reranker_score, 3.4);
    assert_eq!(selected.reranker_rank, 1);
    let serialized = serde_json::to_value(SkillMatch {
        job_skill: "Kubernetes security".to_string(),
        user_skill: "Kubernetes audit detections".to_string(),
        similarity: selected.dense_score,
        reranker_score: Some(selected.reranker_score),
        reranker_rank: Some(selected.reranker_rank),
    })
    .unwrap();
    let fields = serialized.as_object().unwrap();
    assert_eq!(fields.len(), 5);
    for field in [
        "job_skill",
        "user_skill",
        "similarity",
        "reranker_score",
        "reranker_rank",
    ] {
        assert!(fields.contains_key(field));
    }
}

#[test]
fn reranker_selection_rejects_untrusted_or_inconsistent_output() {
    let dense = [(2, 0.72), (5, 0.88)];
    for scores in [
        vec![score("9", 3.4, 1), score("2", -1.2, 2)],
        vec![score("2", 3.4, 1), score("2", -1.2, 2)],
        vec![score("5", f32::NAN, 1), score("2", -1.2, 2)],
        vec![score("5", 3.4, 2), score("2", -1.2, 1)],
        vec![score("5", -1.2, 1), score("2", 3.4, 2)],
        vec![score("5", 3.4, 1)],
    ] {
        assert!(select_reranked_match(&dense, &scores).is_err());
    }
    for dense_score in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.1, 1.1] {
        assert!(select_reranked_match(&[(2, dense_score)], &[score("2", 3.4, 1)]).is_err());
    }
    assert!(select_reranked_match(&[(2, 0.8), (2, 0.7)], &[]).is_err());
}

#[test]
fn reranker_acceptance_abstains_below_calibrated_minimum() {
    let accepted = RerankedMatch {
        user_index: 0,
        dense_score: 0.42,
        reranker_score: 5.24,
        reranker_rank: 1,
    };
    let rejected = RerankedMatch {
        user_index: 1,
        dense_score: 0.48,
        reranker_score: 1.25,
        reranker_rank: 1,
    };

    assert_eq!(
        apply_reranker_acceptance(Some(accepted), 3.0)
            .unwrap()
            .unwrap()
            .user_index,
        0
    );
    assert!(apply_reranker_acceptance(Some(rejected), 3.0)
        .unwrap()
        .is_none());
    assert!(apply_reranker_acceptance(None, 3.0).unwrap().is_none());
    assert!(apply_reranker_acceptance(None, f32::NAN).is_err());
}

#[test]
fn qwen_abstention_reasons_distinguish_retrieval_from_acceptance() {
    let selected = RerankedMatch {
        user_index: 0,
        dense_score: 0.42,
        reranker_score: 5.24,
        reranker_rank: 1,
    };

    assert_eq!(
        qwen_unmatched_reason(&[], None),
        Some(SemanticUnmatchedReason::BelowRetrievalThreshold)
    );
    assert_eq!(
        qwen_unmatched_reason(&[(0, 0.42)], None),
        Some(SemanticUnmatchedReason::BelowRerankerAcceptance)
    );
    assert_eq!(qwen_unmatched_reason(&[(0, 0.42)], Some(&selected)), None);
}

fn score(candidate_id: &str, score: f32, rank: usize) -> RerankScore {
    RerankScore {
        candidate_id: candidate_id.to_string(),
        score,
        rank,
    }
}
