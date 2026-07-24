//! Integration and unit tests for token-backed withdrawals.
//!
//! These tests verify that withdrawals transfer real Stellar Asset Contract (SAC)
//! tokens from contract custody to authorized users, enforce unlocked balance
//! boundaries, protect locked funds, and revert atomically on failure.

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_withdraw_transfers_tokens_to_user() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    // Mint 1,000 tokens to user and deposit 600 into vault
    token_admin.mint(&user, &1000);
    client.deposit(&user, &600);

    assert_eq!(token_client.balance(&user), 400);
    assert_eq!(token_client.balance(&client.address), 600);
    assert_eq!(client.get_balance(&user), 600);

    // Withdraw 250 tokens
    client.withdraw(&user, &250);

    // Tokens must be transferred from contract to user
    assert_eq!(token_client.balance(&user), 650);
    assert_eq!(token_client.balance(&client.address), 350);
    assert_eq!(client.get_balance(&user), 350);
}

#[test]
fn test_withdraw_lock_transfers_matured_tokens() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2_000);
    assert_eq!(token_client.balance(&user), 0);
    assert_eq!(token_client.balance(&client.address), 1000);

    // Advance time to lock maturity
    set_ledger_timestamp(&env, 2_000);

    // Withdraw matured lock
    client.withdraw_lock(&user, &lock_id);

    // Tokens must be transferred to user
    assert_eq!(token_client.balance(&user), 400);
    assert_eq!(token_client.balance(&client.address), 600);
    assert_eq!(client.get_locked_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_cannot_exceed_unlocked_balance() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &500);

    // Lock 300, leaving 200 liquid available
    client.lock_funds(&user, &300, &5_000);

    // Attempting to withdraw 201 (> 200 liquid available) before maturity must panic
    client.withdraw(&user, &201);
}

#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_locked_funds_remain_protected_early_withdrawal_fails() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &500, &5_000);

    // Attempting to withdraw lock at t=2000 (< 5000) must fail
    set_ledger_timestamp(&env, 2_000);
    client.withdraw_lock(&user, &lock_id);
}

#[test]
#[should_panic]
fn test_withdraw_requires_authorisation() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token_address);
    token_admin.mock_all_auths().mint(&user, &1000);
    client.mock_all_auths().deposit(&user, &500);

    // Calling withdraw without auth mocking must trigger require_auth panic
    client.withdraw(&user, &100);
}
