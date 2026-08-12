//! Proves reviewed source installation, authorization, and fixture binding fail closed.

use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, SourceStopCondition,
        GREENHOUSE_REQUEST_LIMIT_PER_HOUR, JOBSWITHGPT_SOURCE_MANIFEST_V1,
        LEVER_REQUEST_LIMIT_PER_HOUR,
    },
};
use jobsentinel_storage::Database;

use crate::v3_source_governance::{
    authorize_greenhouse, authorize_jobswithgpt, authorize_lever, install_greenhouse,
    install_jobswithgpt, install_lever, jobswithgpt_policy,
};

#[tokio::test]
async fn reviewed_public_ats_manifests_authorize_only_their_policy_rates() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_greenhouse(&database).await.unwrap();
    install_lever(&database).await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();

    assert_eq!(
        authorize_greenhouse(&database, SourceOperation::ScheduledCheck, today)
            .await
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: GREENHOUSE_REQUEST_LIMIT_PER_HOUR,
            connectivity_required: true,
        }
    );
    assert_eq!(
        authorize_lever(&database, SourceOperation::ConnectivityCheck, today)
            .await
            .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: LEVER_REQUEST_LIMIT_PER_HOUR,
            connectivity_required: true,
        }
    );
}

#[tokio::test]
async fn public_ats_authorization_fails_closed_without_installed_manifests() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert!(
        authorize_greenhouse(&database, SourceOperation::ScheduledCheck, today)
            .await
            .is_err()
    );
    assert!(
        authorize_lever(&database, SourceOperation::ScheduledCheck, today)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn jobswithgpt_stays_disabled_while_provider_policy_is_unresolved() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_jobswithgpt(&database).await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        authorize_jobswithgpt(&database, SourceOperation::ScheduledCheck, today)
            .await
            .unwrap(),
        SourceActionDecision::Blocked(SourceStopCondition::PolicyDisabled)
    );
}

#[test]
fn jobswithgpt_simulator_binds_parser_and_provider_review_fixtures() {
    let policy = jobswithgpt_policy().unwrap();
    let manifest = parse_source_manifest(JOBSWITHGPT_SOURCE_MANIFEST_V1, &policy).unwrap();
    let fixtures = [
        (
            "crates/jobsentinel-sources/src/fixtures/jobswithgpt_list_v1.json",
            include_bytes!("../../jobsentinel-sources/src/fixtures/jobswithgpt_list_v1.json")
                .as_slice(),
        ),
        (
            "crates/jobsentinel-domain/src/fixtures/source_reviews/jobswithgpt_v1.json",
            include_bytes!(
                "../../jobsentinel-domain/src/fixtures/source_reviews/jobswithgpt_v1.json"
            )
            .as_slice(),
        ),
    ];

    assert_eq!(
        manifest
            .simulate(
                &policy,
                SourceOperation::ScheduledCheck,
                NaiveDate::from_ymd_opt(2026, 7, 19).unwrap(),
                SourceGrantState::Missing,
                &fixtures,
            )
            .unwrap()
            .decision,
        SourceActionDecision::Blocked(SourceStopCondition::PolicyDisabled)
    );
}
