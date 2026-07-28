//! Withdrawal available-balance invariant tests (issue #347).
//!
//! These tests prove that a withdrawal only ever reduces the user's *unlocked*
//! available balance and never disturbs locked funds, unrelated totals, or
//! other users' balances — the core accounting correctness guarantee.

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

// ---------------------------------------------------------------------------
// Withdrawal only touches unlocked balance
// ---------------------------------------------------------------------------

/// After a plain deposit, a withdrawal reduces only available balance.
#[test]
fn test_withdraw_reduces_only_available_balance() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 1_000);
    client.withdraw(&user, &400);

    assert_eq!(client.get_balance(&user), 600);
    assert_eq!(client.get_locked_balance(&user), 0);
}

/// A withdrawal leaves a separate locked balance untouched.
#[test]
fn test_withdraw_leaves_locked_balance_unchanged() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &250, &(env.ledger().timestamp() + 100));

    client.withdraw(&user, &500);

    // Available dropped by 500; locked portion stays at 250.
    assert_eq!(client.get_balance(&user), 250);
    assert_eq!(client.get_locked_balance(&user), 250);
    assert!(!client.get_lock(&user, &id).expect("lock").withdrawn);
}

/// A failed (immature) lock withdrawal must not change available balance.
#[test]
fn test_failed_lock_withdrawal_preserves_available_balance() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &250, &(env.ledger().timestamp() + 1_000));

    // Attempt to withdraw the immature lock — must panic, leaving state intact.
    let res = client.try_withdraw_lock(&user, &id);
    assert!(res.is_err(), "immature lock withdrawal must fail");

    assert_eq!(client.get_balance(&user), 750);
    assert_eq!(client.get_locked_balance(&user), 250);
}

// ---------------------------------------------------------------------------
// Excess / invalid withdrawal rejection
// ---------------------------------------------------------------------------

/// Withdrawing more than the available balance is rejected and state is kept.
#[test]
#[should_panic]
fn test_excess_withdraw_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 500);
    client.withdraw(&user, &999); // more than available
}

/// After a rejected excess withdrawal, the balance is unchanged.
#[test]
fn test_excess_withdraw_leaves_state_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 500);
    let res = client.try_withdraw(&user, &999);
    assert!(res.is_err());

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 0);
}

// ---------------------------------------------------------------------------
// Cross-user isolation
// ---------------------------------------------------------------------------

/// One user's withdrawal never affects another user's balances.
#[test]
fn test_withdraw_is_isolated_across_users() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    fund(&client, &alice, 1_000);
    fund(&client, &bob, 800);

    client.withdraw(&alice, &300);

    assert_eq!(client.get_balance(&alice), 700);
    assert_eq!(client.get_balance(&bob), 800);
    assert_eq!(client.get_locked_balance(&bob), 0);
}
