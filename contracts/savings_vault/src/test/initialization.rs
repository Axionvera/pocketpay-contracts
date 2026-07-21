use super::*;
use soroban_sdk::{testutils::Address as _, Address};
use crate::test::test_helpers::*;

#[test]
fn test_initialize() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);
}

#[test]
fn test_initialize_success() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);

    // Should succeed on first call
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);
    client.initialize(&admin, &token);
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_fails_on_second_call() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let admin = new_user(&env);
    let token = new_user(&env);

    // First init
    client.initialize(&admin, &token);

    // Second init with different admin to ensure overwriting is blocked
    let attacker_admin = new_user(&env);
    client.initialize(&attacker_admin, &token);
}

#[test]
#[should_panic(expected = "Contract not initialized")]
fn test_deposit_before_initialization_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    client.deposit(&user, &100);
}

#[test]
#[should_panic(expected = "Contract not initialized")]
fn test_withdraw_before_initialization_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    client.withdraw(&user, &100);
}

#[test]
#[should_panic(expected = "Contract not initialized")]
fn test_lock_funds_before_initialization_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    client.lock_funds(&user, &100, &1000);
}

#[test]
fn test_read_functions_before_initialization() {
    // Verify that read functions safely return default/0 before initialization
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.can_withdraw(&user), false);
}