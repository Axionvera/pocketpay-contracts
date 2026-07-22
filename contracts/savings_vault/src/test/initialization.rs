use crate::test::test_helpers::*;
use crate::{SavingsVault, SavingsVaultClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Test 1: First initialization succeeds correctly.
#[test]
fn test_initialize() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    // Note: init_contract already initialized it, so calling again will test duplicate guard if desired, 
    // or test a separate uninitialized instance.
}

#[test]
fn test_initialize_success() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = new_user(&env);

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First initialization should succeed without error.
    client.initialize(&admin, &token);
}

/// Test 2: Repeated initialization (idempotency guard) panics.
/// Ensures the contract rejects subsequent initialization attempts to prevent state overwriting.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env); // already initialized by helper
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_fails_on_second_call() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = new_user(&env);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First initialization succeeds
    client.initialize(&admin, &token);

    // Second init with different admin
    let attacker_admin = new_user(&env);
    client.initialize(&attacker_admin, &token);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_deposit_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.deposit(&user, &100);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_withdraw_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.withdraw(&user, &100);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_lock_funds_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.lock_funds(&user, &100, &1000);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_read_functions_before_initialization() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.can_withdraw(&user), false);
}

/// Storage version tests

#[test]
#[should_panic(expected = "Unsupported storage version")]
fn test_storage_version_missing_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = new_user(&env);

    // Manually initialize without setting StorageVersion (to test missing version case!)
    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Token, &token);
        // Intentionally NOT setting DataKey::StorageVersion!
    });

    // Now try to call a function that checks storage version!
    let user = new_user(&env);
    client.get_balance(&user);
}

#[test]
#[should_panic(expected = "Unsupported storage version")]
fn test_storage_version_invalid_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = new_user(&env);

    // Manually initialize with invalid storage version 2!
    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::StorageVersion, &2_u64);
    });

    // Now try to call a function!
    let user = new_user(&env);
    client.deposit(&user, &100);
}

#[test]
fn test_storage_version_current_succeeds() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = new_user(&env);

    // Normal initialization!
    client.initialize(&admin, &token);

    // Now verify StorageVersion should be set to 1!
    let stored_version: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap()
    });
    assert_eq!(stored_version, 1);

    // All functions should work!
    let user = new_user(&env);
    client.get_balance(&user);
    client.get_locked_balance(&user);
    client.can_withdraw(&user);
}