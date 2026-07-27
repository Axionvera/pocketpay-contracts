//! Event ordering unit tests for the Savings Vault contract.
//!
//! These tests verify that multi-step vault operations emit events in exact,
//! predictable chronological order. Indexers and off-chain SDKs rely on this
//! event ordering guarantee to maintain consistent balance accounting state
//! and trace cross-contract SAC token transfers.
//!
//! ## Event Emission Sequence Rules
//!
//! 1. **Deposit**:
//!    - Event 0: SAC Token `transfer` event (`from: user`, `to: contract`, `amount`).
//!    - Event 1: SavingsVault `deposit` event (`user`, `(amount, new_balance)`).
//!
//! 2. **Withdrawal**:
//!    - Event 0: SAC Token `transfer` event (`from: contract`, `to: user`, `amount`).
//!    - Event 1: SavingsVault `withdraw` event (`user`, `(amount, new_balance)`).
//!
//! 3. **Lock Creation**:
//!    - Event 0: SavingsVault `lock` event (`user`, `(amount, unlock_time, new_balance, new_locked)`).
//!    - (No SAC transfer event since funds remain inside the contract).
//!
//! 4. **Lock Withdrawal**:
//!    - Event 0: SAC Token `transfer` event (`from: contract`, `to: user`, `amount`).
//!    - Event 1: SavingsVault `withdraw_lock` event (`user`, `(lock_id, amount)`).

extern crate std;

use super::test_helpers::*;
use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, Symbol, TryIntoVal,
};

struct EventOrderingFixture {
    env: Env,
    contract_id: Address,
    client: SavingsVaultClient<'static>,
    user: Address,
    token_address: Address,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup_event_fixture() -> EventOrderingFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = new_user(&env);
    token_admin.mint(&user, &100_000);

    let token_address = env
        .as_contract(&contract_id, || {
            env.storage().instance().get(&DataKey::Token).unwrap()
        });

    EventOrderingFixture {
        env,
        contract_id,
        client,
        user,
        token_address,
        token_admin,
    }
}

// =========================================================================
// 1. Deposit Event Ordering Tests
// =========================================================================

/// Verifies that calling `deposit` emits:
/// 1. Token SAC `transfer` event (from user to vault contract)
/// 2. Vault `deposit` event (with updated available balance)
#[test]
fn test_deposit_event_ordering() {
    let f = setup_event_fixture();
    let deposit_amount: i128 = 1_000;

    f.client.deposit(&f.user, &deposit_amount);

    let events = f.env.events().all();
    assert_eq!(
        events.len(),
        2,
        "deposit must emit exactly 2 events: SAC transfer then Vault deposit"
    );

    // Event 0: SAC Transfer Event (emitted by Token Contract)
    let (contract_0, topics_0, data_0) = events.get(0).unwrap();
    assert_eq!(contract_0, f.token_address, "event 0 must originate from SAC token contract");
    let topic0_0: Symbol = topics_0.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_0, symbol_short!("transfer"), "event 0 topic 0 must be 'transfer'");
    let from_0: Address = topics_0.get(1).unwrap().try_into_val(&f.env).unwrap();
    let to_0: Address = topics_0.get(2).unwrap().try_into_val(&f.env).unwrap();
    let amount_0: i128 = data_0.try_into_val(&f.env).unwrap();
    assert_eq!(from_0, f.user, "transfer from must be depositor");
    assert_eq!(to_0, f.contract_id, "transfer to must be vault contract");
    assert_eq!(amount_0, deposit_amount);

    // Event 1: SavingsVault Deposit Event (emitted by Vault Contract)
    let (contract_1, topics_1, data_1) = events.get(1).unwrap();
    assert_eq!(contract_1, f.contract_id, "event 1 must originate from vault contract");
    let topic0_1: Symbol = topics_1.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_1, symbol_short!("deposit"), "event 1 topic 0 must be 'deposit'");
    let user_1: Address = topics_1.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(user_1, f.user);
    let (amount_1, new_balance_1): (i128, i128) = data_1.try_into_val(&f.env).unwrap();
    assert_eq!(amount_1, deposit_amount);
    assert_eq!(new_balance_1, deposit_amount);
}

/// Verifies that multiple deposit operations maintain strict chronological ordering.
#[test]
fn test_multiple_deposits_event_ordering() {
    let f = setup_event_fixture();

    // `env.events().all()` only returns events from the single most recent
    // top-level contract call, so each call's events must be captured
    // immediately afterwards and accumulated manually.
    f.client.deposit(&f.user, &500);
    let mut events = f.env.events().all();

    f.client.deposit(&f.user, &300);
    events.append(&f.env.events().all());

    assert_eq!(events.len(), 4, "two deposits must produce 4 total events (2 per deposit)");

    // Deposit 1 (events 0, 1)
    let (contract_0, topics_0, _) = events.get(0).unwrap();
    assert_eq!(contract_0, f.token_address);
    let topic_0_symbol: Symbol = topics_0.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic_0_symbol, symbol_short!("transfer"));

    let (contract_1, topics_1, data_1) = events.get(1).unwrap();
    assert_eq!(contract_1, f.contract_id);
    let topic_1_symbol: Symbol = topics_1.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic_1_symbol, symbol_short!("deposit"));
    let (_, balance_1): (i128, i128) = data_1.try_into_val(&f.env).unwrap();
    assert_eq!(balance_1, 500);

    // Deposit 2 (events 2, 3)
    let (contract_2, topics_2, _) = events.get(2).unwrap();
    assert_eq!(contract_2, f.token_address);
    let topic_2_symbol: Symbol = topics_2.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic_2_symbol, symbol_short!("transfer"));

    let (contract_3, topics_3, data_3) = events.get(3).unwrap();
    assert_eq!(contract_3, f.contract_id);
    let topic_3_symbol: Symbol = topics_3.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic_3_symbol, symbol_short!("deposit"));
    let (_, balance_3): (i128, i128) = data_3.try_into_val(&f.env).unwrap();
    assert_eq!(balance_3, 800); // 500 + 300
}

// =========================================================================
// 2. Withdrawal Event Ordering Tests
// =========================================================================

/// Verifies that calling `withdraw` emits:
/// 1. Token SAC `transfer` event (from vault contract to user)
/// 2. Vault `withdraw` event (with updated available balance)
#[test]
fn test_withdraw_event_ordering() {
    let f = setup_event_fixture();
    f.client.deposit(&f.user, &2_000);

    // Clear event log buffer before testing withdrawal event sequence
    let _ = f.env.events().all();

    let withdraw_amount: i128 = 800;
    f.client.withdraw(&f.user, &withdraw_amount);

    let events = f.env.events().all();
    assert_eq!(
        events.len(),
        2,
        "withdraw must emit exactly 2 events: SAC transfer then Vault withdraw"
    );

    // Event 0: SAC Transfer Event (from Vault Contract to User)
    let (contract_0, topics_0, data_0) = events.get(0).unwrap();
    assert_eq!(contract_0, f.token_address, "event 0 must originate from SAC token contract");
    let topic0_0: Symbol = topics_0.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_0, symbol_short!("transfer"));
    let from_0: Address = topics_0.get(1).unwrap().try_into_val(&f.env).unwrap();
    let to_0: Address = topics_0.get(2).unwrap().try_into_val(&f.env).unwrap();
    let amount_0: i128 = data_0.try_into_val(&f.env).unwrap();
    assert_eq!(from_0, f.contract_id, "transfer from must be vault contract");
    assert_eq!(to_0, f.user, "transfer to must be withdrawer");
    assert_eq!(amount_0, withdraw_amount);

    // Event 1: SavingsVault Withdraw Event
    let (contract_1, topics_1, data_1) = events.get(1).unwrap();
    assert_eq!(contract_1, f.contract_id, "event 1 must originate from vault contract");
    let topic0_1: Symbol = topics_1.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_1, symbol_short!("withdraw"));
    let user_1: Address = topics_1.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(user_1, f.user);
    let (amount_1, new_balance_1): (i128, i128) = data_1.try_into_val(&f.env).unwrap();
    assert_eq!(amount_1, withdraw_amount);
    assert_eq!(new_balance_1, 1_200); // 2000 - 800
}

// =========================================================================
// 3. Lock Event Ordering Tests
// =========================================================================

/// Verifies that calling `lock_funds` emits:
/// 1. Vault `lock` event (with lock details and accounting balances)
/// (No SAC transfer event since funds are held internally)
#[test]
fn test_lock_event_ordering() {
    let f = setup_event_fixture();
    set_ledger_timestamp(&f.env, 10_000);
    f.client.deposit(&f.user, &3_000);
    // `env.events().all()` only returns events from the single most recent
    // top-level contract call, so it must be captured right after deposit.
    let mut events = f.env.events().all();

    let lock_amount: i128 = 1_000;
    let unlock_time: u64 = 20_000;

    let lock_id = f.client.lock_funds(&f.user, &lock_amount, &unlock_time);
    assert_eq!(lock_id, 1);
    events.append(&f.env.events().all());

    // After deposit (2 events), lock_funds produces 1 event
    assert_eq!(events.len(), 3, "deposit (2) + lock (1) = 3 total events");

    // Event 2 (last event): Vault Lock Event
    let (contract_2, topics_2, data_2) = events.get(2).unwrap();
    assert_eq!(contract_2, f.contract_id, "lock event must originate from vault contract");
    let topic0_2: Symbol = topics_2.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_2, symbol_short!("lock"));
    let user_2: Address = topics_2.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(user_2, f.user);

    let (amount, time, available, locked): (i128, u64, i128, i128) =
        data_2.try_into_val(&f.env).unwrap();
    assert_eq!(amount, lock_amount);
    assert_eq!(time, unlock_time);
    assert_eq!(available, 2_000); // 3000 - 1000
    assert_eq!(locked, 1_000);
}

// =========================================================================
// 4. Lock Withdrawal Event Ordering Tests
// =========================================================================

/// Verifies that calling `withdraw_lock` emits:
/// 1. Token SAC `transfer` event (from vault contract to user for lock amount)
/// 2. Vault `withdraw_lock` event (with lock_id and released amount)
#[test]
fn test_withdraw_lock_event_ordering() {
    let f = setup_event_fixture();
    set_ledger_timestamp(&f.env, 1_000);
    // `env.events().all()` only returns events from the single most recent
    // top-level contract call, so each call's events must be captured
    // immediately afterwards and accumulated manually.
    f.client.deposit(&f.user, &2_000);
    let mut events = f.env.events().all();

    let lock_id = f.client.lock_funds(&f.user, &1_500, &5_000);
    events.append(&f.env.events().all());

    // Fast-forward timestamp past lock maturity
    set_ledger_timestamp(&f.env, 6_000);

    // Call withdraw_lock
    f.client.withdraw_lock(&f.user, &lock_id);
    events.append(&f.env.events().all());

    // Events: Deposit (2) + Lock (1) + WithdrawLock (2) = 5 total events
    assert_eq!(events.len(), 5, "total event stream count must be 5");

    // Event 3: SAC Transfer for Matured Lock Release
    let (contract_3, topics_3, data_3) = events.get(3).unwrap();
    assert_eq!(contract_3, f.token_address, "event 3 must originate from SAC token contract");
    let topic0_3: Symbol = topics_3.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_3, symbol_short!("transfer"));
    let from_3: Address = topics_3.get(1).unwrap().try_into_val(&f.env).unwrap();
    let to_3: Address = topics_3.get(2).unwrap().try_into_val(&f.env).unwrap();
    let amount_3: i128 = data_3.try_into_val(&f.env).unwrap();
    assert_eq!(from_3, f.contract_id);
    assert_eq!(to_3, f.user);
    assert_eq!(amount_3, 1_500);

    // Event 4: Vault WithdrawLock Event
    let (contract_4, topics_4, data_4) = events.get(4).unwrap();
    assert_eq!(contract_4, f.contract_id, "event 4 must originate from vault contract");
    let topic0_4: Symbol = topics_4.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0_4, Symbol::new(&f.env, "withdraw_lock"));
    let user_4: Address = topics_4.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(user_4, f.user);
    let (emitted_lock_id, amount_4): (u64, i128) = data_4.try_into_val(&f.env).unwrap();
    assert_eq!(emitted_lock_id, lock_id);
    assert_eq!(amount_4, 1_500);
}

// =========================================================================
// 5. Multi-Step Full Vault Lifecycle Event Ordering Test
// =========================================================================

/// Comprehensive multi-step verification tracing the entire event log stream
/// through a complete vault lifecycle:
/// Step 1: Deposit (SAC Transfer -> Vault Deposit)
/// Step 2: Lock Funds (Vault Lock)
/// Step 3: Withdraw Lock (SAC Transfer -> Vault WithdrawLock)
/// Step 4: Withdraw Available (SAC Transfer -> Vault Withdraw)
#[test]
fn test_full_vault_lifecycle_event_ordering() {
    let f = setup_event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    // `env.events().all()` only returns events from the single most recent
    // top-level contract call, so each step's events must be captured
    // immediately afterwards and accumulated manually.

    // Step 1: Deposit 5,000 tokens
    f.client.deposit(&f.user, &5_000);
    let mut events = f.env.events().all();

    // Step 2: Lock 2,000 tokens until t = 10,000
    let lock_id = f.client.lock_funds(&f.user, &2_000, &10_000);
    events.append(&f.env.events().all());

    // Step 3: Advance time and withdraw locked funds
    set_ledger_timestamp(&f.env, 12_000);
    f.client.withdraw_lock(&f.user, &lock_id);
    events.append(&f.env.events().all());

    // Step 4: Withdraw remaining available balance (3,000 tokens)
    f.client.withdraw(&f.user, &3_000);
    events.append(&f.env.events().all());

    assert_eq!(
        events.len(),
        7,
        "full lifecycle must produce exactly 7 events in chronological order"
    );

    // Expected sequence summary:
    // Event 0: SAC Transfer (User -> Contract, 5000)
    // Event 1: Vault Deposit (User, amount=5000, new_balance=5000)
    // Event 2: Vault Lock (User, amount=2000, unlock_time=10000, available=3000, locked=2000)
    // Event 3: SAC Transfer (Contract -> User, 2000)
    // Event 4: Vault WithdrawLock (User, lock_id=1, amount=2000)
    // Event 5: SAC Transfer (Contract -> User, 3000)
    // Event 6: Vault Withdraw (User, amount=3000, new_balance=0)

    // Event 0
    let (c0, t0, d0) = events.get(0).unwrap();
    assert_eq!(c0, f.token_address);
    let t0_symbol: Symbol = t0.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t0_symbol, symbol_short!("transfer"));
    let amt0: i128 = d0.try_into_val(&f.env).unwrap();
    assert_eq!(amt0, 5_000);

    // Event 1
    let (c1, t1, d1) = events.get(1).unwrap();
    assert_eq!(c1, f.contract_id);
    let t1_symbol: Symbol = t1.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t1_symbol, symbol_short!("deposit"));
    let (amt1, bal1): (i128, i128) = d1.try_into_val(&f.env).unwrap();
    assert_eq!(amt1, 5_000);
    assert_eq!(bal1, 5_000);

    // Event 2
    let (c2, t2, d2) = events.get(2).unwrap();
    assert_eq!(c2, f.contract_id);
    let t2_symbol: Symbol = t2.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t2_symbol, symbol_short!("lock"));
    let (amt2, time2, avail2, locked2): (i128, u64, i128, i128) = d2.try_into_val(&f.env).unwrap();
    assert_eq!(amt2, 2_000);
    assert_eq!(time2, 10_000);
    assert_eq!(avail2, 3_000);
    assert_eq!(locked2, 2_000);

    // Event 3
    let (c3, t3, d3) = events.get(3).unwrap();
    assert_eq!(c3, f.token_address);
    let t3_symbol: Symbol = t3.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t3_symbol, symbol_short!("transfer"));
    let amt3: i128 = d3.try_into_val(&f.env).unwrap();
    assert_eq!(amt3, 2_000);

    // Event 4
    let (c4, t4, d4) = events.get(4).unwrap();
    assert_eq!(c4, f.contract_id);
    let t4_symbol: Symbol = t4.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t4_symbol, Symbol::new(&f.env, "withdraw_lock"));
    let (lid4, amt4): (u64, i128) = d4.try_into_val(&f.env).unwrap();
    assert_eq!(lid4, lock_id);
    assert_eq!(amt4, 2_000);

    // Event 5
    let (c5, t5, d5) = events.get(5).unwrap();
    assert_eq!(c5, f.token_address);
    let t5_symbol: Symbol = t5.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t5_symbol, symbol_short!("transfer"));
    let amt5: i128 = d5.try_into_val(&f.env).unwrap();
    assert_eq!(amt5, 3_000);

    // Event 6
    let (c6, t6, d6) = events.get(6).unwrap();
    assert_eq!(c6, f.contract_id);
    let t6_symbol: Symbol = t6.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(t6_symbol, symbol_short!("withdraw"));
    let (amt6, bal6): (i128, i128) = d6.try_into_val(&f.env).unwrap();
    assert_eq!(amt6, 3_000);
    assert_eq!(bal6, 0);
}
