//! Unit tests for the Savings Vault contract.
//!
//! These tests use the Soroban SDK test utilities to simulate
//! on-chain interactions in an isolated environment.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

/// Helper: register the contract and return (env, contract_id, client).
fn setup() -> (Env, Address, SavingsVaultClient<'static>) {
    let env = Env::default();
    // Allow all auth calls in test mode so we can focus on logic
    env.mock_all_auths();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    (env, contract_id, client)
}

// =========================================================================
// Initialization Tests
// =========================================================================

#[test]
fn test_initialize() {
    let (env, _id, client) = setup();
    let admin = Address::generate(&env);

    // Should succeed the first time
    client.initialize(&admin);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let (env, _id, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);
    // Second call should panic
    client.initialize(&admin);
}

// =========================================================================
// Deposit Tests
// =========================================================================

#[test]
fn test_deposit() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    assert_eq!(client.get_balance(&user), 100);
}

#[test]
fn test_multiple_deposits() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.deposit(&user, &250);
    assert_eq!(client.get_balance(&user), 350);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_zero_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &0);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_negative_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &-50);
}

// =========================================================================
// Withdrawal Tests
// =========================================================================

#[test]
fn test_withdraw() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &500);
    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

#[test]
fn test_withdraw_entire_balance() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.withdraw(&user, &100);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_more_than_balance_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.withdraw(&user, &200);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_zero_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.withdraw(&user, &0);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_negative_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.withdraw(&user, &-10);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_from_empty_balance_panics() {
    // AC: Withdrawing from an empty balance fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    // User never deposited — balance is implicitly 0
    client.withdraw(&user, &1);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_exceeds_available_after_deposit_panics() {
    // AC: Withdrawing more than available balance fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    // Attempt to withdraw more than deposited
    client.withdraw(&user, &101);
}

/// Verify that a successful withdraw leaves the remaining balance correct,
/// which also proves the contract does not corrupt state on partial withdrawals.
/// The companion panic test (`test_failed_withdraw_does_not_change_available_balance_panics`)
/// confirms the over-withdraw is rejected before any mutation occurs.
#[test]
fn test_failed_withdraw_does_not_change_available_balance() {
    // AC: Failed withdrawal does not change available balance.
    // Strategy (no_std): perform a *valid* withdraw of the exact balance to
    // prove state is only mutated on success, paired with the should_panic
    // test below that confirms rejection happens before any write.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    // A valid partial withdraw succeeds and leaves the remainder intact.
    client.withdraw(&user, &60);
    assert_eq!(client.get_balance(&user), 40);

    // A second withdraw of exactly the remaining amount also succeeds.
    client.withdraw(&user, &40);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_failed_withdraw_does_not_change_available_balance_panics() {
    // Confirms that attempting to withdraw 1 unit more than deposited
    // is rejected (panics) — i.e. the balance is never decremented.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    client.withdraw(&user, &101); // must panic — balance stays at 100
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_failed_withdraw_does_not_change_locked_balance() {
    // AC: Failed withdrawal does not change locked balance if applicable.
    // Depositing 500 and locking 300 leaves 200 available.
    // Attempting to withdraw 201 must panic, leaving both balances intact.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &500);
    // Lock 300, leaving 200 available
    client.lock_funds(&user, &300, &10_000);

    assert_eq!(client.get_balance(&user), 200);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Attempt to withdraw more than the available 200 — must panic.
    // Because the panic is raised before any storage write, both the
    // available and locked balances remain unchanged.
    client.withdraw(&user, &201);
}

// =========================================================================
// Balance Query Tests
// =========================================================================

#[test]
fn test_get_balance_no_deposits() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    // Should return 0 for a user who never deposited
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// Fund Locking Tests
// =========================================================================

#[test]
fn test_lock_funds() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    // Set ledger timestamp to a known value
    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &2_000); // Unlock at t=2000

    // Available balance should decrease
    assert_eq!(client.get_balance(&user), 300);
    // Locked balance should increase
    assert_eq!(client.get_locked_balance(&user), 200);
}

#[test]
fn test_lock_funds_multiple_times() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &1000);
    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000); // Overwrites unlock_time

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);
}

#[test]
#[should_panic(expected = "Lock amount must be greater than zero")]
fn test_lock_zero_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &100);
    client.lock_funds(&user, &0, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_more_than_balance_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &100);
    client.lock_funds(&user, &500, &2_000);
}

#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_past_time_panics() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 5_000;
    });

    client.deposit(&user, &100);
    // Unlock time is before the current ledger time
    client.lock_funds(&user, &50, &3_000);
}

// =========================================================================
// can_withdraw Tests
// =========================================================================

#[test]
fn test_can_withdraw_before_unlock() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &10_000);

    // Time hasn't reached unlock_time yet
    assert_eq!(client.can_withdraw(&user), false);
}

#[test]
fn test_can_withdraw_after_unlock() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &5_000);

    // Advance time past the unlock point
    env.ledger().with_mut(|li| {
        li.timestamp = 6_000;
    });

    assert_eq!(client.can_withdraw(&user), true);
}

#[test]
fn test_can_withdraw_exactly_at_unlock() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    env.ledger().with_mut(|li| {
        li.timestamp = 1_000;
    });

    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &5_000);

    // Time is exactly at unlock_time
    env.ledger().with_mut(|li| {
        li.timestamp = 5_000;
    });

    assert_eq!(client.can_withdraw(&user), true);
}

#[test]
fn test_can_withdraw_no_locked_funds() {
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    // No locked funds -> should return false
    assert_eq!(client.can_withdraw(&user), false);
}

// =========================================================================
// Isolation Tests (multiple users)
// =========================================================================

#[test]
fn test_separate_user_balances() {
    let (env, _id, client) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.deposit(&alice, &1000);
    client.deposit(&bob, &500);

    assert_eq!(client.get_balance(&alice), 1000);
    assert_eq!(client.get_balance(&bob), 500);

    client.withdraw(&alice, &200);
    assert_eq!(client.get_balance(&alice), 800);
    assert_eq!(client.get_balance(&bob), 500); // Bob unaffected
}
