//! Tests for lock read helpers (`get_lock`, `list_locks`).
//!
//! These tests verify the contract's read-only lock inspection surface:
//!
//! ## Response Shapes
//!
//! ### `get_lock(user, lock_id) -> Option<LockEntry>`
//! - Returns `Some(LockEntry { id, amount, unlock_time })` when the lock exists.
//! - Returns `None` when the user has no matching lock (non-existent ID or
//!   no locks at all).
//!
//! ### `list_locks(user, offset, limit) -> Vec<LockEntry>`
//! - Returns a vector of up to `limit` lock entries (capped at `MAX_LOCK_PAGE_SIZE = 50`).
//! - Entries are returned in creation order (oldest first).
//! - Returns an empty vector when:
//!   - The user has no locks.
//!   - `limit` is 0.
//!   - `offset` is past the total number of locks.
//!
//! ## Invariants
//!
//! - All read helpers are stateless: calling them does not mutate user
//!   balances, lock records, or any other contract storage.
//! - Lock IDs are unique per user and start at 1.
//! - Lock entries contain: `id: u64`, `amount: i128`, `unlock_time: u64`.

use super::test_helpers::*;
use super::*;

/// Helper: set up a contract with a user who has `amount` deposited.
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
// get_lock: empty / non-existent
// =========================================================================

#[test]
fn test_get_lock_empty_user_returns_none() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    // User has never interacted with the vault — no locks stored.
    assert_eq!(client.get_lock(&user, &1), None);
    assert_eq!(client.get_lock(&user, &0), None);
    assert_eq!(client.get_lock(&user, &u64::MAX), None);
    assert_eq!(client.list_locks(&user, &0, &10).len(), 0);
}

#[test]
fn test_get_lock_nonexistent_id_returns_none() {
    let (_env, client, user) = setup_user_with_deposit(500);
    let lock_id = client.lock_funds(&user, &200, &5_000);

    // Lock exists for lock_id, but not for lock_id+1 or 999.
    assert!(client.get_lock(&user, &lock_id).is_some());
    assert_eq!(client.get_lock(&user, &(lock_id + 1)), None);
    assert_eq!(client.get_lock(&user, &999), None);
}

// =========================================================================
// get_lock: single lock
// =========================================================================

#[test]
fn test_get_lock_single_lock() {
    let (_env, client, user) = setup_user_with_deposit(500);
    let lock_id = client.lock_funds(&user, &200, &5_000);

    let expected = LockEntry {
        id: lock_id,
        owner: user.clone(),
        amount: 200,
        created_time: 1_000,
        unlock_time: 5_000,
        withdrawn: false,
    };
    assert_eq!(client.get_lock(&user, &lock_id), Some(expected.clone()));
    assert_eq!(client.list_locks(&user, &0, &10).len(), 1);
    assert_eq!(client.list_locks(&user, &0, &10).get(0).unwrap(), expected);
}

// =========================================================================
// get_lock + list_locks: multi-lock and pagination
// =========================================================================

#[test]
fn test_get_lock_multi_lock_and_pagination() {
    let (_env, client, user) = setup_user_with_deposit(1_000);
    let id1 = client.lock_funds(&user, &100, &3_000);
    let id2 = client.lock_funds(&user, &200, &4_000);
    let id3 = client.lock_funds(&user, &300, &5_000);

    // get_lock for each
    assert_eq!(
        client.get_lock(&user, &id1),
        Some(LockEntry {
            id: id1,
            owner: user.clone(),
            amount: 100,
            created_time: 1_000,
            unlock_time: 3_000,
            withdrawn: false,
        })
    );
    assert_eq!(
        client.get_lock(&user, &id2),
        Some(LockEntry {
            id: id2,
            owner: user.clone(),
            amount: 200,
            created_time: 1_000,
            unlock_time: 4_000,
            withdrawn: false,
        })
    );
    assert_eq!(
        client.get_lock(&user, &id3),
        Some(LockEntry {
            id: id3,
            owner: user.clone(),
            amount: 300,
            created_time: 1_000,
            unlock_time: 5_000,
            withdrawn: false,
        })
    );
    assert_eq!(client.get_lock(&user, &999), None);

    // list_locks pagination
    assert_eq!(client.list_locks(&user, &0, &10).len(), 3);
    assert_eq!(client.list_locks(&user, &0, &2).len(), 2);
    assert_eq!(client.list_locks(&user, &2, &10).len(), 1);
    assert_eq!(client.list_locks(&user, &3, &10).len(), 0);
    assert_eq!(client.list_locks(&user, &0, &0).len(), 0);
}

#[test]
fn test_list_locks_offset_past_end_returns_empty() {
    let (_env, client, user) = setup_user_with_deposit(500);
    client.lock_funds(&user, &100, &3_000);

    assert_eq!(client.list_locks(&user, &5, &10).len(), 0);
    assert_eq!(client.list_locks(&user, &1, &10).len(), 0);
    assert_eq!(client.list_locks(&user, &u32::MAX, &10).len(), 0);
}

#[test]
fn test_list_locks_limit_one_returns_single_entry() {
    let (_env, client, user) = setup_user_with_deposit(1_000);
    client.lock_funds(&user, &100, &3_000);
    client.lock_funds(&user, &200, &4_000);

    let page0 = client.list_locks(&user, &0, &1);
    assert_eq!(page0.len(), 1);
    assert_eq!(page0.get(0).unwrap().amount, 100);

    let page1 = client.list_locks(&user, &1, &1);
    assert_eq!(page1.len(), 1);
    assert_eq!(page1.get(0).unwrap().amount, 200);

    let page2 = client.list_locks(&user, &2, &1);
    assert_eq!(page2.len(), 0);
}

// =========================================================================
// list_locks: MAX_LOCK_PAGE_SIZE cap
// =========================================================================

#[test]
fn test_list_locks_respects_max_page_size() {
    let (_env, client, user) = setup_user_with_deposit(10_000);

    // Create 3 locks and request a huge page — capped at 3 since only 3 exist.
    client.lock_funds(&user, &100, &3_000);
    client.lock_funds(&user, &200, &4_000);
    client.lock_funds(&user, &300, &5_000);

    let all = client.list_locks(&user, &0, &u32::MAX);
    assert_eq!(
        all.len(),
        3,
        "should return all 3 locks even with u32::MAX limit"
    );
}

// =========================================================================
// State mutation check (read-only invariant)
// =========================================================================

#[test]
fn test_read_helpers_do_not_mutate_state() {
    let (env, client, user) = setup_user_with_deposit(1_000);
    client.lock_funds(&user, &300, &3_000);
    client.lock_funds(&user, &200, &5_000);

    // Snapshot state before read calls.
    let balance_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);
    let locks_before = client.list_locks(&user, &0, &100);

    // Call read helpers multiple times.
    let _ = client.get_lock(&user, &1);
    let _ = client.get_lock(&user, &2);
    let _ = client.get_lock(&user, &999);
    let _ = client.list_locks(&user, &0, &10);
    let _ = client.list_locks(&user, &1, &5);
    let _ = client.list_locks(&user, &0, &1);

    // Verify nothing changed.
    let balance_after = client.get_balance(&user);
    let locked_after = client.get_locked_balance(&user);
    let locks_after = client.list_locks(&user, &0, &100);

    assert_eq!(
        balance_before, balance_after,
        "get_lock/list_locks must not change available balance"
    );
    assert_eq!(
        locked_before, locked_after,
        "get_lock/list_locks must not change locked balance"
    );
    assert_eq!(
        locks_before.len(),
        locks_after.len(),
        "get_lock/list_locks must not change lock count"
    );
    for i in 0..locks_before.len() {
        assert_eq!(
            locks_before.get(i).unwrap(),
            locks_after.get(i).unwrap(),
            "lock entry at index {} changed after read calls",
            i
        );
    }
}

#[test]
fn test_read_helpers_do_not_mutate_empty_state() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    // Call read helpers on a user with no data.
    let _ = client.get_lock(&user, &1);
    let _ = client.list_locks(&user, &0, &10);

    // State should still be empty.
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.list_locks(&user, &0, &10).len(), 0);
    assert_eq!(client.get_lock(&user, &1), None);
}

// =========================================================================
// Multi-user isolation
// =========================================================================

#[test]
fn test_locks_are_isolated_per_user() {
    let (env, client, user_a) = setup_user_with_deposit(1_000);
    let user_b = new_user(&env);

    client.lock_funds(&user_a, &300, &3_000);
    client.lock_funds(&user_a, &200, &5_000);

    // user_b has no locks.
    assert_eq!(client.get_lock(&user_b, &1), None);
    assert_eq!(client.get_lock(&user_b, &2), None);
    assert_eq!(client.list_locks(&user_b, &0, &10).len(), 0);

    // user_a has 2 locks.
    assert_eq!(client.list_locks(&user_a, &0, &10).len(), 2);
    assert!(client.get_lock(&user_a, &1).is_some());
    assert!(client.get_lock(&user_a, &2).is_some());
}

// =========================================================================
// Uninitialized panics
// =========================================================================

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_lock_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_lock(&user, &1);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_list_locks_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.list_locks(&user, &0, &10);
}
