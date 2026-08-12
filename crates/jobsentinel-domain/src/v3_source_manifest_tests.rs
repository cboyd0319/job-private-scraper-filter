//! Proves source manifests remain policy-bound, fresh, and fail closed.

use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_authorization::*,
    v3_source_manifest::*,
};

const BASELINE: &str = include_str!("fixtures/v3_source_manifest_v1.json");

fn fixture() -> Value {
    serde_json::from_str(BASELINE).expect("source manifest fixture must be valid JSON")
}

fn scheduled_policy() -> SourcePolicy {
    SourcePolicy {
        source_id: "synthetic-official-jobs".to_string(),
        source_class: SourceClass::OfficialPublicApi,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour: 60,
        user_review_required: false,
        policy_ref: "synthetic-official-jobs-v1".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    }
}

fn restricted_fixture() -> Value {
    let mut value = fixture();
    value["source_id"] = json!("synthetic-restricted-jobs");
    value["display_name"] = json!("Synthetic Restricted Jobs");
    value["source_class"] = json!("restricted_user_opened");
    value["endpoint_patterns"] = json!(["https://restricted.example.com/jobs/"]);
    value["auth_requirement"] = json!("user_session");
    value["policy_ref"] = json!("synthetic-restricted-jobs-v1");
    value["actions"] = json!([{
        "operation": "visible_page_capture",
        "permission": "paired_browser_grant"
    }]);
    value["parser_id"] = json!("visible-page-v1");
    value["lineage"][0]["link_id"] = json!("source-lineage-synthetic-restricted-jobs");
    value["lineage"][0]["source_id"] = json!("synthetic-restricted-jobs");
    value
}

fn restricted_policy() -> SourcePolicy {
    SourcePolicy {
        source_id: "synthetic-restricted-jobs".to_string(),
        source_class: SourceClass::RestrictedUserOpened,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "synthetic-restricted-jobs-v1".to_string(),
        revision: 1,
        restriction_reason_code: Some("account-backed-source".to_string()),
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    }
}

fn user_import_policy() -> SourcePolicy {
    SourcePolicy {
        source_id: "smart-paste".to_string(),
        source_class: SourceClass::UserImport,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "smart-paste-v1".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap(),
    }
}

#[test]
fn source_manifest_rejects_unknown_unsafe_or_unverified_input() {
    let policy = scheduled_policy();
    for (pointer, invalid) in [
        ("/endpoint_patterns/0", json!("https://localhost/private")),
        (
            "/fixtures/0/payload_sha256",
            json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ),
        ("/fixtures", json!([])),
        ("/confidence_percent", json!(0)),
        ("/max_age_days", json!(0)),
        ("/lineage", json!([])),
        ("/stop_conditions", json!([])),
        (
            "/fixtures/0/path",
            json!("../../private/source-fixture.json"),
        ),
    ] {
        let mut value = fixture();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            parse_source_manifest(&value.to_string(), &policy).is_err(),
            "{pointer} must fail closed"
        );
    }

    let mut value = fixture();
    value["unreviewed_extension"] = json!(true);
    assert!(parse_source_manifest(&value.to_string(), &policy).is_err());

    for required in ["policy_disabled", "parser_drift"] {
        let mut value = fixture();
        value["stop_conditions"]
            .as_array_mut()
            .unwrap()
            .retain(|condition| condition != required);
        assert!(parse_source_manifest(&value.to_string(), &policy).is_err());
    }

    let mut value = fixture();
    value["fixtures"]
        .as_array_mut()
        .unwrap()
        .retain(|fixture| fixture["kind"] != "policy");
    assert!(parse_source_manifest(&value.to_string(), &policy).is_err());
}

#[test]
fn source_manifest_rejects_policy_identity_or_revision_drift() {
    for mutate in [
        |policy: &mut SourcePolicy| policy.source_id = "other-source".to_string(),
        |policy: &mut SourcePolicy| policy.source_class = SourceClass::PublicAts,
        |policy: &mut SourcePolicy| policy.policy_ref = "other-policy".to_string(),
        |policy: &mut SourcePolicy| policy.revision = 2,
    ] {
        let mut policy = scheduled_policy();
        mutate(&mut policy);
        assert!(parse_source_manifest(BASELINE, &policy).is_err());
    }
}

#[test]
fn review_state_and_review_freshness_stop_source_actions() {
    let policy = scheduled_policy();
    let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    let mut value = fixture();
    value["terms_review"] = json!({
        "status": "review_required",
        "reference": "source-review.synthetic-official-jobs.terms",
        "reviewed_on": null
    });
    let manifest = parse_source_manifest(&value.to_string(), &policy).unwrap();
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::ReviewRequired
    );

    value = fixture();
    value["terms_review"]["reviewed_on"] = json!("2025-01-01");
    let manifest = parse_source_manifest(&value.to_string(), &policy).unwrap();
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::ScheduledCheck,
                today,
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Blocked(SourceStopCondition::ReviewExpired)
    );

    value = fixture();
    value["terms_review"]["reviewed_on"] = json!("2026-07-21");
    let manifest = parse_source_manifest(&value.to_string(), &policy).unwrap();
    assert!(manifest
        .authorize(
            &policy,
            SourceOperation::ScheduledCheck,
            today,
            SourceGrantState::NotRequired,
        )
        .is_err());
}

#[test]
fn local_imports_need_no_invented_network_endpoint_or_documentation() {
    let mut value = fixture();
    value["source_id"] = json!("smart-paste");
    value["display_name"] = json!("Smart Paste");
    value["source_class"] = json!("user_import");
    value["endpoint_patterns"] = json!([]);
    value["documentation_url"] = Value::Null;
    value["policy_ref"] = json!("smart-paste-v1");
    value["actions"] = json!([{
        "operation": "smart_paste",
        "permission": "user_review"
    }]);
    value["lineage"] = json!([]);
    value["risk_note_refs"] = json!([]);

    assert!(parse_source_manifest(&value.to_string(), &user_import_policy()).is_ok());
}

#[test]
fn workbench_connectivity_is_derived_before_the_user_opens_a_source() {
    let policy = restricted_policy();
    let mut value = restricted_fixture();
    value["actions"] = json!([{
        "operation": "restricted_workbench",
        "permission": "user_review"
    }]);

    assert!(parse_source_manifest(&value.to_string(), &policy).is_ok());
}

#[test]
fn fixture_hashes_bind_to_real_synthetic_payloads() {
    let value = fixture();
    let fixtures = value["fixtures"].as_array().unwrap();
    let expected = [
        (
            "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_list.json",
            include_bytes!("fixtures/source_simulator/synthetic_official_list.json").as_slice(),
        ),
        (
            "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_detail.json",
            include_bytes!("fixtures/source_simulator/synthetic_official_detail.json").as_slice(),
        ),
        (
            "crates/jobsentinel-domain/src/fixtures/source_reviews/synthetic_official_v1.json",
            include_bytes!("fixtures/source_reviews/synthetic_official_v1.json").as_slice(),
        ),
    ];
    assert_eq!(fixtures.len(), expected.len());
    for (fixture, (path, payload)) in fixtures.iter().zip(expected) {
        assert_eq!(fixture["path"], path);
        assert_eq!(
            fixture["payload_sha256"].as_str().unwrap(),
            hex::encode(Sha256::digest(payload))
        );
    }
}

#[test]
fn reviewed_source_manifests_bind_policy_evidence() {
    for raw in [
        BASELINE,
        USAJOBS_SOURCE_MANIFEST_V1,
        REMOTEOK_SOURCE_MANIFEST_V1,
        WEWORKREMOTELY_SOURCE_MANIFEST_V1,
        HN_HIRING_SOURCE_MANIFEST_V1,
        GREENHOUSE_SOURCE_MANIFEST_V1,
        LEVER_SOURCE_MANIFEST_V1,
        LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1,
    ] {
        let value: Value = serde_json::from_str(raw).unwrap();
        assert!(
            value["fixtures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|fixture| fixture["kind"] == "policy"),
            "{} must hash-bind its dated policy review",
            value["source_id"]
        );
    }
}

#[test]
fn source_manifest_accepts_reviewed_web_fixture_formats_only() {
    let policy = scheduled_policy();
    for extension in ["json", "xml"] {
        let mut value = fixture();
        value["fixtures"][0]["path"] = json!(format!(
            "crates/jobsentinel-sources/src/fixtures/source.{extension}"
        ));
        assert!(
            parse_source_manifest(&value.to_string(), &policy).is_ok(),
            "{extension} fixtures should be supported"
        );
    }

    for extension in ["html", "txt"] {
        let mut value = fixture();
        value["fixtures"][0]["path"] = json!(format!(
            "crates/jobsentinel-sources/src/fixtures/source.{extension}"
        ));
        assert!(
            parse_source_manifest(&value.to_string(), &policy).is_err(),
            "{extension} fixtures should remain unsupported"
        );
    }
}

#[test]
fn explicit_user_actions_cannot_be_permissionless() {
    let policy = scheduled_policy();
    for operation in ["restricted_workbench", "smart_paste", "applied_logging"] {
        let mut value = fixture();
        value["actions"] = json!([{
            "operation": operation,
            "permission": "none"
        }]);
        assert!(
            parse_source_manifest(&value.to_string(), &policy).is_err(),
            "{operation} must require explicit user permission"
        );
    }
}

#[test]
fn source_lineage_rejects_non_lineage_edges() {
    let policy = scheduled_policy();
    let mut value = fixture();
    value["lineage"][0]["relation"] = json!("related");

    assert!(parse_source_manifest(&value.to_string(), &policy).is_err());
}

#[test]
fn restricted_capture_requires_an_exact_current_grant() {
    let policy = restricted_policy();
    let manifest = parse_source_manifest(&restricted_fixture().to_string(), &policy).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();

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
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::VisiblePageCapture,
                today,
                SourceGrantState::Granted {
                    source_id: "synthetic-restricted-jobs".to_string(),
                    policy_ref: "synthetic-restricted-jobs-v1".to_string(),
                    permission: SourcePermission::PairedBrowserGrant,
                    operation: SourceOperation::VisiblePageCapture,
                    policy_revision: 1,
                },
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );

    for grant in [
        SourceGrantState::Revoked,
        SourceGrantState::Granted {
            source_id: "synthetic-restricted-jobs".to_string(),
            policy_ref: "synthetic-restricted-jobs-v1".to_string(),
            permission: SourcePermission::UserReview,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 1,
        },
        SourceGrantState::Granted {
            source_id: "synthetic-restricted-jobs".to_string(),
            policy_ref: "synthetic-restricted-jobs-v1".to_string(),
            permission: SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::AppliedLogging,
            policy_revision: 1,
        },
        SourceGrantState::Granted {
            source_id: "synthetic-restricted-jobs".to_string(),
            policy_ref: "synthetic-restricted-jobs-v1".to_string(),
            permission: SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 2,
        },
        SourceGrantState::Granted {
            source_id: "different-source".to_string(),
            policy_ref: "synthetic-restricted-jobs-v1".to_string(),
            permission: SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 1,
        },
        SourceGrantState::Granted {
            source_id: "synthetic-restricted-jobs".to_string(),
            policy_ref: "different-policy".to_string(),
            permission: SourcePermission::PairedBrowserGrant,
            operation: SourceOperation::VisiblePageCapture,
            policy_revision: 1,
        },
    ] {
        assert_eq!(
            manifest
                .authorize(&policy, SourceOperation::VisiblePageCapture, today, grant,)
                .unwrap(),
            SourceActionDecision::Revoked
        );
    }
}

#[test]
fn visible_capture_cannot_downgrade_its_browser_permission() {
    let policy = restricted_policy();
    for permission in ["none", "user_review"] {
        let mut value = restricted_fixture();
        value["actions"][0]["permission"] = json!(permission);
        assert!(
            parse_source_manifest(&value.to_string(), &policy).is_err(),
            "{permission} must not authorize visible capture"
        );
    }
}

#[test]
fn source_action_decisions_fail_closed_for_stale_disabled_and_unsupported_paths() {
    let manifest = parse_source_manifest(BASELINE, &scheduled_policy()).unwrap();
    assert_eq!(
        manifest
            .authorize(
                &scheduled_policy(),
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2027, 1, 1).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Stale
    );
    assert_eq!(
        manifest
            .authorize(
                &scheduled_policy(),
                SourceOperation::SmartPaste,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Unsupported
    );

    let mut policy = scheduled_policy();
    policy.access = SourceAccess::Disabled;
    policy.request_limit_per_hour = 0;
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Blocked(SourceStopCondition::PolicyDisabled)
    );

    let mut changed_policy = scheduled_policy();
    changed_policy.revision = 2;
    assert_eq!(
        manifest
            .authorize(
                &changed_policy,
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Blocked(SourceStopCondition::PolicyChanged)
    );
}
