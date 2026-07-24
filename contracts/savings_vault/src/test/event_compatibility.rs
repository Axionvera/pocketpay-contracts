//! Vault event schema regression tests (closes #6, follows up on #245 / #270).
//!
//! These tests lock down the on-chain event contract that SDKs, mobile
//! wallets, and indexers rely on. Each test asserts the full published
//! event: the exact topic tuple `(Symbol, Address)`, the typed data
//! payload, and the number of events produced by a single call. A
//! previous version of this file only checked topic0 presence, which let
//! payload shape and event count drift silently.

extern crate std;

use crate::test::test_helpers::*;
use crate::{DataKey, SavingsVault, SavingsVaultClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, Symbol, TryIntoVal, Val, Vec,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register the vault + SAC and initialize with a caller-owned admin address.
///
/// Returns the admin used in the initialize call so tests can assert against
/// it directly without invoking another contract call (which would clear the
/// per-invocation events buffer).
fn setup_with_admin() -> (Env, Address, Address, Address, SavingsVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_issuer = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_issuer)
        .address();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    client.initialize(&admin, &token);

    (env, contract_id, admin, token, client)
}

fn stored_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    })
}

/// Returns every event whose first topic equals the given symbol.
fn events_with_topic0(env: &Env, expected: &Symbol) -> std::vec::Vec<(Address, Vec<Val>, Val)> {
    let all = env.events().all();
    let mut out = std::vec::Vec::new();
    for i in 0..all.len() {
        let (contract, topics, data) = all.get(i).unwrap();
        if topics.len() == 0 {
            continue;
        }
        let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(env).ok());
        if t0.as_ref() == Some(expected) {
            out.push((contract, topics, data));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// initialize — topics: (Symbol("initialize"), admin), data: token
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_event_schema() {
    let (env, _cid, admin, token, _client) = setup_with_admin();

    // Immediately inspect the events buffer; any other client/contract call
    // would clear it in the test env.
    let matches = events_with_topic0(&env, &Symbol::new(&env, "initialize"));
    assert_eq!(
        matches.len(),
        1,
        "initialize must emit exactly one event; the prior double-publish was a bug"
    );

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2, "initialize topic tuple is (Symbol, admin)");
    let topic1_admin: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_admin, admin);

    let emitted_token: Address = data.try_into_val(&env).unwrap();
    assert_eq!(emitted_token, token);
}

// ---------------------------------------------------------------------------
// deposit — topics: (Symbol("deposit"), user), data: (amount, new_balance)
// ---------------------------------------------------------------------------

#[test]
fn test_deposit_event_schema() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);

    client.deposit(&user, &100);

    let matches = events_with_topic0(&env, &symbol_short!("deposit"));
    assert_eq!(matches.len(), 1, "deposit emits exactly one deposit event");

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_user: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_user, user);

    let (amount, new_balance): (i128, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(amount, 100);
    assert_eq!(new_balance, 100);
}

#[test]
fn test_deposit_event_reflects_running_balance() {
    // Guards against a subtle regression where a future refactor emits the
    // deposit `amount` in the `new_balance` slot instead of the post-deposit
    // running total. The test env clears events between invocations, so we
    // seed a non-zero prior balance and then check only the most recent
    // deposit's payload.
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);
    client.deposit(&user, &100);

    client.deposit(&user, &250);

    let matches = events_with_topic0(&env, &symbol_short!("deposit"));
    assert!(
        !matches.is_empty(),
        "second deposit must publish a deposit event"
    );
    let (_c, _t, d) = matches.last().unwrap();
    let (amount, new_balance): (i128, i128) = d.try_into_val(&env).unwrap();
    assert_eq!(amount, 250, "amount slot carries the deposit amount");
    assert_eq!(
        new_balance, 350,
        "new_balance slot carries the running total, not the delta"
    );
}

// ---------------------------------------------------------------------------
// withdraw — topics: (Symbol("withdraw"), user),
//            data: (amount, new_balance, new_locked)
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_event_schema() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);
    client.deposit(&user, &500);

    client.withdraw(&user, &200);

    let matches = events_with_topic0(&env, &symbol_short!("withdraw"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_user: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_user, user);

    let (amount, new_balance, new_locked): (i128, i128, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(amount, 200);
    assert_eq!(new_balance, 300);
    assert_eq!(new_locked, 0);
}

#[test]
fn test_withdraw_event_carries_locked_balance() {
    // With an active lock in place, the third payload field must reflect the
    // *unmatured* locked total, not just the deposited-balance delta.
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &10_000);
    client.withdraw(&user, &100);

    let matches = events_with_topic0(&env, &symbol_short!("withdraw"));
    let (_c, _t, d) = matches.last().unwrap();
    let (amount, new_balance, new_locked): (i128, i128, i128) = d.try_into_val(&env).unwrap();
    assert_eq!(amount, 100);
    assert_eq!(new_balance, 200);
    assert_eq!(new_locked, 200);
}

// ---------------------------------------------------------------------------
// lock_funds — topics: (Symbol("lock"), user),
//              data: (amount, unlock_time, new_balance, new_locked)
// ---------------------------------------------------------------------------

#[test]
fn test_lock_event_schema() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);
    set_ledger_timestamp(&env, 1_000);
    client.deposit(&user, &500);

    client.lock_funds(&user, &200, &5_000);

    let matches = events_with_topic0(&env, &symbol_short!("lock"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_user: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_user, user);

    let (amount, unlock_time, new_balance, new_locked): (i128, u64, i128, i128) =
        data.try_into_val(&env).unwrap();
    assert_eq!(amount, 200);
    assert_eq!(unlock_time, 5_000);
    assert_eq!(new_balance, 300);
    assert_eq!(new_locked, 200);
}

// ---------------------------------------------------------------------------
// withdraw_lock — topics: (Symbol("withdraw_lock"), user), data: (lock_id, amount)
// ---------------------------------------------------------------------------

#[test]
fn test_withdraw_lock_event_schema() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &1_000);
    set_ledger_timestamp(&env, 1_000);
    client.deposit(&user, &500);
    let lid = client.lock_funds(&user, &200, &3_000);
    set_ledger_timestamp(&env, 5_000);

    client.withdraw_lock(&user, &lid);

    let matches = events_with_topic0(&env, &Symbol::new(&env, "withdraw_lock"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_user: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_user, user);

    let (emitted_lock_id, amount): (u64, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(emitted_lock_id, lid);
    assert_eq!(amount, 200);
}

// ---------------------------------------------------------------------------
// pause — topics: (Symbol("pause"), admin), data: expiry (u64)
// ---------------------------------------------------------------------------

#[test]
fn test_pause_event_schema() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    set_ledger_timestamp(&env, 1_000);

    client.pause(&admin, &3_600);

    let matches = events_with_topic0(&env, &symbol_short!("pause"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_admin: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_admin, admin);

    let expiry: u64 = data.try_into_val(&env).unwrap();
    assert_eq!(expiry, 1_000 + 3_600);
}

// ---------------------------------------------------------------------------
// unpause — topics: (Symbol("unpause"), admin), data: ()
// ---------------------------------------------------------------------------

#[test]
fn test_unpause_event_schema() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    client.pause(&admin, &3_600);
    client.unpause(&admin);

    let matches = events_with_topic0(&env, &symbol_short!("unpause"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_admin: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_admin, admin);

    // Payload is the unit value ().
    let empty: () = data.try_into_val(&env).unwrap();
    let _ = empty;
}

// ---------------------------------------------------------------------------
// transfer_admin — topics: (Symbol("xferadmin"), old_admin), data: new_admin
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_admin_event_schema() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    let new_admin = Address::generate(&env);

    client.transfer_admin(&admin, &new_admin);

    let matches = events_with_topic0(&env, &symbol_short!("xferadmin"));
    assert_eq!(matches.len(), 1);

    let (_contract, topics, data) = &matches[0];
    assert_eq!(topics.len(), 2);
    let topic1_old_admin: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1_old_admin, admin);

    let emitted_new_admin: Address = data.try_into_val(&env).unwrap();
    assert_eq!(emitted_new_admin, new_admin);
}

// ---------------------------------------------------------------------------
// Revert paths must not leak events.
// ---------------------------------------------------------------------------

#[test]
fn test_reverted_deposit_emits_no_event() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let user = Address::generate(&env);
    ta.mint(&user, &10);

    // Attempting to deposit more than the user's token balance reverts inside
    // the SAC transfer; the deposit event must not appear.
    let before = events_with_topic0(&env, &symbol_short!("deposit")).len();
    let _ = client.try_deposit(&user, &100);
    let after = events_with_topic0(&env, &symbol_short!("deposit")).len();
    assert_eq!(
        before, after,
        "failed deposit must not emit a deposit event"
    );
}
