//! Tests for matured-lock discovery helpers (issue #414).
//!
//! These tests verify the contract's on-chain matured-lock discovery surface:
//!
//! ## `list_matured_locks(user, offset, limit) -> Vec<LockEntry>`
//! - Returns only locks where `current_time >= unlock_time` AND `withdrawn == false`.
//! - Respects pagination via `offset` and `limit`, capped at `MAX_LOCK_PAGE_SIZE`.
//! - Returns an empty vector when no matured locks exist, `limit` is 0, or `offset`
//!   is past the number of matured locks.
//!
//! ## `get_matured_lock_count(user) -> u32`
//! - Returns the count of matured, non-withdrawn locks.
//!
//! ## `get_matured_balance(user) -> i128`
//! - Returns the sum of amounts across all matured, non-withdrawn locks.
//!
//! ## Invariants
//! - All discovery helpers are read-only: calling them does not mutate state.
//! - Withdrawn locks are excluded even if they were matured.
//! - Immature locks are excluded regardless of amount.

use super::test_helpers::*;
use super::*;

/// Helper: set up a contract with a user who has `amount` deposited at timestamp 1_000.
fn setup_user_with_deposit(
    amount: i128,
) -> (soroban_sdk::Env, SavingsVaultClient<'static>, Address) {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &amount);
    deposit_balance(&client, &user, amount);
    (env, client, user)
}

// =========================================================================
// list_matured_locks: empty / no matured locks
// =========================================================================

#[test]
fn test_list_matured_locks_empty_user() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    assert_eq!(client.list_matured_locks(&user, &0, &10).len(), 0);
    assert_eq!(client.get_matured_lock_count(&user), 0);
    assert_eq!(client.get_matured_balance(&user), 0);
}

#[test]
fn test_list_matured_locks_all_immature() {
    let (_env, client, user) = setup_user_with_deposit(1_000);
    // Create locks that mature at 5_000 and 6_000
    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);

    // At T=1_000, nothing is matured
    assert_eq!(client.list_matured_locks(&user, &0, &10).len(), 0);
    assert_eq!(client.get_matured_lock_count(&user), 0);
    assert_eq!(client.get_matured_balance(&user), 0);
}

#[test]
fn test_list_matured_locks_limit_zero_returns_empty() {
    let (env, client, user) = setup_user_with_deposit(500);
    client.lock_funds(&user, &200, &2_000);
    set_ledger_timestamp(&env, 3_000);

    // limit=0 always returns empty
    assert_eq!(client.list_matured_locks(&user, &0, &0).len(), 0);
}

// =========================================================================
// list_matured_locks: basic filtering
// =========================================================================

#[test]
fn test_list_matured_locks_filters_correctly() {
    let (env, client, user) = setup_user_with_deposit(2_000);

    // Lock 1 matures at 3_000, Lock 2 at 5_000, Lock 3 at 4_000
    let id1 = client.lock_funds(&user, &300, &3_000);
    let _id2 = client.lock_funds(&user, &400, &5_000);
    let id3 = client.lock_funds(&user, &500, &4_000);

    // At T=3_500: only lock 1 is matured
    set_ledger_timestamp(&env, 3_500);
    let matured = client.list_matured_locks(&user, &0, &10);
    assert_eq!(matured.len(), 1);
    assert_eq!(matured.get(0).unwrap().id, id1);
    assert_eq!(client.get_matured_lock_count(&user), 1);
    assert_eq!(client.get_matured_balance(&user), 300);

    // At T=4_000: locks 1 and 3 are matured
    set_ledger_timestamp(&env, 4_000);
    let matured = client.list_matured_locks(&user, &0, &10);
    assert_eq!(matured.len(), 2);
    assert_eq!(matured.get(0).unwrap().id, id1);
    assert_eq!(matured.get(1).unwrap().id, id3);
    assert_eq!(client.get_matured_lock_count(&user), 2);
    assert_eq!(client.get_matured_balance(&user), 800); // 300 + 500

    // At T=5_000: all 3 locks are matured
    set_ledger_timestamp(&env, 5_000);
    let matured = client.list_matured_locks(&user, &0, &10);
    assert_eq!(matured.len(), 3);
    assert_eq!(client.get_matured_lock_count(&user), 3);
    assert_eq!(client.get_matured_balance(&user), 1_200); // 300 + 400 + 500
}

// =========================================================================
// list_matured_locks: withdrawn locks excluded
// =========================================================================

#[test]
fn test_matured_locks_exclude_withdrawn() {
    let (env, client, user) = setup_user_with_deposit(2_000);

    let id1 = client.lock_funds(&user, &300, &3_000);
    let id2 = client.lock_funds(&user, &400, &3_000);
    let id3 = client.lock_funds(&user, &500, &3_000);

    // All mature at T=3_000
    set_ledger_timestamp(&env, 3_000);
    assert_eq!(client.get_matured_lock_count(&user), 3);
    assert_eq!(client.get_matured_balance(&user), 1_200);

    // Withdraw lock 1
    client.withdraw_lock(&user, &id1);

    // Now only 2 matured locks remain
    let matured = client.list_matured_locks(&user, &0, &10);
    assert_eq!(matured.len(), 2);
    assert_eq!(matured.get(0).unwrap().id, id2);
    assert_eq!(matured.get(1).unwrap().id, id3);
    assert_eq!(client.get_matured_lock_count(&user), 2);
    assert_eq!(client.get_matured_balance(&user), 900); // 400 + 500

    // Withdraw all remaining
    client.withdraw_lock(&user, &id2);
    client.withdraw_lock(&user, &id3);

    assert_eq!(client.list_matured_locks(&user, &0, &10).len(), 0);
    assert_eq!(client.get_matured_lock_count(&user), 0);
    assert_eq!(client.get_matured_balance(&user), 0);
}

// =========================================================================
// list_matured_locks: pagination
// =========================================================================

#[test]
fn test_list_matured_locks_pagination() {
    let (env, client, user) = setup_user_with_deposit(5_000);

    // Create 5 locks, all maturing at 2_000
    let id1 = client.lock_funds(&user, &100, &2_000);
    let id2 = client.lock_funds(&user, &200, &2_000);
    let id3 = client.lock_funds(&user, &300, &2_000);
    let id4 = client.lock_funds(&user, &400, &2_000);
    let id5 = client.lock_funds(&user, &500, &2_000);

    set_ledger_timestamp(&env, 2_000);

    // Page 1: offset=0, limit=2
    let page1 = client.list_matured_locks(&user, &0, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().id, id1);
    assert_eq!(page1.get(1).unwrap().id, id2);

    // Page 2: offset=2, limit=2
    let page2 = client.list_matured_locks(&user, &2, &2);
    assert_eq!(page2.len(), 2);
    assert_eq!(page2.get(0).unwrap().id, id3);
    assert_eq!(page2.get(1).unwrap().id, id4);

    // Page 3: offset=4, limit=2 -> only 1 remaining
    let page3 = client.list_matured_locks(&user, &4, &2);
    assert_eq!(page3.len(), 1);
    assert_eq!(page3.get(0).unwrap().id, id5);

    // Page 4: offset=5, limit=2 -> empty
    let page4 = client.list_matured_locks(&user, &5, &2);
    assert_eq!(page4.len(), 0);
}

#[test]
fn test_list_matured_locks_pagination_skips_immature() {
    let (env, client, user) = setup_user_with_deposit(5_000);

    // Lock 1: matures at 2_000
    // Lock 2: matures at 9_000 (immature at test time)
    // Lock 3: matures at 2_000
    // Lock 4: matures at 9_000 (immature at test time)
    // Lock 5: matures at 2_000
    let id1 = client.lock_funds(&user, &100, &2_000);
    let _id2 = client.lock_funds(&user, &200, &9_000);
    let id3 = client.lock_funds(&user, &300, &2_000);
    let _id4 = client.lock_funds(&user, &400, &9_000);
    let id5 = client.lock_funds(&user, &500, &2_000);

    set_ledger_timestamp(&env, 3_000);

    // Only 3 matured locks (ids 1, 3, 5), immature ones are skipped
    assert_eq!(client.get_matured_lock_count(&user), 3);

    // Page: offset=0, limit=2 -> locks 1 and 3
    let page = client.list_matured_locks(&user, &0, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().id, id1);
    assert_eq!(page.get(1).unwrap().id, id3);

    // Page: offset=2, limit=10 -> lock 5 only
    let page = client.list_matured_locks(&user, &2, &10);
    assert_eq!(page.len(), 1);
    assert_eq!(page.get(0).unwrap().id, id5);
}

#[test]
fn test_list_matured_locks_offset_past_end() {
    let (env, client, user) = setup_user_with_deposit(1_000);
    client.lock_funds(&user, &200, &2_000);
    set_ledger_timestamp(&env, 3_000);

    // Only 1 matured lock; offset=5 is past the end
    assert_eq!(client.list_matured_locks(&user, &5, &10).len(), 0);
    assert_eq!(client.list_matured_locks(&user, &u32::MAX, &10).len(), 0);
}

#[test]
fn test_list_matured_locks_respects_max_page_size() {
    let (env, client, user) = setup_user_with_deposit(10_000);

    // Create 3 matured locks and request u32::MAX limit — capped at 3 since only 3 exist
    client.lock_funds(&user, &100, &2_000);
    client.lock_funds(&user, &200, &2_000);
    client.lock_funds(&user, &300, &2_000);
    set_ledger_timestamp(&env, 3_000);

    let all = client.list_matured_locks(&user, &0, &u32::MAX);
    assert_eq!(all.len(), 3);
}

// =========================================================================
// Exact boundary conditions
// =========================================================================

#[test]
fn test_matured_locks_exact_boundary() {
    let (env, client, user) = setup_user_with_deposit(1_000);
    let id1 = client.lock_funds(&user, &500, &3_000);

    // T=2_999: 1 second before maturity -> not matured
    set_ledger_timestamp(&env, 2_999);
    assert_eq!(client.get_matured_lock_count(&user), 0);
    assert_eq!(client.get_matured_balance(&user), 0);
    assert_eq!(client.list_matured_locks(&user, &0, &10).len(), 0);

    // T=3_000: exact maturity second -> matured
    set_ledger_timestamp(&env, 3_000);
    assert_eq!(client.get_matured_lock_count(&user), 1);
    assert_eq!(client.get_matured_balance(&user), 500);
    let matured = client.list_matured_locks(&user, &0, &10);
    assert_eq!(matured.len(), 1);
    assert_eq!(matured.get(0).unwrap().id, id1);

    // T=3_001: 1 second after maturity -> still matured
    set_ledger_timestamp(&env, 3_001);
    assert_eq!(client.get_matured_lock_count(&user), 1);
    assert_eq!(client.get_matured_balance(&user), 500);
}

// =========================================================================
// Multi-user isolation
// =========================================================================

#[test]
fn test_matured_locks_isolated_per_user() {
    let (env, client, user_a) = setup_user_with_deposit(2_000);
    let user_b = new_user(&env);

    // user_a creates a lock that matures at 2_000
    client.lock_funds(&user_a, &500, &2_000);
    set_ledger_timestamp(&env, 3_000);

    // user_a has 1 matured lock
    assert_eq!(client.get_matured_lock_count(&user_a), 1);
    assert_eq!(client.get_matured_balance(&user_a), 500);
    assert_eq!(client.list_matured_locks(&user_a, &0, &10).len(), 1);

    // user_b has nothing
    assert_eq!(client.get_matured_lock_count(&user_b), 0);
    assert_eq!(client.get_matured_balance(&user_b), 0);
    assert_eq!(client.list_matured_locks(&user_b, &0, &10).len(), 0);
}

// =========================================================================
// Read-only invariant (no state mutation)
// =========================================================================

#[test]
fn test_discovery_helpers_do_not_mutate_state() {
    let (env, client, user) = setup_user_with_deposit(2_000);
    client.lock_funds(&user, &300, &2_000);
    client.lock_funds(&user, &400, &5_000);

    set_ledger_timestamp(&env, 3_000);

    // Snapshot state before discovery calls
    let balance_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);
    let locks_before = client.list_locks(&user, &0, &100);

    // Call discovery helpers multiple times
    let _ = client.list_matured_locks(&user, &0, &10);
    let _ = client.list_matured_locks(&user, &0, &1);
    let _ = client.list_matured_locks(&user, &1, &10);
    let _ = client.get_matured_lock_count(&user);
    let _ = client.get_matured_balance(&user);

    // Verify nothing changed
    assert_eq!(
        balance_before,
        client.get_balance(&user),
        "discovery helpers must not change available balance"
    );
    assert_eq!(
        locked_before,
        client.get_locked_balance(&user),
        "discovery helpers must not change locked balance"
    );
    let locks_after = client.list_locks(&user, &0, &100);
    assert_eq!(
        locks_before.len(),
        locks_after.len(),
        "discovery helpers must not change lock count"
    );
    for i in 0..locks_before.len() {
        assert_eq!(
            locks_before.get(i).unwrap(),
            locks_after.get(i).unwrap(),
            "lock entry at index {} changed after discovery calls",
            i
        );
    }
}

// =========================================================================
// Uninitialized panics
// =========================================================================

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_list_matured_locks_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.list_matured_locks(&user, &0, &10);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_matured_lock_count_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_matured_lock_count(&user);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_matured_balance_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_matured_balance(&user);
}

// =========================================================================
// Consistency with can_withdraw
// =========================================================================

#[test]
fn test_matured_count_consistent_with_can_withdraw() {
    let (env, client, user) = setup_user_with_deposit(1_000);
    client.lock_funds(&user, &500, &3_000);

    // Before maturity: can_withdraw false, count 0
    set_ledger_timestamp(&env, 2_000);
    assert!(!client.can_withdraw(&user));
    assert_eq!(client.get_matured_lock_count(&user), 0);

    // At maturity: can_withdraw true, count 1
    set_ledger_timestamp(&env, 3_000);
    assert!(client.can_withdraw(&user));
    assert_eq!(client.get_matured_lock_count(&user), 1);

    // After withdrawal: can_withdraw false, count 0
    client.withdraw_lock(&user, &1);
    assert!(!client.can_withdraw(&user));
    assert_eq!(client.get_matured_lock_count(&user), 0);
}
