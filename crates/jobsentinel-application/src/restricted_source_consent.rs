use chrono::{DateTime, Utc};
use jobsentinel_domain::{
    v3_foundation::{SourceAccess, SourcePolicy},
    v3_manifests::{DataCategory, SourceClass},
    v3_source_consent::{
        SourceConsentContext, SourceConsentOperation, SourceConsentReviewReason,
        SourceConsentStatus,
    },
    v3_source_manifest::{
        parse_source_manifest, BUILTIN_SOURCE_MANIFEST_V2, DICE_SOURCE_MANIFEST_V2,
        GLASSDOOR_SOURCE_MANIFEST_V2, SIMPLYHIRED_SOURCE_MANIFEST_V2,
    },
};
use jobsentinel_storage::Database;
use sha2::{Digest, Sha256};

use crate::{
    v3_foundation::{
        review_source_consent, revoke_source_consent, set_source_policy, FoundationError,
    },
    Config,
};

const RESTRICTED_SOURCE_IDS: [&str; 4] = ["builtin", "dice", "simplyhired", "glassdoor"];
const POLICY_REVISION: u32 = 2;
const WARNING_VERSION: u32 = 1;
const BEHAVIOR_REVISION: u32 = 1;
const POLICY_REVIEWED_AT: &str = "2026-07-19T00:00:00Z";

pub async fn review_restricted_source_consent(
    database: &Database,
    config: &Config,
    source_id: &str,
) -> Result<SourceConsentStatus, FoundationError> {
    let context = consent_context(config, source_id)?;
    review_source_consent(database, &context).await
}

pub async fn refresh_restricted_source_acknowledgements(
    database: &Database,
    config: &mut Config,
) -> Result<(), FoundationError> {
    for source_id in RESTRICTED_SOURCE_IDS {
        ensure_current_policy(database, source_id).await?;
        match review_restricted_source_consent(database, config, source_id).await? {
            SourceConsentStatus::Remembered { .. } => {
                revoke_source_consent(database, source_id).await?;
            }
            SourceConsentStatus::ReviewRequired {
                reason: SourceConsentReviewReason::ContextChanged,
                latest_event_id: Some(_),
            } => {
                revoke_source_consent(database, source_id).await?;
            }
            SourceConsentStatus::ReviewRequired { .. } => {}
        }
        retire_source_config(config, source_id);
    }
    Ok(())
}

pub async fn reconcile_restricted_source_consents(
    database: &Database,
    _previous: &Config,
    requested: &mut Config,
) -> Result<(), FoundationError> {
    refresh_restricted_source_acknowledgements(database, requested).await
}

fn consent_context(
    config: &Config,
    source_id: &str,
) -> Result<SourceConsentContext, FoundationError> {
    let policy = current_policy(source_id)?;
    Ok(SourceConsentContext {
        source_id: source_id.to_string(),
        operation: SourceConsentOperation::ScheduledCheck,
        warning_version: WARNING_VERSION,
        behavior_revision: BEHAVIOR_REVISION,
        policy_ref: policy.policy_ref,
        policy_revision: policy.revision,
        source_class: policy.source_class,
        data_categories: data_categories(source_id)?,
        destination_sha256: hash_parts(&[destination(source_id)?.as_bytes()])?,
        request_sha256: request_hash(config, source_id)?,
    })
}

fn current_policy(source_id: &str) -> Result<SourcePolicy, FoundationError> {
    let reviewed_at = POLICY_REVIEWED_AT
        .parse::<DateTime<Utc>>()
        .map_err(|_| FoundationError::InvalidInput)?;
    Ok(SourcePolicy {
        source_id: source_id.to_string(),
        source_class: SourceClass::RestrictedPublicScheduled,
        access: SourceAccess::Disabled,
        request_limit_per_hour: 0,
        user_review_required: true,
        policy_ref: format!("jobsentinel.source-policy.{source_id}.scheduled"),
        revision: POLICY_REVISION,
        restriction_reason_code: Some("provider-automation-prohibited".to_string()),
        reviewed_at,
    })
}

async fn ensure_current_policy(
    database: &Database,
    source_id: &str,
) -> Result<(), FoundationError> {
    let expected = current_policy(source_id)?;
    match database
        .get_source_policy(source_id)
        .await
        .map_err(|_| FoundationError::Storage("source-policy"))?
    {
        Some(stored) if stored == expected => Ok(()),
        Some(stored) if stored.revision >= expected.revision => Err(FoundationError::Conflict),
        _ => {
            let stored = set_source_policy(database, &expected).await?;
            if stored == expected {
                Ok(())
            } else {
                Err(FoundationError::Conflict)
            }
        }
    }?;

    let raw_manifest = match source_id {
        "builtin" => BUILTIN_SOURCE_MANIFEST_V2,
        "dice" => DICE_SOURCE_MANIFEST_V2,
        "simplyhired" => SIMPLYHIRED_SOURCE_MANIFEST_V2,
        "glassdoor" => GLASSDOOR_SOURCE_MANIFEST_V2,
        _ => return Err(FoundationError::InvalidInput),
    };
    let expected_manifest = parse_source_manifest(raw_manifest, &expected)
        .map_err(|_| FoundationError::InvalidInput)?;
    match database
        .get_source_manifest(source_id)
        .await
        .map_err(|_| FoundationError::Storage("source-manifest"))?
    {
        Some(stored) if stored == expected_manifest => Ok(()),
        Some(stored) if stored.policy_revision >= expected_manifest.policy_revision => {
            Err(FoundationError::Conflict)
        }
        _ => database
            .store_source_manifest(&expected_manifest)
            .await
            .map_err(|_| FoundationError::Storage("source-manifest")),
    }
}

fn destination(source_id: &str) -> Result<&'static str, FoundationError> {
    match source_id {
        "builtin" => Ok("https://builtin.com"),
        "dice" => Ok("https://www.dice.com"),
        "simplyhired" => Ok("https://www.simplyhired.com"),
        "glassdoor" => Ok("https://www.glassdoor.com"),
        _ => Err(FoundationError::InvalidInput),
    }
}

fn data_categories(source_id: &str) -> Result<Vec<DataCategory>, FoundationError> {
    match source_id {
        "builtin" => Ok(vec![
            DataCategory::PublicJobPosting,
            DataCategory::CareerGoals,
        ]),
        "dice" | "simplyhired" | "glassdoor" => Ok(vec![
            DataCategory::PublicJobPosting,
            DataCategory::CareerGoals,
            DataCategory::LocationPreferences,
        ]),
        _ => Err(FoundationError::InvalidInput),
    }
}

fn request_hash(config: &Config, source_id: &str) -> Result<String, FoundationError> {
    match source_id {
        "builtin" => hash_parts(&[
            source_id.as_bytes(),
            bool_bytes(config.builtin.enabled),
            bool_bytes(config.builtin.remote_only),
            config.builtin.limit.to_string().as_bytes(),
        ]),
        "dice" => hash_query_request(
            source_id,
            config.dice.enabled,
            &config.dice.query,
            config.dice.location.as_deref(),
            config.dice.limit,
        ),
        "simplyhired" => hash_query_request(
            source_id,
            config.simplyhired.enabled,
            &config.simplyhired.query,
            config.simplyhired.location.as_deref(),
            config.simplyhired.limit,
        ),
        "glassdoor" => hash_query_request(
            source_id,
            config.glassdoor.enabled,
            &config.glassdoor.query,
            config.glassdoor.location.as_deref(),
            config.glassdoor.limit,
        ),
        _ => Err(FoundationError::InvalidInput),
    }
}

fn hash_query_request(
    source_id: &str,
    enabled: bool,
    query: &str,
    location: Option<&str>,
    limit: usize,
) -> Result<String, FoundationError> {
    let limit = limit.to_string();
    let (location_state, location) = match location {
        Some(location) => (b"some".as_slice(), location),
        None => (b"none".as_slice(), ""),
    };
    hash_parts(&[
        source_id.as_bytes(),
        bool_bytes(enabled),
        query.as_bytes(),
        location_state,
        location.as_bytes(),
        limit.as_bytes(),
    ])
}

fn hash_parts(parts: &[&[u8]]) -> Result<String, FoundationError> {
    let mut hasher = Sha256::new();
    for part in parts {
        let length = u64::try_from(part.len()).map_err(|_| FoundationError::InvalidInput)?;
        hasher.update(length.to_be_bytes());
        hasher.update(part);
    }
    Ok(hex::encode(hasher.finalize()))
}

const fn bool_bytes(value: bool) -> &'static [u8] {
    if value {
        b"true"
    } else {
        b"false"
    }
}

fn retire_source_config(config: &mut Config, source_id: &str) {
    match source_id {
        "builtin" => {
            config.builtin.enabled = false;
            config.restricted_source_acknowledgements.builtin = false;
        }
        "dice" => {
            config.dice.enabled = false;
            config.restricted_source_acknowledgements.dice = false;
        }
        "simplyhired" => {
            config.simplyhired.enabled = false;
            config.restricted_source_acknowledgements.simplyhired = false;
        }
        "glassdoor" => {
            config.glassdoor.enabled = false;
            config.restricted_source_acknowledgements.glassdoor = false;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use jobsentinel_domain::{
        v3_foundation::SourceAccess,
        v3_source_consent::{SourceConsentReviewReason, SourceConsentStatus},
    };
    use jobsentinel_storage::Database;

    use crate::Config;

    use super::{
        reconcile_restricted_source_consents, refresh_restricted_source_acknowledgements,
        review_restricted_source_consent,
    };

    async fn database() -> Database {
        let database = Database::connect_memory().await.unwrap();
        database.migrate().await.unwrap();
        database
    }

    fn dice_config() -> Config {
        let mut config = Config::first_run();
        config.dice.enabled = true;
        config.dice.query = "security analyst".to_string();
        config.dice.location = Some("Remote".to_string());
        config.dice.limit = 25;
        config
    }

    #[tokio::test]
    async fn legacy_boolean_cannot_authorize_a_restricted_source() {
        let database = database().await;
        let mut config = dice_config();
        config.restricted_source_acknowledgements.dice = true;

        let status = review_restricted_source_consent(&database, &config, "dice")
            .await
            .unwrap();
        assert!(matches!(
            status,
            SourceConsentStatus::ReviewRequired {
                reason: SourceConsentReviewReason::ContextChanged,
                latest_event_id: None
            }
        ));

        refresh_restricted_source_acknowledgements(&database, &mut config)
            .await
            .unwrap();
        assert!(!config.restricted_source_acknowledgements.dice);
    }

    #[tokio::test]
    async fn provider_policy_retirement_cannot_be_overridden_by_local_consent() {
        let database = database().await;
        let previous = dice_config();
        let mut requested = previous.clone();
        requested.builtin.enabled = true;
        requested.simplyhired.enabled = true;
        requested.glassdoor.enabled = true;
        requested.restricted_source_acknowledgements.builtin = true;
        requested.restricted_source_acknowledgements.dice = true;
        requested.restricted_source_acknowledgements.simplyhired = true;
        requested.restricted_source_acknowledgements.glassdoor = true;

        reconcile_restricted_source_consents(&database, &previous, &mut requested)
            .await
            .unwrap();

        assert!(!requested.restricted_source_acknowledgements.builtin);
        assert!(!requested.restricted_source_acknowledgements.dice);
        assert!(!requested.restricted_source_acknowledgements.simplyhired);
        assert!(!requested.restricted_source_acknowledgements.glassdoor);
        assert!(!requested.builtin.enabled);
        assert!(!requested.dice.enabled);
        assert!(!requested.simplyhired.enabled);
        assert!(!requested.glassdoor.enabled);
        for source_id in ["builtin", "dice", "simplyhired", "glassdoor"] {
            let policy = database
                .get_source_policy(source_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(policy.access, SourceAccess::Disabled);
            assert_eq!(policy.revision, 2);
            assert!(database
                .get_source_manifest(source_id)
                .await
                .unwrap()
                .is_some());
            assert!(!matches!(
                review_restricted_source_consent(&database, &requested, source_id)
                    .await
                    .unwrap(),
                SourceConsentStatus::Remembered { .. }
            ));
        }
    }
}
