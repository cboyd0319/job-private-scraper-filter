// Verifies active companion pairing replacement, expiry, and atomic consumption.

use super::*;
use crate::bookmarklet::{
    companion_request_for_test, CompanionPairing, CompanionPairingCode, CompanionPairingError,
};
use chrono::DateTime;
use jobsentinel_domain::v3_source_manifest::SourceOperation;
use std::sync::Barrier;

fn pairing(origin: &str) -> (CompanionPairing, CompanionPairingCode, DateTime<Utc>) {
    let now = Utc::now();
    let (pairing, code) = CompanionPairing::issue(
        "browser-import",
        "user-source-actions",
        "jobsentinel.source-policy.user-source-actions",
        1,
        origin,
        vec![SourceOperation::VisiblePageCapture],
        now,
    )
    .unwrap();
    (pairing, code, now)
}

#[test]
fn active_pairing_is_atomic_and_one_use() {
    let (pairing, code, now) = pairing("https://jobs.example");
    let active = new_active_pairing();
    replace_active_pairing(&active, pairing);

    assert!(active_pairing_is_current(&active, now));
    assert!(
        consume_active_pairing(&active, &companion_request_for_test(&code, "nonce-1"), now).is_ok()
    );
    assert!(!active_pairing_is_current(&active, now));
    assert_eq!(
        consume_active_pairing(&active, &companion_request_for_test(&code, "nonce-2"), now),
        Err(CompanionPairingError::Revoked)
    );
}

#[test]
fn replacing_pairing_invalidates_the_prior_code() {
    let (first, first_code, now) = pairing("https://jobs.example");
    let (second, second_code, _) = pairing("https://careers.example");
    let active = new_active_pairing();
    replace_active_pairing(&active, first);
    replace_active_pairing(&active, second);

    assert_eq!(
        consume_active_pairing(
            &active,
            &companion_request_for_test(&first_code, "nonce-1"),
            now
        ),
        Err(CompanionPairingError::ScopeMismatch)
    );
    assert!(consume_active_pairing(
        &active,
        &companion_request_for_test(&second_code, "nonce-2"),
        now
    )
    .is_ok());
}

#[test]
fn concurrent_pairing_consumption_allows_one_success() {
    let (pairing, code, now) = pairing("https://jobs.example");
    let active = new_active_pairing();
    replace_active_pairing(&active, pairing);
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|index| {
            let active = active.clone();
            let barrier = barrier.clone();
            let request = companion_request_for_test(&code, &format!("nonce-{index}"));
            std::thread::spawn(move || {
                barrier.wait();
                consume_active_pairing(&active, &request, now).is_ok()
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|consumed| *consumed)
            .count(),
        1
    );
}
