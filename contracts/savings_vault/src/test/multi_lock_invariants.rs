//! Multi-lock accounting invariant tests (issue #243).
//!
//! These tests verify that lock totals remain consistent through complex
//! operation sequences involving multiple locks, multiple users, partial
//! maturities, interleaved deposits/withdrawals, and failed operations.
//!
//! Invariants verified after every step:
//! 1. **Conservation**: `get_balance + get_locked_balance == net_deposited`
//! 2. **Non-negativity**: available >= 0, locked >= 0
//! 3. **Cross-user isolation**: ops on user A never affect user B
//! 4. **Failed ops don't mutate**: invalid ops leave balances unchanged
//! 5. **Lock sum consistency**: sum of individual unmatured amounts == locked
//! 6. **Lock ID uniqueness**: IDs unique + monotonically increasing per user

extern crate std;

use alloc::vec;
use alloc::vec::Vec as StdVec;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

use super::test_helpers::*;

type SvClient<'a> = crate::SavingsVaultClient<'a>;

// ─────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────

fn snapshot(client: &SvClient, user: &Address) -> (i128, i128) {
    (client.get_balance(user), client.get_locked_balance(user))
}

fn assert_conserved(client: &SvClient, user: &Address, expected_total: i128) {
    let available = client.get_balance(user);
    let locked = client.get_locked_balance(user);
    assert!(available >= 0, "available balance negative: {available}");
    assert!(locked >= 0, "locked balance negative: {locked}");
    assert_eq!(
        available + locked,
        expected_total,
        "conservation: {available} + {locked} != {expected_total}"
    );
}

fn assert_lock_sum_consistency(
    env: &Env,
    client: &SvClient,
    user: &Address,
) {
    let locked = client.get_locked_balance(user);
    let locks = client.list_locks(user, &0u32, &50u32);
    let now = env.ledger().timestamp();
    let mut sum: i128 = 0;
    for i in 0..locks.len() {
        let lock = locks.get(i).unwrap();
        if now < lock.unlock_time {
            sum += lock.amount;
        }
    }
    assert_eq!(
        sum, locked,
        "lock sum: unmatured entries sum {sum} != get_locked_balance {locked}"
    );
}

fn assert_lock_ids_unique(client: &SvClient, user: &Address) {
    let locks = client.list_locks(user, &0u32, &50u32);
    let mut seen: StdVec<u64> = alloc::vec![];
    for i in 0..locks.len() {
        let id = locks.get(i).unwrap().id;
        assert!(
            !seen.contains(&id),
            "duplicate lock ID {id}"
        );
        seen.push(id);
    }
}

// ─────────────────────────────────────────────────────────────
// Test 1: Staggered maturity — conservation holds throughout
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_staggered_maturity_conservation() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    let mut expected: i128 = 0;

    // Deposit 1000
    client.deposit(&user, &1_000);
    expected += 1_000;
    assert_conserved(&client, &user, expected);

    // Create 5 locks
    let amounts: [i128; 5] = [100, 200, 150, 300, 50];
    let unlocks: [u64; 5] = [2_000, 3_000, 4_000, 5_000, 6_000];

    for i in 0..5 {
        client.lock_funds(&user, &amounts[i], &unlocks[i]);
        assert_conserved(&client, &user, expected);
        assert_lock_sum_consistency(&env, &client, &user);
        assert_lock_ids_unique(&client, &user);
    }

    assert_eq!(client.get_balance(&user), 200);
    assert_eq!(client.get_locked_balance(&user), 800);

    // Mature first 2 locks
    set_ledger_timestamp(&env, 3_000);
    assert_conserved(&client, &user, expected);
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // Withdraw 400
    client.withdraw(&user, &400);
    expected -= 400;
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);

    // Mature all
    set_ledger_timestamp(&env, 6_000);
    assert_conserved(&client, &user, expected);

    // Withdraw remaining
    client.withdraw(&user, &expected);
    assert_conserved(&client, &user, 0);
}

// ─────────────────────────────────────────────────────────────
// Test 2: Cross-user isolation — 3 users, independent locks
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_cross_user_isolation() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    set_ledger_timestamp(&env, 1_000);

    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let user_c = Address::generate(&env);
    token_admin.mint(&user_a, &10_000);
    token_admin.mint(&user_b, &10_000);
    token_admin.mint(&user_c, &10_000);

    let mut ta: i128 = 0;
    let mut tb: i128 = 0;
    let mut tc: i128 = 0;

    // Each deposits + locks independently
    client.deposit(&user_a, &500); ta += 500;
    client.deposit(&user_b, &700); tb += 700;
    client.deposit(&user_c, &300); tc += 300;

    client.lock_funds(&user_a, &100, &3_000);
    client.lock_funds(&user_a, &150, &6_000);
    client.lock_funds(&user_b, &200, &4_000);
    client.lock_funds(&user_c, &100, &5_000);

    assert_conserved(&client, &user_a, ta);
    assert_conserved(&client, &user_b, tb);
    assert_conserved(&client, &user_c, tc);

    // User A withdraws available (500 - 250 = 250 available)
    client.withdraw(&user_a, &250);
    ta -= 250;
    assert_conserved(&client, &user_a, ta);

    // B and C unaffected
    assert_conserved(&client, &user_b, tb);
    assert_conserved(&client, &user_c, tc);

    // Mature A's first lock
    set_ledger_timestamp(&env, 3_000);
    client.withdraw(&user_a, &100);
    ta -= 100;
    assert_conserved(&client, &user_a, ta);
    assert_conserved(&client, &user_b, tb);
    assert_conserved(&client, &user_c, tc);

    // Mature all
    set_ledger_timestamp(&env, 6_000);
    client.withdraw(&user_a, &ta);
    client.withdraw(&user_b, &tb);
    client.withdraw(&user_c, &tc);
    assert_conserved(&client, &user_a, 0);
    assert_conserved(&client, &user_b, 0);
    assert_conserved(&client, &user_c, 0);
}

// ─────────────────────────────────────────────────────────────
// Test 3: Failed operations don't mutate — multi-lock context
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_failed_operations_do_not_mutate() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &1_000);
    let expected: i128 = 1_000;
    assert_conserved(&client, &user, expected);

    // Create 3 locks: available=400, locked=600
    client.lock_funds(&user, &200, &3_000);
    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &100, &7_000);
    assert_conserved(&client, &user, expected);

    let before = snapshot(&client, &user);

    // Lock more than available
    let res = client.try_lock_funds(&user, &401, &10_000);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // Lock zero
    let res = client.try_lock_funds(&user, &0, &10_000);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // Lock with past unlock
    let res = client.try_lock_funds(&user, &50, &500);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // Withdraw more than available
    let res = client.try_withdraw(&user, &401);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // Withdraw zero
    let res = client.try_withdraw(&user, &0);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // Deposit zero / negative
    let res = client.try_deposit(&user, &0);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    let res = client.try_deposit(&user, &-10);
    assert!(res.is_err());
    assert_eq!(snapshot(&client, &user), before);

    // State still intact
    assert_lock_sum_consistency(&env, &client, &user);
    assert_lock_ids_unique(&client, &user);
    assert_eq!(client.get_locked_balance(&user), 600);
}

// ─────────────────────────────────────────────────────────────
// Test 4: Lock ID uniqueness + monotonic
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_ids_unique_and_monotonic() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    client.deposit(&user, &10_000);

    let mut prev: u64 = 0;
    for i in 1u64..=10 {
        let id = client.lock_funds(&user, &100, &(2_000 + i * 1_000));
        assert_eq!(id, i, "lock ID should be sequential {i}, got {id}");
        assert!(id > prev, "lock ID must increase: {id} <= {prev}");
        prev = id;
    }

    assert_lock_ids_unique(&client, &user);
    let locks = client.list_locks(&user, &0u32, &50u32);
    assert_eq!(locks.len(), 10);
    assert_eq!(client.get_locked_balance(&user), 1_000);
    assert_eq!(client.get_balance(&user), 9_000);
}

// ─────────────────────────────────────────────────────────────
// Test 5: Partial maturity — withdraw matured, unmatured stay
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_partial_maturity_withdraw_keeps_unmatured_locked() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    let mut expected: i128 = 0;
    client.deposit(&user, &1_000);
    expected += 1_000;

    client.lock_funds(&user, &300, &3_000);
    client.lock_funds(&user, &200, &6_000);
    client.lock_funds(&user, &100, &9_000);
    // available=400, locked=600

    // Mature first lock only
    set_ledger_timestamp(&env, 3_000);
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Withdraw 500 (400 available + 100 from matured lock)
    client.withdraw(&user, &500);
    expected -= 500;
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Mature all remaining
    set_ledger_timestamp(&env, 9_000);
    assert_conserved(&client, &user, expected);
    assert_eq!(client.get_locked_balance(&user), 0);

    client.withdraw(&user, &expected);
    assert_conserved(&client, &user, 0);
}

// ─────────────────────────────────────────────────────────────
// Test 6: Interleaved deposit-lock-withdraw — stress
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_interleaved_deposit_lock_withdraw() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    let mut expected: i128 = 0;

    client.deposit(&user, &500); expected += 500;
    client.lock_funds(&user, &200, &3_000);
    client.deposit(&user, &300); expected += 300;
    client.lock_funds(&user, &150, &5_000);

    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);

    client.withdraw(&user, &200); expected -= 200;
    assert_conserved(&client, &user, expected);

    client.lock_funds(&user, &100, &7_000);
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);

    // Mature first lock
    set_ledger_timestamp(&env, 3_000);
    assert_conserved(&client, &user, expected);
    assert_eq!(client.get_balance(&user), 350);
    assert_eq!(client.get_locked_balance(&user), 250);

    client.withdraw(&user, &350); expected -= 350;
    assert_conserved(&client, &user, expected);

    set_ledger_timestamp(&env, 7_000);
    assert_conserved(&client, &user, expected);
    assert_eq!(client.get_locked_balance(&user), 0);
}

// ─────────────────────────────────────────────────────────────
// Test 7: withdraw_lock on specific lock
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_withdraw_specific_lock_preserves_accounting() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _tc, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &10_000);
    set_ledger_timestamp(&env, 1_000);

    let mut expected: i128 = 0;
    client.deposit(&user, &1_000);
    expected += 1_000;

    let id1 = client.lock_funds(&user, &200, &3_000);
    let id2 = client.lock_funds(&user, &300, &5_000);
    let id3 = client.lock_funds(&user, &100, &7_000);

    // Mature lock1 and lock2
    set_ledger_timestamp(&env, 5_000);

    assert_eq!(client.get_locked_balance(&user), 100);
    assert_lock_sum_consistency(&env, &client, &user);
    assert_conserved(&client, &user, expected);

    // withdraw_lock on specific locks
    client.withdraw_lock(&user, &id1); expected -= 200;
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);

    client.withdraw_lock(&user, &id2); expected -= 300;
    assert_conserved(&client, &user, expected);
    assert_lock_sum_consistency(&env, &client, &user);

    assert_eq!(client.get_locked_balance(&user), 100);

    // Can't withdraw unmatured lock
    let res = client.try_withdraw_lock(&user, &id3);
    assert!(res.is_err());

    // Mature lock3
    set_ledger_timestamp(&env, 7_000);
    client.withdraw_lock(&user, &id3); expected -= 100;
    assert_conserved(&client, &user, expected);
    assert_eq!(client.get_locked_balance(&user), 0);
}

// ─────────────────────────────────────────────────────────────
// Test 8: Table-driven multi-lock scenarios
// ─────────────────────────────────────────────────────────────

#[test]
fn multi_lock_scenario_matrix() {
    // Each case: (label, deposit_amount, lock_amounts_with_unlocks, time_jumps, withdraw_seq, expected_final_locked)

    struct Case {
        label: &'static str,
        deposit: i128,
        lock_specs: &'static [(i128, u64)],
        jump_to: u64,
        withdraws: &'static [i128],
        final_locked: i128,
    }

    let cases = &[
        Case {
            label: "single lock, single withdraw after maturity",
            deposit: 500,
            lock_specs: &[(200, 3_000)],
            jump_to: 3_000,
            withdraws: &[200],
            final_locked: 0,
        },
        Case {
            label: "two locks, withdraw partial after first maturity",
            deposit: 1_000,
            lock_specs: &[(300, 3_000), (400, 5_000)],
            jump_to: 3_000,
            withdraws: &[300, 200],
            final_locked: 400,
        },
        Case {
            label: "three locks, all mature, full withdraw",
            deposit: 2_000,
            lock_specs: &[(500, 3_000), (500, 4_000), (500, 5_000)],
            jump_to: 5_000,
            withdraws: &[500, 500, 500],
            final_locked: 0,
        },
        Case {
            label: "deposit > lock, withdraw available, locks stay",
            deposit: 1_000,
            lock_specs: &[(200, 5_000), (300, 6_000)],
            jump_to: 2_000, // before maturity
            withdraws: &[300],
            final_locked: 500,
        },
        Case {
            label: "all locks matured, partial withdrawal",
            deposit: 1_000,
            lock_specs: &[(600, 3_000)],
            jump_to: 4_000,
            withdraws: &[800],
            final_locked: 0,
        },
    ];

    for case in cases {
        let (env, contract_id, client) = setup();
        let (env, _admin, client, _tc, token_admin) =
            test_token(env, contract_id, client);
        let user = Address::generate(&env);
        token_admin.mint(&user, &10_000);
        set_ledger_timestamp(&env, 1_000);

        let mut total: i128 = 0;
        client.deposit(&user, &case.deposit);
        total += case.deposit;

        for &(amount, unlock) in case.lock_specs {
            client.lock_funds(&user, &amount, &unlock);
        }

        set_ledger_timestamp(&env, case.jump_to);

        for &amount in case.withdraws {
            client.withdraw(&user, &amount);
            total -= amount;
        }

        assert_conserved(&client, &user, total);
        assert_eq!(
            client.get_locked_balance(&user),
            case.final_locked,
            "case '{}': expected locked={}",
            case.label,
            case.final_locked,
        );
        assert_lock_sum_consistency(&env, &client, &user);
        assert_lock_ids_unique(&client, &user);
    }
}
