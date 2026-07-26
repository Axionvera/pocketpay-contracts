//! Unit tests for `extend_lock` functionality in Savings Vault smart contract.

extern crate std;

use super::test_helpers::*;
use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, Symbol, TryIntoVal,
};

struct ExtensionFixture {
    env: Env,
    contract_id: Address,
    client: SavingsVaultClient<'static>,
    user: Address,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup_extension_fixture() -> ExtensionFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);
    client.deposit(&user, &5_000);

    ExtensionFixture {
        env,
        contract_id,
        client,
        user,
        token_client,
        token_admin,
    }
}

// =========================================================================
// 1. Success Cases
// =========================================================================

/// Verifies that calling `extend_lock` updates the lock's `unlock_time` in storage,
/// emits an `extend_lock` event, and preserves accounting balances.
#[test]
fn test_extend_lock_success() {
    let f = setup_extension_fixture();
    let initial_unlock: u64 = 5_000;
    let lock_amount: i128 = 1_500;

    let lock_id = f.client.lock_funds(&f.user, &lock_amount, &initial_unlock);

    // Assert initial state
    assert_eq!(f.client.get_balance(&f.user), 3_500);
    assert_eq!(f.client.get_locked_balance(&f.user), 1_500);

    // Extend unlock duration to 10,000
    let new_unlock: u64 = 10_000;
    f.client.extend_lock(&f.user, &lock_id, &new_unlock);

    // Verify updated lock entry in storage
    let updated_lock = f.client.get_lock(&f.user, &lock_id).unwrap();
    assert_eq!(updated_lock.unlock_time, new_unlock);
    assert_eq!(updated_lock.amount, lock_amount);
    assert!(!updated_lock.withdrawn);

    // Verify accounting balances remain unchanged
    assert_eq!(f.client.get_balance(&f.user), 3_500);
    assert_eq!(f.client.get_locked_balance(&f.user), 1_500);

    // Verify event emission
    let events = f.env.events().all();
    let (contract, topics, data) = events.get(events.len() - 1).unwrap();
    assert_eq!(contract, f.contract_id);
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, Symbol::new(&f.env, "extend_lock"));
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.user);

    let (emitted_id, old_time, new_time, amount): (u64, u64, u64, i128) =
        data.try_into_val(&f.env).unwrap();
    assert_eq!(emitted_id, lock_id);
    assert_eq!(old_time, initial_unlock);
    assert_eq!(new_time, new_unlock);
    assert_eq!(amount, lock_amount);
}

/// Verifies that at the old unlock timestamp, the extended lock is NOT withdrawable,
/// but becomes withdrawable at the new unlock timestamp.
#[test]
fn test_extend_lock_defers_maturity() {
    let f = setup_extension_fixture();
    let lock_id = f.client.lock_funds(&f.user, &1_000, &3_000);

    // Extend from 3,000 to 7,000
    f.client.extend_lock(&f.user, &lock_id, &7_000);

    // Advance timestamp to old maturity (3,000)
    set_ledger_timestamp(&f.env, 3_000);

    // Must NOT be withdrawable at old unlock timestamp
    assert!(
        !f.client.can_withdraw(&f.user),
        "can_withdraw must return false at old maturity timestamp"
    );

    // Advance timestamp to new maturity (7,000)
    set_ledger_timestamp(&f.env, 7_000);

    // Must be withdrawable at new maturity timestamp
    assert!(
        f.client.can_withdraw(&f.user),
        "can_withdraw must return true at new maturity timestamp"
    );

    // Withdrawal succeeds
    f.client.withdraw_lock(&f.user, &lock_id);
}

// =========================================================================
// 2. Failure & Rejection Boundary Cases
// =========================================================================

/// Attempting to shorten the unlock duration (`new_unlock < current_unlock`) must panic.
#[test]
#[should_panic(expected = "New unlock time must be strictly greater than current unlock time")]
fn test_extend_lock_shortening_rejected() {
    let f = setup_extension_fixture();
    let lock_id = f.client.lock_funds(&f.user, &1_000, &5_000);

    // Try to reduce unlock time from 5,000 to 4,000
    f.client.extend_lock(&f.user, &lock_id, &4_000);
}

/// Attempting to extend with the exact same unlock duration (`new_unlock == current_unlock`) must panic.
#[test]
#[should_panic(expected = "New unlock time must be strictly greater than current unlock time")]
fn test_extend_lock_same_duration_rejected() {
    let f = setup_extension_fixture();
    let lock_id = f.client.lock_funds(&f.user, &1_000, &5_000);

    // Try to pass same unlock_time (5,000)
    f.client.extend_lock(&f.user, &lock_id, &5_000);
}

/// Attempting to extend with a past timestamp (`new_unlock <= current_ledger_time`) must panic.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_extend_lock_past_timestamp_rejected() {
    let f = setup_extension_fixture();
    set_ledger_timestamp(&f.env, 8_000);
    let lock_id = f.client.lock_funds(&f.user, &1_000, &10_000);

    // Advance timestamp past 10,000
    set_ledger_timestamp(&f.env, 12_000);

    // Attempting to set new_unlock_time = 11,000 (which is < current timestamp 12,000) must panic
    f.client.extend_lock(&f.user, &lock_id, &11_000);
}

/// Attempting to extend an already withdrawn lock must panic.
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_extend_already_withdrawn_lock_rejected() {
    let f = setup_extension_fixture();
    let lock_id = f.client.lock_funds(&f.user, &1_000, &3_000);

    set_ledger_timestamp(&f.env, 3_000);
    f.client.withdraw_lock(&f.user, &lock_id);

    // Attempt to extend after withdrawal
    f.client.extend_lock(&f.user, &lock_id, &8_000);
}

/// Attempting to extend a non-existent lock ID must panic.
#[test]
#[should_panic(expected = "Lock not found")]
fn test_extend_nonexistent_lock_rejected() {
    let f = setup_extension_fixture();
    f.client.extend_lock(&f.user, &999, &5_000);
}

/// Verifies that `extend_lock` is blocked when contract is paused.
#[test]
#[should_panic(expected = "Contract is paused")]
fn test_extend_lock_while_paused_rejected() {
    let f = setup_extension_fixture();
    let admin = f.client.get_admin();
    let lock_id = f.client.lock_funds(&f.user, &1_000, &5_000);

    // Admin pauses vault
    f.client.pause(&admin, &300);

    // Attempting to extend lock must panic
    f.client.extend_lock(&f.user, &lock_id, &10_000);
}
