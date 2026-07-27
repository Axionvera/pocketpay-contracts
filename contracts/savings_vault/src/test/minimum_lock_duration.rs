//! Minimum lock duration rule tests for the Savings Vault contract (issue #344).
//!
//! These tests verify that the admin-configurable minimum lock duration is
//! enforced by `lock_funds`, that it can be disabled (set to 0), and that only
//! the admin can change the rule.

use soroban_sdk::{testutils::Address as _, Address, Env};

fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Registers + initializes the vault and returns (env, admin, client).
fn init_with_admin(env: &Env) -> (Address, crate::SavingsVaultClient<'static>) {
    let contract_id = env.register(crate::SavingsVault, ());
    let client = crate::SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = {
        let issuer = Address::generate(env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.initialize(&admin, &token);
    (admin, client)
}

/// Mints `amount` to `user` and deposits it, so the user has available balance
/// to lock.
fn fund(client: &crate::SavingsVaultClient<'static>, user: &Address, amount: i128) {
    let env = client.env.clone();
    let token: Address = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&crate::DataKey::Token)
            .expect("token should be set during initialization")
    });
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin.mint(user, &amount);
    client.deposit(user, &amount);
}

// ---------------------------------------------------------------------------
// Rule enforcement
// ---------------------------------------------------------------------------

/// With no rule set (default), a short lock succeeds.
#[test]
fn test_lock_below_default_succeeds_when_rule_unset() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);
    let _ = admin;

    assert_eq!(client.get_min_lock_duration(), 0_u64);

    fund(&client, &user, 1_000);
    let unlock = env.ledger().timestamp() + 1;
    client.lock_funds(&user, &500, &unlock);
}

/// A lock at or above the configured minimum succeeds.
#[test]
fn test_lock_at_minimum_succeeds() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_lock_duration(&admin, &3600);
    assert_eq!(client.get_min_lock_duration(), 3600_u64);

    fund(&client, &user, 1_000);
    let unlock = env.ledger().timestamp() + 3600;
    client.lock_funds(&user, &500, &unlock);
}

/// A lock above the configured minimum succeeds.
#[test]
fn test_lock_above_minimum_succeeds() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_lock_duration(&admin, &3600);

    fund(&client, &user, 1_000);
    let unlock = env.ledger().timestamp() + 7200;
    client.lock_funds(&user, &500, &unlock);
}

/// A lock strictly below the configured minimum is rejected.
#[test]
#[should_panic(expected = "Lock duration below minimum")]
fn test_lock_below_minimum_panics() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_lock_duration(&admin, &3600);

    fund(&client, &user, 1_000);
    let unlock = env.ledger().timestamp() + 3599;
    client.lock_funds(&user, &500, &unlock);
}

/// Setting the rule to 0 disables the lower bound; short locks succeed again.
#[test]
fn test_minimum_rule_can_be_disabled() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.set_min_lock_duration(&admin, &3600);
    client.set_min_lock_duration(&admin, &0);
    assert_eq!(client.get_min_lock_duration(), 0_u64);

    fund(&client, &user, 1_000);
    let unlock = env.ledger().timestamp() + 1;
    client.lock_funds(&user, &500, &unlock);
}

// ---------------------------------------------------------------------------
// Configuration guards
// ---------------------------------------------------------------------------

/// Non-admin callers cannot change the rule.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_set_min_lock_duration_requires_admin() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    let _ = admin;
    client.set_min_lock_duration(&attacker, &3600);
}
