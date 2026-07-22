//! Fee-related accounting invariant tests for the Savings Vault (issue #225).
//!
//! Invariants under test:
//!   - Deposits credit exactly the deposited amount (no fee deductions).
//!   - Withdrawals deduct exactly the withdrawn amount.
//!   - Lock/unlock operations do not change the total (available + locked).
//!   - Failed operations leave all balances unchanged.
//!   - Contract SAC token balance always equals the sum of user balances.

use super::test_helpers::*;
use super::*;
use alloc::vec::Vec as StdVec;
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Operation model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum FeeOp {
    Deposit(i128),
    Withdraw(i128),
    Lock { amount: i128, unlock_time: u64 },
}

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

fn op_sequence_strategy() -> impl Strategy<Value = StdVec<FeeOp>> {
    proptest::collection::vec(
        prop_oneof![
            (1i128..=1_000_000i128).prop_map(FeeOp::Deposit),
            (1i128..=500_000i128).prop_map(FeeOp::Withdraw),
            ((1i128..=500_000i128), (1_001u64..=50_000u64)).prop_map(|(amount, unlock_time)| {
                FeeOp::Lock {
                    amount,
                    unlock_time,
                }
            }),
        ],
        1..=20usize,
    )
}

// ---------------------------------------------------------------------------
// Proptest fuzz harnesses
// ---------------------------------------------------------------------------

proptest! {
    /// After every operation the sum `available + locked` must equal the net
    /// deposited amount.  This catches any accidental fee deduction, rounding
    /// error, or balance drift introduced by deposit / withdraw / lock.
    #[test]
    fn prop_fee_free_one_to_one_accounting(ops in op_sequence_strategy()) {
        let (env, contract_id, client) = setup();
        let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
        let user = Address::generate(&env);
        token_admin.mint(&user, &10_000_000_000i128);
        set_ledger_timestamp(&env, 1_000);

        let mut expected: i128 = 0;

        for op in &ops {
            match op {
                FeeOp::Deposit(amount) => {
                    let before = (client.get_balance(&user), client.get_locked_balance(&user));
                    if client.try_deposit(&user, amount).is_ok() {
                        expected += amount;
                    } else {
                        assert_eq!((client.get_balance(&user), client.get_locked_balance(&user)), before);
                    }
                }
                FeeOp::Withdraw(amount) => {
                    let before = (client.get_balance(&user), client.get_locked_balance(&user));
                    if client.try_withdraw(&user, amount).is_ok() {
                        expected -= amount;
                    } else {
                        assert_eq!((client.get_balance(&user), client.get_locked_balance(&user)), before);
                    }
                }
                FeeOp::Lock { amount, unlock_time } => {
                    let before = (client.get_balance(&user), client.get_locked_balance(&user));
                    if client.try_lock_funds(&user, amount, unlock_time).is_ok() {
                        // lock moves balance between available↔locked; total unchanged
                    } else {
                        assert_eq!((client.get_balance(&user), client.get_locked_balance(&user)), before);
                    }
                }
            }
            let available = client.get_balance(&user);
            let locked = client.get_locked_balance(&user);
            assert!(available >= 0, "available must be >= 0");
            assert!(locked >= 0, "locked must be >= 0");
            assert_eq!(available + locked, expected, "conservation failed");
        }
    }

    /// The contract's SAC token balance must always equal the sum of every
    /// user's (available + locked) balance.  This proves the contract never
    /// holds more or fewer tokens than it owes — no hidden fee sink.
    #[test]
    fn prop_no_fee_token_custody(ops in op_sequence_strategy()) {
        let (env, contract_id, client) = setup();
        let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id.clone(), client);
        let user = Address::generate(&env);
        token_admin.mint(&user, &10_000_000_000i128);
        set_ledger_timestamp(&env, 1_000);

        for op in &ops {
            match op {
                FeeOp::Deposit(amount) => { client.deposit(&user, amount); }
                FeeOp::Withdraw(amount) => {
                    if *amount > 0 && client.get_balance(&user) >= *amount {
                        client.withdraw(&user, amount);
                    }
                }
                FeeOp::Lock { amount, unlock_time } => {
                    if *amount > 0 && *amount <= client.get_balance(&user) {
                        client.lock_funds(&user, amount, unlock_time);
                    }
                }
            }
            let contract_balance = token_client.balance(&contract_id);
            let user_total = client.get_balance(&user) + client.get_locked_balance(&user);
            assert_eq!(contract_balance, user_total, "token custody mismatch");
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests — failed-operation accounting
// ---------------------------------------------------------------------------

/// A failed deposit must not change any balance.
#[test]
fn test_failed_deposit_has_zero_balance_change() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &50);
    let before_available = client.get_balance(&user);
    let before_locked = client.get_locked_balance(&user);
    assert!(client.try_deposit(&user, &100).is_err());
    assert_eq!(client.get_balance(&user), before_available);
    assert_eq!(client.get_locked_balance(&user), before_locked);
}

/// A failed withdrawal must not change any balance.
#[test]
fn test_failed_withdraw_has_zero_balance_change() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &200);
    client.deposit(&user, &100);
    let before_available = client.get_balance(&user);
    let before_locked = client.get_locked_balance(&user);
    assert!(client.try_withdraw(&user, &200).is_err());
    assert_eq!(client.get_balance(&user), before_available);
    assert_eq!(client.get_locked_balance(&user), before_locked);
}

/// A failed lock must not change any balance.
#[test]
fn test_failed_lock_has_zero_balance_change() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &200);
    set_ledger_timestamp(&env, 1_000);
    client.deposit(&user, &100);
    let before_available = client.get_balance(&user);
    let before_locked = client.get_locked_balance(&user);
    assert!(client.try_lock_funds(&user, &200, &2_000).is_err());
    assert_eq!(client.get_balance(&user), before_available);
    assert_eq!(client.get_locked_balance(&user), before_locked);
}

// ---------------------------------------------------------------------------
// Unit tests — exact-amount accounting
// ---------------------------------------------------------------------------

/// Each deposit must credit exactly the deposited amount.
#[test]
fn test_deposit_credits_exact_amount() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000_000);
    for amt in &[1i128, 10, 100, 1_000, 10_000, 100_000, 500_000] {
        let before = client.get_balance(&user);
        client.deposit(&user, amt);
        assert_eq!(client.get_balance(&user), before + amt);
    }
}

/// Each withdrawal must deduct exactly the withdrawn amount.
#[test]
fn test_withdraw_deducts_exact_amount() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &2_000_000);
    client.deposit(&user, &1_000_000);
    for amt in &[1i128, 100, 50_000, 500_000, 449_899] {
        let before = client.get_balance(&user);
        client.withdraw(&user, amt);
        assert_eq!(client.get_balance(&user), before - amt);
    }
}
