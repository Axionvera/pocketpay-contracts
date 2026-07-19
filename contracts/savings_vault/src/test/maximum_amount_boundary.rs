//! Maximum amount boundary tests for the Savings Vault contract.
//!
//! These tests exercise deposit, withdrawal, and lock operations near the
//! `i128` numeric limits to surface overflow and accounting edge cases.
//!
//! Boundary values chosen:
//! - `I128_MAX`      : largest positive `i128` value.
//! - `I128_MAX - 1`  : one below maximum, still safely representable.
//! - `I128_MAX / 2`  : half of maximum, used to avoid accidental overflow
//!                     when two large values interact.
//!
//! Soroban note: the workspace enables `overflow-checks = true` for the
//! release profile, and Rust debug builds panic on overflow by default.
//! Therefore, arithmetic overflow in this contract currently aborts the
//! transaction rather than silently wrapping, which preserves state
//! consistency.

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Largest positive i128 value.
const I128_MAX: i128 = i128::MAX;

/// One less than the largest positive i128 value.
const I128_MAX_MINUS_1: i128 = i128::MAX - 1;

/// Half of the largest positive i128 value.
const I128_MAX_HALF: i128 = i128::MAX / 2;

// ---------------------------------------------------------------------------
// Large deposit behaviour
// ---------------------------------------------------------------------------

/// Depositing `i128::MAX` into an empty vault should succeed and record the
/// full amount as the available balance.
#[test]
fn test_deposit_i128_max_succeeds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX);
    assert_eq!(client.get_balance(&user), I128_MAX);
}

/// Depositing `i128::MAX` followed by any positive amount would overflow the
/// available balance. With overflow-checks enabled the contract panics, which
/// aborts the transaction and leaves state intact.
#[test]
#[should_panic(expected = "attempt to add with overflow")]
fn test_deposit_after_i128_max_panics_on_overflow() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX);
    client.deposit(&user, &1);
}

/// Multiple large deposits using a safe half-maximum value should accumulate
/// correctly without overflow.
#[test]
fn test_multiple_large_deposits_without_overflow() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX_HALF);
    client.deposit(&user, &I128_MAX_HALF);

    assert_eq!(client.get_balance(&user), I128_MAX);
}

// ---------------------------------------------------------------------------
// Large withdrawal behaviour
// ---------------------------------------------------------------------------

/// Withdrawing the full `i128::MAX` balance after depositing it should succeed
/// and leave the available balance at zero.
#[test]
fn test_withdraw_i128_max_after_deposit_succeeds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX);
    client.withdraw(&user, &I128_MAX);

    assert_eq!(client.get_balance(&user), 0);
}

/// Withdrawing a very large amount that exceeds the available balance must fail
/// and leave the balance unchanged.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_over_large_balance_does_not_mutate() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX_MINUS_1);
    client.withdraw(&user, &I128_MAX);
}

/// Partial withdrawal of a very large balance should leave the remainder
/// intact and keep balances non-negative.
#[test]
fn test_withdraw_partial_from_large_balance_preserves_remainder() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX);
    client.withdraw(&user, &1);

    assert_eq!(client.get_balance(&user), I128_MAX - 1);
    assert!(
        client.get_balance(&user) >= 0,
        "partial withdrawal must not make balance negative"
    );
}

// ---------------------------------------------------------------------------
// Large lock behaviour
// ---------------------------------------------------------------------------

/// Locking the full deposited `i128::MAX` amount should succeed and move all
/// funds to locked state, leaving available balance at zero.
#[test]
fn test_lock_i128_max_succeeds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &I128_MAX);
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &I128_MAX, &unlock_time);

    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), I128_MAX);
}

/// Locking more than the available balance at very large scale must fail and
/// leave both available and locked balances unchanged.
#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_over_large_balance_does_not_mutate() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &I128_MAX_MINUS_1);
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &I128_MAX, &unlock_time);
}

/// Partial lock of a very large balance should leave the correct remainder
/// available and record the locked portion.
#[test]
fn test_lock_partial_from_large_balance_preserves_state() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &I128_MAX);
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &1, &unlock_time);

    assert_eq!(client.get_balance(&user), I128_MAX - 1);
    assert_eq!(client.get_locked_balance(&user), 1);
}

// ---------------------------------------------------------------------------
// Overflow-adjacent arithmetic
// ---------------------------------------------------------------------------

/// Depositing `i128::MAX / 2` twice should exactly equal `i128::MAX` without
/// overflow, proving the contract handles the largest safe sum correctly.
#[test]
fn test_deposit_half_max_twice_equals_max() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &I128_MAX_HALF);
    client.deposit(&user, &I128_MAX_HALF);

    assert_eq!(client.get_balance(&user), I128_MAX);
}

/// After locking a very large amount, `get_balance` and
/// `get_locked_balance` must remain internally consistent: their sum equals
/// the total deposited.
#[test]
fn test_large_lock_keeps_available_and_locked_consistent() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    let deposited = I128_MAX_HALF;
    client.deposit(&user, &deposited);
    let lock_amount = I128_MAX_HALF - 1;
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &lock_amount, &unlock_time);

    let available = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);

    assert_eq!(available + locked, deposited);
    assert!(available >= 0, "available balance must not be negative");
    assert!(locked >= 0, "locked balance must not be negative");
}

/// A withdrawal that spans both available and matured locked funds at very
/// large scale must reduce both balances correctly and never leave a negative
/// remainder.
#[test]
fn test_large_withdraw_spans_available_and_matured_locks() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    let total_deposited = I128_MAX_HALF;
    client.deposit(&user, &total_deposited);

    // Lock half, mature it immediately.
    let lock_amount = total_deposited / 2;
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &lock_amount, &unlock_time);

    // Advance time so the lock matures.
    set_ledger_timestamp(&env, unlock_time + 1);

    // Withdraw everything: available + matured lock.
    client.withdraw(&user, &total_deposited);

    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
}

// ---------------------------------------------------------------------------
// Boundary value documentation
// ---------------------------------------------------------------------------

/// Documents why `i128::MAX / 2` was chosen as a safe large value: adding two
/// of them equals exactly `i128::MAX`, which is the largest representable
/// positive sum without overflow.
#[test]
fn test_documentation_i128_max_half_boundary() {
    assert_eq!(I128_MAX_HALF + I128_MAX_HALF, I128_MAX);
    assert!(I128_MAX_HALF > 0);
}

/// Documents the absolute maximum value and confirms it is representable.
#[test]
fn test_documentation_i128_max_is_representable() {
    assert!(I128_MAX > 0);
    assert!(I128_MAX_MINUS_1 > 0);
    assert_eq!(I128_MAX - 1, I128_MAX_MINUS_1);
}
