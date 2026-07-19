use super::*;
use soroban_sdk::{testutils::Address as _, Address};
use crate::test::test_helpers::*;

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