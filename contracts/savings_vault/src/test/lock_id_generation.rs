//! Vault lock ID generation tests for the Savings Vault contract (issue #351).
//!
//! These tests verify that `lock_funds` hands out stable, sequential,
//! per-user lock IDs and never reuses an ID (the `NextLockId` counter only
//! increases). Lock IDs are the handle every other lock operation
//! (`get_lock`, `extend_lock`, `withdraw_lock`) relies on, so their
//! generation must be deterministic and collision-free.

use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};


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

/// Mints `amount` to `user` and deposits it, so the user has available balance
/// to lock.
fn fund(client: &savings_vault::SavingsVaultClient<'static>, user: &Address, amount: i128) {
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
// Sequential, gap-free generation
// ---------------------------------------------------------------------------

/// Consecutive `lock_funds` calls produce strictly increasing IDs (1, 2, 3…).
#[test]
fn test_lock_ids_are_sequential() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 1_000);
    let id1 = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 10));
    let id2 = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 20));
    let id3 = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 30));

    assert_eq!(id1, 1_u64);
    assert_eq!(id2, 2_u64);
    assert_eq!(id3, 3_u64);
}

/// IDs are generated per-user, so two independent users each start at 1.
#[test]
fn test_lock_ids_are_per_user() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    fund(&client, &user_a, 1_000);
    fund(&client, &user_b, 1_000);

    let id_a = client.lock_funds(&user_a, &100, &(env.ledger().timestamp() + 10));
    let id_b = client.lock_funds(&user_b, &100, &(env.ledger().timestamp() + 10));

    // Each user has an independent counter starting at 1.
    assert_eq!(id_a, 1_u64);
    assert_eq!(id_b, 1_u64);
}

// ---------------------------------------------------------------------------
// Stability + referential integrity
// ---------------------------------------------------------------------------

/// A returned ID resolves to exactly the lock that was created with it.
#[test]
fn test_lock_id_resolves_to_created_lock() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &250, &(env.ledger().timestamp() + 100));

    let lock = client
        .get_lock(&user, &id)
        .expect("lock must exist for returned id");
    assert_eq!(lock.id, id);
    assert_eq!(lock.amount, 250_i128);
    assert_eq!(lock.owner, user);
}

/// An ID stays bound to its record even after other locks are created.
#[test]
fn test_lock_id_stays_stable_across_other_locks() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 2_000);
    let first = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 10));
    // Unrelated later lock must not disturb the first ID's record.
    let _second = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 20));

    let lock = client
        .get_lock(&user, &first)
        .expect("first lock must still resolve");
    assert_eq!(lock.id, first);
    assert_eq!(lock.amount, 100_i128);
}

// ---------------------------------------------------------------------------
// Non-reuse
// ---------------------------------------------------------------------------

/// Withdrawing a matured lock must not free its ID for reuse.
#[test]
fn test_lock_id_is_not_reused_after_withdrawal() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    // Start at a known timestamp so we can mature the lock deterministically.
    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);

    let id = client.lock_funds(&user, &100, &2_000); // matures at T=2_000
    let _other = client.lock_funds(&user, &100, &3_000);

    // Mature and withdraw the first lock.
    env.ledger().set_timestamp(2_000);
    client.withdraw_lock(&user, &id);

    // The next lock must continue the counter (id 3), not reuse id 1.
    let next = client.lock_funds(&user, &100, &4_000);
    assert_eq!(next, 3_u64);
}
