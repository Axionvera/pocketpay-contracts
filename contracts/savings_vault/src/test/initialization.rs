use crate::test::test_helpers::*;
use crate::{SavingsVault, SavingsVaultClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Test 1: First initialization succeeds correctly.
#[test]
fn test_initialize_first_time_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

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
fn test_initialize_repeated_call_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First initialization succeeds
    client.initialize(&admin, &token);

    // Second initialization with a different admin must panic (idempotency enforcement)
    let another_admin = Address::generate(&env);
    client.initialize(&another_admin, &token);
}

/// Test 3: Unauthorized initialization input/attempt fails.
/// Ensures that initialization enforces required administrative authentication (`require_auth()`).
#[test]
#[should_panic]
fn test_initialize_unauthorized_admin_fails() {
    let env = Env::default();
    // Intentionally omit mock_all_auths() to test auth rejection on initialization inputs

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Calling initialize without authorized signer signatures must fail
    client.initialize(&admin, &token);
}