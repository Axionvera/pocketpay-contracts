//! Tests for lock read helpers (`get_lock`, `list_locks`).

use super::test_helpers::*;
use super::*;

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

#[test]
fn test_get_lock_empty_user_returns_none() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    assert_eq!(client.get_lock(&user, &1), None);
    assert_eq!(client.list_locks(&user, &0, &10).len(), 0);
}

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

#[test]
fn test_get_lock_multi_lock_and_pagination() {
    let (_env, client, user) = setup_user_with_deposit(1_000);
    let id1 = client.lock_funds(&user, &100, &3_000);
    let id2 = client.lock_funds(&user, &200, &4_000);
    let id3 = client.lock_funds(&user, &300, &5_000);

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

    assert_eq!(client.list_locks(&user, &0, &10).len(), 3);
    assert_eq!(client.list_locks(&user, &0, &2).len(), 2);
    assert_eq!(client.list_locks(&user, &2, &10).len(), 1);
    assert_eq!(client.list_locks(&user, &3, &10).len(), 0);
    assert_eq!(client.list_locks(&user, &0, &0).len(), 0);
}

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
