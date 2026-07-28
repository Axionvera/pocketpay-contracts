//! Comprehensive negative-path test suite for the Savings Vault contract.
//!
//! Covers unauthorized access, invalid inputs, early withdrawals, 
//! missing initialization, and state consistency after failures.

use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{Address, Env, IntoVal};
use crate::ContractError;
use test_helpers::*;

// =========================================================================
// 1. Missing Initialization
// =========================================================================

#[test]
#[should_panic]
fn test_uninitialized_deposit_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.deposit(&user, &100);
}

#[test]
#[should_panic]
fn test_uninitialized_withdraw_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.withdraw(&user, &100);
}

#[test]
#[should_panic]
fn test_uninitialized_lock_funds_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.lock_funds(&user, &100, &2_000);
}

#[test]
#[should_panic]
fn test_uninitialized_pause_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.pause(&admin, &3600);
}

// =========================================================================
// 2. Unauthorized Access (Admin Boundaries)
// =========================================================================

#[test]
#[should_panic]
fn test_unauthorized_pause_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let rando = Address::generate(&env);
    
    // Attempt to pause as a non-admin user
    client.pause(&rando, &3600);
}

#[test]
#[should_panic]
fn test_unauthorized_unpause_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let rando = Address::generate(&env);
    
    env.mock_all_auths();
    client.pause(&vault_admin, &3600);
    
    // Intentionally omit mock_all_auths() to test auth rejection
    client.unpause(&rando);
}

#[test]
#[should_panic]
fn test_unauthorized_set_min_deposit_amount_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let rando = Address::generate(&env);
    
    client.set_min_deposit_amount(&rando, &1000);
}

// =========================================================================
// 3. Invalid Inputs
// =========================================================================

#[test]
fn test_invalid_deposit_amounts_fail() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    
    // Case 1: Zero deposit
    let res = client.try_deposit(&user, &0);
    assert!(res.is_err());
    
    // Case 2: Negative deposit
    let res = client.try_deposit(&user, &-1);
    assert!(res.is_err());
}

#[test]
fn test_deposit_below_minimum_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    
    env.mock_all_auths();
    client.set_min_deposit_amount(&vault_admin, &1000);
    
    let user = Address::generate(&env);
    token_admin.mint(&user, &2000);
    
    // Attempt deposit below minimum floor
    let res = client.try_deposit(&user, &500);
    assert!(res.is_err());
}

#[test]
fn test_lock_duration_boundary_failures() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    
    env.mock_all_auths();
    client.set_max_lock_duration(&vault_admin, &10_000);
    client.set_min_lock_duration(&vault_admin, &1_000);
    
    let user = Address::generate(&env);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    
    set_ledger_timestamp(&env, 1000);
    
    // Case 1: Duration too long
    let res = client.try_lock_funds(&user, &100, &12_000); // 11,000s duration
    assert!(res.is_err());
    
    // Case 2: Duration too short
    let res = client.try_lock_funds(&user, &100, &1_500); // 500s duration
    assert!(res.is_err());
}

// =========================================================================
// 4. Early Withdrawals & Invalid Lock States
// =========================================================================

#[test]
fn test_early_lock_withdrawal_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    
    let lock_id = client.lock_funds(&user, &500, &5000);
    
    // Attempt withdrawal at T=4999 (1s before maturity)
    set_ledger_timestamp(&env, 4999);
    let res = client.try_withdraw_lock(&user, &lock_id);
    assert!(res.is_err());
}

#[test]
fn test_extend_withdrawn_lock_fails() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    
    let lock_id = client.lock_funds(&user, &500, &5000);
    set_ledger_timestamp(&env, 5000);
    client.withdraw_lock(&user, &lock_id);
    
    // Attempt to extend a lock that has already been withdrawn
    let res = client.try_extend_lock(&user, &lock_id, &10_000);
    assert!(res.is_err());
}

// =========================================================================
// 5. State Consistency After Failures
// =========================================================================

#[test]
fn test_state_remains_consistent_after_failed_lock() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    
    env.mock_all_auths();
    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);
    
    let initial_balance = client.get_balance(&user);
    let initial_locked = client.get_locked_balance(&user);
    
    // Attempt to lock more than available
    let _ = client.try_lock_funds(&user, &1001, &5000);
    
    assert_eq!(client.get_balance(&user), initial_balance, "Balance should not change after failed lock");
    assert_eq!(client.get_locked_balance(&user), initial_locked, "Locked balance should not change after failed lock");
}

#[test]
fn test_state_consistency_after_failed_token_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id.clone(), client);
    let user = Address::generate(&env);
    
    // User has 50 tokens, tries to deposit 100
    token_admin.mint(&user, &50);
    let initial_user_tokens = token_client.balance(&user);
    let initial_vault_tokens = token_client.balance(&contract_id);
    
    env.mock_all_auths();
    let res = client.try_deposit(&user, &100);
    assert!(res.is_err(), "Deposit should fail due to insufficient SAC balance");
    
    // Verify internal accounting and external token balances are unchanged
    assert_eq!(client.get_balance(&user), 0, "Internal balance should not be credited");
    assert_eq!(token_client.balance(&user), initial_user_tokens, "User tokens should not have moved");
    assert_eq!(token_client.balance(&contract_id), initial_vault_tokens, "Vault tokens should not have changed");
}
