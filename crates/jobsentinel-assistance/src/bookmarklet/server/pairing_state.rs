use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use jobsentinel_domain::v3_source_authorization::SourceGrantState;

use crate::bookmarklet::{CompanionPairing, CompanionPairingError, CompanionRequest};

pub(super) type ActiveCompanionPairing = Arc<RwLock<Option<CompanionPairing>>>;

pub(super) fn new_active_pairing() -> ActiveCompanionPairing {
    Arc::new(RwLock::new(None))
}

pub(super) fn replace_active_pairing(active: &ActiveCompanionPairing, pairing: CompanionPairing) {
    let mut active = match active.write() {
        Ok(active) => active,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(mut previous) = active.replace(pairing) {
        previous.revoke();
    }
}

pub(super) fn revoke_active_pairing(active: &ActiveCompanionPairing) {
    let mut active = match active.write() {
        Ok(active) => active,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(mut pairing) = active.take() {
        pairing.revoke();
    }
}

pub(super) fn active_pairing_is_current(
    active: &ActiveCompanionPairing,
    now: DateTime<Utc>,
) -> bool {
    let active = match active.read() {
        Ok(active) => active,
        Err(poisoned) => poisoned.into_inner(),
    };
    active
        .as_ref()
        .is_some_and(|pairing| pairing.is_current(now))
}

pub(super) fn consume_active_pairing(
    active: &ActiveCompanionPairing,
    request: &CompanionRequest,
    now: DateTime<Utc>,
) -> Result<SourceGrantState, CompanionPairingError> {
    let mut active = match active.write() {
        Ok(active) => active,
        Err(poisoned) => poisoned.into_inner(),
    };
    let grant = active
        .as_mut()
        .ok_or(CompanionPairingError::Revoked)?
        .authorize(request, now)?;
    if let Some(mut pairing) = active.take() {
        pairing.revoke();
    }
    Ok(grant)
}
