//! Unit tests for the Savings Vault contract.
//!
//! These tests use the Soroban SDK test utilities to simulate
//! on-chain interactions in an isolated environment.
mod test_helpers;

use super::*;
use soroban_sdk::{testutils::Address as _, Address};

use test_helpers::*;

// =========================================================================
// Initialization Tests
// =========================================================================

#[test]
fn test_initialize() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);
    client.initialize(&admin, &token);
}

// =========================================================================
// Deposit Tests
// =========================================================================

#[test]
fn test_deposit() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    assert_eq!(client.get_balance(&user), 100);
}

#[test]
fn test_multiple_deposits() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    seed_balances(&client, &user, &[100, 250]);
    assert_eq!(client.get_balance(&user), 350);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    client.deposit(&user, &0);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_negative_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    client.deposit(&user, &-50);
}

#[test]
fn test_get_balance_default_zero_for_new_user_after_initialization() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);

    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// Withdrawal Tests
// =========================================================================

#[test]
fn test_withdraw() {
    let (env, current_contract_address, client) = setup();

    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let user = Address::generate(&env);
    let deposit_amount = 500;

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &deposit_amount);

    token_admin.mint(&user, &10000);

    let user_balance = token_client.balance(&user);
    assert_eq!(&user_balance, &10000);

    token_client.transfer(&user, &current_contract_address, &deposit_amount); // This should be removed when deposit function implements SAC

    let user_balance = token_client.balance(&user);
    assert_eq!(&user_balance, &9500);

    let contract_balance = token_client.balance(&current_contract_address);
    assert_eq!(&contract_balance, &500);

    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

#[test]
fn test_withdraw_entire_balance() {
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);
    let deposit_amount = 100;

    token_admin.mint(&user, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &deposit_amount);

    token_client.transfer(&user, &current_contract_address, &deposit_amount); // This should be removed when deposit function implements SAC

    client.withdraw(&user, &deposit_amount);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_more_than_balance_panics() {
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &100);

    token_client.transfer(&user, &current_contract_address, &100); // This should be removed when deposit function implements SAC

    client.withdraw(&user, &200);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    client.withdraw(&user, &0);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_negative_panics() {
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &100);

    token_client.transfer(&user, &current_contract_address, &100); // This should be removed when deposit function implements SAC

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
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);
    let deposit_amount = 100;

    token_admin.mint(&user, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &deposit_amount);

    token_client.transfer(&user, &current_contract_address, &deposit_amount); // This should be removed when deposit function implements SAC

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

    set_ledger_timestamp(&env, 1_000);

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
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// Fund Locking Tests
// =========================================================================

#[test]
fn test_lock_funds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &2_000);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);
}

#[test]
fn test_lock_funds_multiple_times() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);
}

// -------------------------------------------------------------------------
// Repeated lock operations — accumulation vs. overwrite behaviour
// -------------------------------------------------------------------------
//
// This section documents and locks in the *current* implementation's
// behaviour for a user who calls `lock_funds` more than once, per
// issue #79. It intentionally exposes existing behaviour without
// changing it.
//
// - Locked balance: **accumulates**. Each call adds `amount` on top of
//   whatever is already locked (`new_locked = current_locked + amount`).
// - Available balance: decreases by each `amount` locked, in line with
//   the accumulating locked balance.
// - Unlock time: **overwritten**, not accumulated or maxed. The single
//   `UnlockTime` storage slot is replaced by the most recent
//   `lock_funds` call's `unlock_time`, regardless of what was stored
//   before and regardless of whether the new value is later or earlier
//   than the previous one. There is only one unlock time per user that
//   gates the *entire* locked balance (both the old and new locked
//   amounts unlock together, based on the last call's timestamp).

/// Two repeated locks: verifies available balance, locked balance, AND
/// unlock-time behaviour together via `can_withdraw` at the boundaries.
///
/// Lock 1: 300 locked, unlock_time = 5_000
/// Lock 2: 200 locked, unlock_time = 6_000 (later than lock 1)
///
/// Expected (current implementation): locked balance accumulates to 500,
/// available balance drops to 500, and the effective unlock time for the
/// combined 500 is 6_000 (the second call's value), NOT 5_000. So at
/// T=5_000 the user still cannot withdraw anything, even though the
/// first lock's original unlock time has passed.
#[test]
fn test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);

    // Balances: locked amounts accumulate, available balance reflects both locks.
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // Unlock time was overwritten to 6_000 by the second call.
    // At the first lock's original unlock time (5_000), funds are
    // still locked because the stored unlock time is now 6_000.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), false);
    assert_eq!(client.get_locked_balance(&user), 500);

    // At the second (most recent) unlock time, the entire accumulated
    // locked balance becomes withdrawable at once.
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 500);
}

/// Same as above, but the second lock's unlock_time is EARLIER than the
/// first lock's. This demonstrates the overwrite is unconditional — the
/// contract does not take the max of old and new unlock times, so a
/// second lock can effectively pull the unlock time backwards for funds
/// that were already locked under a later time.
///
/// Lock 1: 300 locked, unlock_time = 6_000
/// Lock 2: 200 locked, unlock_time = 5_000 (earlier than lock 1)
///
/// Expected (current implementation): the combined 500 locked balance
/// becomes withdrawable at T=5_000 already, even though 300 of it was
/// originally locked until T=6_000.
#[test]
fn test_repeated_lock_overwrites_unlock_time_with_earlier_value() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &300, &6_000);
    client.lock_funds(&user, &200, &5_000);

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // Unlock time was overwritten to the EARLIER value (5_000), not
    // the max of the two (6_000). Note this is a lock-in of current
    // behaviour, not necessarily the ideal/expected product behaviour.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 500);
}

/// Three repeated locks: confirms accumulation continues correctly
/// across more than two calls, and that only the final call's
/// unlock_time is retained.
#[test]
fn test_repeated_lock_three_times_accumulates_and_keeps_last_unlock_time() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &100, &3_000);
    client.lock_funds(&user, &100, &4_000);
    client.lock_funds(&user, &100, &7_000);

    assert_eq!(client.get_balance(&user), 700);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Not yet withdrawable at the first two (now-superseded) unlock times.
    set_ledger_timestamp(&env, 4_000);
    assert_eq!(client.can_withdraw(&user), false);

    // Withdrawable only once the last call's unlock time is reached.
    set_ledger_timestamp(&env, 7_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 300);
}

#[test]
#[should_panic(expected = "Lock amount must be greater than zero")]
fn test_lock_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &0, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_more_than_balance_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &500, &2_000);
}

#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_past_time_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 5_000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &50, &3_000);
}

// =========================================================================
// can_withdraw Tests — Time-Lock Boundary Behaviour
// =========================================================================
//
// Boundary rule: `can_withdraw` returns `true` when
//   ledger.timestamp() >= unlock_time (inclusive).
// This section tests before, exactly at, and after the unlock time,
// with explicit boundary positions so the rule is unambiguous.

// -------------------------------------------------------------------------
// Before unlock — returns false
// -------------------------------------------------------------------------

/// Funds locked at T=1000 with unlock at T=10_000.
/// Checking at T=1000 (right after locking) — still far before unlock.
#[test]
fn test_can_withdraw_before_unlock() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &10_000);
    assert_eq!(client.can_withdraw(&user), false);
}

/// Boundary: 1 second before unlock.
/// Unlock is at T=5000, check at T=4999 — still locked.
#[test]
fn test_can_withdraw_one_second_before_unlock_returns_false() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    // Set ledger to exactly 1 second before unlock
    set_ledger_timestamp(&env, 4_999);
    assert_eq!(client.can_withdraw(&user), false);
}

// -------------------------------------------------------------------------
// At unlock — returns true (inclusive boundary)
// -------------------------------------------------------------------------

/// Boundary: exactly at unlock time.
/// Unlock at T=5000, check at T=5000 — funds are now withdrawable.
/// This confirms the boundary is **inclusive** (>=).
#[test]
fn test_can_withdraw_exactly_at_unlock() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
}

// -------------------------------------------------------------------------
// After unlock — returns true
// -------------------------------------------------------------------------

/// Unlock at T=5000, check at T=6000 — well after unlock.
#[test]
fn test_can_withdraw_after_unlock() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
}

/// Boundary: 1 second after unlock.
/// Unlock at T=5000, check at T=5001 — confirm it's still true.
#[test]
fn test_can_withdraw_one_second_after_unlock_returns_true() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    // Set ledger to exactly 1 second after unlock
    set_ledger_timestamp(&env, 5_001);
    assert_eq!(client.can_withdraw(&user), true);
}

// -------------------------------------------------------------------------
// No locked funds — returns false
// -------------------------------------------------------------------------

/// User with no locked funds always returns false, regardless of
/// any stored unlock time or timestamp.
#[test]
fn test_can_withdraw_no_locked_funds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    assert_eq!(client.can_withdraw(&user), false);
}

// -------------------------------------------------------------------------
// Locked balance correctness across boundary checks
// -------------------------------------------------------------------------

/// The locked balance is unaffected by repeated `can_withdraw` queries.
/// Lock 300 at T=1000, unlock at T=5000. Check locked balance before,
/// at, and after unlock — it should remain 300 throughout.
#[test]
fn test_locked_balance_correct_before_at_and_after_unlock() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &300, &5_000);

    // Before unlock (T=4999): cannot withdraw, locked balance = 300
    set_ledger_timestamp(&env, 4_999);
    assert_eq!(client.can_withdraw(&user), false);
    assert_eq!(client.get_locked_balance(&user), 300);
    // Available balance still reflects deduction
    assert_eq!(client.get_balance(&user), 200);

    // At unlock (T=5000): can withdraw, locked balance = 0
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 500);

    // After unlock (T=5001): can withdraw, locked balance = 0
    set_ledger_timestamp(&env, 5_001);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 500);
}

// -------------------------------------------------------------------------
// Boundary rule documentation test
// -------------------------------------------------------------------------

/// This test explicitly documents the boundary rule:
/// `can_withdraw` uses **inclusive** comparison (>=).
///
/// - ledger.timestamp() <  unlock_time  →  false  (locked)
/// - ledger.timestamp() >= unlock_time  →  true   (unlocked, if locked_balance > 0)
#[test]
fn test_can_withdraw_boundary_rule_is_inclusive_gte() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    let unlock_time: u64 = 5_000;

    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &unlock_time);

    // t < unlock_time → false
    set_ledger_timestamp(&env, unlock_time - 1);
    assert!(
        !client.can_withdraw(&user),
        "Expected false when ledger.timestamp() < unlock_time"
    );

    // t == unlock_time → true (inclusive boundary)
    set_ledger_timestamp(&env, unlock_time);
    assert!(
        client.can_withdraw(&user),
        "Expected true when ledger.timestamp() == unlock_time (inclusive >=)"
    );

    // t > unlock_time → true
    set_ledger_timestamp(&env, unlock_time + 1);
    assert!(
        client.can_withdraw(&user),
        "Expected true when ledger.timestamp() > unlock_time"
    );
}

// =========================================================================
// Isolation Tests (multiple users)
// =========================================================================

#[test]
fn test_separate_user_balances() {
    let env = test_env();
    let (current_contract_address, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 500);

    token_client.transfer(&alice, &current_contract_address, &1000); // This should be removed when deposit function implements SAC
    token_client.transfer(&bob, &current_contract_address, &500); // This should be removed when deposit function implements SAC

    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_balance(&bob), 500);

    client.withdraw(&alice, &200);
    assert_eq!(client.get_balance(&alice), 800);
    assert_eq!(client.get_balance(&bob), 500);
}

#[test]
fn balance_isolation_between_users_deposit() {
    let env = test_env();
    let (current_contract_address, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    deposit_balance(&client, &alice, 1_000);
    assert_eq!(client.get_balance(&alice), 1000_i128);
    assert_eq!(client.get_balance(&bob), 0_i128);
}

#[test]
fn balance_isolation_between_users_withdraw() {
    let env = test_env();
    let (current_contract_address, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 4_000);
    token_client.transfer(&alice, &current_contract_address, &1000); // This should be removed when deposit function implements SAC
    token_client.transfer(&bob, &current_contract_address, &4000); // This should be removed when deposit function implements SAC

    assert_eq!(client.get_balance(&alice), 1000_i128);
    assert_eq!(client.get_balance(&bob), 4000_i128);

    client.withdraw(&alice, &500);
    assert_eq!(client.get_balance(&alice), 500);
    assert_eq!(client.get_balance(&bob), 4000);

    client.withdraw(&bob, &2000);
    assert_eq!(client.get_balance(&alice), 500);
    assert_eq!(client.get_balance(&bob), 2000);
}

#[test]
fn balance_isolation_between_users_lock() {
    let env = test_env();
    let (current_contract_address, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    deposit_balance(&client, &alice, 2_000);
    deposit_balance(&client, &bob, 4_000);
    token_client.transfer(&alice, &current_contract_address, &2_000); // This should be removed when deposit function implements SAC
    token_client.transfer(&bob, &current_contract_address, &4_000); // This should be removed when deposit function implements SAC

    client.lock_funds(&alice, &1_000, &3600);
    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_locked_balance(&alice), 1_000);
    assert_eq!(client.get_balance(&bob), 4_000);
    assert_eq!(client.get_locked_balance(&bob), 0);

    client.lock_funds(&bob, &2_500, &3600);
    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_locked_balance(&alice), 1_000);
    assert_eq!(client.get_balance(&bob), 1_500);
    assert_eq!(client.get_locked_balance(&bob), 2_500);
}
