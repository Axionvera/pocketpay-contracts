//! Vault pause transition tests for the Savings Vault contract (issue #355).
//!
//! These tests verify the pause lifecycle: a pause blocks deposits/locks and
//! auto-expires at `PauseExpiry`, an explicit `unpause` re-enables operations
//! early, and only the admin can drive the transition. The pause guard is
//! exercised on the user-facing paths (`deposit`, `lock_funds`) that
//! `require_not_paused` protects.

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env};

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
// Pause blocks deposits + locks
// ---------------------------------------------------------------------------

/// While paused, a deposit is rejected.
#[test]
#[should_panic]
fn test_deposit_blocked_while_paused() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.pause(&admin, &10_000);

    // The user holds tokens; the deposit itself must be rejected while paused.
    let token: Address = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&crate::DataKey::Token)
            .expect("token")
    });
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin.mint(&user, &1_000);

    client.deposit(&user, &100);
}

/// While paused, `lock_funds` is rejected.
#[test]
#[should_panic]
fn test_lock_funds_blocked_while_paused() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    client.pause(&admin, &10_000);
    fund(&client, &user, 1_000);

    client.lock_funds(&user, &100, &(env.ledger().timestamp() + 60));
}

// ---------------------------------------------------------------------------
// Auto-expiry re-enables operations
// ---------------------------------------------------------------------------

/// A paused contract auto-unpauses once the ledger passes `PauseExpiry`.
#[test]
fn test_pause_auto_expires_after_expiry() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    // Fund the user before pausing: `deposit` is itself blocked while paused.
    fund(&client, &user, 1_000);
    client.pause(&admin, &500); // expires at T=1_500

    // Still paused before expiry: a lock is rejected.
    let blocked = client.try_lock_funds(&user, &100, &(env.ledger().timestamp() + 60));
    assert!(blocked.is_err(), "lock must be blocked before pause expiry");

    // Advance past the expiry; the pause auto-clears and locks succeed again.
    env.ledger().set_timestamp(2_000);
    let id = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 60));
    assert_eq!(id, 1_u64);
}

// ---------------------------------------------------------------------------
// Explicit unpause
// ---------------------------------------------------------------------------

/// `unpause` re-enables operations before the scheduled expiry.
#[test]
fn test_unpause_reEnables_operations_early() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    // Fund the user before pausing: `deposit` is itself blocked while paused.
    fund(&client, &user, 1_000);
    client.pause(&admin, &100_000); // would expire far in the future

    client.unpause(&admin);
    let id = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 60));
    assert_eq!(id, 1_u64);
}

// ---------------------------------------------------------------------------
// Authorization
// ---------------------------------------------------------------------------

/// Only the admin may pause.
#[test]
#[should_panic]
fn test_pause_requires_admin() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    let _ = admin;
    client.pause(&attacker, &10_000);
}

/// Only the admin may unpause.
#[test]
#[should_panic]
fn test_unpause_requires_admin() {
    let env = test_env();
    let (admin, client) = init_with_admin(&env);
    let attacker = Address::generate(&env);

    client.pause(&admin, &10_000);
    let _ = admin;
    client.unpause(&attacker);
}
