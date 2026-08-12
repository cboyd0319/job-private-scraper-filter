//! Proves the immutable production pack-publisher trust registry and authority ceilings.

use jobsentinel_domain::v3_manifests::{
    AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackType,
    PrivacyLabel,
};
use sha2::{Digest, Sha256};

use crate::pack_runtime::production_trusted_publishers;

#[test]
fn production_trust_contains_only_the_five_approved_publishers() {
    let publishers = production_trusted_publishers();

    assert_eq!(publishers.len(), 5);
    assert_eq!(
        publishers
            .iter()
            .map(|publisher| publisher.publisher_key_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "jobsentinel-source-publisher-v1",
            "jobsentinel-skill-publisher-v1",
            "jobsentinel-evaluation-publisher-v1",
            "jobsentinel-evidence-reviewer-publisher-v1",
            "jobsentinel-packet-builder-publisher-v1",
        ]
    );
    assert!(publishers.iter().all(|publisher| !publisher.revoked));
    assert!(publishers
        .iter()
        .all(|publisher| !publisher.allow_gateway_external_ai));
    assert!(publishers.iter().all(|publisher| !publisher
        .allowed_actions
        .contains(&PackAction::RequestExternalAi)));
}

#[test]
fn production_public_keys_and_fingerprints_match_the_approved_identities() {
    let expected = [
        (
            "jobsentinel-source-publisher-v1",
            "d2acad540bbe9cb004e0f19aec25c54b90191fe2997eded48b3ee8e69986f619",
            "e95ab6838ec2cc62459ce81c629fcd98ba3a9e3c5179149c8ac0d47ba2a0ff88",
        ),
        (
            "jobsentinel-skill-publisher-v1",
            "d3b0211f87fc3ef2a62aabf405fe64816fc4a67e96e7adbd8bee3a40ab3867a2",
            "a1e608db96c70bde06f45b1584ca2de039995afe7ba6ad19eb93d593d8af0188",
        ),
        (
            "jobsentinel-evaluation-publisher-v1",
            "7c014a968e56a5277ef73ebb9f2ed0e47cdf015283e6f5ea3f211b73e8dbfc35",
            "ed5ba053d83a263ee9cff4374598ba64d981257f63a53907a79db2f4dbc36010",
        ),
        (
            "jobsentinel-evidence-reviewer-publisher-v1",
            "08b42b9e1143462de82d641b7d62802e2098e4f5be81070abd645ad8b8aaf655",
            "1a1d3845cc74254f0397d4955ea75fd074e8e27710bfd4f9dbe49ef047185297",
        ),
        (
            "jobsentinel-packet-builder-publisher-v1",
            "3385e840b859e6a8afb60baf03d145c097a52be69a612397bd3570a84d9a750d",
            "7a78e4dc5fd257f494d6c9d94948aa4a5169d0e3a1e41248c90acb2b0979d5c3",
        ),
    ];

    for (publisher_key_id, public_key_hex, fingerprint) in expected {
        let publisher = production_trusted_publishers()
            .iter()
            .find(|publisher| publisher.publisher_key_id == publisher_key_id)
            .unwrap();
        assert_eq!(hex::encode(publisher.public_key), public_key_hex);
        assert_eq!(
            hex::encode(Sha256::digest(publisher.public_key)),
            fingerprint
        );
    }
}

#[test]
fn production_publishers_have_the_approved_exact_authority_ceilings() {
    let publishers = production_trusted_publishers();
    for (publisher_key_id, pack_type) in [
        ("jobsentinel-source-publisher-v1", PackType::Source),
        ("jobsentinel-skill-publisher-v1", PackType::Skill),
        ("jobsentinel-evaluation-publisher-v1", PackType::Evaluation),
    ] {
        let publisher = publishers
            .iter()
            .find(|publisher| publisher.publisher_key_id == publisher_key_id)
            .unwrap();
        assert_eq!(publisher.pack_type, pack_type);
        assert_eq!(publisher.execution_class, PackExecutionClass::StaticContent);
        assert_eq!(publisher.allowed_privacy_labels, [PrivacyLabel::LocalOnly]);
        assert!(publisher.allowed_data_categories.is_empty());
        assert!(publisher.allowed_task_kinds.is_empty());
        assert!(publisher.allowed_actions.is_empty());
        assert!(publisher.allowed_approval_gates.is_empty());
    }

    let evidence = publishers
        .iter()
        .find(|publisher| {
            publisher.publisher_key_id == "jobsentinel-evidence-reviewer-publisher-v1"
        })
        .unwrap();
    assert_agent_ceiling(
        evidence,
        &[DataCategory::ResumeEvidence],
        AgentTaskKind::EvidenceReview,
        &[
            PackAction::ReadSelectedResumeEvidence,
            PackAction::WriteLocalEvent,
        ],
    );

    let packet = publishers
        .iter()
        .find(|publisher| publisher.publisher_key_id == "jobsentinel-packet-builder-publisher-v1")
        .unwrap();
    assert_agent_ceiling(
        packet,
        &[DataCategory::PublicJobPosting, DataCategory::ResumeEvidence],
        AgentTaskKind::DraftPacket,
        &[
            PackAction::ReadSelectedCaseFile,
            PackAction::ReadSelectedResumeEvidence,
            PackAction::ReadPublicJobPosting,
            PackAction::CreateDraftApplicationPacket,
            PackAction::WriteLocalEvent,
        ],
    );
}

fn assert_agent_ceiling(
    publisher: &jobsentinel_domain::v3_signed_packs::TrustedPublisherKey,
    data_categories: &[DataCategory],
    task_kind: AgentTaskKind,
    actions: &[PackAction],
) {
    assert_eq!(publisher.pack_type, PackType::Agent);
    assert_eq!(
        publisher.execution_class,
        PackExecutionClass::ReviewedTypedWorkflow
    );
    assert_eq!(
        publisher.allowed_privacy_labels,
        [PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive]
    );
    assert_eq!(publisher.allowed_data_categories, data_categories);
    assert_eq!(publisher.allowed_task_kinds, [task_kind]);
    assert_eq!(publisher.allowed_actions, actions);
    assert_eq!(
        publisher.allowed_approval_gates,
        [ApprovalGate::PerExecutionReview]
    );
}
