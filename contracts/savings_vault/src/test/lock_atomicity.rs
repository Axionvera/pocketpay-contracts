//! Lock creation atomicity tests (issue #346).
//!
//! These tests prove that `lock_funds` updates the relevant state atomically:
//! a successful lock moves the amount from available into locked balance and
//! records the lock; a *failed* lock (invalid amount / invalid unlock time)
//! leaves available balance, locked balance, and lock records completely
//! unchanged so no partial state survives.

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
// Successful lock creation updates all state consistently
// ---------------------------------------------------------------------------

/// A successful lock moves the amount into locked balance and stores the record.
#[test]
fn test_successful_lock_creation_updates_state() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let id = client.lock_funds(&user, &300, &(env.ledger().timestamp() + 100));

    // Available drops by 300; locked rises by 300; record exists.
    assert_eq!(client.get_balance(&user), 700);
    assert_eq!(client.get_locked_balance(&user), 300);
    let lock = client.get_lock(&user, &id).expect("lock record must exist");
    assert_eq!(lock.amount, 300_i128);
    assert!(!lock.withdrawn);
}

/// Two sequential locks keep available + locked balances consistent.
#[test]
fn test_multiple_locks_stay_consistent() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let _a = client.lock_funds(&user, &200, &(env.ledger().timestamp() + 100));
    let _b = client.lock_funds(&user, &150, &(env.ledger().timestamp() + 200));

    assert_eq!(client.get_balance(&user), 650);
    assert_eq!(client.get_locked_balance(&user), 350);
}

// ---------------------------------------------------------------------------
// Failed lock creation leaves state unchanged
// ---------------------------------------------------------------------------

/// A non-positive amount is rejected and leaves balances untouched.
#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_lock_zero_amount_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    client.lock_funds(&user, &0, &(env.ledger().timestamp() + 100));
}

/// A non-positive amount leaves state fully unchanged (no partial record).
#[test]
fn test_lock_zero_amount_leaves_state_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);
    let res = client.try_lock_funds(&user, &0, &(env.ledger().timestamp() + 100));
    assert!(res.is_err());

    assert_eq!(client.get_balance(&user), 1_000);
    assert_eq!(client.get_locked_balance(&user), 0);
}

/// An unlock time in the past is rejected and leaves state unchanged.
#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_past_unlock_rejected() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(5_000);
    fund(&client, &user, 1_000);
    client.lock_funds(&user, &100, &1_000); // already in the past
}

/// A past unlock time leaves available + locked balances untouched.
#[test]
fn test_lock_past_unlock_leaves_state_intact() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(5_000);
    fund(&client, &user, 1_000);
    let res = client.try_lock_funds(&user, &100, &1_000);
    assert!(res.is_err());

    assert_eq!(client.get_balance(&user), 1_000);
    assert_eq!(client.get_locked_balance(&user), 0);
}

/// A failed lock does not create a partially-written lock record.
#[test]
fn test_failed_lock_creates_no_partial_record() {
    let env = test_env();
    let (_admin, client) = init_with_admin(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    fund(&client, &user, 1_000);

    // First, one valid lock so id=1 is taken.
    let good = client.lock_funds(&user, &100, &(env.ledger().timestamp() + 100));
    assert_eq!(good, 1_u64);

    // A failing lock attempt must not consume id=2 or write a record.
    let res = client.try_lock_funds(&user, &0, &(env.ledger().timestamp() + 100));
    assert!(res.is_err());

    assert!(
        client.get_lock(&user, &2).is_none(),
        "no partial record at id 2"
    );
    assert_eq!(client.get_balance(&user), 900);
    assert_eq!(client.get_locked_balance(&user), 100);
}
