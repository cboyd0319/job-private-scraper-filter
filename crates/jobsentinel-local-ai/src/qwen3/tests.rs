use std::path::PathBuf;

use crate::model::ModelManager;
use crate::qwen3::pooling::project_and_normalize;
use crate::qwen3::tokenization::{format_embedding_input, format_rerank_inputs};
use crate::runtime::{
    EmbeddingBackend, EmbeddingInput, EmbeddingInputKind, RerankCandidate, RerankQuery,
    RerankQueryKind, RerankerBackend,
};
use crate::{Qwen3EmbeddingBackend, Qwen3RerankerBackend};

fn embedding_input(kind: EmbeddingInputKind, instruction: Option<&str>) -> EmbeddingInput {
    EmbeddingInput {
        text: "Built customer help content for a complex product.".to_string(),
        instruction: instruction.map(ToString::to_string),
        input_kind: kind,
    }
}

#[test]
fn query_formatting_uses_instruction_prompt() {
    let input = embedding_input(
        EmbeddingInputKind::Requirement,
        Some("Retrieve concrete resume evidence."),
    );
    let formatted = format_embedding_input(&input);

    assert!(formatted.starts_with("Instruct: Retrieve concrete resume evidence."));
    assert!(formatted.contains("\nQuery: Built customer help content"));
}

#[test]
fn document_formatting_does_not_over_prompt() {
    let input = embedding_input(
        EmbeddingInputKind::ResumeChunk,
        Some("Retrieve concrete resume evidence."),
    );

    assert_eq!(
        format_embedding_input(&input),
        "Built customer help content for a complex product."
    );
}

#[test]
fn projection_truncates_and_renormalizes() {
    let projected = project_and_normalize(vec![3.0, 4.0, 12.0], 2).unwrap();
    let norm = projected
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();

    assert_eq!(projected.len(), 2);
    assert!((norm - 1.0).abs() < 1e-6);
    assert!((projected[0] - 0.6).abs() < 1e-6);
    assert!((projected[1] - 0.8).abs() < 1e-6);
}

#[test]
fn rerank_formatting_uses_qwen3_instruction_shape() {
    let query = RerankQuery {
        text: "Experience with Kubernetes security detections".to_string(),
        instruction: Some("Score concrete evidence only.".to_string()),
        query_kind: RerankQueryKind::ResumeRequirement,
    };
    let candidate = RerankCandidate {
        id: "candidate".to_string(),
        text: "Built Kubernetes audit-log detections for suspicious RBAC changes.".to_string(),
        metadata: serde_json::Value::Null,
    };
    let formatted = format_rerank_inputs(&query, &[&candidate]);

    assert_eq!(formatted.len(), 1);
    assert!(formatted[0].starts_with("<Instruct>: Score concrete evidence only."));
    assert!(formatted[0].contains("\n<Query>: Experience with Kubernetes security"));
    assert!(formatted[0].contains("\n<Document>: Built Kubernetes audit-log"));
}

#[test]
fn rerank_default_instruction_is_query_kind_specific() {
    let query = RerankQuery {
        text: "Experience with FedRAMP ATO".to_string(),
        instruction: None,
        query_kind: RerankQueryKind::GapAnalysis,
    };
    let candidate = RerankCandidate {
        id: "candidate".to_string(),
        text: "Supported SOC 2 evidence collection.".to_string(),
        metadata: serde_json::Value::Null,
    };
    let formatted = format_rerank_inputs(&query, &[&candidate]);

    assert!(formatted[0].contains("closes the gap with concrete evidence"));
}

#[test]
#[ignore]
fn qwen3_backend_embeds_with_pinned_downloaded_model() {
    let temp_dir;
    let app_data_dir = if let Some(path) = std::env::var_os("JOBSENTINEL_QWEN3_TEST_CACHE") {
        PathBuf::from(path)
    } else {
        temp_dir = tempfile::tempdir().expect("tempdir should be created");
        temp_dir.path().to_path_buf()
    };

    crate::model::set_download_policy_env(&app_data_dir);
    let manager = ModelManager::new(app_data_dir);
    let spec =
        ModelManager::default_embedding_model_spec().expect("default embedding should exist");
    if !manager.is_model_downloaded_for(&spec) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should start");
        runtime
            .block_on(manager.download_model_by_id(&spec.id))
            .unwrap_or_else(|error| panic!("Qwen3 model should download and verify: {error:#?}"));
    }

    let backend = Qwen3EmbeddingBackend::from_manager(&manager, spec).expect("backend should load");
    let vector = backend
        .embed_query(&EmbeddingInput {
            text: "Experience writing clear help content for customers.".to_string(),
            instruction: Some("Retrieve concrete professional evidence.".to_string()),
            input_kind: EmbeddingInputKind::Requirement,
        })
        .expect("query should embed");
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

    assert_eq!(backend.model_id(), "qwen3-embedding-0.6b");
    assert_eq!(backend.dimension(), 768);
    assert_eq!(vector.len(), 768);
    assert!((norm - 1.0).abs() < 1e-4);
}

#[test]
#[ignore]
fn qwen3_reranker_ranks_direct_evidence_above_near_miss() {
    let temp_dir;
    let app_data_dir = if let Some(path) = std::env::var_os("JOBSENTINEL_QWEN3_TEST_CACHE") {
        PathBuf::from(path)
    } else {
        temp_dir = tempfile::tempdir().expect("tempdir should be created");
        temp_dir.path().to_path_buf()
    };

    crate::model::set_download_policy_env(&app_data_dir);
    let manager = ModelManager::new(app_data_dir);
    let spec = ModelManager::default_reranker_model_spec().expect("reranker should exist");
    if !manager.is_model_downloaded_for(&spec) {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should start");
        runtime
            .block_on(manager.download_model_by_id(&spec.id))
            .unwrap_or_else(|error| {
                panic!("Qwen3 reranker should download and verify: {error:#?}")
            });
    }

    let backend = Qwen3RerankerBackend::from_manager(&manager, spec).expect("reranker should load");
    let scores = backend
        .rerank(
            &RerankQuery {
                text: "Experience building Kubernetes security detections from audit logs."
                    .to_string(),
                instruction: None,
                query_kind: RerankQueryKind::ResumeRequirement,
            },
            &[
                RerankCandidate {
                    id: "direct".to_string(),
                    text: "Built Kubernetes audit-log detections for suspicious RBAC changes and privilege escalation.".to_string(),
                    metadata: serde_json::Value::Null,
                },
                RerankCandidate {
                    id: "near_miss".to_string(),
                    text: "Managed Kubernetes deployments and wrote Helm charts for application releases.".to_string(),
                    metadata: serde_json::Value::Null,
                },
            ],
        )
        .expect("rerank should score candidates");

    assert_eq!(backend.model_id(), "qwen3-reranker-0.6b");
    assert_eq!(scores.len(), 2);
    assert_eq!(scores[0].candidate_id, "direct");
    assert!(scores[0].score > scores[1].score);
}
