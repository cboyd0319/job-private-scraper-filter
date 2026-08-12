use super::*;

#[cfg(not(feature = "embedded-ml"))]
#[test]
fn diagnostics_explain_disabled_build_without_private_data() {
    let diagnostics = semantic_matching_diagnostics().expect("diagnostics should build");

    assert!(!diagnostics.build_enabled);
    assert!(matches!(
        diagnostics.runtime_status,
        SemanticMatchingRuntimeStatus::DisabledInThisBuild
    ));
    assert!(diagnostics.models.is_empty());
    assert!(diagnostics.privacy_mode.contains("No resume or job text"));
}

#[cfg(feature = "embedded-ml")]
#[test]
fn diagnostics_report_qwen3_model_lock_entries() {
    let diagnostics = semantic_matching_diagnostics().expect("diagnostics should build");

    assert!(diagnostics.build_enabled);
    assert_eq!(diagnostics.manifest_hash.as_deref().map(str::len), Some(64));
    assert!(diagnostics
        .models
        .iter()
        .any(|model| model.id == "qwen3-embedding-0.6b"
            && model.role == "Default embedding"
            && matches!(
                model.health,
                ModelCacheHealth::Missing | ModelCacheHealth::Ready
            )
            && model.required_for_qwen3_runtime));
    assert!(diagnostics
        .models
        .iter()
        .any(|model| model.id == "qwen3-reranker-0.6b"
            && model.role == "Default reranker"
            && model.required_for_qwen3_runtime));
    assert!(diagnostics
        .scoring_signals
        .iter()
        .any(|signal| signal.id == "qwen3_reranker"));
}

#[cfg(feature = "embedded-ml")]
#[test]
fn qwen3_runtime_status_distinguishes_absent_damaged_and_ready_caches() {
    assert_eq!(
        semantic_runtime_status(
            ModelCacheHealth::Missing,
            ModelCacheHealth::Missing,
            ModelCacheHealth::Missing,
        ),
        SemanticMatchingRuntimeStatus::NeedsModelDownload
    );
    assert_eq!(
        semantic_runtime_status(
            ModelCacheHealth::Incomplete,
            ModelCacheHealth::Missing,
            ModelCacheHealth::Missing,
        ),
        SemanticMatchingRuntimeStatus::Misconfigured
    );
    assert_eq!(
        semantic_runtime_status(
            ModelCacheHealth::IntegrityMismatch,
            ModelCacheHealth::Ready,
            ModelCacheHealth::Ready,
        ),
        SemanticMatchingRuntimeStatus::Misconfigured
    );
    assert_eq!(
        semantic_runtime_status(
            ModelCacheHealth::Ready,
            ModelCacheHealth::Ready,
            ModelCacheHealth::IntegrityMismatch,
        ),
        SemanticMatchingRuntimeStatus::Ready
    );
    assert_eq!(
        semantic_runtime_status(
            ModelCacheHealth::Missing,
            ModelCacheHealth::Missing,
            ModelCacheHealth::IntegrityMismatch,
        ),
        SemanticMatchingRuntimeStatus::Misconfigured
    );
}

#[cfg(feature = "embedded-ml")]
#[test]
fn runtime_copy_names_verified_minilm_instead_of_exact_only_fallback() {
    let (active_profile, user_action) = semantic_runtime_copy(
        SemanticMatchingRuntimeStatus::Misconfigured,
        ModelCacheHealth::Ready,
    );

    assert!(active_profile.contains("MiniLM"));
    assert!(user_action.unwrap().contains("MiniLM"));
    assert!(!active_profile.contains("Exact-only"));
}

#[cfg(feature = "embedded-ml")]
#[test]
fn partial_cache_diagnostics_expose_no_path_or_file_contents() {
    let app_data_dir = tempfile::tempdir().unwrap();
    let manifest = load_model_manifest().unwrap();
    let model = manifest.default_embedding().unwrap();
    let file = model.required_files().next().unwrap();
    let model_file = app_data_dir
        .path()
        .join("ml_models")
        .join(&model.id)
        .join(&model.revision)
        .join(model_lock_hash())
        .join(&file.path);
    std::fs::create_dir_all(model_file.parent().unwrap()).unwrap();
    std::fs::write(&model_file, b"private partial model contents").unwrap();

    let diagnostics = semantic_matching_diagnostics_for(
        &manifest,
        &ModelManager::new(app_data_dir.path().to_path_buf()),
    );
    let serialized = serde_json::to_string(&diagnostics).unwrap();

    assert_eq!(
        diagnostics.runtime_status,
        SemanticMatchingRuntimeStatus::Misconfigured
    );
    assert_eq!(
        diagnostics
            .models
            .iter()
            .find(|diagnostic| diagnostic.id == model.id)
            .unwrap()
            .health,
        ModelCacheHealth::IntegrityMismatch
    );
    assert!(!serialized.contains(&app_data_dir.path().display().to_string()));
    assert!(!serialized.contains("private partial model contents"));
}
