use chrono::NaiveDate;
use jobsentinel_domain::{
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{SourceOperation, SourcePermission},
};
use jobsentinel_storage::Database;

use crate::{
    v3_foundation::{set_source_policy, FoundationError},
    v3_source_governance::{
        authorize_user_source_action, install_startup_source_governance,
        install_user_source_actions, usajobs_policy, user_source_actions_policy,
    },
};

fn grant(operation: SourceOperation, permission: SourcePermission) -> SourceGrantState {
    SourceGrantState::Granted {
        source_id: "user-source-actions".to_string(),
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        permission,
        operation,
        policy_revision: 1,
    }
}

#[tokio::test]
async fn user_source_action_governance_is_installed_with_graph_lineage() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();

    install_user_source_actions(&database).await.unwrap();

    assert_eq!(
        database
            .get_source_policy("user-source-actions")
            .await
            .unwrap(),
        Some(user_source_actions_policy().unwrap())
    );
    assert_eq!(
        database
            .list_source_graph_links("user-source-actions")
            .await
            .unwrap()
            .len(),
        4
    );
}

#[tokio::test]
async fn user_source_action_authorization_requires_the_persisted_exact_grant() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_user_source_actions(&database).await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::EmployerDiscovery,
            today,
            SourceGrantState::Missing,
        )
        .await
        .unwrap(),
        SourceActionDecision::ReviewRequired
    );
    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::EmployerDiscovery,
            today,
            grant(
                SourceOperation::EmployerDiscovery,
                SourcePermission::UserReview,
            ),
        )
        .await
        .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: true,
        }
    );

    let mut newer_policy = user_source_actions_policy().unwrap();
    newer_policy.revision = 2;
    database.upsert_source_policy(&newer_policy).await.unwrap();
    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::EmployerDiscovery,
            today,
            grant(
                SourceOperation::EmployerDiscovery,
                SourcePermission::UserReview,
            ),
        )
        .await,
        Err(FoundationError::Conflict)
    );
}

#[tokio::test]
async fn visible_page_capture_requires_persisted_pairing_authority() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_user_source_actions(&database).await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::VisiblePageCapture,
            today,
            SourceGrantState::Missing,
        )
        .await
        .unwrap(),
        SourceActionDecision::ReviewRequired
    );
    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::VisiblePageCapture,
            today,
            grant(
                SourceOperation::VisiblePageCapture,
                SourcePermission::PairedBrowserGrant,
            ),
        )
        .await
        .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
}

#[tokio::test]
async fn smart_paste_and_applied_logging_use_distinct_persisted_authority() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    install_user_source_actions(&database).await.unwrap();
    let today = NaiveDate::from_ymd_opt(2026, 7, 19).unwrap();

    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::SmartPaste,
            today,
            grant(SourceOperation::SmartPaste, SourcePermission::UserReview),
        )
        .await
        .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::AppliedLogging,
            today,
            grant(
                SourceOperation::AppliedLogging,
                SourcePermission::PairedBrowserGrant,
            ),
        )
        .await
        .unwrap(),
        SourceActionDecision::Allowed {
            request_limit_per_hour: 0,
            connectivity_required: false,
        }
    );
    assert_eq!(
        authorize_user_source_action(
            &database,
            SourceOperation::AppliedLogging,
            today,
            grant(
                SourceOperation::VisiblePageCapture,
                SourcePermission::PairedBrowserGrant,
            ),
        )
        .await
        .unwrap(),
        SourceActionDecision::Revoked
    );
}

#[tokio::test]
async fn startup_installs_other_source_governance_after_one_source_conflicts() {
    let database = Database::connect_memory().await.unwrap();
    database.migrate().await.unwrap();
    let mut conflict = usajobs_policy().unwrap();
    conflict.policy_ref = "conflicting-usajobs-policy".to_string();
    set_source_policy(&database, &conflict).await.unwrap();

    install_startup_source_governance(&database).await;

    assert!(database
        .get_source_manifest("usajobs")
        .await
        .unwrap()
        .is_none());
    assert!(database
        .get_source_manifest("user-source-actions")
        .await
        .unwrap()
        .is_some());
    assert!(database
        .get_source_manifest("lever")
        .await
        .unwrap()
        .is_some());
}
