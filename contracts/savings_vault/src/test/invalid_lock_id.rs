//! Invalid lock ID failure tests for the Savings Vault contract (issue #352).
//!
//! Every lock operation resolves its target by `Lock(owner, lock_id)`. These
//! tests verify that operations asked to act on a non-existent or already-spent
//! lock ID fail safely, without mutating unrelated state, so callers (and the
//! SDK) get a clear, deterministic error instead of silent corruption.

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

/// Creates one lock for `user` (with a balance to lock) and returns its id.
fn make_one_lock(
    client: &savings_vault::SavingsVaultClient<'static>,
    user: &Address,
    unlock_time: u64,
) -> u64 {
    client.lock_funds(user, &100, &unlock_time)
}

// ---------------------------------------------------------------------------
// `get_lock` — read on a missing id
// ---------------------------------------------------------------------------

/// Reading a never-created lock ID yields `None` rather than panicking.
#[test]
fn test_get_lock_on_missing_id_returns_none() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    // A user who never locked anything: id 1 must not exist.
    let result = client.get_lock(&user, &1);
    assert!(result.is_none(), "missing lock id must return None");
}

// ---------------------------------------------------------------------------
// `extend_lock` — act on a missing / spent id
// ---------------------------------------------------------------------------

/// Extending a lock that does not exist fails.
#[test]
#[should_panic(expected = "Lock not found")]
fn test_extend_lock_on_missing_id_panics() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    fund(&client, &user, 1_000);
    // id 999 was never created.
    client.extend_lock(&user, &999, &(env.ledger().timestamp() + 50_000));
}

/// Extending an already-withdrawn lock fails (its record is marked spent).
#[test]
#[should_panic(expected = "Lock already withdrawn")]
fn test_extend_lock_on_withdrawn_id_panics() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = make_one_lock(&client, &user, 2_000);

    env.ledger().set_timestamp(2_000);
    client.withdraw_lock(&user, &id);

    // The lock exists but is spent — extension must be rejected.
    client.extend_lock(&user, &id, &5_000);
}

// ---------------------------------------------------------------------------
// `withdraw_lock` — act on a missing / immature id
// ---------------------------------------------------------------------------

/// Withdrawing a lock that does not exist fails.
#[test]
#[should_panic(expected = "Lock not found")]
fn test_withdraw_lock_on_missing_id_panics() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    // id 7 was never created.
    client.withdraw_lock(&user, &7);
}

/// Withdrawing an immature lock fails (state must remain unchanged).
#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_withdraw_lock_on_immature_id_panics() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = make_one_lock(&client, &user, 100_000); // matures far in the future

    // Attempt to withdraw before maturity — must panic and leave the lock intact.
    client.withdraw_lock(&user, &id);

    // Guard: the lock record must still be present and unwithdrawn.
    let lock = client.get_lock(&user, &id).expect("lock must still exist");
    assert!(
        !lock.withdrawn,
        "immature withdraw must not mark the lock spent"
    );
}

/// A rejected invalid-id operation must not disturb an unrelated valid lock.
#[test]
fn test_invalid_id_operation_leaves_other_locks_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let good = make_one_lock(&client, &user, 5_000);

    // A bad id must fail without touching the good lock.
    let res = client.try_withdraw_lock(&user, &999);
    assert!(res.is_err(), "withdraw on missing id must fail");

    let still_there = client
        .get_lock(&user, &good)
        .expect("good lock must persist");
    assert!(!still_there.withdrawn);
    assert_eq!(still_there.amount, 100_i128);
}
