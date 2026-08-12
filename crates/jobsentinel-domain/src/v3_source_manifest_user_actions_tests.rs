use chrono::{NaiveDate, TimeZone, Utc};

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, SourcePermission, USER_SOURCE_ACTIONS_MANIFEST_V1,
    },
};

fn policy() -> SourcePolicy {
    SourcePolicy {
        source_id: "user-source-actions".to_string(),
        source_class: SourceClass::UserImport,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    }
}

fn grant(operation: SourceOperation, permission: SourcePermission) -> SourceGrantState {
    SourceGrantState::Granted {
        source_id: "user-source-actions".to_string(),
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        permission,
        operation,
        policy_revision: 1,
    }
}

#[test]
fn employer_discovery_requires_the_exact_review_grant() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::EmployerDiscovery,
                today,
                SourceGrantState::Missing,
            )
            .unwrap(),
        SourceActionDecision::ReviewRequired
    );
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::EmployerDiscovery,
                today,
                grant(
                    SourceOperation::EmployerDiscovery,
                    SourcePermission::UserReview,
                ),
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: true,
        }
    );
}

#[test]
fn employer_discovery_rejects_cross_operation_and_browser_grants() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    for grant in [
        grant(
            SourceOperation::VisiblePageCapture,
            SourcePermission::UserReview,
        ),
        grant(
            SourceOperation::EmployerDiscovery,
            SourcePermission::PairedBrowserGrant,
        ),
    ] {
        assert_eq!(
            manifest
                .authorize(&policy, SourceOperation::EmployerDiscovery, today, grant)
                .unwrap(),
            SourceActionDecision::Revoked
        );
    }
}

#[test]
fn visible_page_capture_requires_the_exact_pairing_grant() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::VisiblePageCapture,
                today,
                SourceGrantState::Missing,
            )
            .unwrap(),
        SourceActionDecision::ReviewRequired
    );
    for wrong_grant in [
        grant(
            SourceOperation::VisiblePageCapture,
            SourcePermission::UserReview,
        ),
        grant(
            SourceOperation::EmployerDiscovery,
            SourcePermission::PairedBrowserGrant,
        ),
        SourceGrantState::Granted {
            source_id: "user-source-actions".to_string(),
            policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
            permission: SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 2,
        },
    ] {
        assert_eq!(
            manifest
                .authorize(
                    &policy,
                    SourceOperation::VisiblePageCapture,
                    today,
                    wrong_grant,
                )
                .unwrap(),
            SourceActionDecision::Revoked
        );
    }
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::VisiblePageCapture,
                today,
                grant(
                    SourceOperation::VisiblePageCapture,
                    SourcePermission::PairedBrowserGrant,
                ),
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
}

#[test]
fn smart_paste_requires_the_exact_review_grant_without_connectivity() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::SmartPaste,
                today,
                SourceGrantState::Missing,
            )
            .unwrap(),
        SourceActionDecision::ReviewRequired
    );
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::SmartPaste,
                today,
                grant(SourceOperation::SmartPaste, SourcePermission::UserReview),
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
}

#[test]
fn browser_applied_logging_requires_the_exact_pairing_grant() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    for grant in [
        SourceGrantState::Missing,
        grant(
            SourceOperation::AppliedLogging,
            SourcePermission::UserReview,
        ),
        grant(
            SourceOperation::VisiblePageCapture,
            SourcePermission::PairedBrowserGrant,
        ),
    ] {
        assert_ne!(
            manifest
                .authorize(&policy, SourceOperation::AppliedLogging, today, grant)
                .unwrap(),
            SourceActionDecision::Allowed {
                request_limit_per_hour: 0,
                connectivity_required: false,
            }
        );
    }
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::AppliedLogging,
                today,
                grant(
                    SourceOperation::AppliedLogging,
                    SourcePermission::PairedBrowserGrant,
                ),
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
}

#[test]
fn user_source_action_fixture_hash_gates_every_declared_operation() {
    let policy = policy();
    let manifest = parse_source_manifest(USER_SOURCE_ACTIONS_MANIFEST_V1, &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();
    let path =
        "crates/jobsentinel-domain/src/fixtures/source_simulator/user_source_actions_v1.json";
    let fixture = include_bytes!("fixtures/source_simulator/user_source_actions_v1.json");

    for (operation, permission) in [
        (
            SourceOperation::EmployerDiscovery,
            SourcePermission::UserReview,
        ),
        (
            SourceOperation::VisiblePageCapture,
            SourcePermission::PairedBrowserGrant,
        ),
        (SourceOperation::SmartPaste, SourcePermission::UserReview),
        (
            SourceOperation::AppliedLogging,
            SourcePermission::PairedBrowserGrant,
        ),
    ] {
        assert!(matches!(
            manifest
                .simulate(
                    &policy,
                    operation,
                    today,
                    grant(operation, permission),
                    &[(path, fixture)],
                )
                .unwrap()
                .decision,
            SourceActionDecision::Allowed { .. }
        ));
    }
    assert!(matches!(
        manifest
            .simulate(
                &policy,
                SourceOperation::EmployerDiscovery,
                today,
                SourceGrantState::Missing,
                &[(path, b"changed")],
            )
            .unwrap()
            .decision,
        SourceActionDecision::Blocked(_)
    ));
}
