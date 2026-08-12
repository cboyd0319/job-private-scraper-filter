use chrono::{DateTime, NaiveDate, Utc};
use jobsentinel_domain::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::SourceClass,
    v3_source_authorization::{SourceActionDecision, SourceGrantState},
    v3_source_manifest::{
        parse_source_manifest, SourceOperation, GREENHOUSE_REQUEST_LIMIT_PER_HOUR,
        GREENHOUSE_SOURCE_MANIFEST_V1, HN_HIRING_REQUEST_LIMIT_PER_HOUR,
        HN_HIRING_SOURCE_MANIFEST_V1, JOBSWITHGPT_SOURCE_MANIFEST_V1, LEVER_REQUEST_LIMIT_PER_HOUR,
        LEVER_SOURCE_MANIFEST_V1, LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1,
        REMOTEOK_REQUEST_LIMIT_PER_HOUR, REMOTEOK_SOURCE_MANIFEST_V1,
        USAJOBS_REQUEST_LIMIT_PER_HOUR, USAJOBS_SOURCE_MANIFEST_V1,
        USER_SOURCE_ACTIONS_MANIFEST_V1, WEWORKREMOTELY_REQUEST_LIMIT_PER_HOUR,
        WEWORKREMOTELY_SOURCE_MANIFEST_V1,
    },
};
use jobsentinel_storage::Database;

use crate::v3_foundation::{map_error, set_source_policy, FoundationError};

const POLICY_REVIEWED_AT: &str = "2026-07-19T00:00:00Z";

fn reviewed_policy(
    source_id: &str,
    source_class: SourceClass,
    policy_ref: &str,
    request_limit_per_hour: u16,
) -> Result<SourcePolicy, FoundationError> {
    Ok(SourcePolicy {
        source_id: source_id.to_string(),
        source_class,
        access: SourceAccess::ScheduledPublic,
        request_limit_per_hour,
        user_review_required: false,
        policy_ref: policy_ref.to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: POLICY_REVIEWED_AT
            .parse::<DateTime<Utc>>()
            .map_err(|_| FoundationError::InvalidInput)?,
    })
}

pub(crate) fn usajobs_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "usajobs",
        SourceClass::OfficialPublicApi,
        "jobsentinel.source-policy.usajobs.api",
        USAJOBS_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn remoteok_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "remoteok",
        SourceClass::OfficialPublicApi,
        "jobsentinel.source-policy.remoteok.api",
        REMOTEOK_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn weworkremotely_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "weworkremotely",
        SourceClass::OfficialPublicApi,
        "jobsentinel.source-policy.weworkremotely.rss",
        WEWORKREMOTELY_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn hn_hiring_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "hn_hiring",
        SourceClass::OfficialPublicApi,
        "jobsentinel.source-policy.hn-hiring.algolia",
        HN_HIRING_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn greenhouse_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "greenhouse",
        SourceClass::PublicAts,
        "jobsentinel.source-policy.greenhouse.public-job-board-api",
        GREENHOUSE_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn lever_policy() -> Result<SourcePolicy, FoundationError> {
    reviewed_policy(
        "lever",
        SourceClass::PublicAts,
        "jobsentinel.source-policy.lever.public-postings-api",
        LEVER_REQUEST_LIMIT_PER_HOUR,
    )
}

pub(crate) fn jobswithgpt_policy() -> Result<SourcePolicy, FoundationError> {
    Ok(SourcePolicy {
        source_id: "jobswithgpt".to_string(),
        source_class: SourceClass::RestrictedPublicScheduled,
        access: SourceAccess::Disabled,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.jobswithgpt.provider-review".to_string(),
        revision: 1,
        restriction_reason_code: Some("provider-policy-review-required".to_string()),
        reviewed_at: POLICY_REVIEWED_AT
            .parse::<DateTime<Utc>>()
            .map_err(|_| FoundationError::InvalidInput)?,
    })
}

pub(crate) fn linkedin_workbench_policy() -> Result<SourcePolicy, FoundationError> {
    Ok(SourcePolicy {
        source_id: "linkedin-workbench".to_string(),
        source_class: SourceClass::RestrictedUserOpened,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.linkedin-workbench".to_string(),
        revision: 1,
        restriction_reason_code: Some("account-backed-source".to_string()),
        reviewed_at: POLICY_REVIEWED_AT
            .parse::<DateTime<Utc>>()
            .map_err(|_| FoundationError::InvalidInput)?,
    })
}

pub(crate) fn user_source_actions_policy() -> Result<SourcePolicy, FoundationError> {
    Ok(SourcePolicy {
        source_id: "user-source-actions".to_string(),
        source_class: SourceClass::UserImport,
        access: SourceAccess::UserOpened,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: "jobsentinel.source-policy.user-source-actions".to_string(),
        revision: 1,
        restriction_reason_code: None,
        reviewed_at: POLICY_REVIEWED_AT
            .parse::<DateTime<Utc>>()
            .map_err(|_| FoundationError::InvalidInput)?,
    })
}

async fn install_reviewed(
    database: &Database,
    expected: SourcePolicy,
    raw_manifest: &str,
) -> Result<(), FoundationError> {
    match database
        .get_source_policy(&expected.source_id)
        .await
        .map_err(map_error)?
    {
        Some(current) if current.revision > expected.revision => return Ok(()),
        Some(current) if current.revision == expected.revision && current != expected => {
            return Err(FoundationError::Conflict);
        }
        Some(current) if current == expected => {}
        _ => {
            set_source_policy(database, &expected).await?;
        }
    }
    let manifest = parse_source_manifest(raw_manifest, &expected)
        .map_err(|_| FoundationError::InvalidInput)?;
    database
        .store_source_manifest(&manifest)
        .await
        .map_err(map_error)
}

async fn authorize_reviewed(
    database: &Database,
    expected_policy: SourcePolicy,
    raw_manifest: &str,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed_with_grant(
        database,
        expected_policy,
        raw_manifest,
        operation,
        today,
        SourceGrantState::NotRequired,
    )
    .await
}

async fn authorize_reviewed_with_grant(
    database: &Database,
    expected_policy: SourcePolicy,
    raw_manifest: &str,
    operation: SourceOperation,
    today: NaiveDate,
    grant: SourceGrantState,
) -> Result<SourceActionDecision, FoundationError> {
    let policy = database
        .get_source_policy(&expected_policy.source_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    if policy != expected_policy {
        return Err(FoundationError::Conflict);
    }
    let expected_manifest = parse_source_manifest(raw_manifest, &expected_policy)
        .map_err(|_| FoundationError::InvalidInput)?;
    let manifest = database
        .get_source_manifest(&expected_policy.source_id)
        .await
        .map_err(map_error)?
        .ok_or(FoundationError::Conflict)?;
    if manifest != expected_manifest {
        return Err(FoundationError::Conflict);
    }
    manifest
        .authorize(&policy, operation, today, grant)
        .map_err(|_| FoundationError::InvalidInput)
}

pub(crate) async fn install_usajobs(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(database, usajobs_policy()?, USAJOBS_SOURCE_MANIFEST_V1).await
}

pub(crate) async fn authorize_usajobs(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        usajobs_policy()?,
        USAJOBS_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_remoteok(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(database, remoteok_policy()?, REMOTEOK_SOURCE_MANIFEST_V1).await
}

pub(crate) async fn authorize_remoteok(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        remoteok_policy()?,
        REMOTEOK_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_weworkremotely(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(
        database,
        weworkremotely_policy()?,
        WEWORKREMOTELY_SOURCE_MANIFEST_V1,
    )
    .await
}

pub(crate) async fn authorize_weworkremotely(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        weworkremotely_policy()?,
        WEWORKREMOTELY_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_hn_hiring(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(database, hn_hiring_policy()?, HN_HIRING_SOURCE_MANIFEST_V1).await
}

pub(crate) async fn authorize_hn_hiring(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        hn_hiring_policy()?,
        HN_HIRING_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_greenhouse(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(
        database,
        greenhouse_policy()?,
        GREENHOUSE_SOURCE_MANIFEST_V1,
    )
    .await
}

pub(crate) async fn authorize_greenhouse(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        greenhouse_policy()?,
        GREENHOUSE_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_lever(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(database, lever_policy()?, LEVER_SOURCE_MANIFEST_V1).await
}

pub(crate) async fn authorize_lever(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        lever_policy()?,
        LEVER_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}

pub(crate) async fn install_jobswithgpt(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(
        database,
        jobswithgpt_policy()?,
        JOBSWITHGPT_SOURCE_MANIFEST_V1,
    )
    .await
}

pub(crate) async fn install_linkedin_workbench(database: &Database) -> Result<(), FoundationError> {
    install_reviewed(
        database,
        linkedin_workbench_policy()?,
        LINKEDIN_WORKBENCH_SOURCE_MANIFEST_V1,
    )
    .await
}

pub(crate) async fn install_user_source_actions(
    database: &Database,
) -> Result<(), FoundationError> {
    install_reviewed(
        database,
        user_source_actions_policy()?,
        USER_SOURCE_ACTIONS_MANIFEST_V1,
    )
    .await
}

pub(crate) async fn authorize_user_source_action(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
    grant: SourceGrantState,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed_with_grant(
        database,
        user_source_actions_policy()?,
        USER_SOURCE_ACTIONS_MANIFEST_V1,
        operation,
        today,
        grant,
    )
    .await
}

pub(crate) async fn install_startup_source_governance(database: &Database) {
    for (source, result) in [
        ("USAJobs", install_usajobs(database).await),
        ("RemoteOK", install_remoteok(database).await),
        ("We Work Remotely", install_weworkremotely(database).await),
        (
            "Hacker News Who Is Hiring",
            install_hn_hiring(database).await,
        ),
        ("Greenhouse", install_greenhouse(database).await),
        ("Lever", install_lever(database).await),
        ("connected job source", install_jobswithgpt(database).await),
        (
            "LinkedIn Workbench",
            install_linkedin_workbench(database).await,
        ),
        (
            "user-directed source actions",
            install_user_source_actions(database).await,
        ),
    ] {
        if let Err(error) = result {
            tracing::warn!(
                error = %error,
                source,
                "Source governance could not be refreshed; affected actions remain blocked"
            );
        }
    }
}

pub(crate) async fn authorize_jobswithgpt(
    database: &Database,
    operation: SourceOperation,
    today: NaiveDate,
) -> Result<SourceActionDecision, FoundationError> {
    authorize_reviewed(
        database,
        jobswithgpt_policy()?,
        JOBSWITHGPT_SOURCE_MANIFEST_V1,
        operation,
        today,
    )
    .await
}
