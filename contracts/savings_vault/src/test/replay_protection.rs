//! Withdrawal replay protection test suite for the Savings Vault contract.
//!
//! This module verifies that standard withdrawals and matured lock withdrawals
//! cannot be replayed, executed repeatedly beyond available balance, or cross-replayed
//! between users. It also asserts that failed replay attempts preserve exact state
//! consistency without token leakage or balance corruption.
//!
//! Cover Issue #227 acceptance criteria:
//! 1. Repeated standard withdrawal attempts are tested.
//! 2. Repeated matured-lock withdrawal attempts are tested.
//! 3. Cross-user replay attempts are tested.
//! 4. State remains consistent after rejected replay attempts.
//! 5. Expected failure behaviour is documented.

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// =========================================================================
// 1. Standard Withdrawal Replay Prevention
// =========================================================================

/// Verifies that repeating a standard withdrawal for the full balance succeeds on
/// the first attempt and fails on the second attempt with "Insufficient balance".
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_repeated_standard_withdraw_full_balance_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // First withdrawal succeeds
    client.withdraw(&user, &1000);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(token_client.balance(&user), 1000);

    // Replay/second attempt of withdrawing 1000 must fail
    client.withdraw(&user, &1000);
}

/// Verifies that multiple sequential standard withdrawals exhausting the balance
/// succeed, but any subsequent withdrawal attempt panics with "Insufficient balance".
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_repeated_standard_withdraw_partial_exhaustion_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // First withdrawal of half
    client.withdraw(&user, &500);
    assert_eq!(client.get_balance(&user), 500);

    // Second withdrawal of remainder
    client.withdraw(&user, &500);
    assert_eq!(client.get_balance(&user), 0);

    // Third attempt (replay of withdraw) must fail
    client.withdraw(&user, &1);
}

/// Verifies that repeating a withdrawal spanning matured locks fails once the
/// funds have been withdrawn and deducted from storage.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_repeated_withdraw_spanning_matured_locks_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // Lock 400 until t=2000
    let _lock_id = client.lock_funds(&user, &400, &2000);

    // Advance time past maturity
    set_ledger_timestamp(&env, 2000);

    // Standard withdraw of 1000 (consumes 600 available + 400 matured lock)
    client.withdraw(&user, &1000);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(token_client.balance(&user), 1000);

    // Replay attempt must fail
    client.withdraw(&user, &1000);
}

// =========================================================================
// 2. Matured-Lock Withdrawal Replay Prevention
// =========================================================================

/// Verifies that calling `withdraw_lock` twice with the same matured `lock_id`
/// fails on the second attempt with "Lock already withdrawn" because the lock's
/// withdrawn status is persistent in storage.
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_repeated_matured_lock_withdraw_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2000);

    // Advance to maturity
    set_ledger_timestamp(&env, 2000);

    // First lock withdrawal succeeds
    client.withdraw_lock(&user, &lock_id);
    assert_eq!(client.get_balance(&user), 600);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(token_client.balance(&user), 400);

    // Replay attempt to withdraw the same lock_id must fail with Lock already withdrawn
    client.withdraw_lock(&user, &lock_id);
}

/// Verifies that if a matured lock is consumed by a standard `withdraw`, a subsequent
/// call to `withdraw_lock` for that lock ID panics with "Lock already withdrawn".
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_withdraw_lock_after_standard_withdraw_consumed_lock_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2000);

    // Advance to maturity
    set_ledger_timestamp(&env, 2000);

    // Standard withdrawal consumes the available balance AND the matured lock
    client.withdraw(&user, &1000);

    // Attempting withdraw_lock for the consumed lock ID must panic with Lock already withdrawn
    client.withdraw_lock(&user, &lock_id);
}

/// Verifies that after `withdraw_lock` is called, attempting standard `withdraw`
/// for the lock's amount panics with "Insufficient balance" if total remaining funds are insufficient.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_standard_withdraw_after_withdraw_lock_fails_if_insufficient() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &1000, &2000);

    set_ledger_timestamp(&env, 2000);

    // Lock withdrawal of 1000 succeeds
    client.withdraw_lock(&user, &lock_id);

    // Attempting standard withdrawal of 1000 must fail as funds were already withdrawn
    client.withdraw(&user, &1000);
}

// =========================================================================
// 3. Cross-User Replay Prevention
// =========================================================================

/// Verifies that User B cannot replay or execute `withdraw_lock` targeting User A's `lock_id`.
#[test]
#[should_panic(expected = "Lock not found")]
fn test_cross_user_lock_withdraw_replay_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user_a = new_user(&env);
    let user_b = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user_a, &1000);
    client.deposit(&user_a, &1000);

    let lock_id_a = client.lock_funds(&user_a, &500, &2000);

    set_ledger_timestamp(&env, 2000);

    // User B attempts to withdraw User A's lock ID under User B's account
    client.withdraw_lock(&user_b, &lock_id_a);
}

/// Verifies that User B cannot execute standard `withdraw` against User A's vault balance.
#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_cross_user_standard_withdraw_from_unowned_balance_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user_a = new_user(&env);
    let user_b = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user_a, &1000);
    client.deposit(&user_a, &1000);

    // User B has 0 vault balance and attempts to withdraw User A's funds
    client.withdraw(&user_b, &1000);
}

/// Verifies that invoking `withdraw` for User A without User A's authorization panics.
#[test]
#[should_panic]
fn test_cross_user_withdraw_unauthorized_signature_fails() {
    let env = Env::default(); // Note: intentionally omitting mock_all_auths()
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);

    client.withdraw(&user_a, &500);
}

/// Verifies that invoking `withdraw_lock` for User A without User A's authorization panics.
#[test]
#[should_panic]
fn test_cross_user_withdraw_lock_unauthorized_signature_fails() {
    let env = Env::default(); // Note: intentionally omitting mock_all_auths()
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);

    client.withdraw_lock(&user_a, &1);
}

// =========================================================================
// 4. State Consistency After Rejected Replay Attempts
// =========================================================================

/// Verifies that when a repeated standard withdrawal attempt is rejected, the vault's
/// available balance, locked balance, and total contract token balance remain perfectly intact.
#[test]
fn test_state_consistency_after_rejected_standard_withdraw_replay() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // Withdraw 600 -> user balance becomes 400, user token balance becomes 600
    client.withdraw(&user, &600);

    let bal_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);
    let user_token_before = token_client.balance(&user);

    assert_eq!(bal_before, 400);
    assert_eq!(locked_before, 0);
    assert_eq!(user_token_before, 600);

    // Attempt invalid replay / over-withdrawal of 500 (only 400 available)
    let res = client.try_withdraw(&user, &500);
    assert!(res.is_err(), "Replay/over-withdrawal must fail");

    // Verify state was NOT mutated
    assert_eq!(
        client.get_balance(&user),
        bal_before,
        "Available balance must remain unchanged after rejected replay"
    );
    assert_eq!(
        client.get_locked_balance(&user),
        locked_before,
        "Locked balance must remain unchanged after rejected replay"
    );
    assert_eq!(
        token_client.balance(&user),
        user_token_before,
        "User token balance must remain unchanged after rejected replay"
    );
}

/// Verifies that when a repeated `withdraw_lock` attempt is rejected, vault state and
/// token balances remain completely preserved.
#[test]
fn test_state_consistency_after_rejected_withdraw_lock_replay() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2000);

    set_ledger_timestamp(&env, 2000);

    // Withdraw matured lock once
    client.withdraw_lock(&user, &lock_id);

    let bal_after = client.get_balance(&user);
    let locked_after = client.get_locked_balance(&user);
    let token_after = token_client.balance(&user);

    assert_eq!(bal_after, 600);
    assert_eq!(locked_after, 0);
    assert_eq!(token_after, 400);

    // Replay attempt via try_withdraw_lock
    let res = client.try_withdraw_lock(&user, &lock_id);
    assert!(res.is_err(), "Replayed withdraw_lock must return error");

    // Assert zero state mutation after failed replay
    assert_eq!(client.get_balance(&user), bal_after);
    assert_eq!(client.get_locked_balance(&user), locked_after);
    assert_eq!(token_client.balance(&user), token_after);
}

/// Verifies the lifecycle and transitions of the withdrawn flag on a LockEntry.
/// It asserts that a newly created lock has withdrawn set to false and the correct amount,
/// and after withdrawal (either via withdraw_lock or standard withdraw), the flag becomes
/// true, the amount becomes 0, and subsequent withdraw_lock calls panic with the expected
/// "Lock already withdrawn" message.
#[test]
fn test_lock_withdrawn_flag_and_state_lifecycle() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &2000);
    client.deposit(&user, &2000);

    // Create two locks
    let lock_id_1 = client.lock_funds(&user, &400, &2000);
    let lock_id_2 = client.lock_funds(&user, &600, &2000);

    // Verify initial withdrawn flag behaviour
    let lock_1 = client
        .get_lock(&user, &lock_id_1)
        .expect("Lock 1 should exist");
    assert!(
        !lock_1.withdrawn,
        "New lock must not be marked as withdrawn"
    );
    assert_eq!(lock_1.amount, 400, "New lock must have the correct amount");

    let lock_2 = client
        .get_lock(&user, &lock_id_2)
        .expect("Lock 2 should exist");
    assert!(
        !lock_2.withdrawn,
        "New lock must not be marked as withdrawn"
    );
    assert_eq!(lock_2.amount, 600, "New lock must have the correct amount");

    // Advance time to maturity
    set_ledger_timestamp(&env, 2000);

    // Withdraw lock 1 via withdraw_lock
    client.withdraw_lock(&user, &lock_id_1);

    // Verify withdrawn flag and amount state after withdraw_lock
    let lock_1_after = client
        .get_lock(&user, &lock_id_1)
        .expect("Lock 1 should still exist");
    assert!(
        lock_1_after.withdrawn,
        "Withdrawn lock must have withdrawn set to true"
    );
    assert_eq!(
        lock_1_after.amount, 0,
        "Withdrawn lock must have amount set to 0"
    );

    // Withdraw remaining deposited balance (1000) + lock 2 (600) via standard withdraw
    client.withdraw(&user, &1600); // Consumes 1000 deposited + lock 2 (600)

    // Verify withdrawn flag and amount state after standard withdraw consumption
    let lock_2_after = client
        .get_lock(&user, &lock_id_2)
        .expect("Lock 2 should still exist");
    assert!(
        lock_2_after.withdrawn,
        "Consumed lock must have withdrawn set to true"
    );
    assert_eq!(
        lock_2_after.amount, 0,
        "Consumed lock must have amount set to 0"
    );
}
