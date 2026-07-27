//! Lock maturity boundary condition unit tests for the Savings Vault contract.
//!
//! These tests systematically verify exact time boundary behaviors for locked funds:
//! 1. `current_time == unlock_time - 1` (1 second BEFORE maturity): Rejected.
//! 2. `current_time == unlock_time` (EXACT maturity second): Allowed.
//! 3. `current_time == unlock_time + 1` (1 second AFTER maturity): Allowed.
//! 4. `current_time > unlock_time + N` (Long after maturity): Allowed.
//! 5. Transition checks: `can_withdraw` flips from `false` to `true` at exact maturity second.

extern crate std;

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

struct LockBoundaryFixture {
    env: Env,
    contract_id: Address,
    client: SavingsVaultClient<'static>,
    user: Address,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup_boundary_fixture(initial_time: u64) -> LockBoundaryFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, initial_time);
    token_admin.mint(&user, &10_000);
    client.deposit(&user, &5_000);

    LockBoundaryFixture {
        env,
        contract_id,
        client,
        user,
        token_client,
        token_admin,
    }
}

// =========================================================================
// 1. One Second Before Maturity (Rejection)
// =========================================================================

/// Verifies that calling `withdraw_lock` at `unlock_time - 1` is strictly rejected
/// and `can_withdraw` returns false.
#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_boundary_one_second_before_maturity_rejected() {
    let f = setup_boundary_fixture(1_000);
    let unlock_time: u64 = 5_000;
    let lock_id = f.client.lock_funds(&f.user, &1_000, &unlock_time);

    // Set time to 4,999 (1 second before maturity)
    set_ledger_timestamp(&f.env, unlock_time - 1);

    // can_withdraw must return false
    assert!(
        !f.client.can_withdraw(&f.user),
        "can_withdraw must be false 1 second before maturity"
    );

    // Attempting to withdraw must panic
    f.client.withdraw_lock(&f.user, &lock_id);
}

// =========================================================================
// 2. Exact Maturity Second (Success)
// =========================================================================

/// Verifies that calling `withdraw_lock` at `unlock_time` (exact second of maturity)
/// succeeds, updates state, and transfers tokens.
#[test]
fn test_boundary_exact_maturity_second_succeeds() {
    let f = setup_boundary_fixture(1_000);
    let unlock_time: u64 = 5_000;
    let lock_amount: i128 = 1_200;
    let lock_id = f.client.lock_funds(&f.user, &lock_amount, &unlock_time);

    // Set time to exact maturity second (5,000)
    set_ledger_timestamp(&f.env, unlock_time);

    // can_withdraw must return true at exact maturity
    assert!(
        f.client.can_withdraw(&f.user),
        "can_withdraw must be true at exact maturity second"
    );

    // Withdrawal must succeed
    f.client.withdraw_lock(&f.user, &lock_id);

    // Verify lock state updated
    let lock_entry = f.client.get_lock(&f.user, &lock_id).unwrap();
    assert!(lock_entry.withdrawn);
    assert_eq!(lock_entry.amount, 0);

    // Verify token balance received
    assert_eq!(f.token_client.balance(&f.user), 5_000); // 3800 available + 1200 withdrawn lock
}

// =========================================================================
// 3. One Second After Maturity (Success)
// =========================================================================

/// Verifies that calling `withdraw_lock` at `unlock_time + 1` succeeds.
#[test]
fn test_boundary_one_second_after_maturity_succeeds() {
    let f = setup_boundary_fixture(1_000);
    let unlock_time: u64 = 5_000;
    let lock_id = f.client.lock_funds(&f.user, &800, &unlock_time);

    // Set time to 5,001 (1 second after maturity)
    set_ledger_timestamp(&f.env, unlock_time + 1);

    assert!(f.client.can_withdraw(&f.user));

    f.client.withdraw_lock(&f.user, &lock_id);

    let lock_entry = f.client.get_lock(&f.user, &lock_id).unwrap();
    assert!(lock_entry.withdrawn);
}

// =========================================================================
// 4. Long After Maturity (Success)
// =========================================================================

/// Verifies that calling `withdraw_lock` far into the future past maturity succeeds.
#[test]
fn test_boundary_long_after_maturity_succeeds() {
    let f = setup_boundary_fixture(1_000);
    let unlock_time: u64 = 5_000;
    let lock_id = f.client.lock_funds(&f.user, &500, &unlock_time);

    // Set time to 100,000 (95,000 seconds after maturity)
    set_ledger_timestamp(&f.env, 100_000);

    assert!(f.client.can_withdraw(&f.user));

    f.client.withdraw_lock(&f.user, &lock_id);

    let lock_entry = f.client.get_lock(&f.user, &lock_id).unwrap();
    assert!(lock_entry.withdrawn);
}

// =========================================================================
// 5. State Transition across Exact Boundary
// =========================================================================

/// Tests the step-by-step state transition of `can_withdraw` and lock retrieval
/// as timestamp advances from immature to exact maturity.
#[test]
fn test_boundary_transition_step_by_step() {
    let f = setup_boundary_fixture(1_000);
    let unlock_time: u64 = 3_000;
    let lock_id = f.client.lock_funds(&f.user, &1_000, &unlock_time);

    // Step 1: Creation time (1,000) -> Immature
    set_ledger_timestamp(&f.env, 1_000);
    assert!(!f.client.can_withdraw(&f.user));

    // Step 2: Midpoint (2,000) -> Immature
    set_ledger_timestamp(&f.env, 2_000);
    assert!(!f.client.can_withdraw(&f.user));

    // Step 3: One second before maturity (2,999) -> Immature
    set_ledger_timestamp(&f.env, 2_999);
    assert!(!f.client.can_withdraw(&f.user));

    // Step 4: Exact maturity (3,000) -> Matured
    set_ledger_timestamp(&f.env, 3_000);
    assert!(
        f.client.can_withdraw(&f.user),
        "can_withdraw must transition to true at exact maturity second 3,000"
    );

    // Execute withdrawal successfully
    f.client.withdraw_lock(&f.user, &lock_id);

    // Step 5: Post-withdrawal at maturity second -> can_withdraw becomes false (lock withdrawn)
    assert!(
        !f.client.can_withdraw(&f.user),
        "can_withdraw must return false after the only matured lock is withdrawn"
    );
}

// =========================================================================
// 6. Creation Time Boundary Checks (lock_funds)
// =========================================================================

/// Verifies that creating a lock at `unlock_time == current_time` (zero duration) is rejected.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_boundary_lock_creation_same_timestamp_rejected() {
    let f = setup_boundary_fixture(2_000);
    // Attempting unlock_time == current_time (2,000)
    f.client.lock_funds(&f.user, &500, &2_000);
}

/// Verifies that creating a lock in the past (`unlock_time < current_time`) is rejected.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_boundary_lock_creation_past_timestamp_rejected() {
    let f = setup_boundary_fixture(2_000);
    // Attempting unlock_time in the past (1,999)
    f.client.lock_funds(&f.user, &500, &1_999);
}

/// Verifies that creating a lock 1 second in the future (`unlock_time == current_time + 1`) succeeds.
#[test]
fn test_boundary_lock_creation_minimum_future_duration_succeeds() {
    let f = setup_boundary_fixture(2_000);
    // Minimum future unlock_time (2,001)
    let lock_id = f.client.lock_funds(&f.user, &500, &2_001);
    assert_eq!(lock_id, 1);

    // Immature at T = 2,000
    assert!(!f.client.can_withdraw(&f.user));

    // Matures at T = 2,001
    set_ledger_timestamp(&f.env, 2_001);
    assert!(f.client.can_withdraw(&f.user));
    f.client.withdraw_lock(&f.user, &lock_id);
}

// =========================================================================
// 7. Multiple Locks Independent Maturity Boundaries
// =========================================================================

/// Verifies that multiple locks with different maturity timestamps evaluate independently
/// at their exact boundary thresholds.
#[test]
fn test_multiple_locks_independent_boundaries() {
    let f = setup_boundary_fixture(1_000);

    let lock_1 = f.client.lock_funds(&f.user, &400, &2_000);
    let lock_2 = f.client.lock_funds(&f.user, &600, &4_000);

    // T = 1,999: Both locks immature
    set_ledger_timestamp(&f.env, 1_999);
    assert!(!f.client.can_withdraw(&f.user));

    // T = 2,000: Lock 1 exact maturity, Lock 2 immature
    set_ledger_timestamp(&f.env, 2_000);
    assert!(f.client.can_withdraw(&f.user));

    // Lock 1 succeeds
    f.client.withdraw_lock(&f.user, &lock_1);

    // Lock 2 still immature at T = 2,000 -> can_withdraw is false (Lock 1 withdrawn, Lock 2 immature)
    assert!(!f.client.can_withdraw(&f.user));

    // T = 3,999: Lock 2 still immature 1 sec prior
    set_ledger_timestamp(&f.env, 3_999);
    assert!(!f.client.can_withdraw(&f.user));

    // T = 4,000: Lock 2 exact maturity
    set_ledger_timestamp(&f.env, 4_000);
    assert!(f.client.can_withdraw(&f.user));
    f.client.withdraw_lock(&f.user, &lock_2);
}
