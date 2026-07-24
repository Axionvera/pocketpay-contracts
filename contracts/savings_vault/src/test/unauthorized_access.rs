//! Security test suite covering unauthorized access, cross-user isolation,
//! and signature validation failures (Issue #153).

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Test 1: An unauthorized deposit attempt fails when authorization is missing.
#[test]
#[should_panic]
fn test_unauthorized_deposit_fails() {
    let env = Env::default();
    // Intentionally omit mock_all_auths() to test auth rejection
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);

    // Calling deposit without authorized signer must panic
    client.deposit(&user_a, &500);
}

/// Test 2: An unauthorized withdrawal attempt fails when authorization is missing.
#[test]
#[should_panic]
fn test_unauthorized_withdraw_fails() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);

    // Calling withdraw without authorized signer must panic
    client.withdraw(&user_a, &500);
}

/// Test 3: An unauthorized fund locking attempt fails when authorization is missing.
#[test]
#[should_panic]
fn test_unauthorized_lock_fails() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);
    let unlock_time = 3600;

    // Calling lock_funds without authorized signer must panic
    client.lock_funds(&user_a, &500, &unlock_time);
}

/// Test 4: An unauthorized withdraw_lock attempt fails when authorization is missing.
#[test]
#[should_panic]
fn test_unauthorized_withdraw_lock_fails() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user_a = Address::generate(&env);

    // Calling withdraw_lock without authorized signer must panic
    client.withdraw_lock(&user_a, &1);
}
