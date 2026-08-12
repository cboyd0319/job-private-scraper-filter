use super::resume_military_transition_commands::{
    is_exact_military_transition_review_token, validate_military_transition_prepare_args,
    MilitaryTransitionWordingInput, MilitaryWordingMappingInput,
};
use crate::application::v3_foundation::{
    confirm_pending_saved_match_military_transition_review, confirm_saved_match_military_evidence,
    prepare_pending_saved_match_military_transition_review, MilitaryBranch,
    MilitaryTransitionWording, MilitaryWordingMapping, PendingMilitaryTransitionReviews,
    SavedMatchMilitaryEvidenceKind, SavedMatchMilitaryTransitionConfirmation,
};
use chrono::Utc;
use jobsentinel_application::Job;

fn military_transition_wording_input() -> MilitaryTransitionWordingInput {
    MilitaryTransitionWordingInput {
        occupation_code: "25B".to_string(),
        civilian_role: "Technical support specialist".to_string(),
        responsibility_mappings: vec![MilitaryWordingMappingInput {
            military_evidence: "Configured tactical networks".to_string(),
            civilian_wording: "Maintained secure network services".to_string(),
        }],
        credential_mappings: Vec::new(),
        current_clearance: Some("Secret".to_string()),
    }
}

fn application_wording() -> MilitaryTransitionWording {
    MilitaryTransitionWording {
        occupation_code: "25B".to_string(),
        civilian_role: "Technical support specialist".to_string(),
        responsibility_mappings: vec![MilitaryWordingMapping {
            military_evidence: "Configured tactical networks".to_string(),
            civilian_wording: "Maintained secure network services".to_string(),
        }],
        credential_mappings: vec![MilitaryWordingMapping {
            military_evidence: "CompTIA Security+".to_string(),
            civilian_wording: "CompTIA Security+".to_string(),
        }],
        current_clearance: Some("Secret".to_string()),
    }
}

#[test]
fn military_transition_command_inputs_are_closed_and_renderer_safe() {
    assert_eq!(
        serde_json::from_str::<SavedMatchMilitaryEvidenceKind>("\"military_service\"").unwrap(),
        SavedMatchMilitaryEvidenceKind::MilitaryService
    );
    assert!(serde_json::from_str::<SavedMatchMilitaryEvidenceKind>("\"other\"").is_err());

    let wording = serde_json::from_str::<MilitaryTransitionWordingInput>(
        r#"{
            "occupationCode":"25B",
            "civilianRole":"Technical support specialist",
            "responsibilityMappings":[{
                "militaryEvidence":"Configured tactical networks",
                "civilianWording":"Maintained secure network services"
            }],
            "credentialMappings":[],
            "currentClearance":"Secret"
        }"#,
    )
    .unwrap();
    assert_eq!(wording.occupation_code, "25B");
    assert!(serde_json::from_str::<MilitaryTransitionWordingInput>(
        r#"{
            "occupationCode":"25B",
            "civilianRole":"Technical support specialist",
            "responsibilityMappings":[],
            "credentialMappings":[],
            "currentClearance":null,
            "caseFileId":"must not cross IPC"
        }"#,
    )
    .is_err());
    assert!(serde_json::from_str::<MilitaryTransitionWordingInput>(
        r#"{
            "occupation_code":"25B",
            "civilian_role":"Technical support specialist",
            "responsibility_mappings":[],
            "credential_mappings":[],
            "current_clearance":null
        }"#,
    )
    .is_err());
    fn assert_serializable<T: serde::Serialize>() {}
    assert_serializable::<SavedMatchMilitaryTransitionConfirmation>();
}

#[test]
fn military_transition_command_validates_exact_saved_match_and_review_boundaries() {
    let wording = military_transition_wording_input();
    assert_eq!(
        validate_military_transition_prepare_args("saved-job", 7, &wording),
        Ok(())
    );
    assert!(validate_military_transition_prepare_args("", 7, &wording).is_err());
    assert!(validate_military_transition_prepare_args("saved-job", 0, &wording).is_err());
    assert!(validate_military_transition_prepare_args("bad\u{0000}job", 7, &wording).is_err());
    assert!(validate_military_transition_prepare_args(&"a".repeat(129), 7, &wording).is_err());
    assert!(validate_military_transition_prepare_args(
        "saved-job",
        7,
        &MilitaryTransitionWordingInput {
            civilian_role: " role".to_string(),
            ..military_transition_wording_input()
        },
    )
    .is_err());
    assert!(validate_military_transition_prepare_args(
        "saved-job",
        7,
        &MilitaryTransitionWordingInput {
            responsibility_mappings: vec![
                MilitaryWordingMappingInput {
                    military_evidence: "evidence".to_string(),
                    civilian_wording: "wording".to_string(),
                };
                17
            ],
            ..military_transition_wording_input()
        },
    )
    .is_err());
    assert!(validate_military_transition_prepare_args(
        "saved-job",
        7,
        &MilitaryTransitionWordingInput {
            civilian_role: "é".repeat(257),
            ..military_transition_wording_input()
        },
    )
    .is_err());
    assert_eq!(
        validate_military_transition_prepare_args(
            "saved-job",
            7,
            &MilitaryTransitionWordingInput {
                responsibility_mappings: vec![
                    MilitaryWordingMappingInput {
                        military_evidence: "evidence".to_string(),
                        civilian_wording: "wording".to_string(),
                    };
                    16
                ],
                credential_mappings: vec![
                    MilitaryWordingMappingInput {
                        military_evidence: "credential".to_string(),
                        civilian_wording: "credential wording".to_string(),
                    };
                    16
                ],
                ..military_transition_wording_input()
            },
        ),
        Ok(())
    );
    assert!(is_exact_military_transition_review_token(
        "550e8400-e29b-41d4-a716-446655440000"
    ));
    assert!(!is_exact_military_transition_review_token(
        "550E8400-E29B-41D4-A716-446655440000"
    ));
    assert!(!is_exact_military_transition_review_token(
        "550e8400-e29b-11d4-a716-446655440000"
    ));
    assert!(!is_exact_military_transition_review_token("not-a-token"));
}

#[tokio::test]
async fn military_transition_confirmation_serializes_only_the_fixed_safe_projection() {
    let database = crate::desktop::Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let now = Utc::now();
    let job = Job {
        id: 0,
        hash: "saved-military-ipc".to_string(),
        title: "Support Specialist".to_string(),
        company: "Example".to_string(),
        url: "https://example.com/jobs/saved-military-ipc".to_string(),
        location: Some("Remote".to_string()),
        description: Some("Required: network support".to_string()),
        score: None,
        score_reasons: None,
        source: "test".to_string(),
        remote: Some(true),
        salary_min: None,
        salary_max: None,
        currency: None,
        created_at: now,
        updated_at: now,
        last_seen: now,
        times_seen: 1,
        immediate_alert_sent: false,
        hidden: false,
        included_in_digest: false,
        bookmarked: false,
        notes: None,
        ghost_score: None,
        ghost_reasons: None,
        first_seen: None,
        repost_count: 0,
    };
    database.insert_job_if_new(&job).await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("military.txt");
    std::fs::write(
        &path,
        "Army 25B\nConfigured tactical networks\nCompTIA Security+\nCurrent Secret clearance",
    )
    .unwrap();
    let resume_id = database
        .resume_matcher()
        .upload_resume("Military Resume", path.to_str().unwrap())
        .await
        .unwrap();
    database
        .resume_matcher()
        .match_resume_to_job(resume_id, &job.hash)
        .await
        .unwrap();
    for kind in [
        SavedMatchMilitaryEvidenceKind::MilitaryService,
        SavedMatchMilitaryEvidenceKind::CurrentClearance,
    ] {
        assert!(
            confirm_saved_match_military_evidence(&database, &job.hash, resume_id, kind)
                .await
                .unwrap()
        );
    }

    let pending = PendingMilitaryTransitionReviews::default();
    let token = prepare_pending_saved_match_military_transition_review(
        &database,
        &pending,
        &job.hash,
        resume_id,
        MilitaryBranch::Army,
        application_wording(),
    )
    .await
    .unwrap();
    let confirmation =
        confirm_pending_saved_match_military_transition_review(&database, &pending, &token)
            .await
            .unwrap();

    let value = serde_json::to_value(confirmation).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "civilian_role": "Technical support specialist",
            "civilian_responsibilities": ["Maintained secure network services"],
            "credential_wording": ["CompTIA Security+"],
            "user_confirmed_current_clearance": "Secret",
            "boundary": "suggestion_only",
            "clearance_currentness": "not_verified",
            "military_civilian_equivalence": "not_verified",
        })
    );
    let serialized = value.to_string();
    for forbidden in [
        "case_file_id",
        "evidence_id",
        "occupation_code",
        "review_id",
        "military_evidence",
        "Configured tactical networks",
        "Army 25B",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "must not serialize {forbidden}"
        );
    }
}
