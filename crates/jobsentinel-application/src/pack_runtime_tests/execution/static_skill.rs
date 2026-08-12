//! Proves verified static-skill review and its deterministic Evidence Reviewer handoff.

use super::*;
use crate::pack_runtime::{evaluate_active_static_skill_handoff_pack, open_active_static_skill};

async fn activated_static_skill_pack(
    database: &Database,
) -> (tempfile::TempDir, TrustedPublisherKey, u64) {
    activated_static_skill_pack_with_handoff(
        database,
        json!({
            "task_kind": "evidence_review",
            "label": "Open Resume Evidence Reviewer"
        }),
    )
    .await
}

async fn activated_static_skill_pack_with_handoff(
    database: &Database,
    handoff: serde_json::Value,
) -> (tempfile::TempDir, TrustedPublisherKey, u64) {
    let artifact_root = tempfile::tempdir().unwrap();
    let (publisher, envelope) = signed_static_skill_pack_with_handoff(1, handoff);
    let staged = stage_pack_artifact(
        database,
        artifact_root.path(),
        &envelope,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();
    let active = activate_pack_artifact(
        database,
        artifact_root.path(),
        SKILL_PUBLISHER_ID,
        SKILL_PACK_ID,
        1,
        staged.generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();
    (artifact_root, publisher, active.generation)
}

#[tokio::test]
async fn active_static_skill_reloads_verified_plain_content() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_static_skill_pack(&database).await;

    let skill = open_active_static_skill(
        &database,
        artifact_root.path(),
        SKILL_PUBLISHER_ID,
        SKILL_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(skill.skill_name, "resume-evidence-review");
    assert!(skill.skill_md.contains("## Guardrails"));
    assert_eq!(skill.resources.len(), 1);
    assert_eq!(skill.resources[0].path, "references/rubric.md");
    assert_eq!(
        skill.resources[0].content,
        "# Evidence rubric\n\nUse only confirmed local evidence.\n"
    );
    let handoff = skill.handoff.unwrap();
    assert_eq!(handoff.task_kind, AgentTaskKind::EvidenceReview);
    assert_eq!(handoff.label, "Open Resume Evidence Reviewer");
}

#[tokio::test]
async fn active_static_skill_passes_the_evidence_reviewer_handoff_target() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_static_skill_pack(&database).await;

    let evaluation = evaluate_active_static_skill_handoff_pack(
        &database,
        artifact_root.path(),
        SKILL_PUBLISHER_ID,
        SKILL_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        evaluation.target,
        "static_skill_evidence_reviewer_handoff_v1"
    );
    assert_eq!(evaluation.passed_cases, 1);
    assert_eq!(evaluation.failed_cases, 0);
    assert_eq!(
        serde_json::to_value(&evaluation).unwrap(),
        json!({
            "target": "static_skill_evidence_reviewer_handoff_v1",
            "passedCases": 1,
            "failedCases": 0
        })
    );
}

#[tokio::test]
async fn static_skill_handoff_target_rejects_missing_or_different_reviewed_workflow() {
    for handoff in [
        serde_json::Value::Null,
        json!({ "task_kind": "draft_packet", "label": "Open Packet Builder" }),
    ] {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        let (artifact_root, publisher, generation) =
            activated_static_skill_pack_with_handoff(&database, handoff).await;

        assert!(evaluate_active_static_skill_handoff_pack(
            &database,
            artifact_root.path(),
            SKILL_PUBLISHER_ID,
            SKILL_PACK_ID,
            generation,
            std::slice::from_ref(&publisher),
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn inactive_or_stale_static_skill_cannot_be_opened() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_static_skill_pack(&database).await;
    let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();

    for expected_generation in [generation - 1] {
        assert!(evaluate_active_static_skill_handoff_pack(
            &database,
            artifact_root.path(),
            SKILL_PUBLISHER_ID,
            SKILL_PACK_ID,
            expected_generation,
            std::slice::from_ref(&publisher),
            today,
        )
        .await
        .is_err());
    }
    let disabled = disable_pack_artifact(&database, SKILL_PUBLISHER_ID, SKILL_PACK_ID, generation)
        .await
        .unwrap();
    for expected_generation in [disabled.generation] {
        assert!(open_active_static_skill(
            &database,
            artifact_root.path(),
            SKILL_PUBLISHER_ID,
            SKILL_PACK_ID,
            expected_generation,
            std::slice::from_ref(&publisher),
            today,
        )
        .await
        .is_err());
        assert!(evaluate_active_static_skill_handoff_pack(
            &database,
            artifact_root.path(),
            SKILL_PUBLISHER_ID,
            SKILL_PACK_ID,
            expected_generation,
            std::slice::from_ref(&publisher),
            today,
        )
        .await
        .is_err());
    }
}

#[tokio::test]
async fn altered_active_static_skill_is_quarantined_without_exposing_content() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let (artifact_root, publisher, generation) = activated_static_skill_pack(&database).await;
    std::fs::write(walk_files(artifact_root.path()).pop().unwrap(), b"altered").unwrap();

    assert!(open_active_static_skill(
        &database,
        artifact_root.path(),
        SKILL_PUBLISHER_ID,
        SKILL_PACK_ID,
        generation,
        std::slice::from_ref(&publisher),
        NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
    )
    .await
    .is_err());
    let stream = database
        .get_pack_stream(SKILL_PUBLISHER_ID, SKILL_PACK_ID)
        .await
        .unwrap();
    assert_eq!(stream.availability, PackAvailability::Quarantined);
    assert_eq!(stream.active_release_sequence, None);
}
