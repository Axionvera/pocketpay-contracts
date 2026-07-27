//! Security and governance test suite (Issue #407).
//!
//! Tests governance boundaries, unauthorized admin configuration rejection,
//! pause non-interference with user withdrawals, and parameter immutability.

use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env};

fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn init_with_admin(env: &Env) -> (Address, crate::SavingsVaultClient<'static>, Address) {
    let contract_id = env.register(crate::SavingsVault, ());
    let client = crate::SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = {
        let issuer = Address::generate(env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.initialize(&admin, &token);
    (admin, client, token)
}

fn fund(client: &crate::SavingsVaultClient<'static>, user: &Address, amount: i128) {
    let env = client.env.clone();
    let token: Address = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&crate::DataKey::Token)
            .expect("token should be set during initialization")
    });
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mint(user, &amount);
    client.deposit(user, &amount);
}

// ---------------------------------------------------------------------------
// 1. Unauthorized Admin Configuration Rejection
// ---------------------------------------------------------------------------

/// A non-admin user calling `pause` is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_pause_panics() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.pause(&attacker, &3_600);
}

/// A non-admin user calling `unpause` is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_unpause_panics() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.pause(&admin, &3_600);
    client.unpause(&attacker);
}

/// A non-admin user calling `set_min_deposit_amount` is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_set_min_deposit_panics() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.set_min_deposit_amount(&attacker, &100);
}

/// A non-admin user calling `set_max_lock_duration` is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_set_max_lock_duration_panics() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.set_max_lock_duration(&attacker, &86_400);
}

/// A non-admin user calling `set_min_lock_duration` is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_set_min_lock_duration_panics() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.set_min_lock_duration(&attacker, &3_600);
}

// ---------------------------------------------------------------------------
// 2. Admin Cannot Withdraw or Lock User Funds Without Auth
// ---------------------------------------------------------------------------

/// An admin cannot execute `withdraw` for a user without user authorization.
#[test]
#[should_panic]
fn test_admin_cannot_withdraw_user_funds_without_user_auth() {
    let env = Env::default(); // unmocked environment
    let contract_id = env.register(crate::SavingsVault, ());
    let client = crate::SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = {
        let issuer = Address::generate(&env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token);

    // Fund user using mocked auth
    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mock_all_auths().mint(&user, &1_000);
    client.mock_all_auths().deposit(&user, &500);

    // Unmocked call: Attempting to withdraw for user without user's signature fails
    client.withdraw(&user, &500);
}

/// An admin cannot execute `lock_funds` for a user without user authorization.
#[test]
#[should_panic]
fn test_admin_cannot_lock_user_funds_without_user_auth() {
    let env = Env::default(); // unmocked environment
    let contract_id = env.register(crate::SavingsVault, ());
    let client = crate::SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = {
        let issuer = Address::generate(&env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token);

    let token_admin = token::StellarAssetClient::new(&env, &token);
    token_admin.mock_all_auths().mint(&user, &1_000);
    client.mock_all_auths().deposit(&user, &500);
    env.ledger().set_timestamp(1_000);

    // Unmocked call: Attempting to lock funds for user without user's signature fails
    client.lock_funds(&user, &200, &5_000);
}

// ---------------------------------------------------------------------------
// 3. Emergency Pause Protection: User Withdrawals Remain Open
// ---------------------------------------------------------------------------

/// When the vault is actively paused, `withdraw` and `withdraw_lock` still succeed.
#[test]
fn test_pause_does_not_block_user_withdrawals() {
    let env = test_env();
    let (admin, client, token) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);

    // Create a time lock maturing at t = 2_000
    let lock_id = client.lock_funds(&user, &400, &2_000);
    assert_eq!(client.get_balance(&user), 600);

    // Admin activates emergency pause
    client.pause(&admin, &10_000);
    assert!(client.is_paused());

    // 1. Available withdrawal succeeds during pause
    client.withdraw(&user, &300);
    assert_eq!(client.get_balance(&user), 300);

    // Fast-forward to lock maturity
    env.ledger().set_timestamp(2_000);

    // 2. Matured lock withdrawal succeeds during pause
    client.withdraw_lock(&user, &lock_id);

    let token_client = token::Client::new(&env, &token);
    assert_eq!(token_client.balance(&user), 700); // 300 + 400 withdrawn
}

// ---------------------------------------------------------------------------
// 4. Immutable Parameters Protection
// ---------------------------------------------------------------------------

/// Re-initializing an active vault to alter token/admin panics.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_reinitialization_blocked() {
    let env = test_env();
    let (admin, client, token) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.initialize(&attacker, &token);
}
