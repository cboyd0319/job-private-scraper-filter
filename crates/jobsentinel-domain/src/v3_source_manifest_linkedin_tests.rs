use chrono::{TimeZone, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, SourcePermission,
        LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1,
    },
};

#[test]
fn manifest_has_only_reviewed_user_opened_actions() {
    let policy = SourcePolicy {
        source_id: "linkedin-workbench".to_string(),
        source_class: SourceClass::RestrictedUserOpened,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.linkedin-workbench".to_string(),
        revision: 1,
        restriction_reason_code: Some("account-backed-source".to_string()),
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    };
    let manifest = parse_source_manifest(LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1, &policy).unwrap();

    assert_eq!(manifest.source_class, SourceClass::RestrictedUserOpened);
    assert_eq!(
        manifest
            .actions
            .iter()
            .map(|action| (action.operation, action.permission))
            .collect::<Vec<_>>(),
        vec![(
            SourceOperation::RestrictedWorkbench,
            SourcePermission::UserReview,
        )]
    );
    assert!(!manifest
        .actions
        .iter()
        .any(|action| action.operation == SourceOperation::ScheduledCheck));
    assert_eq!(
        manifest.fixtures[0].payload_sha256,
        hex::encode(Sha256::digest(include_bytes!(
            "fixtures/source_reviews/linkedin_workbench_v1.json"
        )))
    );
}

#[test]
fn applied_logging_cannot_downgrade_to_user_review() {
    let policy = SourcePolicy {
        source_id: "linkedin-workbench".to_string(),
        source_class: SourceClass::RestrictedUserOpened,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.linkedin-workbench".to_string(),
        revision: 1,
        restriction_reason_code: Some("account-backed-source".to_string()),
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    };
    let mut manifest: serde_json::Value =
        serde_json::from_str(LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1).unwrap();
    manifest["actions"][0]["operation"] = json!("applied_logging");

    assert!(parse_source_manifest(&manifest.to_string(), &policy).is_err());
}
