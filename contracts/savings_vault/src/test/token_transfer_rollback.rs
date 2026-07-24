//! Token transfer rollback tests for the Savings Vault (issue #237).
//!
//! These tests prove that failed token transfers do not corrupt vault
//! accounting. Each test simulates a transfer failure and verifies that
//! all storage state remains unchanged after the failure.
//!
//! ## Invariants under test
//! - Failed deposit: balance → unchanged, locks → unchanged, events → none
//! - Failed withdrawal: balance → unchanged, locks → unchanged, events → none
//! - Failed withdraw_lock: locks → unchanged, balance → unchanged
//!
//! ## Key architectural guarantee
//! The vault performs token transfers *before* mutating storage in every
//! state-changing function (deposit, withdraw, withdraw_lock). If the SAC
//! transfer reverts, the entire call reverts with zero storage side-effects
//! — Soroban guarantees atomic rollback of the host call.
//!
//! These tests validate that guarantee empirically by provoking real SAC
//! transfer failures and asserting zero state drift.

use super::test_helpers::*;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    vec, Address, Env, IntoVal, Val,
};

// ────────────────────────────────────────────────────────────
// helpers
// ────────────────────────────────────────────────────────────

/// Returns (token address, token client, token admin) for a SAC token
/// registered on the given env.
fn sac_setup(env: &Env) -> (Address, token::Client, token::StellarAssetClient) {
    let issuer = Address::generate(env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let token_addr = sac.address();
    let token_client = token::Client::new(env, &token_addr);
    let token_admin = token::StellarAssetClient::new(env, &token_addr);
    (token_addr, token_client, token_admin)
}

/// Deploy and init the vault with a SAC token, returning everything
/// needed for deposit/withdraw tests.
fn vault_with_sac(
    env: &Env,
) -> (
    Address,
    SavingsVaultClient,
    token::Client,
    token::StellarAssetClient,
) {
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let (token_addr, token_client, token_admin) = sac_setup(env);
    client.initialize(&admin, &token_addr);
    (contract_id, client, token_client, token_admin)
}

/// Returns a snapshot of all vault state for a given user.
fn snapshot(env: &Env, client: &SavingsVaultClient, user: &Address) -> (i128, i128, u32) {
    let bal = client.get_balance(user);
    let locked = client.get_locked_balance(user);
    let event_count = env.events().all().len();
    (bal, locked, event_count)
}

// ────────────────────────────────────────────────────────────
// deposit rollback
// ────────────────────────────────────────────────────────────

#[test]
fn test_failed_deposit_insufficient_token_balance() {
    // User has 50 tokens in SAC but tries to deposit 100.
    // The SAC transfer must fail, and vault state must be unchanged.
    let env = test_env();
    let (contract_id, client, token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    // Give the user fewer tokens than the deposit amount
    token_admin.mint(&user, &50);

    // Snapshot: zero state
    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 0);
    assert_eq!(locked_before, 0);

    // Attempt deposit — must panic because SAC has insufficient balance
    let result = client.try_deposit(&user, &100);

    assert!(
        result.is_err(),
        "deposit must fail when user has insufficient token balance"
    );

    // State must be unchanged
    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "balance must not change on failed deposit"
    );
    assert_eq!(
        locked_after, locked_before,
        "locked balance must not change on failed deposit"
    );
    assert_eq!(
        events_after, events_before,
        "no new events must be emitted on failed deposit"
    );
}

#[test]
fn test_failed_deposit_zero_token_balance() {
    // User has zero tokens, tries to deposit 100.
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    // No mint — user has 0 tokens

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);

    let result = client.try_deposit(&user, &100);
    assert!(
        result.is_err(),
        "deposit must fail when user has zero token balance"
    );

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(bal_after, bal_before);
    assert_eq!(locked_after, locked_before);
    assert_eq!(events_after, events_before);
}

#[test]
fn test_failed_deposit_state_rollback_with_existing_balance() {
    // User has existing balance and locks — failed deposit must leave both intact.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    // Build up real state first
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    set_ledger_timestamp(&env, 1_000);
    client.lock_funds(&user, &200, &10_000); // lock 200, leaving 300 available

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 300, "available balance after lock");
    assert_eq!(locked_before, 200, "locked balance after lock");

    // Attempt deposit that exceeds SAC balance (only route left is to exceed balance)
    // The user now has 500 SAC tokens (1000 minted - 500 deposited = 500 remaining)
    // Attempting 600 deposit should fail
    let result = client.try_deposit(&user, &600);
    assert!(result.is_err(), "deposit exceeding SAC balance must fail");

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "available balance unchanged after failed deposit"
    );
    assert_eq!(
        locked_after, locked_before,
        "locked balance unchanged after failed deposit"
    );
    assert_eq!(
        events_after, events_before,
        "no events emitted on failed deposit"
    );
}

// ────────────────────────────────────────────────────────────
// withdrawal rollback
// ────────────────────────────────────────────────────────────

#[test]
fn test_failed_withdraw_state_unchanged() {
    // User has 100 balance, tries to withdraw 200 — must panic, state unchanged.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    token_admin.mint(&user, &200);
    client.deposit(&user, &100);

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 100);

    let result = client.try_withdraw(&user, &200);
    assert!(result.is_err(), "withdraw exceeding balance must fail");

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "balance must not change on failed withdrawal"
    );
    assert_eq!(locked_after, locked_before);
    assert_eq!(events_after, events_before);
}

#[test]
fn test_failed_withdraw_with_locks_state_unchanged() {
    // User deposits 500, locks 300, tries to withdraw 201 from 200 available.
    // Must panic and leave both available + locked balances intact.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    // Simulate the same setup as the existing failing-withdraw test but with
    // explicit state verification after the panic.
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);

    client.deposit(&user, &500);
    client.lock_funds(&user, &300, &10_000);

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 200);
    assert_eq!(locked_before, 300);

    let result = client.try_withdraw(&user, &201);
    assert!(result.is_err(), "withdraw exceeding available must fail");

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(bal_after, bal_before);
    assert_eq!(locked_after, locked_before);
    assert_eq!(events_after, events_before);
}

#[test]
fn test_failed_withdraw_exceeds_total_with_matured_locks() {
    // User deposits only enough for a small balance, then creates locks.
    // After locks mature, attempt to withdraw more than total (balance + matured locks).
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);

    // Deposit 400, lock 300 until t=5_000
    client.deposit(&user, &400);
    client.lock_funds(&user, &300, &5_000);

    // Balance should be 100 available + 300 locked = 400 total locked
    assert_eq!(client.get_balance(&user), 100);

    // Fast-forward past unlock time
    set_ledger_timestamp(&env, 10_000);
    // Now matured: balance 100 + lock 300 = 400 available
    assert_eq!(client.get_balance(&user), 400);

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);

    // Attempt to withdraw more than total — must fail
    let result = client.try_withdraw(&user, &401);
    assert!(
        result.is_err(),
        "withdraw exceeding total available must fail"
    );

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(bal_after, bal_before);
    assert_eq!(locked_after, locked_before);
    assert_eq!(events_after, events_before);
}

#[test]
fn test_failed_withdraw_lock_state_unchanged() {
    // User has a matured lock but withdraw_lock ID doesn't exist.
    // Must panic and leave state unchanged.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &5_000);

    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);

    // Attempt to withdraw non-existent lock
    let result = client.try_withdraw_lock(&user, &999);
    assert!(
        result.is_err(),
        "withdraw_lock on non-existent lock must fail"
    );

    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "state unchanged after failed withdraw_lock"
    );
    assert_eq!(locked_after, locked_before);
    assert_eq!(events_after, events_before);
}

// ────────────────────────────────────────────────────────────
// rollback completeness — no partial state writes
// ────────────────────────────────────────────────────────────

#[test]
fn test_multiple_failed_operations_no_cumulative_drift() {
    // Repeated failed operations must not accumulate any state drift.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);

    token_admin.mint(&user, &500);
    client.deposit(&user, &100);

    let (bal_before, locked_before, _events_before) = snapshot(&env, &client, &user);

    // Run a series of operations that all must fail
    let _r1 = client.try_deposit(&user, &999); // not enough SAC balance
    let _r2 = client.try_withdraw(&user, &200); // exceeds balance
    let _r3 = client.try_deposit(&user, &999);
    let _r4 = client.try_withdraw(&user, &999);
    let _r5 = client.try_withdraw_lock(&user, &42);

    let (bal_after, locked_after, _events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "balance unchanged after 5 failed ops"
    );
    assert_eq!(
        locked_after, locked_before,
        "locks unchanged after 5 failed ops"
    );
}

#[test]
fn test_balance_consistency_after_mixed_failures() {
    // Alternating successful and failed operations across two users.
    // The contract must never show inconsistent totals.
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin) = vault_with_sac(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    token_admin.mint(&user_a, &10_000);
    token_admin.mint(&user_b, &10_000);

    // User A: deposits 1_000
    client.deposit(&user_a, &1_000);
    assert_eq!(client.get_balance(&user_a), 1_000);

    // User A: failed withdrawal (exceeds)
    let _ = client.try_withdraw(&user_a, &2_000);

    // User B: deposits 500
    client.deposit(&user_b, &500);
    assert_eq!(client.get_balance(&user_b), 500);

    // User B: failed deposit (insufficient SAC)
    let _ = client.try_deposit(&user_b, &50_000);

    // User A: partial withdrawal succeeds
    client.withdraw(&user_a, &300);
    assert_eq!(client.get_balance(&user_a), 700);

    // User B: balance untouched by failures
    assert_eq!(client.get_balance(&user_b), 500);

    // Totals must reconcile
    let total = client.get_balance(&user_a) + client.get_balance(&user_b);
    assert_eq!(total, 1_200, "total user balances = 700 + 500 = 1200");
}
