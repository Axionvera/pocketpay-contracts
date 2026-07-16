//! Unit tests for the Savings Vault contract.
//!
//! These tests use the Soroban SDK test utilities to simulate
//! on-chain interactions in an isolated environment.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

mod test_helpers;
use test_helpers::*;

// =========================================================================
// Initialization Tests
// =========================================================================

#[test]
fn test_initialize() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    client.initialize(&admin);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    client.initialize(&admin);
    client.initialize(&admin);
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

// =========================================================================
// Withdrawal Tests
// =========================================================================

#[test]
fn test_withdraw() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 500);
    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

#[test]
fn test_withdraw_entire_balance() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    client.withdraw(&user, &100);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_more_than_balance_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
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
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    client.withdraw(&user, &-10);
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
// can_withdraw Tests
// =========================================================================

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

#[test]
fn test_can_withdraw_no_locked_funds() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    assert_eq!(client.can_withdraw(&user), false);
}

// =========================================================================
// Isolation Tests (multiple users)
// =========================================================================

#[test]
fn test_separate_user_balances() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let alice = new_user(&env);
    let bob = new_user(&env);
    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 500);
    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_balance(&bob), 500);
    client.withdraw(&alice, &200);
    assert_eq!(client.get_balance(&alice), 800);
    assert_eq!(client.get_balance(&bob), 500);
}
