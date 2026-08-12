use super::*;
use crate::test_support::test_job;
use jobsentinel_domain::{
    v3_foundation::{
        CaseFileEventInput, CaseFileEventKind, EventMetadata, EventOrigin, GraphProvenance,
    },
    v3_manifests::PrivacyLabel,
    ResumeEvidenceCitation, ResumeEvidenceSnapshot,
};
use jobsentinel_storage::Database;
use uuid::Uuid;

fn user_confirmation(case_file_id: &str, evidence_id: &str) -> CaseFileEventInput {
    CaseFileEventInput {
        case_file_id: case_file_id.to_string(),
        kind: CaseFileEventKind::EvidenceLinked,
        origin: EventOrigin::User,
        user_action: true,
        privacy_labels: [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        metadata: EventMetadata::LocalReference {
            reference_id: evidence_id.to_string(),
        },
    }
}

#[tokio::test]
async fn application_requires_explicit_exact_resume_evidence_confirmation() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-1", "Office Assistant", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-1").await.unwrap();
    let snapshot = ResumeEvidenceSnapshot {
        source_id: "resume-1".to_string(),
        revision: "revision-1".to_string(),
    };
    let citation =
        ResumeEvidenceCitation::for_field(&snapshot, "experience.0.achievements.0").unwrap();
    let confirmation = user_confirmation(&case_file.case_file_id, &citation.evidence_id);

    assert_eq!(
        record_case_file_event(&database, &confirmation)
            .await
            .unwrap_err(),
        FoundationError::InvalidInput
    );
    assert!(database
        .list_case_file_events(&case_file.case_file_id)
        .await
        .unwrap()
        .is_empty());
    assert!(
        confirm_resume_evidence_for_case(&database, &snapshot, &citation, &confirmation)
            .await
            .unwrap()
    );
    let links = read_case_file_evidence_links(&database, &case_file.case_file_id)
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].object_id, citation.evidence_id);
    assert_eq!(links[0].provenance, GraphProvenance::UserConfirmed);

    let mut tampered = citation.clone();
    tampered.field_path = "summary".to_string();
    assert_eq!(
        confirm_resume_evidence_for_case(&database, &snapshot, &tampered, &confirmation)
            .await
            .unwrap_err(),
        FoundationError::InvalidInput
    );

    let edited_snapshot = ResumeEvidenceSnapshot {
        source_id: snapshot.source_id.clone(),
        revision: "revision-2".to_string(),
    };
    let edited_citation =
        ResumeEvidenceCitation::for_field(&edited_snapshot, &citation.field_path).unwrap();
    assert_eq!(
        confirm_resume_evidence_for_case(&database, &edited_snapshot, &citation, &confirmation)
            .await
            .unwrap_err(),
        FoundationError::InvalidInput
    );
    assert!(links
        .iter()
        .all(|link| link.object_id != edited_citation.evidence_id));

    let mut system_event = user_confirmation(&case_file.case_file_id, &edited_citation.evidence_id);
    system_event.origin = EventOrigin::System;
    system_event.user_action = false;
    assert_eq!(
        confirm_resume_evidence_for_case(
            &database,
            &edited_snapshot,
            &edited_citation,
            &system_event,
        )
        .await
        .err()
        .unwrap(),
        FoundationError::InvalidInput
    );
}

pub(super) async fn saved_snapshot(database: &Database) -> ResumeEvidenceSnapshot {
    saved_snapshot_with_text(database, "Customer service and scheduling").await
}

pub(super) async fn saved_snapshot_with_text(
    database: &Database,
    text: &str,
) -> ResumeEvidenceSnapshot {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("resume.txt");
    std::fs::write(&path, text).unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .get_resume_evidence_snapshot(resume_id)
        .await
        .unwrap()
        .unwrap()
}

pub(super) async fn confirm_field(
    database: &Database,
    case_file_id: &str,
    snapshot: &ResumeEvidenceSnapshot,
    field_path: &str,
) -> ResumeEvidenceCitation {
    let citation = ResumeEvidenceCitation::for_field(snapshot, field_path).unwrap();
    confirm_resume_evidence_for_case(
        database,
        snapshot,
        &citation,
        &user_confirmation(case_file_id, &citation.evidence_id),
    )
    .await
    .unwrap();
    citation
}
#[tokio::test]
async fn packet_claim_uses_only_current_user_confirmed_local_evidence() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-packet", "Office Assistant", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-packet")
        .await
        .unwrap();
    let snapshot = saved_snapshot(&database).await;
    let experience = confirm_field(
        &database,
        &case_file.case_file_id,
        &snapshot,
        "experience.0.achievements.0",
    )
    .await;
    let clearance = confirm_field(&database, &case_file.case_file_id, &snapshot, "clearance").await;
    let military = confirm_field(
        &database,
        &case_file.case_file_id,
        &snapshot,
        "military_info",
    )
    .await;
    let before_links = read_case_file_evidence_links(&database, &case_file.case_file_id)
        .await
        .unwrap()
        .len();
    let before_events = database
        .list_case_file_events(&case_file.case_file_id)
        .await
        .unwrap()
        .len();
    let claim_id = Uuid::new_v4().to_string();
    let text = "Led a twelve-person operations team.".to_string();
    let claim = prepare_evidence_bound_packet_claim(
        &database,
        &case_file.case_file_id,
        &claim_id,
        text.clone(),
        &snapshot,
        &[experience.clone(), clearance.clone(), military.clone()],
    )
    .await
    .unwrap();
    assert_eq!(claim.case_file_id(), case_file.case_file_id);
    assert_eq!(claim.claim_id(), claim_id);
    assert_eq!(claim.text(), text);
    assert_eq!(
        claim.evidence_ids(),
        [
            experience.evidence_id,
            clearance.evidence_id,
            military.evidence_id
        ]
    );
    assert_eq!(
        claim.boundaries(),
        [
            PacketEvidenceBoundary::ClearanceCurrentnessUnverified,
            PacketEvidenceBoundary::MilitaryCivilianEquivalenceUnverified,
        ]
    );
    assert_eq!(
        read_case_file_evidence_links(&database, &case_file.case_file_id)
            .await
            .unwrap()
            .len(),
        before_links
    );
    assert_eq!(
        database
            .list_case_file_events(&case_file.case_file_id)
            .await
            .unwrap()
            .len(),
        before_events
    );
}

#[tokio::test]
async fn packet_claim_rejects_stale_draft_tampered_and_unconfirmed_evidence() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-guards", "Office Assistant", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-guards")
        .await
        .unwrap();
    let snapshot = saved_snapshot(&database).await;
    let confirmed = confirm_field(
        &database,
        &case_file.case_file_id,
        &snapshot,
        "experience.0.achievements.0",
    )
    .await;
    let claim_id = Uuid::new_v4().to_string();
    database
        .insert_job_if_new(&test_job("job-other", "Other role", "Example"))
        .await
        .unwrap();
    let other_case = create_or_reuse_case_file(&database, "job-other")
        .await
        .unwrap();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &other_case.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &snapshot,
            &[confirmed.clone()],
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );
    let mut tampered = confirmed.clone();
    tampered.field_path = "summary".to_string();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &case_file.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &snapshot,
            &[tampered],
        )
        .await
        .err()
        .unwrap(),
        FoundationError::InvalidInput
    );

    let unconfirmed = ResumeEvidenceCitation::for_field(&snapshot, "summary").unwrap();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &case_file.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &snapshot,
            &[unconfirmed],
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );

    let resume_id = snapshot
        .source_id
        .strip_prefix("resume:")
        .unwrap()
        .parse::<i64>()
        .unwrap();
    database
        .resume_matcher()
        .add_user_skill(
            resume_id,
            jobsentinel_storage::resume::NewSkill {
                skill_name: "Case management".to_string(),
                ..jobsentinel_storage::resume::NewSkill::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &case_file.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &snapshot,
            &[confirmed.clone()],
        )
        .await
        .err()
        .unwrap(),
        FoundationError::Conflict
    );

    let draft = ResumeEvidenceSnapshot {
        source_id: format!("resume-draft:{resume_id}"),
        revision: snapshot.revision.clone(),
    };
    let draft_citation = ResumeEvidenceCitation::for_field(&draft, "summary").unwrap();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &case_file.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &draft,
            &[draft_citation],
        )
        .await
        .err()
        .unwrap(),
        FoundationError::InvalidInput
    );
}

#[tokio::test]
async fn packet_claim_rejects_invalid_identity_text_and_citation_sets() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    database
        .insert_job_if_new(&test_job("job-inputs", "Office Assistant", "Example"))
        .await
        .unwrap();
    let case_file = create_or_reuse_case_file(&database, "job-inputs")
        .await
        .unwrap();
    let snapshot = saved_snapshot(&database).await;
    let citation = confirm_field(
        &database,
        &case_file.case_file_id,
        &snapshot,
        "experience.0.achievements.0",
    )
    .await;
    let claim_id = Uuid::new_v4().to_string();

    for (id, text, citations) in [
        ("not-a-uuid", "Claim".to_string(), vec![citation.clone()]),
        (&claim_id, " \n ".to_string(), vec![citation.clone()]),
        (&claim_id, "x".repeat(8_193), vec![citation.clone()]),
        (&claim_id, "Claim".to_string(), Vec::new()),
        (
            &claim_id,
            "Claim".to_string(),
            vec![citation.clone(), citation.clone()],
        ),
    ] {
        assert_eq!(
            prepare_evidence_bound_packet_claim(
                &database,
                &case_file.case_file_id,
                id,
                text,
                &snapshot,
                &citations,
            )
            .await
            .err()
            .unwrap(),
            FoundationError::InvalidInput
        );
    }

    let excessive = (0..33)
        .map(|index| {
            ResumeEvidenceCitation::for_field(&snapshot, &format!("skills.{index}")).unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        prepare_evidence_bound_packet_claim(
            &database,
            &case_file.case_file_id,
            &claim_id,
            "Claim".to_string(),
            &snapshot,
            &excessive,
        )
        .await
        .err()
        .unwrap(),
        FoundationError::InvalidInput
    );
}
