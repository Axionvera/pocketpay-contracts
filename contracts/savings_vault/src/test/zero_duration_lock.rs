//! Zero-duration and minimal-duration lock tests for the Savings Vault contract.
//!
//! `lock_funds` validates `unlock_time` with `unlock_time <= current_time`,
//! which means the check is **inclusive**: a lock whose `unlock_time` equals
//! the current ledger timestamp (a zero-second duration) is rejected exactly
//! like a lock in the past. There is no code path that allows creating a lock
//! that is already matured the instant it is created.
//!
//! This module documents and verifies that behaviour explicitly, and then
//! exercises the smallest duration the contract *does* accept — one second —
//! to confirm such a lock stays locked at creation time and matures (and
//! becomes withdrawable) as soon as the ledger timestamp advances to it.

use super::test_helpers::*;
use soroban_sdk::{testutils::Address as _, Address};

// ---------------------------------------------------------------------------
// Zero-duration locks (unlock_time == current_time) are rejected
// ---------------------------------------------------------------------------

/// A lock whose `unlock_time` equals the current ledger timestamp has a
/// duration of zero seconds and must be rejected with the same message used
/// for past timestamps, since the contract's check is `<=`, not `<`.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_zero_duration_unlock_time_equals_now_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    // Zero-second duration: unlock_time == current_time.
    client.lock_funds(&user, &50, &1_000);
}

/// Same as above, confirmed at a different (non-zero) ledger timestamp so the
/// boundary isn't coincidentally tied to timestamp `0`.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_zero_duration_at_nonzero_timestamp_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 999_999);
    deposit_balance(&client, &user, 100);

    client.lock_funds(&user, &50, &999_999);
}

/// A zero-duration lock attempt must be rejected before any state mutation:
/// the available balance is unchanged and no lock entry is created.
#[test]
fn test_lock_zero_duration_does_not_mutate_available_or_locked_balance() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    let balance_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.lock_funds(&user, &50, &1_000);
    }));

    assert!(
        result.is_err(),
        "expected zero-duration lock to be rejected"
    );
    assert_eq!(
        client.get_balance(&user),
        balance_before,
        "available balance must not change after a rejected zero-duration lock"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        locked_before,
        "locked balance must not change after a rejected zero-duration lock"
    );
    assert_eq!(
        client.can_withdraw(&user),
        false,
        "no lock entry should have been created"
    );
}

/// Depositing zero-duration-rejected funds still leaves the balance fully
/// available for a subsequent valid operation, proving the failed lock left
/// no partial state behind.
#[test]
fn test_lock_zero_duration_panic_allows_subsequent_valid_lock() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.lock_funds(&user, &50, &1_000);
    }));
    assert!(result.is_err());

    // The full 100 is still available, so a valid lock for the full amount
    // should succeed with no leftover effects from the rejected attempt.
    client.lock_funds(&user, &100, &2_000);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 100);
}

// ---------------------------------------------------------------------------
// Minimal duration (one second) — the shortest lock the contract accepts
// ---------------------------------------------------------------------------

/// `unlock_time == current_time + 1` is the shortest duration `lock_funds`
/// accepts. It must succeed and, at the moment of creation, remain locked
/// (not yet matured).
#[test]
fn test_lock_minimal_one_second_duration_succeeds_and_is_not_yet_matured() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    client.lock_funds(&user, &100, &1_001);

    // Still at T=1_000, one second before unlock: locked, not withdrawable.
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 100);
    assert_eq!(client.can_withdraw(&user), false);
}

/// Once the ledger timestamp advances by exactly the lock's one-second
/// duration, the lock matures: it becomes withdrawable and its amount moves
/// from locked to available balance.
#[test]
fn test_lock_minimal_one_second_duration_matures_after_advancing_one_second() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    client.lock_funds(&user, &100, &1_001);

    set_ledger_timestamp(&env, 1_001);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 100);
}

/// A matured minimal-duration lock's funds can be withdrawn in full
/// immediately after maturity, exercising the full deposit -> lock -> mature
/// -> withdraw lifecycle for the shortest possible lock duration.
#[test]
fn test_withdraw_succeeds_immediately_after_minimal_duration_lock_matures() {
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);

    client.deposit(&user, &100);
    token_client.transfer(&user, &current_contract_address, &100);

    // Lock the entire balance for the shortest possible duration.
    client.lock_funds(&user, &100, &1_001);
    assert_eq!(client.get_balance(&user), 0);

    // Advance by exactly one second: the lock matures.
    set_ledger_timestamp(&env, 1_001);
    assert_eq!(client.can_withdraw(&user), true);

    // The full matured amount is withdrawable right away.
    client.withdraw(&user, &100);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
}
