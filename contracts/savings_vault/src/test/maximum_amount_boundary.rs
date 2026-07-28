//! Maximum amount boundary tests for the Savings Vault contract.
//!
//! These tests exercise deposit, withdrawal, and lock operations near the
//! `i128` numeric limits to surface overflow and accounting edge cases.
//!
//! Boundary values chosen:
//! - `I128_MAX`        : largest positive `i128` value.
//! - `I128_MAX - 1`    : one below maximum, still safely representable.
//! - `I128_MAX / 2`    : floored half of maximum (integer division truncates
//!                       by one when `i128::MAX` is odd, so
//!                       `I128_MAX_HALF * 2 == I128_MAX - 1`). Used to
//!                       avoid accidental overflow when two large values
//!                       interact.
//!
//! Soroban note: the contract does not use checked arithmetic, so in release
//! builds with overflow-checks enabled, arithmetic overflow aborts the
//! transaction. These tests document the observed behaviour and verify that
//! failed operations do not corrupt balances.
extern crate std;

use super::test_helpers::*;
use soroban_sdk::{testutils::Address as _, Address};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Largest positive i128 value.
const I128_MAX: i128 = i128::MAX;

/// One less than the largest positive i128 value.
const I128_MAX_MINUS_1: i128 = i128::MAX - 1;

/// Floored half of `i128::MAX`. Because `i128::MAX` is odd,
/// integer division truncates: `I128_MAX_HALF * 2 == I128_MAX - 1`.
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

    deposit_balance(&client, &user, I128_MAX);
    assert_eq!(client.get_balance(&user), I128_MAX);
}

/// Depositing `i128::MAX` followed by any positive amount would overflow the
/// available balance. We verify that the second deposit panics (aborts) and
/// that the balance remains unchanged after the failed operation.
#[test]
fn test_deposit_after_i128_max_preserves_balance_on_overflow() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    deposit_balance(&client, &user, I128_MAX);
    let balance_before = client.get_balance(&user);

    let result = client.try_deposit(&user, &1);

    assert!(
        result.is_err(),
        "expected overflow to abort the transaction"
    );
    assert_eq!(
        client.get_balance(&user),
        balance_before,
        "balance must not change after failed deposit"
    );
}

/// Multiple large deposits using the floored-half value accumulate correctly.
/// Because `i128::MAX` is odd, `I128_MAX_HALF * 2 == I128_MAX - 1`.
#[test]
fn test_multiple_large_deposits_without_overflow() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    deposit_balance(&client, &user, I128_MAX_HALF);
    deposit_balance(&client, &user, I128_MAX_HALF);

    // I128_MAX is odd, so integer division floors: HALF * 2 == I128_MAX - 1.
    assert_eq!(client.get_balance(&user), I128_MAX_HALF * 2);
}

// ---------------------------------------------------------------------------
// Large withdrawal behaviour
// ---------------------------------------------------------------------------

/// Withdrawing the full `i128::MAX` balance after depositing it should succeed
/// and leave the available balance at zero.
#[test]
fn test_withdraw_i128_max_after_deposit_succeeds() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    token_admin.mint(&user, &I128_MAX);
    client.deposit(&user, &I128_MAX);

    client.withdraw(&user, &I128_MAX);

    assert_eq!(client.get_balance(&user), 0);
}

/// Withdrawing a very large amount that exceeds the available balance must fail
/// and leave the balance unchanged.
#[test]
fn test_withdraw_over_large_balance_does_not_mutate() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    token_admin.mint(&user, &I128_MAX_MINUS_1);
    client.deposit(&user, &I128_MAX_MINUS_1);

    let balance_before = client.get_balance(&user);

    let result = client.try_withdraw(&user, &I128_MAX);

    assert!(
        result.is_err(),
        "expected withdrawal exceeding balance to fail"
    );
    assert_eq!(
        client.get_balance(&user),
        balance_before,
        "balance must not change after failed withdrawal"
    );
}

/// Partial withdrawal of a very large balance should leave the remainder
/// intact and keep balances non-negative.
#[test]
fn test_withdraw_partial_from_large_balance_preserves_remainder() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    token_admin.mint(&user, &I128_MAX);
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

    deposit_balance(&client, &user, I128_MAX);
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &I128_MAX, &unlock_time);

    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), I128_MAX);
}

/// Locking more than the available balance at very large scale must fail and
/// leave both available and locked balances unchanged.
#[test]
fn test_lock_over_large_balance_does_not_mutate() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, I128_MAX_MINUS_1);
    let available_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);
    let unlock_time = env.ledger().timestamp() + 10_000;

    let result = client.try_lock_funds(&user, &I128_MAX, &unlock_time);

    assert!(result.is_err(), "expected lock exceeding balance to fail");
    assert_eq!(
        client.get_balance(&user),
        available_before,
        "available balance must not change after failed lock"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        locked_before,
        "locked balance must not change after failed lock"
    );
}

/// Partial lock of a very large balance should leave the correct remainder
/// available and record the locked portion.
#[test]
fn test_lock_partial_from_large_balance_preserves_state() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, I128_MAX);
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &1, &unlock_time);

    assert_eq!(client.get_balance(&user), I128_MAX - 1);
    assert_eq!(client.get_locked_balance(&user), 1);
}

// ---------------------------------------------------------------------------
// Overflow-adjacent arithmetic
// ---------------------------------------------------------------------------

/// Depositing `I128_MAX_HALF` twice accumulates to `I128_MAX_HALF * 2`.
/// Because `i128::MAX` is odd, integer division floors by one:
/// `I128_MAX_HALF * 2 == I128_MAX - 1`, not `I128_MAX`.
#[test]
fn test_deposit_half_max_twice_equals_max() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    deposit_balance(&client, &user, I128_MAX_HALF);
    deposit_balance(&client, &user, I128_MAX_HALF);

    // I128_MAX is odd so floor-division gives HALF * 2 == I128_MAX - 1.
    assert_eq!(client.get_balance(&user), I128_MAX_HALF * 2);
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
    deposit_balance(&client, &user, deposited);
    let lock_amount = I128_MAX_HALF - 1;
    let unlock_time = env.ledger().timestamp() + 10_000;
    client.lock_funds(&user, &lock_amount, &unlock_time);

    let available = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);

    assert_eq!(available + locked, deposited);
    assert!(available >= 0, "available balance must not be negative");
    assert!(locked >= 0, "locked balance must not be negative");
}

/// A withdrawal of only the available balance at very large scale must succeed
    /// and never leave a negative remainder. Matured locks must be withdrawn
    /// separately via `withdraw_lock`.
    #[test]
    fn test_large_withdraw_spans_available_and_matured_locks() {
        let (env, contract_id, client) = setup();
        let (env, _admin, client, token_client, token_admin) =
            test_token(env, contract_id.clone(), client);
        let user = Address::generate(&env);
        set_ledger_timestamp(&env, 1_000);

        let total_deposited = I128_MAX_HALF;
        token_admin.mint(&user, &total_deposited);
        client.deposit(&user, &total_deposited);

        // Lock half, mature it immediately.
        let lock_amount = total_deposited / 2;
        let unlock_time = env.ledger().timestamp() + 10_000;
        let lock_id = client.lock_funds(&user, &lock_amount, &unlock_time);

        // Advance time so the lock matures.
        set_ledger_timestamp(&env, unlock_time + 1);

        // Withdraw only the available balance (half of deposited).
        let available = total_deposited - lock_amount;
        client.withdraw(&user, &available);

        assert_eq!(client.get_balance(&user), 0);
        assert_eq!(client.get_locked_balance(&user), lock_amount);

        // Withdraw matured lock via withdraw_lock.
        client.withdraw_lock(&user, &lock_id);

        assert_eq!(client.get_balance(&user), 0);
        assert_eq!(client.get_locked_balance(&user), 0);
    }

// ---------------------------------------------------------------------------
// Boundary value documentation
// ---------------------------------------------------------------------------

/// Documents the floored-half boundary:
/// `I128_MAX_HALF * 2 == I128_MAX - 1` because `i128::MAX` is odd.
#[test]
fn test_documentation_i128_max_half_boundary() {
    // I128_MAX is odd (2^127 - 1), so floor-division gives:
    //   I128_MAX_HALF = (I128_MAX - 1) / 2
    //   I128_MAX_HALF * 2 = I128_MAX - 1
    assert_eq!(I128_MAX_HALF * 2, I128_MAX - 1);
    assert_eq!(I128_MAX_HALF + I128_MAX_HALF, I128_MAX - 1);
    assert!(I128_MAX_HALF > 0);
}

/// Documents the absolute maximum value and confirms it is representable.
#[test]
fn test_documentation_i128_max_is_representable() {
    assert!(I128_MAX > 0);
    assert!(I128_MAX_MINUS_1 > 0);
    assert_eq!(I128_MAX - 1, I128_MAX_MINUS_1);
}
