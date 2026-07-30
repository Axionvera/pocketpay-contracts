//! Example invariant tests demonstrating the patterns from the invariant checklist.
//!
//! This file provides concrete examples of how to test the critical invariants
//! documented in docs/invariant-test-checklist.md. These patterns should be
//! followed when adding tests for contract changes.
//!
//! See: https://github.com/Axionvera/pocketpay-contracts/blob/main/docs/invariant-test-checklist.md

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Pattern 1: Balance Conservation Test
// ---------------------------------------------------------------------------

#[test]
fn example_balance_conservation_after_operations() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Track net deposited
    let mut net_deposited = 0i128;

    // Deposit
    let deposit_amount = 1000i128;
    token_admin.mint(&user, &deposit_amount);
    client.deposit(&user, &deposit_amount);
    net_deposited += deposit_amount;

    // Verify invariant: A(u) + L(u) = net_deposited
    let available = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);
    assert_eq!(
        available + locked,
        net_deposited,
        "Balance conservation violated after deposit"
    );

    // Lock funds
    let lock_amount = 300i128;
    let unlock_time = 2000u64;
    set_ledger_timestamp(&env, 1000);
    client.lock_funds(&user, &lock_amount, &unlock_time);

    // Verify invariant still holds: locking is internal reclassification
    let available = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);
    assert_eq!(
        available + locked,
        net_deposited,
        "Balance conservation violated after lock_funds"
    );

    // Withdraw available funds
    let withdraw_amount = 200i128;
    client.withdraw(&user, &withdraw_amount);
    net_deposited -= withdraw_amount;

    // Verify invariant: A(u) + L(u) = net_deposited
    let available = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);
    assert_eq!(
        available + locked,
        net_deposited,
        "Balance conservation violated after withdraw"
    );
}

// ---------------------------------------------------------------------------
// Pattern 2: Cross-User Isolation Test
// ---------------------------------------------------------------------------

#[test]
fn example_cross_user_isolation() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    // Setup user A
    token_admin.mint(&user_a, &1000);
    client.deposit(&user_a, &1000);

    // Setup user B
    token_admin.mint(&user_b, &500);
    client.deposit(&user_b, &500);

    // Snapshot user B's state before user A's operation
    let user_b_initial_available = client.get_balance(&user_b);
    let user_b_initial_locked = client.get_locked_balance(&user_b);

    // User A locks funds
    set_ledger_timestamp(&env, 1000);
    client.lock_funds(&user_a, &200, &2000);

    // Verify user B's state unchanged (isolation invariant)
    assert_eq!(
        client.get_balance(&user_b),
        user_b_initial_available,
        "User A's lock affected user B's available balance"
    );
    assert_eq!(
        client.get_locked_balance(&user_b),
        user_b_initial_locked,
        "User A's lock affected user B's locked balance"
    );

    // User A withdraws
    client.withdraw(&user_a, &100);

    // Verify user B's state still unchanged
    assert_eq!(
        client.get_balance(&user_b),
        user_b_initial_available,
        "User A's withdrawal affected user B's available balance"
    );
    assert_eq!(
        client.get_locked_balance(&user_b),
        user_b_initial_locked,
        "User A's withdrawal affected user B's locked balance"
    );
}

// ---------------------------------------------------------------------------
// Pattern 3: Authorization Failure Test
// ---------------------------------------------------------------------------

#[test]
fn example_withdraw_unauthorized_caller_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);
    let attacker = Address::generate(&env);

    // Setup user with balance
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // Attacker tries to withdraw user's funds
    // This should fail because user did not authorize the operation
    env.mock_auths(&[]);
    let result = client.try_withdraw(&user, &500);
    assert!(
        result.is_err(),
        "Unauthorized withdrawal should fail due to missing user auth"
    );

    // Verify user's balance unchanged
    assert_eq!(
        client.get_balance(&user),
        1000,
        "Unauthorized withdrawal modified user balance"
    );
}

#[test]
fn example_lock_funds_unauthorized_caller_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);
    let attacker = Address::generate(&env);

    // Setup user with balance
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // Attacker tries to lock user's funds
    set_ledger_timestamp(&env, 1000);
    env.mock_auths(&[]);
    let result = client.try_lock_funds(&user, &500, &2000);
    assert!(
        result.is_err(),
        "Unauthorized lock should fail due to missing user auth"
    );

    // Verify user's state unchanged
    assert_eq!(
        client.get_balance(&user),
        1000,
        "Unauthorized lock modified user balance"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        0,
        "Unauthorized lock created lock entry"
    );
}

// ---------------------------------------------------------------------------
// Pattern 4: Failed Operation Atomicity Test
// ---------------------------------------------------------------------------

#[test]
fn example_failed_deposit_leaves_state_unchanged() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Snapshot initial state
    let initial_balance = client.get_balance(&user);
    let initial_locked = client.get_locked_balance(&user);

    // Attempt deposit with zero amount (should fail)
    let result = client.try_deposit(&user, &0);
    assert!(result.is_err(), "Zero deposit should fail");

    // Verify state unchanged (atomicity invariant)
    assert_eq!(
        client.get_balance(&user),
        initial_balance,
        "Failed deposit modified available balance"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        initial_locked,
        "Failed deposit modified locked balance"
    );
}

#[test]
fn example_failed_withdraw_insufficient_balance_leaves_state_unchanged() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Setup user with balance
    token_admin.mint(&user, &100);
    client.deposit(&user, &100);

    // Snapshot state before failed operation
    let initial_balance = client.get_balance(&user);
    let initial_locked = client.get_locked_balance(&user);

    // Attempt withdrawal exceeding balance (should fail)
    let result = client.try_withdraw(&user, &200);
    assert!(result.is_err(), "Overdraft withdrawal should fail");

    // Verify state unchanged (atomicity invariant)
    assert_eq!(
        client.get_balance(&user),
        initial_balance,
        "Failed withdrawal modified available balance"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        initial_locked,
        "Failed withdrawal modified locked balance"
    );
}

#[test]
fn example_failed_lock_insufficient_balance_leaves_state_unchanged() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Setup user with balance
    token_admin.mint(&user, &100);
    client.deposit(&user, &100);

    // Snapshot state before failed operation
    let initial_balance = client.get_balance(&user);
    let initial_locked = client.get_locked_balance(&user);

    set_ledger_timestamp(&env, 1000);

    // Attempt lock exceeding available balance (should fail)
    let result = client.try_lock_funds(&user, &200, &2000);
    assert!(result.is_err(), "Overdraft lock should fail");

    // Verify state unchanged (atomicity invariant)
    assert_eq!(
        client.get_balance(&user),
        initial_balance,
        "Failed lock modified available balance"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        initial_locked,
        "Failed lock modified locked balance"
    );
}

// ---------------------------------------------------------------------------
// Pattern 5: Non-Negativity Invariant Test
// ---------------------------------------------------------------------------

#[test]
fn example_balances_never_negative() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Initial state: balances should be zero (non-negative)
    assert!(client.get_balance(&user) >= 0, "Initial balance negative");
    assert!(
        client.get_locked_balance(&user) >= 0,
        "Initial locked balance negative"
    );

    // Deposit
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    assert!(
        client.get_balance(&user) >= 0,
        "Balance negative after deposit"
    );
    assert!(
        client.get_locked_balance(&user) >= 0,
        "Locked balance negative after deposit"
    );

    // Lock funds
    set_ledger_timestamp(&env, 1000);
    client.lock_funds(&user, &300, &2000);
    assert!(
        client.get_balance(&user) >= 0,
        "Balance negative after lock"
    );
    assert!(
        client.get_locked_balance(&user) >= 0,
        "Locked balance negative after lock"
    );

    // Withdraw
    client.withdraw(&user, &200);
    assert!(
        client.get_balance(&user) >= 0,
        "Balance negative after withdraw"
    );
    assert!(
        client.get_locked_balance(&user) >= 0,
        "Locked balance negative after withdraw"
    );
}

// ---------------------------------------------------------------------------
// Pattern 6: Lock Maturity Invariant Test
// ---------------------------------------------------------------------------

#[test]
fn example_time_advancement_does_not_modify_accounting_state() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Setup user with lock
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    set_ledger_timestamp(&env, 1000);
    client.lock_funds(&user, &300, &2000);

    // Snapshot state before time advancement
    let available_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);

    // Advance time past lock maturity
    set_ledger_timestamp(&env, 3000);

    // Verify accounting state unchanged (maturity invariant)
    let available_after = client.get_balance(&user);
    let locked_after = client.get_locked_balance(&user);

    assert_eq!(
        available_after, available_before,
        "Time advancement modified available balance"
    );
    assert_eq!(
        locked_after, locked_before,
        "Time advancement modified locked balance"
    );

    // Note: lock is now matured but still in locked balance until withdrawn
    assert_eq!(
        locked_after, 300,
        "Matured lock should remain in locked balance"
    );
}

// ---------------------------------------------------------------------------
// Pattern 7: Token Custody Invariant Test
// ---------------------------------------------------------------------------

#[test]
fn example_token_custody_matches_liabilities() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);

    // Initial: contract has no tokens, no liabilities
    let contract_balance = token_client.balance(&contract_id);
    let user_balance = client.get_balance(&user);
    let user_locked = client.get_locked_balance(&user);
    assert_eq!(
        contract_balance,
        user_balance + user_locked,
        "Custody mismatch at initial state"
    );

    // Deposit: custody should increase by same amount as liability
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    let contract_balance = token_client.balance(&contract_id);
    let user_balance = client.get_balance(&user);
    let user_locked = client.get_locked_balance(&user);
    assert_eq!(
        contract_balance,
        user_balance + user_locked,
        "Custody mismatch after deposit"
    );

    // Lock: custody unchanged, liability reclassified
    set_ledger_timestamp(&env, 1000);
    client.lock_funds(&user, &300, &2000);
    let contract_balance = token_client.balance(&contract_id);
    let user_balance = client.get_balance(&user);
    let user_locked = client.get_locked_balance(&user);
    assert_eq!(
        contract_balance,
        user_balance + user_locked,
        "Custody mismatch after lock"
    );

    // Withdraw: custody and liability both decrease
    client.withdraw(&user, &200);
    let contract_balance = token_client.balance(&contract_id);
    let user_balance = client.get_balance(&user);
    let user_locked = client.get_locked_balance(&user);
    assert_eq!(
        contract_balance,
        user_balance + user_locked,
        "Custody mismatch after withdraw"
    );
}
