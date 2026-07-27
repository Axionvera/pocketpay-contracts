//! Lock maturity boundary and replay-protection test suite for the Savings
//! Vault contract (issue #408).
//!
//! These tests verify predictable, deterministic behaviour of matured-lock
//! withdrawal around the exact maturity boundary, and that a lock cannot be
//! double-spent or touched by a non-owner:
//!
//! 1. Maturity boundary: rejected 1 second before, succeeds at and after.
//! 2. Replay protection: a second `withdraw_lock` on an already-spent lock
//!    panics ("Lock already withdrawn") and never transfers tokens twice.
//! 3. Replay via `extend_lock` after withdrawal is also rejected.
//! 4. Wrong-owner access: locks are namespaced by the `user` argument, so a
//!    different user referencing another user's lock id hits their own (empty)
//!    namespace and fails ("Lock not found") without leaking or mutating state.
//! 5. State consistency after a rejected operation: `withdrawn`, `amount`, and
//!    `get_locked_balance` remain unchanged.
//!
//! No contract logic is modified — these only exercise existing public API.

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

/// Returns the configured token address for a client.
fn token_address(client: &savings_vault::SavingsVaultClient<'static>) -> Address {
    let env = client.env.clone();
    env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&savings_vault::DataKey::Token)
            .expect("token should be set during initialization")
    })
}

/// Mints `amount` to `user` and deposits it, giving available balance to lock.
fn fund(
    client: &savings_vault::SavingsVaultClient<'static>,
    user: &Address,
    amount: i128,
) {
    let env = client.env.clone();
    let token = token_address(client);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    token_admin.mint(user, &amount);
    client.deposit(user, &amount);
}

fn token_balance(client: &savings_vault::SavingsVaultClient<'static>, user: &Address) -> i128 {
    let env = client.env.clone();
    let token = token_address(client);
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    token_client.balance(user)
}

// =========================================================================
// 1. Maturity boundary (rejection 1s before, success at/after)
// =========================================================================

/// Withdrawing 1 second before maturity is rejected and `can_withdraw` is false.
#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_maturity_one_second_before_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &500, &5_000);

    env.ledger().set_timestamp(4_999);
    assert!(
        !client.can_withdraw(&user),
        "can_withdraw must be false 1 second before maturity"
    );
    client.withdraw_lock(&user, &id);
}

/// Withdrawing at the exact maturity second succeeds.
#[test]
fn test_maturity_exact_second_succeeds() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let amount: i128 = 500;
    let id = client.lock_funds(&user, &amount, &5_000);

    env.ledger().set_timestamp(5_000);
    assert!(client.can_withdraw(&user));

    let before = token_balance(&client, &user);
    client.withdraw_lock(&user, &id);
    let after = token_balance(&client, &user);

    assert_eq!(after - before, amount, "exact-second withdraw transfers principal");
    let lock = client.get_lock(&user, &id).expect("lock must still exist");
    assert!(lock.withdrawn);
}

/// Withdrawing 1 second after maturity succeeds.
#[test]
fn test_maturity_one_second_after_succeeds() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &500, &5_000);

    env.ledger().set_timestamp(5_001);
    assert!(client.can_withdraw(&user));
    client.withdraw_lock(&user, &id);
    assert!(client.get_lock(&user, &id).unwrap().withdrawn);
}

// =========================================================================
// 2. Replay protection (double spend is impossible)
// =========================================================================

/// A second `withdraw_lock` on an already-spent lock panics and never
/// transfers tokens twice.
#[test]
fn test_replay_withdraw_already_withdrawn_blocked() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let amount: i128 = 500;
    let id = client.lock_funds(&user, &amount, &5_000);

    env.ledger().set_timestamp(5_000);
    let before = token_balance(&client, &user);
    client.withdraw_lock(&user, &id);
    let after_first = token_balance(&client, &user);
    assert_eq!(after_first - before, amount);

    // Second attempt must fail and must NOT transfer again.
    let res = client.try_withdraw_lock(&user, &id);
    assert!(res.is_err(), "replay withdraw must be rejected");
    let after_replay = token_balance(&client, &user);
    assert_eq!(
        after_replay, after_first,
        "replay must not change token balance"
    );
}

/// Replaying the exact same withdrawal (panic path) surfaces "Lock already withdrawn".
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_replay_withdraw_panics_already_withdrawn() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &500, &5_000);

    env.ledger().set_timestamp(5_000);
    client.withdraw_lock(&user, &id);
    client.withdraw_lock(&user, &id); // second call -> already withdrawn
}

/// Extending a lock after it has been withdrawn is rejected.
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_extend_after_withdraw_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &500, &5_000);

    env.ledger().set_timestamp(5_000);
    client.withdraw_lock(&user, &id);

    env.ledger().set_timestamp(6_000);
    client.extend_lock(&user, &id, &10_000);
}

// =========================================================================
// 3. Wrong-owner access (no cross-user leakage)
// =========================================================================

/// A different user referencing the victim's lock id hits their own (empty)
/// namespace and fails with "Lock not found"; the victim's lock is untouched.
#[test]
fn test_wrong_owner_withdraw_not_found_and_state_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &owner, 1_000);
    let id = client.lock_funds(&owner, &500, &5_000);

    env.ledger().set_timestamp(5_000);
    // Attacker tries to withdraw the owner's lock id under attacker's namespace.
    let res = client.try_withdraw_lock(&attacker, &id);
    assert!(res.is_err(), "wrong-owner withdraw must be rejected");

    // Owner's lock must remain fully intact and withdrawable.
    let lock = client.get_lock(&owner, &id).expect("owner lock must persist");
    assert!(!lock.withdrawn);
    assert_eq!(lock.amount, 500_i128);
    assert_eq!(client.get_locked_balance(&owner), 500_i128);

    // Owner can still withdraw their own matured lock.
    client.withdraw_lock(&owner, &id);
    assert!(client.get_lock(&owner, &id).unwrap().withdrawn);
}

/// Wrong-owner `extend_lock` likewise fails with "Lock not found".
#[test]
#[should_panic(expected = "Lock not found")]
fn test_wrong_owner_extend_not_found() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &owner, 1_000);
    let id = client.lock_funds(&owner, &500, &5_000);

    // Attacker extends their own (non-existent) lock id.
    client.extend_lock(&attacker, &id, &10_000);
}

// =========================================================================
// 4. State consistency after a rejected operation
// =========================================================================

/// A rejected immature withdrawal leaves the lock record and locked balance
/// completely unchanged.
#[test]
fn test_state_consistent_after_rejected_immature_withdraw() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let amount: i128 = 400;
    let id = client.lock_funds(&user, &amount, &50_000);

    env.ledger().set_timestamp(2_000);
    let res = client.try_withdraw_lock(&user, &id);
    assert!(res.is_err());

    let lock = client.get_lock(&user, &id).expect("lock must persist");
    assert!(!lock.withdrawn, "immature reject must not mark withdrawn");
    assert_eq!(lock.amount, amount, "immature reject must not alter amount");
    assert_eq!(
        client.get_locked_balance(&user),
        amount,
        "immature reject must not alter locked balance"
    );
}

/// A wrong-owner attempt must not disturb the victim's locked balance.
#[test]
fn test_state_consistent_after_wrong_owner_attempt() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &owner, 1_000);
    let amount: i128 = 700;
    let id = client.lock_funds(&owner, &amount, &5_000);

    env.ledger().set_timestamp(5_000);
    let _ = client.try_withdraw_lock(&attacker, &id);

    assert_eq!(
        client.get_locked_balance(&owner),
        amount,
        "wrong-owner attempt must not alter owner's locked balance"
    );
    assert!(!client.get_lock(&owner, &id).unwrap().withdrawn);
}

// =========================================================================
// 5. can_withdraw transitions correctly across the boundary + after replay
// =========================================================================

/// `can_withdraw` is false before maturity, true at/after maturity, and false
/// again once the only matured lock is withdrawn (replay guarded).
#[test]
fn test_can_withdraw_transition_and_replay_guard() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &500, &3_000);

    env.ledger().set_timestamp(2_000);
    assert!(!client.can_withdraw(&user));

    env.ledger().set_timestamp(3_000);
    assert!(client.can_withdraw(&user));
    client.withdraw_lock(&user, &id);

    assert!(
        !client.can_withdraw(&user),
        "can_withdraw false after the only matured lock is withdrawn"
    );
}
