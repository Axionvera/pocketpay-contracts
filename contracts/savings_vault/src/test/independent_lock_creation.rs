//! Acceptance-criteria coverage for issue #240 ("Implement independent
//! lock creation").
//!
//! `lock_funds` already creates one `LockEntry` per call, keyed by
//! `DataKey::Lock(owner, id)` with a per-user `NextLockId` counter (see
//! `lib.rs`). This file traces each acceptance criterion from the issue
//! back to a dedicated test so the behaviour is verifiable in isolation:

extern crate std;

use alloc::vec::Vec as StdVec;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address};

use super::test_helpers::*;


// 1. Independent lock records are created

#[test]
fn test_each_lock_call_creates_its_own_independent_record() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let id_a = client.lock_funds(&user, &100, &2_000);
    let id_b = client.lock_funds(&user, &250, &3_000);

    // Each id resolves to its own record with the amount/unlock_time it was created with.
    let lock_a = client.get_lock(&user, &id_a).unwrap();
    let lock_b = client.get_lock(&user, &id_b).unwrap();
    assert_eq!((lock_a.amount, lock_a.unlock_time), (100, 2_000));
    assert_eq!((lock_b.amount, lock_b.unlock_time), (250, 3_000));
}

// 2. Each lock has a stable ID
#[test]
fn test_lock_id_remains_stable_across_later_unrelated_operations() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let id = client.lock_funds(&user, &100, &2_000);
    let before = client.get_lock(&user, &id).unwrap();

    // Unrelated activity: another lock, a deposit, a maturing withdrawal.
    client.lock_funds(&user, &50, &5_000);
    deposit_balance(&client, &user, 200);
    set_ledger_timestamp(&env, 2_000);
    client.withdraw(&user, &100);

    // The original id still resolves to the exact same record.
    let after = client.get_lock(&user, &id).unwrap();
    assert_eq!(before.id, after.id);
    assert_eq!(before.amount, after.amount);
    assert_eq!(before.unlock_time, after.unlock_time);
}

// 3. Lock IDs do not collide
#[test]
fn test_lock_ids_are_unique_and_sequential_for_a_user() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let mut ids = StdVec::new();
    for _ in 0..5 {
        ids.push(client.lock_funds(&user, &50, &2_000));
    }

    // No duplicates, and ids increase by exactly one each call.
    for w in ids.windows(2) {
        assert_eq!(w[1], w[0] + 1);
    }
}

#[test]
fn test_lock_ids_do_not_collide_across_users() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user_a = new_user(&env);
    let user_b = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user_a, &1_000);
    token_admin.mint(&user_b, &1_000);
    deposit_balance(&client, &user_a, 500);
    deposit_balance(&client, &user_b, 500);

    // Both users get id 1 first, but the (owner, id) storage key keeps
    // their records independent — reading one never returns the other's.
    let id_a = client.lock_funds(&user_a, &100, &2_000);
    let id_b = client.lock_funds(&user_b, &200, &3_000);
    assert_eq!((id_a, id_b), (1, 1));
    assert_eq!(client.get_lock(&user_a, &id_a).unwrap().amount, 100);
    assert_eq!(client.get_lock(&user_b, &id_b).unwrap().amount, 200);
}

// 4. Invalid inputs are rejected
#[test]
#[should_panic(expected = "Lock amount must be greater than zero")]
fn test_negative_lock_amount_is_rejected() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &-50, &2_000);
}

#[test]
fn test_rejected_lock_does_not_advance_the_id_counter() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 500);

    let id_before = client.lock_funds(&user, &100, &2_000);

    // An invalid call (insufficient balance) must not consume an id.
    let result = client.try_lock_funds(&user, &10_000, &3_000);
    assert!(result.is_err());

    let id_after = client.lock_funds(&user, &100, &4_000);
    assert_eq!(id_after, id_before + 1);
}

// 5. Repeated lock creation
#[test]
fn test_repeated_lock_creation_keeps_every_prior_lock_retrievable() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let amounts = [50_i128, 75, 100, 125];
    let mut ids = StdVec::new();
    for (i, amount) in amounts.iter().enumerate() {
        let unlock_time = 2_000 + i as u64;
        ids.push(client.lock_funds(&user, amount, &unlock_time));
    }

    // Every earlier lock survives later lock creation, untouched.
    for (i, id) in ids.iter().enumerate() {
        let lock = client.get_lock(&user, id).unwrap();
        assert_eq!(lock.amount, amounts[i]);
    }
    assert_eq!(client.get_locked_balance(&user), 350);
}
