//! Proves source-record freshness can be inspected without granting a source action.

use chrono::{NaiveDate, TimeZone, Utc};

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_authorization::{
        SourceActionDecision, SourceFreshnessReport, SourceFreshnessStatus, SourceGrantState,
    },
    v3_source_manifest::{parse_source_manifest, SourceOperation, SourceStopCondition},
};

const MANIFEST: &str = include_str!("fixtures/v3_source_manifest_v1.json");

fn policy() -> SourcePolicy {
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

#[test]
fn source_manifest_binds_exact_actions_to_existing_policy() {
    let policy = policy();
    let manifest = parse_source_manifest(MANIFEST, &policy).unwrap();
    assert_eq!(
        manifest
            .authorize(
                &policy,
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                SourceGrantState::NotRequired,
            )
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 60,
            connectivity_required: true,
        }
    );
}

#[test]
fn source_freshness_reports_policy_and_review_blocks_without_hiding_history() {
    let original_policy = policy();
    let today = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    let manifest = parse_source_manifest(MANIFEST, &original_policy).unwrap();

    let mut disabled = original_policy.clone();
    disabled.access = SourceAccess::Disabled;
    disabled.request_limit_per_hour = 0;
    assert_eq!(
        manifest.freshness(&disabled, today).unwrap().status,
        SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyDisabled)
    );

    let mut changed = original_policy.clone();
    changed.policy_ref = "changed-policy".to_string();
    changed.revision = 2;
    assert_eq!(
        manifest.freshness(&changed, today).unwrap().status,
        SourceFreshnessStatus::Blocked(SourceStopCondition::PolicyChanged)
    );

    let mut value = serde_json::from_str::<serde_json::Value>(MANIFEST).unwrap();
    value["terms_review"]["status"] = serde_json::json!("review_required");
    value["terms_review"]["reviewed_on"] = serde_json::Value::Null;
    let review_required = parse_source_manifest(&value.to_string(), &original_policy).unwrap();
    assert_eq!(
        review_required
            .freshness(&original_policy, today)
            .unwrap()
            .status,
        SourceFreshnessStatus::ReviewRequired
    );

    value = serde_json::from_str::<serde_json::Value>(MANIFEST).unwrap();
    value["terms_review"]["reviewed_on"] = serde_json::json!("2025-01-01");
    let review_expired = parse_source_manifest(&value.to_string(), &original_policy).unwrap();
    assert_eq!(
        review_expired
            .freshness(&original_policy, today)
            .unwrap()
            .status,
        SourceFreshnessStatus::Blocked(SourceStopCondition::ReviewExpired)
    );
}

#[test]
fn source_freshness_reports_current_stale_and_invalid_dates_without_authorizing_an_action() {
    let policy = policy();
    let manifest = parse_source_manifest(MANIFEST, &policy).unwrap();

    assert_eq!(
        manifest
            .freshness(&policy, NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),)
            .unwrap(),
        SourceFreshnessReport {
            status: SourceFreshnessStatus::Current,
            verified_on: NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
            expires_on: NaiveDate::from_ymd_opt(2026, 10, 17).unwrap(),
        }
    );
    assert_eq!(
        manifest
            .freshness(&policy, NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),)
            .unwrap()
            .status,
        SourceFreshnessStatus::Stale
    );
    assert!(manifest
        .freshness(&policy, NaiveDate::from_ymd_opt(2026, 7, 18).unwrap(),)
        .is_err());
}
