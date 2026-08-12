//! Owns the immutable public-key registry and authority ceilings for production packs.

use std::sync::OnceLock;

use jobsentinel_domain::{
    v3_manifests::{
        AgentTaskKind, ApprovalGate, DataCategory, PackAction, PackExecutionClass, PackType,
        PrivacyLabel,
    },
    v3_signed_packs::TrustedPublisherKey,
};

pub(crate) fn production_trusted_publishers() -> &'static [TrustedPublisherKey] {
    static PUBLISHERS: OnceLock<[TrustedPublisherKey; 5]> = OnceLock::new();
    PUBLISHERS.get_or_init(|| {
        [
            static_publisher(
                "jobsentinel-source-publisher-v1",
                [
                    0xd2, 0xac, 0xad, 0x54, 0x0b, 0xbe, 0x9c, 0xb0, 0x04, 0xe0, 0xf1, 0x9a, 0xec,
                    0x25, 0xc5, 0x4b, 0x90, 0x19, 0x1f, 0xe2, 0x99, 0x7e, 0xde, 0xd4, 0x8b, 0x3e,
                    0xe8, 0xe6, 0x99, 0x86, 0xf6, 0x19,
                ],
                PackType::Source,
            ),
            static_publisher(
                "jobsentinel-skill-publisher-v1",
                [
                    0xd3, 0xb0, 0x21, 0x1f, 0x87, 0xfc, 0x3e, 0xf2, 0xa6, 0x2a, 0xab, 0xf4, 0x05,
                    0xfe, 0x64, 0x81, 0x6f, 0xc4, 0xa6, 0x7e, 0x96, 0xe7, 0xad, 0xbd, 0x8b, 0xee,
                    0x3a, 0x40, 0xab, 0x38, 0x67, 0xa2,
                ],
                PackType::Skill,
            ),
            static_publisher(
                "jobsentinel-evaluation-publisher-v1",
                [
                    0x7c, 0x01, 0x4a, 0x96, 0x8e, 0x56, 0xa5, 0x27, 0x7e, 0xf7, 0x3e, 0xbb, 0x9f,
                    0x2e, 0xd0, 0xe4, 0x7c, 0xdf, 0x01, 0x52, 0x83, 0xe6, 0xf5, 0xea, 0x3f, 0x21,
                    0x1b, 0x73, 0xe8, 0xdb, 0xfc, 0x35,
                ],
                PackType::Evaluation,
            ),
            evidence_reviewer_publisher(),
            packet_builder_publisher(),
        ]
    })
}

fn static_publisher(
    publisher_key_id: &str,
    public_key: [u8; 32],
    pack_type: PackType,
) -> TrustedPublisherKey {
    TrustedPublisherKey {
        publisher_key_id: publisher_key_id.to_string(),
        public_key,
        revoked: false,
        pack_type,
        execution_class: PackExecutionClass::StaticContent,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly],
        allowed_data_categories: vec![],
        allowed_task_kinds: vec![],
        allowed_actions: vec![],
        allowed_approval_gates: vec![],
        allow_gateway_external_ai: false,
    }
}

fn evidence_reviewer_publisher() -> TrustedPublisherKey {
    agent_publisher(
        "jobsentinel-evidence-reviewer-publisher-v1",
        [
            0x08, 0xb4, 0x2b, 0x9e, 0x11, 0x43, 0x46, 0x2d, 0xe8, 0x2d, 0x64, 0x1b, 0x7d, 0x62,
            0x80, 0x2e, 0x20, 0x98, 0xe4, 0xf5, 0xbe, 0x81, 0x07, 0x0a, 0xbd, 0x64, 0x5a, 0xd8,
            0xb8, 0xaa, 0xf6, 0x55,
        ],
        vec![DataCategory::ResumeEvidence],
        AgentTaskKind::EvidenceReview,
        vec![
            PackAction::ReadSelectedResumeEvidence,
            PackAction::WriteLocalEvent,
        ],
    )
}

fn packet_builder_publisher() -> TrustedPublisherKey {
    agent_publisher(
        "jobsentinel-packet-builder-publisher-v1",
        [
            0x33, 0x85, 0xe8, 0x40, 0xb8, 0x59, 0xe6, 0xa8, 0xaf, 0xb6, 0x0b, 0xaf, 0x03, 0xd1,
            0x45, 0xc0, 0x97, 0xa5, 0x2b, 0xe6, 0x9a, 0x61, 0x23, 0x97, 0xbd, 0x35, 0x70, 0xa8,
            0x4d, 0x9a, 0x75, 0x0d,
        ],
        vec![DataCategory::PublicJobPosting, DataCategory::ResumeEvidence],
        AgentTaskKind::DraftPacket,
        vec![
            PackAction::ReadSelectedCaseFile,
            PackAction::ReadSelectedResumeEvidence,
            PackAction::ReadPublicJobPosting,
            PackAction::CreateDraftApplicationPacket,
            PackAction::WriteLocalEvent,
        ],
    )
}

fn agent_publisher(
    publisher_key_id: &str,
    public_key: [u8; 32],
    allowed_data_categories: Vec<DataCategory>,
    task_kind: AgentTaskKind,
    allowed_actions: Vec<PackAction>,
) -> TrustedPublisherKey {
    TrustedPublisherKey {
        publisher_key_id: publisher_key_id.to_string(),
        public_key,
        revoked: false,
        pack_type: PackType::Agent,
        execution_class: PackExecutionClass::ReviewedTypedWorkflow,
        allowed_privacy_labels: vec![PrivacyLabel::LocalOnly, PrivacyLabel::Sensitive],
        allowed_data_categories,
        allowed_task_kinds: vec![task_kind],
        allowed_actions,
        allowed_approval_gates: vec![ApprovalGate::PerExecutionReview],
        allow_gateway_external_ai: false,
    }
}
