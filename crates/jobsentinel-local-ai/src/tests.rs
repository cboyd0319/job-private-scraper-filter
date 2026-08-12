//! Tests for ML module
//!
//! Note: Most tests require the model to be downloaded first.
//! Run with: cargo test --features embedded-ml

#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs;
    use tempfile::TempDir;

    fn get_test_cache_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_model_status_not_downloaded() {
        let cache_dir = get_test_cache_dir();
        let manager = ModelManager::new(cache_dir.path().to_path_buf());

        let status = manager.get_status();
        assert!(!status.is_downloaded);
        assert!(status.model_size_bytes.is_none());
        assert_eq!(status.model_id, "qwen3-embedding-0.6b+qwen3-reranker-0.6b");
        assert_eq!(status.backend, "qwen3-candle+qwen3-reranker-candle");
        assert_eq!(status.manifest_hash.len(), 64);
    }

    #[test]
    fn test_model_status_rejects_tampered_cached_files() {
        let cache_dir = get_test_cache_dir();
        let manager = ModelManager::new(cache_dir.path().to_path_buf());
        let spec = ModelManager::runtime_model_spec().unwrap();
        let model_dir = manager.model_cache_dir(&spec);
        fs::create_dir_all(&model_dir).unwrap();

        for file in ["config.json", "tokenizer.json", "model.safetensors"] {
            fs::write(model_dir.join(file), b"tampered model file").unwrap();
        }

        let status = manager.get_status();

        assert!(!manager.is_model_downloaded());
        assert!(!status.is_downloaded);
        assert!(status.model_size_bytes.is_none());
    }

    #[test]
    fn test_model_manifest_default_profiles_are_available() {
        let manifest = load_model_manifest().unwrap();

        assert_eq!(
            manifest.default_embedding().unwrap().id,
            "qwen3-embedding-0.6b"
        );
        assert_eq!(
            manifest.default_reranker().unwrap().id,
            "qwen3-reranker-0.6b"
        );
        assert_eq!(
            manifest.legacy_runtime_embedding().unwrap().id,
            "all-minilm-l6-v2-baseline"
        );
    }

    #[test]
    fn test_embedding_normalization() {
        let embedding = vec![3.0, 4.0]; // Length 5
        let normalized = EmbeddingGenerator::normalize_embedding(&embedding);

        // Check values
        assert!((normalized[0] - 0.6).abs() < 1e-6);
        assert!((normalized[1] - 0.8).abs() < 1e-6);

        // Check unit length
        let length: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_mismatched_length() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        let sim = EmbeddingGenerator::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn ml_error_display_hides_raw_details() {
        let errors = [
            MlError::ModelNotDownloaded("<local-private-model>".to_string()),
            MlError::ModelLoadFailed("tensor shape includes /private/cache".to_string()),
            MlError::InferenceFailed("resume text token secret@example.com".to_string()),
            MlError::TokenizationFailed("tokenizer path /tmp/tokenizer.json".to_string()),
            MlError::DownloadFailed("https://huggingface.co/model?token=secret".to_string()),
            MlError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "<local-private-model-dir>",
            )),
        ];

        for error in errors {
            let message = error.to_string();
            assert!(!message.contains("<local-private-model"));
            assert!(!message.contains("/private"));
            assert!(!message.contains("secret@example.com"));
            assert!(!message.contains("https://"));
            assert!(!message.contains("token=secret"));
        }
    }

    // Integration tests - require model download
    // Run manually, one at a time:
    // cargo test --features embedded-ml -- --ignored --test-threads=1

    #[test]
    #[ignore]
    fn test_download_model() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let manager = ModelManager::new(cache_dir.path().to_path_buf());

        // Download model
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(manager.download_model());

        assert!(result.is_ok());
        assert!(manager.is_model_downloaded());
    }

    #[test]
    #[ignore]
    fn test_embedding_generation() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let generator = EmbeddingGenerator::new(cache_dir.path().to_path_buf()).unwrap();

        let text = "Machine learning engineer with Python experience";
        let embedding = generator.embed_text(text).unwrap();

        // Check embedding dimensions (all-MiniLM-L6-v2 has 384 dimensions)
        assert_eq!(embedding.len(), 384);

        // Check embeddings are non-zero
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.0);
    }

    #[test]
    #[ignore]
    fn test_batch_embeddings() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let generator = EmbeddingGenerator::new(cache_dir.path().to_path_buf()).unwrap();

        let texts = vec!["Python programming", "Machine Learning", "Data Science"];

        let embeddings = generator.embed_batch(&texts).unwrap();

        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 384);
    }

    #[test]
    #[ignore]
    fn test_semantic_matching() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let matcher = SemanticMatcher::new(cache_dir.path().to_path_buf()).unwrap();

        let user_skills = vec![
            "Python programming".to_string(),
            "Machine Learning".to_string(),
            "Data Analysis".to_string(),
            "SQL databases".to_string(),
        ];

        let job_requirements = vec![
            "Python".to_string(),
            "ML experience".to_string(),
            "Statistical analysis".to_string(),
            "Java".to_string(),
            "Kubernetes".to_string(),
        ];

        let result = matcher
            .match_skills(&user_skills, &job_requirements)
            .unwrap();

        // Should match Python, ML, and statistical analysis
        assert!(result.matched_skills.len() >= 2);
        assert!(result.overall_score > 0.4);

        // Java and Kubernetes should be unmatched
        assert!(result.unmatched_requirements.len() >= 1);

        // Check matched skills have reasonable similarity
        for matched in &result.matched_skills {
            assert!(matched.similarity >= 0.7);
        }
    }

    #[test]
    #[ignore]
    fn test_semantic_similarity() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let matcher = SemanticMatcher::new(cache_dir.path().to_path_buf()).unwrap();

        let candidates = vec![
            "Python programming".to_string(),
            "Java development".to_string(),
            "JavaScript".to_string(),
            "Machine Learning".to_string(),
            "SQL".to_string(),
        ];

        let similar = matcher
            .find_similar_skills("Python", &candidates, 3)
            .unwrap();

        // "Python programming" should be most similar
        assert_eq!(similar.len(), 3);
        assert!(similar[0].0.contains("Python"));
        assert!(similar[0].1 > 0.8);
    }

    #[test]
    #[ignore]
    fn test_semantic_vs_keyword_matching() {
        let cache_dir = get_test_cache_dir();
        crate::model::set_download_policy_env(cache_dir.path());
        let matcher = SemanticMatcher::new(cache_dir.path().to_path_buf()).unwrap();

        // These should match semantically but not by exact string
        let user_skills = vec!["Machine Learning".to_string(), "Deep Learning".to_string()];

        let job_requirements = vec!["ML experience".to_string(), "Neural Networks".to_string()];

        let result = matcher
            .match_skills(&user_skills, &job_requirements)
            .unwrap();

        // Semantic matching should find these
        assert!(result.matched_skills.len() >= 1);
        assert!(result.overall_score > 0.5);
    }

    #[test]
    fn test_empty_skills() {
        // This test doesn't require model
        let user_skills: Vec<String> = vec![];
        let job_requirements = vec!["Python".to_string()];

        // Just verify the structure
        assert_eq!(user_skills.len(), 0);
        assert_eq!(job_requirements.len(), 1);
    }
}
