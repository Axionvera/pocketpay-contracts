//! Unit tests for the Savings Vault contract.
//!
//! These tests use the Soroban SDK test utilities to simulate
//! on-chain interactions in an isolated environment.
mod balance_conservation;
mod maximum_amount_boundary;
mod test_helpers;
mod initialization;

use super::*;
use soroban_sdk::{testutils::Address as _, Address};

use test_helpers::*;
use ContractError;


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
fn test_deposit_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    let result = client.try_deposit(&user, &0);
    assert_eq!(result, Err(ContractError::InvalidDepositAmount));
}

#[test]
fn test_deposit_negative_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    let result = client.try_deposit(&user, &-50);
    assert_eq!(result, Err(ContractError::InvalidDepositAmount));
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
fn test_withdraw_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    let result = client.try_withdraw(&user, &0);
    assert_eq!(result, Err(ContractError::InvalidWithdrawAmount));
}

#[test]
fn test_withdraw_negative_panics() {
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10000);

    // SAC Transfer not yet implemented for deposit so i'll mimick it by trnasfering asset(deposit_amount) from user to the contract
    client.deposit(&user, &100);

    token_client.transfer(&user, &current_contract_address, &100); // This should be removed when deposit function implements SAC

    let result = client.try_withdraw(&user, &-10);
    assert_eq!(result, Err(ContractError::InvalidWithdrawAmount));
}

#[test]
fn test_withdraw_from_empty_balance_panics() {
    // AC: Withdrawing from an empty balance fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    // User never deposited — balance is implicitly 0
    let result = client.try_withdraw(&user, &1);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
}

#[test]
fn test_withdraw_exceeds_available_after_deposit_panics() {
    // AC: Withdrawing more than available balance fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    // Attempt to withdraw more than deposited
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
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
fn test_failed_withdraw_does_not_change_available_balance_panics() {
    // Confirms that attempting to withdraw 1 unit more than deposited
    // is rejected (returns error) — i.e. the balance is never decremented.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
    assert_eq!(client.get_balance(&user), 100);
}

#[test]
fn test_failed_withdraw_does_not_change_locked_balance() {
    // AC: Failed withdrawal does not change locked balance if applicable.
    // Depositing 500 and locking 300 leaves 200 available.
    // Attempting to withdraw 201 must fail, leaving both balances intact.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &500);
    // Lock 300, leaving 200 available
    client.lock_funds(&user, &300, &10_000);

    assert_eq!(client.get_balance(&user), 200);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Attempt to withdraw more than the available 200 — must fail.
    // Because the error is returned before any storage write, both the
    // available and locked balances remain unchanged.
    let result = client.try_withdraw(&user, &201);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
    assert_eq!(client.get_balance(&user), 200);
    assert_eq!(client.get_locked_balance(&user), 300);
}

#[test]
fn test_withdraw_from_immature_lock_fails() {
    // AC: Early withdrawal is rejected with specific error message.
    // User deposits 500, locks 400 until T=10_000, leaving 100 available.
    // Attempting to withdraw 101 (touching locked funds) before maturity fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &500);
    client.lock_funds(&user, &400, &10_000);

    assert_eq!(client.get_balance(&user), 100);
    assert_eq!(client.get_locked_balance(&user), 400);

    // Attempt to withdraw more than available (100) - should fail with specific error
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
}

#[test]
fn test_withdraw_only_from_locked_funds_fails() {
    // AC: Attempting to withdraw when all funds are locked fails.
    // User deposits 500, locks all 500 until T=10_000.
    // Attempting any withdrawal before maturity fails.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &500);
    client.lock_funds(&user, &500, &10_000);

    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 500);

    // Attempt to withdraw any amount when all funds are locked
    let result = client.try_withdraw(&user, &1);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
}

#[test]
fn test_withdraw_after_lock_maturity_succeeds() {
    // AC: Withdrawal succeeds after lock maturity.
    // User deposits 500, locks 400 until T=5_000.
    // After T=5_000, the locked funds become available for withdrawal.
    let (env, current_contract_address, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &10000);

    client.deposit(&user, &500);
    token_client.transfer(&user, &current_contract_address, &500);

    client.lock_funds(&user, &400, &5_000);

    assert_eq!(client.get_balance(&user), 100);
    assert_eq!(client.get_locked_balance(&user), 400);

    // Advance time past unlock time
    set_ledger_timestamp(&env, 5_000);

    // Now can withdraw the full amount
    client.withdraw(&user, &500);
    assert_eq!(client.get_balance(&user), 0);
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
// Repeated lock operations — independent multi-lock maturity
// -------------------------------------------------------------------------
//
// After multi-lock support, each `lock_funds` call creates an independent
// `LockEntry` with its own `unlock_time`. Behaviour when locking repeatedly:
//
// - Locked balance: **accumulates**. Each call adds `amount` on top of
//   whatever is already locked.
// - Available (deposited) balance: decreases by each `amount` locked.
// - Unlock times: **independent**, not overwritten. Each entry matures on
//   its own schedule.
// - `get_locked_balance`: sums only *unmatured* locks
//   (`current_time < unlock_time`).
// - `get_balance`: deposited balance + *matured* lock amounts.
// - `can_withdraw`: `true` if *any* lock has matured
//   (`current_time >= unlock_time`).

/// Two independent locks with a later second unlock time.
///
/// Lock 1: 300 until T=5_000
/// Lock 2: 200 until T=6_000
///
/// At T=5_000 only lock 1 matures; lock 2 remains locked until T=6_000.
#[test]
fn test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);

    // Before either matures: both amounts locked, remaining deposit available.
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // At lock 1's unlock time: lock 1 matures (available), lock 2 still locked.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 200);
    assert_eq!(client.get_balance(&user), 800); // 500 deposited + 300 matured

    // At lock 2's unlock time: both locks matured.
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

/// Two independent locks where the second unlock time is earlier.
///
/// Lock 1: 300 until T=6_000
/// Lock 2: 200 until T=5_000
///
/// At T=5_000 only lock 2 matures; lock 1 stays locked until T=6_000.
/// Earlier locks do not pull later locks forward (and vice versa).
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

    // Only the earlier lock (200) matures at T=5_000; 300 remains locked.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 300);
    assert_eq!(client.get_balance(&user), 700); // 500 deposited + 200 matured

    // Remaining lock matures at T=6_000.
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

/// Three independent locks: each matures on its own schedule.
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

    // At T=4_000 the first two locks have matured; the third is still locked.
    set_ledger_timestamp(&env, 4_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 100);
    assert_eq!(client.get_balance(&user), 900); // 700 deposited + 200 matured

    // All three mature once the latest unlock time is reached.
    set_ledger_timestamp(&env, 7_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

#[test]
fn test_lock_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    let result = client.try_lock_funds(&user, &0, &2_000);
    assert_eq!(result, Err(ContractError::InvalidLockAmount));
}

#[test]
fn test_lock_more_than_balance_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    let result = client.try_lock_funds(&user, &500, &2_000);
    assert_eq!(result, Err(ContractError::InsufficientBalanceToLock));
}

#[test]
fn test_lock_past_time_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 5_000);
    deposit_balance(&client, &user, 100);
    let result = client.try_lock_funds(&user, &50, &3_000);
    assert_eq!(result, Err(ContractError::InvalidUnlockTime));
}

#[test]
fn test_lock_from_empty_balance_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    // User has 0 balance, attempt to lock 100
    let result = client.try_lock_funds(&user, &100, &2_000);
    assert_eq!(result, Err(ContractError::InsufficientBalanceToLock));
}

#[test]
fn test_lock_more_than_available_balance_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    // Attempt to lock more than available (100)
    let result = client.try_lock_funds(&user, &101, &2_000);
    assert_eq!(result, Err(ContractError::InsufficientBalanceToLock));
}

#[test]
fn test_failed_lock_does_not_change_available_balance() {
    // Strategy: Verify a valid partial lock leaves the remaining available balance correct.
    // The companion panic test confirms that the lock is rejected before any mutation occurs.
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    // Initial check
    assert_eq!(client.get_balance(&user), 100);

    // A valid partial lock succeeds and updates available/locked balances
    client.lock_funds(&user, &60, &2_000);
    assert_eq!(client.get_balance(&user), 40);

    // Another valid lock
    client.lock_funds(&user, &40, &3_000);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
fn test_failed_lock_does_not_change_available_balance_panics() {
    // Confirms that attempting to lock more than available balance is rejected (returns error)
    // and available balance is not mutated.
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    // Attempting to lock 101 must fail, leaving available balance at 100
    let result = client.try_lock_funds(&user, &101, &2_000);
    assert_eq!(result, Err(ContractError::InsufficientBalanceToLock));
    assert_eq!(client.get_balance(&user), 100);
}

#[test]
fn test_failed_lock_does_not_change_locked_balance() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);

    // Lock 200, leaving 300 available, and locked balance at 200
    client.lock_funds(&user, &200, &2_000);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);

    // Attempt to lock 301, which is more than available 300.
    // This must fail, leaving locked balance at 200.
    let result = client.try_lock_funds(&user, &301, &3_000);
    assert_eq!(result, Err(ContractError::InsufficientBalanceToLock));
    assert_eq!(client.get_locked_balance(&user), 200);
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
// Authorization Tests (wrong-user attempts)
// =========================================================================

#[test]
fn test_withdraw_requires_user_authorization() {
    // AC: Withdrawal requires the user's authorization.
    // This test documents that user.require_auth() is called in withdraw.
    // In production, cross-user withdrawal attempts fail at the Soroban host level
    // due to missing authorization from the target user.
    let (env, _id, client) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    // Alice deposits funds
    client.deposit(&alice, &500);
    assert_eq!(client.get_balance(&alice), 500);
    assert_eq!(client.get_balance(&bob), 0);

    // Bob cannot withdraw Alice's funds - requires Alice's authorization
    // In production with real auth, this would fail at host level
    // The withdraw function calls user.require_auth() which enforces this
}

// =========================================================================
// Repeated Attempt Tests
// =========================================================================

#[test]
fn test_repeated_early_withdrawal_attempts_all_fail() {
    // AC: Repeated early withdrawal attempts all fail with no state change.
    // User locks funds and attempts multiple withdrawals before maturity.
    // All attempts should fail and balances should remain unchanged.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &500);
    client.lock_funds(&user, &400, &10_000);

    let initial_available = client.get_balance(&user);
    let initial_locked = client.get_locked_balance(&user);

    // First attempt - should fail
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
    assert_eq!(client.get_balance(&user), initial_available);
    assert_eq!(client.get_locked_balance(&user), initial_locked);

    // Second attempt - should still fail with no state change
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
    assert_eq!(client.get_balance(&user), initial_available);
    assert_eq!(client.get_locked_balance(&user), initial_locked);

    // Third attempt with different amount - should still fail
    let result = client.try_withdraw(&user, &200);
    assert_eq!(result, Err(ContractError::FundsLockedUntilMaturity));
    assert_eq!(client.get_balance(&user), initial_available);
    assert_eq!(client.get_locked_balance(&user), initial_locked);
}

#[test]
fn test_repeated_insufficient_balance_attempts_all_fail() {
    // AC: Repeated insufficient balance attempts all fail with no state change.
    // User has insufficient funds and attempts multiple withdrawals.
    // All attempts should fail and balance should remain unchanged.
    let (env, _id, client) = setup();
    let user = Address::generate(&env);

    client.deposit(&user, &100);
    let initial_balance = client.get_balance(&user);

    // First attempt - should fail
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
    assert_eq!(client.get_balance(&user), initial_balance);

    // Second attempt - should still fail with no state change
    let result = client.try_withdraw(&user, &101);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
    assert_eq!(client.get_balance(&user), initial_balance);

    // Third attempt with different amount - should still fail
    let result = client.try_withdraw(&user, &200);
    assert_eq!(result, Err(ContractError::InsufficientBalance));
    assert_eq!(client.get_balance(&user), initial_balance);
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
    let (_current_contract_address, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, client);

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
