//! Vault pause-state read-helper tests (issue #354).
//!
//! Clients need to know whether vault actions are currently unavailable. The
//! `is_paused` read helper must report the active pause state accurately,
//! including explicit unpause and automatic expiry.

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};


fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

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

/// A freshly initialized vault reports not-paused.
#[test]
fn test_is_paused_false_when_not_paused() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    assert!(!client.is_paused());
}

/// After a pause, the read helper reports paused.
#[test]
fn test_is_paused_true_after_pause() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    client.pause(&admin, &3_600);
    assert!(client.is_paused());
}

/// After an explicit unpause, the read helper reports not-paused.
#[test]
fn test_is_paused_false_after_unpause() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    client.pause(&admin, &3_600);
    assert!(client.is_paused());
    client.unpause(&admin);
    assert!(!client.is_paused());
}

/// After the pause duration elapses, the read helper auto-reports not-paused.
#[test]
fn test_is_paused_false_after_auto_expiry() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);

    env.ledger().set_timestamp(1_000);
    client.pause(&admin, &100); // expires at t = 1_100
    assert!(client.is_paused());

    env.ledger().set_timestamp(2_000); // well past expiry
    assert!(!client.is_paused());
}
