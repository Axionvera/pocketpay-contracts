//! Minimum deposit amount rule tests for the Savings Vault contract (issue #342).
//!
//! These tests verify that the admin-configurable minimum deposit amount is
//! enforced by `deposit`, that it can be toggled off, and that negative
//! configuration values are rejected.

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Registers + initializes the vault and returns (env, admin, client).
fn init_with_admin(env: &Env) -> (Address, savings_vault::SavingsVaultClient<'static>) {
    let contract_id = env.register(savings_vault::SavingsVault, ());
    let client = savings_vault::SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = {
        let issuer = Address::generate(env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.initialize(&admin, &token);
    (admin, client)
}

fn deposit_balance(client: &savings_vault::SavingsVaultClient<'static>, user: &Address, amount: i128) {
    let env = client.env.clone();
    let token: Address = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&savings_vault::DataKey::Token)
            .expect("token should be set during initialization")
    });
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin.mint(user, &amount);
    client.deposit(user, &amount);
}

// ---------------------------------------------------------------------------
// Rule enforcement
// ---------------------------------------------------------------------------

/// With no rule set (default), a tiny positive deposit succeeds.
#[test]
fn test_deposit_below_default_succeeds_when_rule_unset() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    assert_eq!(client.get_min_deposit_amount(), 0_i128);
    deposit_balance(&client, &user, 1);
    assert_eq!(client.get_balance(&user), 1_i128);
    let _ = admin;
}

/// A deposit below the configured minimum is rejected.
#[test]
#[should_panic(expected = "Amount is below the minimum deposit amount")]
fn test_deposit_below_minimum_panics() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_deposit_amount(&admin, &100);
    assert_eq!(client.get_min_deposit_amount(), 100_i128);

    deposit_balance(&client, &user, 99);
}

/// A deposit exactly at the configured minimum succeeds.
#[test]
fn test_deposit_at_minimum_succeeds() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_deposit_amount(&admin, &100);
    deposit_balance(&client, &user, 100);
    assert_eq!(client.get_balance(&user), 100_i128);
}

/// A deposit above the configured minimum succeeds.
#[test]
fn test_deposit_above_minimum_succeeds() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_deposit_amount(&admin, &100);
    deposit_balance(&client, &user, 250);
    assert_eq!(client.get_balance(&user), 250_i128);
}

/// Setting the rule to 0 disables the floor; small deposits succeed again.
#[test]
fn test_minimum_rule_can_be_disabled() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_deposit_amount(&admin, &100);
    client.set_min_deposit_amount(&admin, &0);
    assert_eq!(client.get_min_deposit_amount(), 0_i128);

    deposit_balance(&client, &user, 1);
    assert_eq!(client.get_balance(&user), 1_i128);
}

// ---------------------------------------------------------------------------
// Configuration guards
// ---------------------------------------------------------------------------

/// Non-admin callers cannot change the rule.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_set_min_deposit_amount_requires_admin() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    let _ = admin;
    client.set_min_deposit_amount(&attacker, &100);
}

/// Negative minimum values are rejected by the setter.
#[test]
#[should_panic(expected = "Min deposit amount cannot be negative")]
fn test_set_min_deposit_amount_rejects_negative() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);

    client.set_min_deposit_amount(&admin, &-1);
}
