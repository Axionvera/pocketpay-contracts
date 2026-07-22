//! Vault event compatibility tests (issue #245).
//!
//! Verify that every expected event topic is emitted during contract
//! operations. Breaking event changes require intentional test updates.

extern crate std;

use crate::test::test_helpers::*;
use crate::DataKey;
use soroban_sdk::{
    symbol_short, token,
    testutils::{Address as _, Events, Ledger},
    Address, Env, Symbol, TryIntoVal,
};

fn stored_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    })
}

fn has_event(env: &Env, expected: &Symbol) -> bool {
    let events = env.events().all();
    for i in 0..events.len() {
        let (_c, topics, _d) = events.get(i).unwrap();
        if topics.len() > 0 {
            let t0: Option<Symbol> = topics.get(0).and_then(|v| v.try_into_val(env).ok());
            if t0.as_ref() == Some(expected) { return true; }
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────

#[test]
fn test_initialize_event() {
    let (env, _cid, _client) = setup();
    assert!(has_event(&env, &Symbol::new(&env, "initialize")));
}

#[test]
fn test_deposit_event() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let u = Address::generate(&env);
    ta.mint(&u, &1_000);
    client.deposit(&u, &100);
    assert!(has_event(&env, &symbol_short!("deposit")));
}

#[test]
fn test_withdraw_event() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let u = Address::generate(&env);
    ta.mint(&u, &1_000);
    client.deposit(&u, &100);
    client.withdraw(&u, &50);
    assert!(has_event(&env, &symbol_short!("withdraw")));
}

#[test]
fn test_lock_event() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let u = Address::generate(&env);
    ta.mint(&u, &1_000);
    client.deposit(&u, &500);
    set_ledger_timestamp(&env, 1_000);
    client.lock_funds(&u, &200, &5_000);
    assert!(has_event(&env, &symbol_short!("lock")));
}

#[test]
fn test_withdraw_lock_event() {
    let (env, cid, client) = setup();
    let (env, _a, client, _tc, ta) = test_token(env, cid, client);
    let u = Address::generate(&env);
    ta.mint(&u, &1_000);
    set_ledger_timestamp(&env, 1_000);
    client.deposit(&u, &500);
    let lid = client.lock_funds(&u, &200, &3_000);
    set_ledger_timestamp(&env, 5_000);
    client.withdraw_lock(&u, &lid);
    assert!(has_event(&env, &Symbol::new(&env, "withdraw_lock")));
}

#[test]
fn test_pause_event() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    client.pause(&admin, &3_600);
    assert!(has_event(&env, &symbol_short!("pause")));
}

#[test]
fn test_unpause_event() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    client.pause(&admin, &3_600);
    client.unpause(&admin);
    assert!(has_event(&env, &symbol_short!("unpause")));
}

#[test]
fn test_transfer_admin_event() {
    let (env, cid, client) = setup();
    let admin = stored_admin(&env, &cid);
    let (env, _a, client, _tc, _ta) = test_token(env, cid, client);
    let na = Address::generate(&env);
    client.transfer_admin(&admin, &na);
    assert!(has_event(&env, &symbol_short!("xferadmin")));
}
