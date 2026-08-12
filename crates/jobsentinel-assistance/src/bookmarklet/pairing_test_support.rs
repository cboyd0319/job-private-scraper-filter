// Builds exact companion requests shared by bookmarklet pairing tests.

use super::{CompanionPairingCode, CompanionRequest};

pub(crate) fn companion_request_for_test(
    code: &CompanionPairingCode,
    nonce: &str,
) -> CompanionRequest {
    CompanionRequest {
        protocol_version: code.protocol_version,
        pairing_id: code.pairing_id.clone(),
        client_id: code.client_id.clone(),
        source_id: code.source_id.clone(),
        policy_ref: code.policy_ref.clone(),
        policy_revision: code.policy_revision,
        operation: code.operations[0],
        origin: code.origin.clone(),
        nonce: nonce.to_string(),
        token: code.token.clone(),
    }
}
