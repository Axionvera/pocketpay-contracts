//! Event schema tests for the Savings Vault contract.
//!
//! These tests verify that each vault action emits events with the correct
//! topic structure and payload types/values. This serves as a contract for
//! off-chain consumers (indexers, SDKs) that rely on event shape stability.
//!
//! ## Event Schema Reference
//!
//! | Action          | Topic[0] (Symbol)      | Topic[1] (Address) | Payload                                         |
//! |-----------------|------------------------|--------------------|-------------------------------------------------|
//! | initialize      | `"initialize"`         | admin              | token (Address)                                 |
//! | deposit         | `"deposit"`            | user               | (amount: i128, new_balance: i128)               |
//! | withdraw        | `"withdraw"`           | user               | (amount: i128, new_balance: i128, new_locked: i128) |
//! | lock            | `"lock"`               | user               | (amount: i128, unlock_time: u64, available: i128, locked: i128) |
//! | withdraw_lock   | `"withdraw_lock"`      | user               | (lock_id: u64, amount: i128)                    |
//! | transfer_admin  | `"transfer_admin"`     | old_admin          | new_admin (Address)                             |

use super::test_helpers::*;
use super::*;
use soroban_sdk::{symbol_short, Symbol, TryIntoVal};

// =========================================================================
// Helper: set up a vault with a user who has tokens and an initial deposit
// =========================================================================

struct EventFixture {
    env: Env,
    client: SavingsVaultClient<'static>,
    admin: Address,
    user: Address,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
}

/// Creates a fully initialized vault with a user who has 10_000 tokens minted.
fn event_fixture() -> EventFixture {
    let (env, contract_id, client) = setup();
    let (env, admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &10_000);
    EventFixture {
        env,
        client,
        admin,
        user,
        token_client,
        token_admin,
    }
}

// =========================================================================
// Initialize Event
// =========================================================================

/// The initialize event must emit:
/// - topics[0] = Symbol("initialize")
/// - topics[1] = admin address
/// - payload = token address
#[test]
fn event_schema_initialize() {
    let env = test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    client.mock_all_auths().initialize(&admin, &token);

    let events = env.events().all();
    // The contract may emit duplicate initialize events (existing bug);
    // grab the last one to verify schema.
    let last_idx = events.len() - 1;
    let (_contract, topics, data) = events.get(last_idx).unwrap();

    // Topic[0]: Symbol
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "initialize"));

    // Topic[1]: admin address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic1, admin);

    // Payload: token address
    let emitted_token: Address = data.try_into_val(&env).unwrap();
    assert_eq!(emitted_token, token);
}

// =========================================================================
// Deposit Event
// =========================================================================

/// The deposit event must emit:
/// - topics[0] = Symbol("deposit")
/// - topics[1] = user address
/// - payload = (amount: i128, new_balance: i128)
#[test]
fn event_schema_deposit() {
    let f = event_fixture();
    let deposit_amount: i128 = 500;

    f.client.deposit(&f.user, &deposit_amount);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    // Topic[0]: Symbol("deposit")
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("deposit"));

    // Topic[1]: user address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.user);

    // Payload: (amount, new_balance)
    let (amount, new_balance): (i128, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, deposit_amount);
    assert_eq!(new_balance, deposit_amount);
}

/// Multiple deposits must each emit their own event with correct cumulative balance.
#[test]
fn event_schema_deposit_multiple() {
    let f = event_fixture();

    f.client.deposit(&f.user, &200);
    f.client.deposit(&f.user, &300);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("deposit"));

    let (amount, new_balance): (i128, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, 300);
    assert_eq!(new_balance, 500); // 200 + 300
}

// =========================================================================
// Withdraw Event
// =========================================================================

/// The withdraw event must emit:
/// - topics[0] = Symbol("withdraw")
/// - topics[1] = user address
/// - payload = (amount: i128, new_balance: i128, new_locked: i128)
#[test]
fn event_schema_withdraw() {
    let f = event_fixture();

    f.client.deposit(&f.user, &1_000);
    f.client.withdraw(&f.user, &400);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    // Topic[0]: Symbol("withdraw")
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("withdraw"));

    // Topic[1]: user address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.user);

    // Payload: (amount, new_balance, new_locked)
    let (amount, new_balance, new_locked): (i128, i128, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, 400);
    assert_eq!(new_balance, 600);
    assert_eq!(new_locked, 0);
}

/// Withdraw after locking funds must report correct new_locked in the event.
#[test]
fn event_schema_withdraw_with_locks() {
    let f = event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    f.client.deposit(&f.user, &1_000);
    f.client.lock_funds(&f.user, &300, &5_000);
    // available=700, locked=300
    f.client.withdraw(&f.user, &200);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("withdraw"));

    let (amount, new_balance, new_locked): (i128, i128, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, 200);
    assert_eq!(new_balance, 500); // 700 - 200
    assert_eq!(new_locked, 300); // unchanged
}

// =========================================================================
// Lock Event
// =========================================================================

/// The lock event must emit:
/// - topics[0] = Symbol("lock")
/// - topics[1] = user address
/// - payload = (amount: i128, unlock_time: u64, available: i128, locked: i128)
#[test]
fn event_schema_lock() {
    let f = event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    f.client.deposit(&f.user, &1_000);
    f.client.lock_funds(&f.user, &400, &5_000);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    // Topic[0]: Symbol("lock")
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("lock"));

    // Topic[1]: user address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.user);

    // Payload: (amount, unlock_time, available, locked)
    let (amount, unlock_time, available, locked): (i128, u64, i128, i128) =
        data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, 400);
    assert_eq!(unlock_time, 5_000);
    assert_eq!(available, 600); // 1000 - 400
    assert_eq!(locked, 400); // sum of all locks
}

/// Locking again must accumulate locked totals in the event payload.
#[test]
fn event_schema_lock_multiple() {
    let f = event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    f.client.deposit(&f.user, &1_000);
    f.client.lock_funds(&f.user, &200, &3_000);
    f.client.lock_funds(&f.user, &300, &5_000);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, symbol_short!("lock"));

    let (amount, unlock_time, available, locked): (i128, u64, i128, i128) =
        data.try_into_val(&f.env).unwrap();
    assert_eq!(amount, 300);
    assert_eq!(unlock_time, 5_000);
    assert_eq!(available, 500); // 1000 - 200 - 300
    assert_eq!(locked, 500); // 200 + 300
}

// =========================================================================
// Withdraw Lock Event
// =========================================================================

/// The withdraw_lock event must emit:
/// - topics[0] = Symbol("withdraw_lock")
/// - topics[1] = user address
/// - payload = (lock_id: u64, amount: i128)
#[test]
fn event_schema_withdraw_lock() {
    let f = event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    f.client.deposit(&f.user, &1_000);
    let lock_id = f.client.lock_funds(&f.user, &500, &2_000);

    // Advance past unlock time
    set_ledger_timestamp(&f.env, 3_000);
    f.client.withdraw_lock(&f.user, &lock_id);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    // Topic[0]: Symbol("withdraw_lock")
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, Symbol::new(&f.env, "withdraw_lock"));

    // Topic[1]: user address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.user);

    // Payload: (lock_id, amount)
    let (emitted_lock_id, amount): (u64, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(emitted_lock_id, lock_id);
    assert_eq!(amount, 500);
}

/// Withdrawing one of multiple locks emits the correct lock_id and amount.
#[test]
fn event_schema_withdraw_lock_one_of_many() {
    let f = event_fixture();
    set_ledger_timestamp(&f.env, 1_000);

    f.client.deposit(&f.user, &2_000);
    let _lock1 = f.client.lock_funds(&f.user, &200, &2_000);
    let lock2 = f.client.lock_funds(&f.user, &300, &3_000);
    let _lock3 = f.client.lock_funds(&f.user, &400, &4_000);

    // Mature only lock2
    set_ledger_timestamp(&f.env, 3_500);
    f.client.withdraw_lock(&f.user, &lock2);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, Symbol::new(&f.env, "withdraw_lock"));

    let (emitted_lock_id, amount): (u64, i128) = data.try_into_val(&f.env).unwrap();
    assert_eq!(emitted_lock_id, lock2);
    assert_eq!(amount, 300);
}

// =========================================================================
// Transfer Admin Event
// =========================================================================

/// The transfer_admin event must emit:
/// - topics[0] = Symbol("transfer_admin")
/// - topics[1] = old admin address
/// - payload = new admin address
#[test]
fn event_schema_transfer_admin() {
    let f = event_fixture();
    let new_admin = Address::generate(&f.env);

    f.client.transfer_admin(&f.admin, &new_admin);

    let events = f.env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();

    // Topic[0]: Symbol("transfer_admin")
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic0, Symbol::new(&f.env, "transfer_admin"));

    // Topic[1]: old admin address
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.admin);

    // Payload: new admin address
    let emitted_new_admin: Address = data.try_into_val(&f.env).unwrap();
    assert_eq!(emitted_new_admin, new_admin);
}

/// After admin transfer, the new admin is stored and can be queried.
#[test]
fn event_schema_transfer_admin_persists() {
    let f = event_fixture();
    let new_admin = Address::generate(&f.env);

    f.client.transfer_admin(&f.admin, &new_admin);

    // Verify the admin actually changed
    assert_eq!(f.client.get_admin(), new_admin);

    // Verify the event was emitted with correct old_admin
    let events = f.env.events().all();
    let (_contract, topics, _data) = events.get(events.len() - 1).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&f.env).unwrap();
    assert_eq!(topic1, f.admin); // old admin in event
}

// =========================================================================
// Cross-cutting: Topic type assertions
// =========================================================================

/// Verify that topic[0] is always a Symbol and topic[1] is always an Address
/// for every event type. This guards against accidental schema changes.
#[test]
fn event_schema_all_topics_are_correct_types() {
    let f = event_fixture();
    let new_admin = Address::generate(&f.env);
    set_ledger_timestamp(&f.env, 1_000);

    // initialize already emitted by setup()
    f.client.deposit(&f.user, &1_000);
    f.client.lock_funds(&f.user, &200, &5_000);
    set_ledger_timestamp(&f.env, 6_000);
    f.client.withdraw(&f.user, &100);
    f.client.withdraw_lock(&f.user, &1);
    f.client.transfer_admin(&f.admin, &new_admin);

    let events = f.env.events().all();
    for i in 0..events.len() {
        let (_contract, topics, _data) = events.get(i).unwrap();

        // topic[0] must be a Symbol
        let topic0_result: Result<Symbol, _> = topics.get(0).unwrap().try_into_val(&f.env);
        assert!(
            topic0_result.is_ok(),
            "Event {i}: topic[0] must be a Symbol"
        );

        // topic[1] must be an Address
        let topic1_result: Result<Address, _> = topics.get(1).unwrap().try_into_val(&f.env);
        assert!(
            topic1_result.is_ok(),
            "Event {i}: topic[1] must be an Address"
        );
    }
}
