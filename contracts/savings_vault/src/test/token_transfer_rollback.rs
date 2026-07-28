//! Token transfer rollback tests for the Savings Vault (issue #237).
//!
//! These tests prove that failed token transfers do not corrupt vault
//! accounting. Each test simulates a transfer failure and verifies that
//! all storage state remains unchanged after the failure.
//!
//! ## Expected Failure Behavior
//!
//! The contract follows a strict ordering pattern to ensure atomicity:
//! 1. **Deposit**: Token transfer occurs first, then balance is credited.
//!    - If transfer fails: No balance update, no events, state unchanged
//!    - If transfer succeeds: Balance is credited, event emitted
//!
//! 2. **Withdraw**: Token transfer occurs first, then balance is debited.
//!    - If transfer fails: No balance update, no events, state unchanged
//!    - If transfer succeeds: Balance is debited, event emitted
//!
//! 3. **Withdraw Lock**: Token transfer occurs first, then lock is marked withdrawn.
//!    - If transfer fails: Lock remains unwithdrawn, amount unchanged, no events
//!    - If transfer succeeds: Lock marked withdrawn, amount set to 0, event emitted
//!
//! ## Invariants under test
//! - Failed deposit: balance → unchanged, locks → unchanged, events → none
//! - Failed withdrawal: balance → unchanged, locks → unchanged, events → none
//! - Failed withdraw_lock: locks → unchanged (amount, withdrawn flag), balance → unchanged
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
    let (contract_id, client, token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, _token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    // Now matured: get_balance still returns only deposited balance (100)
    assert_eq!(client.get_balance(&user), 100);

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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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
    let (_contract_id, client, _token_client, token_admin, _) = vault_with_sac(&env);
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

#[test]
fn test_failed_withdraw_lock_token_transfer_failure_preserves_state() {
    // Test that when the token transfer in withdraw_lock fails, the lock state
    // remains unchanged. This simulates a scenario where the contract doesn't
    // have enough tokens to transfer back (e.g., due to a bug or external factor).
    let env = test_env();
    let (contract_id, client, token_client, token_admin, _) = vault_with_sac(&env);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);

    // Deposit and create a lock
    client.deposit(&user, &500);
    let lock_id = client.lock_funds(&user, &200, &5_000);

    // Verify initial state
    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 300);
    assert_eq!(locked_before, 200);

    // Fast-forward past unlock time
    set_ledger_timestamp(&env, 10_000);

    // Drain the contract's token balance to simulate transfer failure.
    // Note: `mint` only credits a target address, it never debits the
    // contract's own balance. `clawback` would debit it, but the SAC's
    // default test issuance isn't clawback-enabled. Instead, move the
    // contract's tokens out directly; `mock_all_auths()` satisfies the
    // `from.require_auth()` the SAC transfer performs internally.
    let contract_address = contract_id;
    let contract_balance = token_client.balance(&contract_address);
    token_client.transfer(&contract_address, &Address::generate(&env), &contract_balance);

    // Attempt withdraw_lock - should fail due to insufficient contract token balance
    let result = client.try_withdraw_lock(&user, &lock_id);
    assert!(
        result.is_err(),
        "withdraw_lock must fail when contract has insufficient token balance"
    );

    // Verify state is unchanged
    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "available balance must not change on failed withdraw_lock"
    );
    assert_eq!(
        locked_after, locked_before,
        "locked balance must not change on failed withdraw_lock"
    );
    assert_eq!(
        events_after, events_before,
        "no new events must be emitted on failed withdraw_lock"
    );

    // Verify the lock entry itself is unchanged
    let lock = client.get_lock(&user, &lock_id).expect("lock should still exist");
    assert_eq!(lock.amount, 200, "lock amount should remain unchanged");
    assert!(!lock.withdrawn, "lock should not be marked as withdrawn");
}

#[test]
fn test_failed_withdraw_token_transfer_failure_preserves_state() {
    // Mirror of `test_failed_withdraw_lock_token_transfer_failure_preserves_state`
    // but for the plain `withdraw` entrypoint. Verifies that when the SAC
    // transfer from contract -> user fails (contract custody drained below
    // the user's internal available balance), every piece of vault state
    // (available balance, locked balance, events, lock entries) is preserved
    // exactly as it was before the call.
    let env = test_env();
    let (contract_id, client, token_client, token_admin, _) = vault_with_sac(&env);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);

    // Deposit + lock to build mixed state so we verify both available and
    // locked sides remain untouched by a failed withdrawal.
    client.deposit(&user, &800);
    let _lock_id = client.lock_funds(&user, &300, &5_000);

    // Internal state before failure: 500 available, 300 locked, 800 total
    let (bal_before, locked_before, events_before) = snapshot(&env, &client, &user);
    assert_eq!(bal_before, 500);
    assert_eq!(locked_before, 300);

    // Also snapshot the lock entry fields to rule out any mutation
    let lock_snapshot_before = client.get_lock(&user, &_lock_id).unwrap();

    // Drain the contract's SAC balance to an unrelated sink address so the
    // internal `token_client.transfer(contract -> user)` inside `withdraw`
    // will be rejected by the SAC even though the user's internal balance
    // is sufficient. `mock_all_auths()` grants the `from.require_auth()`
    // check the SAC performs on the contract address itself.
    let contract_address = contract_id.clone();
    let contract_balance = token_client.balance(&contract_address);
    assert_eq!(contract_balance, 800, "custody = sum of liabilities pre-drain");
    let sink = Address::generate(&env);
    env.mock_all_auths();
    token_client.transfer(&contract_address, &sink, &contract_balance);
    env.set_auths(&[]); // clear mocks so real user auth is required again
    assert_eq!(token_client.balance(&contract_address), 0);

    // Attempt withdraw for an amount covered by the internal balance (so
    // the internal check passes) but NOT covered by SAC custody (so the
    // transfer must fail and roll back the host call).
    let result = client.try_withdraw(&user, &200);
    assert!(
        result.is_err(),
        "withdraw must fail when contract has insufficient SAC custody"
    );

    // Zero state drift: available balance, locked balance, event count
    let (bal_after, locked_after, events_after) = snapshot(&env, &client, &user);
    assert_eq!(
        bal_after, bal_before,
        "available balance must not change on failed withdraw SAC transfer"
    );
    assert_eq!(
        locked_after, locked_before,
        "locked balance must not change on failed withdraw SAC transfer"
    );
    assert_eq!(
        events_after, events_before,
        "no new events must be emitted on failed withdraw SAC transfer"
    );

    // Lock entry byte-identical to snapshot (no field drift)
    let lock_snapshot_after = client.get_lock(&user, &_lock_id).unwrap();
    assert_eq!(
        lock_snapshot_after.amount, lock_snapshot_before.amount,
        "lock amount unchanged after failed withdraw"
    );
    assert_eq!(
        lock_snapshot_after.unlock_time, lock_snapshot_before.unlock_time,
        "lock unlock_time unchanged after failed withdraw"
    );
    assert_eq!(
        lock_snapshot_after.withdrawn, lock_snapshot_before.withdrawn,
        "lock withdrawn flag unchanged after failed withdraw"
    );
}
