use chrono::{NaiveDate, TimeZone, Utc};
use sha2::{Digest, Sha256};

use crate::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, SourcePermission, SourceStopCondition,
        BUILTIN_SOURCE_MANIFEST_V2, DICE_SOURCE_MANIFEST_V2, GLASSDOOR_SOURCE_MANIFEST_V2,
        SIMPLYHIRED_SOURCE_MANIFEST_V2,
    },
};

const MANIFEST: &str = include_str!("fixtures/v3_source_manifest_v1.json");
const LIST_PATH: &str =
    "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_list.json";
const DETAIL_PATH: &str =
    "crates/jobsentinel-domain/src/fixtures/source_simulator/synthetic_official_detail.json";
const POLICY_PATH: &str =
    "crates/jobsentinel-domain/src/fixtures/source_reviews/synthetic_official_v1.json";

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

fn fixtures() -> [(&'static str, &'static [u8]); 3] {
    [
        (
            LIST_PATH,
            include_bytes!("fixtures/source_simulator/synthetic_official_list.json").as_slice(),
        ),
        (
            DETAIL_PATH,
            include_bytes!("fixtures/source_simulator/synthetic_official_detail.json").as_slice(),
        ),
        (
            POLICY_PATH,
            include_bytes!("fixtures/source_reviews/synthetic_official_v1.json").as_slice(),
        ),
    ]
}

#[test]
fn disabled_source_manifest_does_not_claim_an_unverified_endpoint() {
    let mut disabled = policy();
    disabled.access = SourceAccess::Disabled;
    disabled.request_limit_per_hour = 0;
    let mut manifest = parse_source_manifest(MANIFEST, &policy()).unwrap();
    manifest.endpoint_patterns.clear();

    assert!(manifest.validate(&disabled).is_ok());
    assert!(manifest.validate(&policy()).is_err());
}

#[test]
fn source_simulator_reuses_authorization_and_reports_expiry() {
    let policy = policy();
    let report = parse_source_manifest(MANIFEST, &policy)
        .unwrap()
        .simulate(
            &policy,
            SourceOperation::ScheduledCheck,
            NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
            SourceGrantState::NotRequired,
            &fixtures(),
        )
        .unwrap();

    assert_eq!(
        report.decision,
        SourceActionDecision::Allowed {
            request_limit_per_hour: 60,
            connectivity_required: true,
        }
    );
    assert_eq!(
        report.manifest_expires_on,
        NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()
    );
    assert_eq!(report.review_expires_on, Some(report.manifest_expires_on));
    assert_eq!(
        report.risk_note_refs,
        ["source-risk.synthetic-official-jobs.coverage"]
    );
}

#[test]
fn source_simulator_blocks_missing_changed_extra_and_duplicate_fixtures() {
    let policy = policy();
    let manifest = parse_source_manifest(MANIFEST, &policy).unwrap();
    let [list, detail, review] = fixtures();

    for fixtures in [
        vec![list, detail],
        vec![(LIST_PATH, b"{}".as_slice()), detail, review],
        vec![list, detail, review, ("extra.json", b"{}".as_slice())],
        vec![list, list, review],
    ] {
        assert_eq!(
            manifest
                .simulate(
                    &policy,
                    SourceOperation::ScheduledCheck,
                    NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                    SourceGrantState::NotRequired,
                    &fixtures,
                )
                .unwrap()
                .decision,
            SourceActionDecision::Blocked(SourceStopCondition::ParserDrift)
        );
    }
}

#[test]
fn retired_restricted_scrapers_bind_policy_evidence_and_ignore_user_grants() {
    let reviewed_at = Utc.with_ymd_and_hms(2026, 7, 19, 0, 0, 0).unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();
    let policy_fixture =
        include_bytes!("fixtures/source_reviews/restricted_scheduled_retirement_v2.json")
            .as_slice();

    for (source_id, policy_ref, raw_manifest) in [
        (
            "builtin",
            "jobsentinel.source-policy.builtin.scheduled",
            BUILTIN_SOURCE_MANIFEST_V2,
        ),
        (
            "dice",
            "jobsentinel.source-policy.dice.scheduled",
            DICE_SOURCE_MANIFEST_V2,
        ),
        (
            "simplyhired",
            "jobsentinel.source-policy.simplyhired.scheduled",
            SIMPLYHIRED_SOURCE_MANIFEST_V2,
        ),
        (
            "glassdoor",
            "jobsentinel.source-policy.glassdoor.scheduled",
            GLASSDOOR_SOURCE_MANIFEST_V2,
        ),
    ] {
        let policy = SourcePolicy {
            source_id: source_id.to_string(),
            source_class: SourceClass::RestrictedPublicScheduled,
            access: SourceAccess::Disabled,
            request_limit_per_hour: 0,
            user_review_required: true,
            policy_ref: policy_ref.to_string(),
            revision: 2,
            restriction_reason_code: Some("provider-automation-prohibited".to_string()),
            reviewed_at,
        };
        let manifest = parse_source_manifest(raw_manifest, &policy).unwrap();

        assert_eq!(
            manifest.fixtures[0].payload_sha256,
            hex::encode(Sha256::digest(policy_fixture))
        );
        assert_eq!(
            manifest
                .authorize(
                    &policy,
                    SourceOperation::ScheduledCheck,
                    today,
                    SourceGrantState::Granted {
                        source_id: source_id.to_string(),
                        policy_ref: policy_ref.to_string(),
                        permission: SourcePermission::UserReview,
                        operation: SourceOperation::ScheduledCheck,
                        policy_revision: 2,
                    },
                )
                .unwrap(),
            SourceActionDecision::Blocked(SourceStopCondition::PolicyDisabled)
        );
    }
}
