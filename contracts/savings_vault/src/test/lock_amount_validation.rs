//! Lock amount validation tests (issue #345).
//!
//! `lock_funds` must reject unusable lock amounts before they can corrupt
//! vault accounting: zero/negative/malformed amounts, and amounts above the
//! user's available balance. Valid amounts within bounds must succeed and
//! leave accounting consistent.

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};

fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

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

/// Zero lock amount is rejected with a clear error.
#[test]
#[should_panic]
fn test_zero_lock_amount_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    client.lock_funds(&user, &0, &(env.ledger().timestamp() + 100));
}

/// Negative (malformed) lock amount is rejected with a clear error.
#[test]
#[should_panic]
fn test_negative_lock_amount_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    client.lock_funds(&user, &-5, &(env.ledger().timestamp() + 100));
}

/// Lock amount above the available balance is rejected.
#[test]
#[should_panic]
fn test_excess_lock_amount_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 500);
    client.lock_funds(&user, &999, &(env.ledger().timestamp() + 100));
}

/// An exact-balance lock succeeds and moves the full amount into locked state.
#[test]
fn test_exact_balance_lock_succeeds() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 500);
    let id = client.lock_funds(&user, &500, &(env.ledger().timestamp() + 100));

    assert_eq!(client.get_balance(&user), 0);
    assert_eq!(client.get_locked_balance(&user), 500);
    let lock = client.get_lock(&user, &id).expect("lock must exist");
    assert_eq!(lock.amount, 500_i128);
    assert!(!lock.withdrawn);
}

/// A large but valid amount (within balance) locks successfully.
#[test]
fn test_large_valid_lock_succeeds() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000_000);
    let id = client.lock_funds(&user, &1_000_000, &(env.ledger().timestamp() + 100));

    assert_eq!(client.get_locked_balance(&user), 1_000_000);
    let lock = client.get_lock(&user, &id).expect("lock must exist");
    assert_eq!(lock.amount, 1_000_000_i128);
}

/// A rejected excess lock leaves the available balance untouched.
#[test]
fn test_excess_lock_leaves_state_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 500);
    let res = client.try_lock_funds(&user, &999, &(env.ledger().timestamp() + 100));
    assert!(res.is_err(), "excess lock must fail");

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 0);
}
