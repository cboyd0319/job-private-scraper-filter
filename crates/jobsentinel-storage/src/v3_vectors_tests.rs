use super::delete_if_unchanged;
use jobsentinel_domain::{
    v3_contracts::SchemaId,
    v3_manifests::{ModelProvenance, VectorFreshness},
};

use crate::{test_support::migrated_database, v3_vectors::StoredVectorRead};

const CONTENT_HASH: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const MANIFEST_HASH: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn model() -> ModelProvenance {
    ModelProvenance {
        schema: SchemaId::ModelProvenanceV1,
        model_id: "local-embedding".to_string(),
        revision: "1".to_string(),
        backend: "qwen3-candle".to_string(),
        dimension: 3,
        tokenizer_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .to_string(),
        manifest_sha256: MANIFEST_HASH.to_string(),
        instruction_profile: "resume-job-v1".to_string(),
        pooling: "mean".to_string(),
        normalization: "l2".to_string(),
    }
}

fn freshness(model: &ModelProvenance) -> VectorFreshness {
    VectorFreshness {
        schema: SchemaId::VectorFreshnessV1,
        model_id: model.model_id.clone(),
        model_revision: model.revision.clone(),
        backend: model.backend.clone(),
        dimension: model.dimension,
        instruction_profile: model.instruction_profile.clone(),
        chunker_version: "chunker-v1".to_string(),
        normalizer_version: "normalizer-v1".to_string(),
        pooling: model.pooling.clone(),
        normalization: model.normalization.clone(),
        model_manifest_sha256: model.manifest_sha256.clone(),
        content_sha256: CONTENT_HASH.to_string(),
    }
}

#[tokio::test]
async fn exact_current_vector_round_trips_locally() {
    let database = migrated_database().await;
    let model = model();
    let freshness = freshness(&model);

    database
        .store_v3_vector("resume:one", &freshness, &model, &[0.25, -0.5, 1.0])
        .await
        .unwrap();

    assert_eq!(
        database
            .read_v3_vector("resume:one", &freshness, Some(&model))
            .await
            .unwrap(),
        StoredVectorRead::Ready(vec![0.25, -0.5, 1.0])
    );
}

#[tokio::test]
async fn stale_or_missing_model_vectors_are_removed_without_exposure() {
    let database = migrated_database().await;
    let original_model = model();
    let original = freshness(&original_model);

    let mut cases = Vec::new();
    let mut changed = original.clone();
    changed.content_sha256 =
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string();
    cases.push((changed, original_model.clone()));

    let mut changed_model = original_model.clone();
    changed_model.revision = "2".to_string();
    cases.push((freshness(&changed_model), changed_model));

    let mut changed = original.clone();
    changed.chunker_version = "chunker-v2".to_string();
    cases.push((changed, original_model.clone()));

    let mut changed = original.clone();
    changed.normalizer_version = "normalizer-v2".to_string();
    cases.push((changed, original_model.clone()));

    let mut changed_model = original_model.clone();
    changed_model.instruction_profile = "resume-job-v2".to_string();
    cases.push((freshness(&changed_model), changed_model));

    let mut changed_model = original_model.clone();
    changed_model.manifest_sha256 =
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
    cases.push((freshness(&changed_model), changed_model));

    let mut changed_model = original_model.clone();
    changed_model.dimension = 2;
    cases.push((freshness(&changed_model), changed_model));

    for (index, (current, current_model)) in cases.iter().enumerate() {
        let subject = format!("resume:stale:{index}");
        database
            .store_v3_vector(&subject, &original, &original_model, &[0.25, -0.5, 1.0])
            .await
            .unwrap();
        assert_eq!(
            database
                .read_v3_vector(&subject, current, Some(current_model))
                .await
                .unwrap(),
            StoredVectorRead::RebuildNeeded
        );
        assert_eq!(
            database
                .read_v3_vector(&subject, current, Some(current_model))
                .await
                .unwrap(),
            StoredVectorRead::Missing
        );
    }

    database
        .store_v3_vector(
            "resume:missing-model",
            &original,
            &original_model,
            &[0.25, -0.5, 1.0],
        )
        .await
        .unwrap();
    assert_eq!(
        database
            .read_v3_vector("resume:missing-model", &original, None)
            .await
            .unwrap(),
        StoredVectorRead::RebuildNeeded
    );
    assert_eq!(
        database
            .read_v3_vector("resume:missing-model", &original, None)
            .await
            .unwrap(),
        StoredVectorRead::Missing
    );
}

#[tokio::test]
async fn malformed_dimension_and_non_finite_rows_are_removed() {
    let database = migrated_database().await;
    let model = model();
    let current = freshness(&model);
    let valid_json = serde_json::to_string(&current).unwrap();

    for (subject, metadata, dimension, blob) in [
        ("resume:bad-json", "{{", 3_i64, vec![0_u8; 12]),
        (
            "resume:bad-dimension",
            valid_json.as_str(),
            2,
            vec![0_u8; 8],
        ),
        (
            "resume:non-finite",
            valid_json.as_str(),
            3,
            [f32::NAN, 0.0, 1.0]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        ),
    ] {
        sqlx::query(
            "INSERT INTO v3_local_vectors
             (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
             VALUES (?, ?, ?, ?, 1, '2026-07-20T00:00:00Z')",
        )
        .bind(subject)
        .bind(metadata)
        .bind(dimension)
        .bind(blob)
        .execute(database.pool())
        .await
        .unwrap();

        assert_eq!(
            database
                .read_v3_vector(subject, &current, Some(&model))
                .await
                .unwrap(),
            StoredVectorRead::RebuildNeeded
        );
        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM v3_local_vectors WHERE subject_id = ?")
                .bind(subject)
                .fetch_one(database.pool())
                .await
                .unwrap();
        assert_eq!(remaining, 0);
    }

    sqlx::query(
        "INSERT INTO v3_local_vectors
         (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
         VALUES (
             'resume:invalid-utf8',
             CAST(X'80FF' AS TEXT),
             3,
             X'000000000000000000000000',
             1,
             '2026-07-20T00:00:00Z'
         )",
    )
    .execute(database.pool())
    .await
    .unwrap();
    assert_eq!(
        database
            .read_v3_vector("resume:invalid-utf8", &current, Some(&model))
            .await
            .unwrap(),
        StoredVectorRead::RebuildNeeded
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM v3_local_vectors WHERE subject_id = ?")
            .bind("resume:invalid-utf8")
            .fetch_one(database.pool())
            .await
            .unwrap();
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn vector_writes_are_bounded_and_finite() {
    let database = migrated_database().await;
    let mut model = model();
    let mut current = freshness(&model);

    for values in [vec![], vec![f32::INFINITY; 3]] {
        assert!(database
            .store_v3_vector("resume:invalid", &current, &model, &values)
            .await
            .is_err());
    }

    model.dimension = 4_097;
    current = freshness(&model);
    assert!(database
        .store_v3_vector("resume:too-large", &current, &model, &vec![0.0; 4_097],)
        .await
        .is_err());

    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v3_local_vectors")
        .fetch_one(database.pool())
        .await
        .unwrap();
    assert_eq!(stored, 0);
}

#[tokio::test]
async fn storage_constraints_reject_non_integer_shape_and_revision() {
    let database = migrated_database().await;
    let current = freshness(&model());
    let metadata = serde_json::to_string(&current).unwrap();

    for (subject, dimension, blob_size, revision) in [
        ("resume:fractional-dimension", 3.5_f64, 14_usize, 1.0_f64),
        ("resume:fractional-revision", 3.0, 12, 1.5),
    ] {
        assert!(sqlx::query(
            "INSERT INTO v3_local_vectors
             (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
             VALUES (?, ?, ?, ?, ?, '2026-07-20T00:00:00Z')",
        )
        .bind(subject)
        .bind(&metadata)
        .bind(dimension)
        .bind(vec![0_u8; blob_size])
        .bind(revision)
        .execute(database.pool())
        .await
        .is_err());
    }
}

#[tokio::test]
async fn stale_cleanup_preserves_an_aba_replacement() {
    let database = migrated_database().await;
    let original_model = model();
    let original = freshness(&original_model);
    database
        .store_v3_vector("resume:aba", &original, &original_model, &[0.25, -0.5, 1.0])
        .await
        .unwrap();
    let selected: (Vec<u8>, i64, Vec<u8>, i64) = sqlx::query_as(
        "SELECT CAST(freshness_json AS BLOB), dimension, vector_blob, revision
         FROM v3_local_vectors WHERE subject_id = 'resume:aba'",
    )
    .fetch_one(database.pool())
    .await
    .unwrap();

    sqlx::query("DELETE FROM v3_local_vectors WHERE subject_id = 'resume:aba'")
        .execute(database.pool())
        .await
        .unwrap();
    let mut replacement = original.clone();
    replacement.content_sha256 =
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string();
    database
        .store_v3_vector(
            "resume:aba",
            &replacement,
            &original_model,
            &[1.0, 0.5, -0.25],
        )
        .await
        .unwrap();

    delete_if_unchanged(&database, "resume:aba", &selected)
        .await
        .unwrap();
    assert_eq!(
        database
            .read_v3_vector("resume:aba", &replacement, Some(&original_model))
            .await
            .unwrap(),
        StoredVectorRead::Ready(vec![1.0, 0.5, -0.25])
    );
}

async fn seed_resume_vector(database: &crate::Database, resume_id: i64) {
    sqlx::query(
        "INSERT INTO v3_local_vectors
         (subject_id, freshness_json, dimension, vector_blob, revision, updated_at)
         VALUES (?, '{}', 1, X'00000000', 1, '2026-07-20T00:00:00Z')",
    )
    .bind(format!("resume:{resume_id}"))
    .execute(database.pool())
    .await
    .unwrap();
}

async fn resume_vector_count(database: &crate::Database, resume_id: i64) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM v3_local_vectors WHERE subject_id = ?")
        .bind(format!("resume:{resume_id}"))
        .fetch_one(database.pool())
        .await
        .unwrap()
}

#[tokio::test]
async fn resume_skill_mutations_invalidate_the_cached_vector() {
    let database = migrated_database().await;
    let matcher = database.resume_matcher();
    let resume_id =
        sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
            .execute(database.pool())
            .await
            .unwrap()
            .last_insert_rowid();

    seed_resume_vector(&database, resume_id).await;
    matcher
        .add_user_skill(
            resume_id,
            crate::resume::NewSkill {
                skill_name: "Rust".to_string(),
                ..crate::resume::NewSkill::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(resume_vector_count(&database, resume_id).await, 0);
}

#[tokio::test]
async fn deleting_a_resume_removes_its_cached_vector() {
    let database = migrated_database().await;
    let matcher = database.resume_matcher();
    let resume_id =
        sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
            .execute(database.pool())
            .await
            .unwrap()
            .last_insert_rowid();
    seed_resume_vector(&database, resume_id).await;

    matcher.delete_resume(resume_id).await.unwrap();

    assert_eq!(resume_vector_count(&database, resume_id).await, 0);
}

#[tokio::test]
async fn late_vector_write_cannot_recreate_a_deleted_resume_owner() {
    let database = migrated_database().await;
    let matcher = database.resume_matcher();
    let resume_id =
        sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
            .execute(database.pool())
            .await
            .unwrap()
            .last_insert_rowid();
    let snapshot = matcher
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap();
    matcher.delete_resume(resume_id).await.unwrap();
    let model = model();

    database
        .store_v3_resume_vector_if_current(
            resume_id,
            &snapshot,
            &freshness(&model),
            &model,
            &[0.25, -0.5, 1.0],
        )
        .await
        .unwrap();

    assert_eq!(resume_vector_count(&database, resume_id).await, 0);
}

#[tokio::test]
async fn late_vector_write_cannot_cross_a_resume_revision_change() {
    let database = migrated_database().await;
    let matcher = database.resume_matcher();
    let resume_id =
        sqlx::query("INSERT INTO resumes (name, file_path) VALUES ('Resume', 'resume.txt')")
            .execute(database.pool())
            .await
            .unwrap()
            .last_insert_rowid();
    let snapshot = matcher
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap();
    matcher
        .add_user_skill(
            resume_id,
            crate::resume::NewSkill {
                skill_name: "Rust".to_string(),
                ..crate::resume::NewSkill::default()
            },
        )
        .await
        .unwrap();
    let model = model();

    database
        .store_v3_resume_vector_if_current(
            resume_id,
            &snapshot,
            &freshness(&model),
            &model,
            &[0.25, -0.5, 1.0],
        )
        .await
        .unwrap();

    assert_eq!(resume_vector_count(&database, resume_id).await, 0);
}
