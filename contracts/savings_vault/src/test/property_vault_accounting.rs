//! Property-based / fuzz tests for vault accounting invariants (issue #197).
//!
//! Invariant under test:
//!   `get_balance(user) + get_locked_balance(user) == net_deposited`
//! where `net_deposited = sum(successful deposits) - sum(successful withdrawals)`.
//!
//! Additional guarantees checked after every step:
//! - available balance is never negative
//! - locked balance is never negative
//! - failed (invalid) operations leave both balances unchanged
//! - user isolation: operations on one user never affect another's balances
//! - global token custody: contract SAC balance == sum of all user balances
//!
//! Uses proptest to generate randomized operation sequences with boundary
//! values and near-MAX amounts to catch overflow and accounting mismatches.

use super::test_helpers::*;
use super::*;
use alloc::vec::Vec as StdVec;
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Operation model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    Deposit(i128),
    Withdraw(i128),
    Lock { amount: i128, unlock_time: u64 },
    SetTime(u64),
}

// ---------------------------------------------------------------------------
// Strategy helpers
// ---------------------------------------------------------------------------

fn deposit_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        Just(1i128),
        Just(0i128),
        Just(-1i128),
        Just(i128::MAX / 4),
        (1i128..=1_000_000i128),
    ]
}

fn withdraw_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        Just(0i128),
        Just(-1i128),
        Just(i128::MAX / 4),
        (1i128..=1_000_000i128),
    ]
}

fn lock_amount_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        Just(0i128),
        Just(-1i128),
        Just(i128::MAX / 4),
        (1i128..=1_000_000i128),
    ]
}

fn unlock_time_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0u64),
        Just(1_000u64),
        Just(1_001u64),
        Just(5_000u64),
        Just(10_000u64),
        (1_001u64..=10_000u64),
    ]
}

fn time_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(1_000u64),
        Just(5_000u64),
        Just(10_000u64),
        (1_000u64..=10_000u64),
    ]
}

fn op_sequence_strategy() -> impl Strategy<Value = StdVec<Op>> {
    proptest::collection::vec(
        prop_oneof![
            deposit_strategy().prop_map(Op::Deposit),
            withdraw_strategy().prop_map(Op::Withdraw),
            (lock_amount_strategy(), unlock_time_strategy()).prop_map(|(amount, unlock_time)| {
                Op::Lock {
                    amount,
                    unlock_time,
                }
            }),
            time_strategy().prop_map(Op::SetTime),
        ],
        1..=15usize,
    )
}

// ---------------------------------------------------------------------------
// Single-user fuzz runner
// ---------------------------------------------------------------------------

struct FuzzFixture {
    env: Env,
    client: SavingsVaultClient<'static>,
    user: Address,
    expected_total: i128,
}

fn new_fuzz_fixture() -> FuzzFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000_000_000);
    set_ledger_timestamp(&env, 1_000);
    FuzzFixture {
        env,
        client,
        user,
        expected_total: 0,
    }
}

fn assert_conserved(client: &SavingsVaultClient, user: &Address, expected: i128) {
    let available = client.get_balance(user);
    let locked = client.get_locked_balance(user);
    assert!(
        available >= 0,
        "available balance must never be negative (got {available})"
    );
    assert!(
        locked >= 0,
        "locked balance must never be negative (got {locked})"
    );
    assert_eq!(
        available + locked,
        expected,
        "conservation failed: available ({available}) + locked ({locked}) != expected ({expected})"
    );
}

fn run_op(f: &mut FuzzFixture, op: &Op) {
    let before = (
        f.client.get_balance(&f.user),
        f.client.get_locked_balance(&f.user),
    );
    match op {
        Op::Deposit(amount) => {
            if *amount <= 0 {
                let res = f.client.try_deposit(&f.user, amount);
                assert!(res.is_err(), "deposit({amount}) should fail");
                assert_eq!(
                    (
                        f.client.get_balance(&f.user),
                        f.client.get_locked_balance(&f.user)
                    ),
                    before,
                    "failed deposit must not mutate balances"
                );
            } else {
                f.client.deposit(&f.user, amount);
                f.expected_total += amount;
            }
        }
        Op::Withdraw(amount) => {
            let available = f.client.get_balance(&f.user);
            if *amount <= 0 || *amount > available {
                let res = f.client.try_withdraw(&f.user, amount);
                assert!(res.is_err(), "withdraw({amount}) should fail");
                assert_eq!(
                    (
                        f.client.get_balance(&f.user),
                        f.client.get_locked_balance(&f.user)
                    ),
                    before,
                    "failed withdraw must not mutate balances"
                );
            } else {
                f.client.withdraw(&f.user, amount);
                f.expected_total -= amount;
            }
        }
        Op::Lock {
            amount,
            unlock_time,
        } => {
            let current_time = f.env.ledger().timestamp();
            let available = f.client.get_balance(&f.user);
            if *amount <= 0 || *amount > available || *unlock_time <= current_time {
                let res = f.client.try_lock_funds(&f.user, amount, unlock_time);
                assert!(
                    res.is_err(),
                    "lock({amount} until {unlock_time}) should fail"
                );
                assert_eq!(
                    (
                        f.client.get_balance(&f.user),
                        f.client.get_locked_balance(&f.user)
                    ),
                    before,
                    "failed lock must not mutate balances"
                );
            } else {
                f.client.lock_funds(&f.user, amount, unlock_time);
            }
        }
        Op::SetTime(ts) => {
            set_ledger_timestamp(&f.env, *ts);
        }
    }
    assert_conserved(&f.client, &f.user, f.expected_total);
}

// ---------------------------------------------------------------------------
// Multi-user fuzz runner
// ---------------------------------------------------------------------------

struct MultiFuzzFixture {
    env: Env,
    client: SavingsVaultClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    users: StdVec<Address>,
    expected_totals: StdVec<i128>,
}

fn new_multi_fuzz_fixture(count: usize) -> MultiFuzzFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let mut users = StdVec::new();
    let mut expected_totals = StdVec::new();
    for _ in 0..count {
        let user = Address::generate(&env);
        token_admin.mint(&user, &1_000_000_000);
        users.push(user);
        expected_totals.push(0);
    }
    set_ledger_timestamp(&env, 1_000);
    MultiFuzzFixture {
        env,
        client,
        token_admin,
        users,
        expected_totals,
    }
}

fn assert_all_conserved(f: &MultiFuzzFixture) {
    for (i, user) in f.users.iter().enumerate() {
        let available = f.client.get_balance(user);
        let locked = f.client.get_locked_balance(user);
        assert!(available >= 0, "user {i}: available negative ({available})");
        assert!(locked >= 0, "user {i}: locked negative ({locked})");
        assert_eq!(
            available + locked,
            f.expected_totals[i],
            "user {i}: conservation failed"
        );
    }
}

fn snapshot_all(f: &MultiFuzzFixture) -> StdVec<(i128, i128)> {
    f.users
        .iter()
        .map(|u| (f.client.get_balance(u), f.client.get_locked_balance(u)))
        .collect()
}

#[derive(Clone, Debug)]
struct UserOp(usize, Op);

fn single_op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        deposit_strategy().prop_map(Op::Deposit),
        withdraw_strategy().prop_map(Op::Withdraw),
        (lock_amount_strategy(), unlock_time_strategy()).prop_map(|(amount, unlock_time)| {
            Op::Lock {
                amount,
                unlock_time,
            }
        }),
        time_strategy().prop_map(Op::SetTime),
    ]
}

fn multi_op_strategy(user_count: usize) -> impl Strategy<Value = StdVec<UserOp>> {
    proptest::collection::vec(
        (0..user_count as usize, single_op_strategy())
            .prop_map(|(idx, op)| UserOp(idx, op)),
        1..=20usize,
    )
}

// ---------------------------------------------------------------------------
// Proptest entry points
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_single_user_conservation(ops in op_sequence_strategy()) {
        let mut f = new_fuzz_fixture();
        assert_conserved(&f.client, &f.user, f.expected_total);
        for op in &ops {
            run_op(&mut f, op);
        }
    }

    #[test]
    fn prop_near_max_deposit_no_overflow(ops in op_sequence_strategy()) {
        let mut f = new_fuzz_fixture();
        f.client.deposit(&f.user, &(i128::MAX / 4));
        f.expected_total += i128::MAX / 4;
        assert_conserved(&f.client, &f.user, f.expected_total);
        for op in &ops {
            run_op(&mut f, op);
        }
    }

    #[test]
    fn prop_cross_user_isolation(ops in multi_op_strategy(2)) {
        let mut f = new_multi_fuzz_fixture(2);
        assert_all_conserved(&f);
        for UserOp(idx, op) in &ops {
            let before = snapshot_all(&f);
            let user = &f.users[*idx];
            match op {
                Op::Deposit(amount) => {
                    if *amount > 0 {
                        f.client.deposit(user, amount);
                        f.expected_totals[*idx] += amount;
                    } else {
                        assert!(f.client.try_deposit(user, amount).is_err());
                        assert_eq!(snapshot_all(&f), before, "failed op mutated state");
                    }
                }
                Op::Withdraw(amount) => {
                    let avail = f.client.get_balance(user);
                    if *amount > 0 && *amount <= avail {
                        f.client.withdraw(user, amount);
                        f.expected_totals[*idx] -= amount;
                    } else {
                        assert!(f.client.try_withdraw(user, amount).is_err());
                        assert_eq!(snapshot_all(&f), before, "failed op mutated state");
                    }
                }
                Op::Lock { amount, unlock_time } => {
                    let ct = f.env.ledger().timestamp();
                    let avail = f.client.get_balance(user);
                    if *amount > 0 && *amount <= avail && *unlock_time > ct {
                        f.client.lock_funds(user, amount, unlock_time);
                    } else {
                        assert!(f.client.try_lock_funds(user, amount, unlock_time).is_err());
                        assert_eq!(snapshot_all(&f), before, "failed op mutated state");
                    }
                }
                Op::SetTime(ts) => set_ledger_timestamp(&f.env, *ts),
            }
            assert_all_conserved(&f);
        }
    }

    #[test]
    fn prop_global_token_custody(ops in multi_op_strategy(3)) {
        let mut f = new_multi_fuzz_fixture(3);
        let token_addr: Address = f.env.as_contract(&f.env.register(SavingsVault, ()), || {
            f.env.storage().instance().get(&DataKey::Token).unwrap()
        });
        let token_client = token::Client::new(&f.env, &token_addr);

        let check_custody = |f: &MultiFuzzFixture| {
            let contract_addr = f.env.register(SavingsVault, ());
            let contract_bal = token_client.balance(&contract_addr);
            let mut sum: i128 = 0;
            for user in &f.users {
                sum += f.client.get_balance(user) + f.client.get_locked_balance(user);
            }
            assert_eq!(contract_bal, sum, "global custody mismatch");
        };

        check_custody(&f);
        for UserOp(idx, op) in &ops {
            let user = &f.users[*idx];
            match op {
                Op::Deposit(amount) => {
                    if *amount > 0 {
                        f.client.deposit(user, amount);
                        f.expected_totals[*idx] += amount;
                    }
                }
                Op::Withdraw(amount) => {
                    let avail = f.client.get_balance(user);
                    if *amount > 0 && *amount <= avail {
                        f.client.withdraw(user, amount);
                        f.expected_totals[*idx] -= amount;
                    }
                }
                Op::Lock { amount, unlock_time } => {
                    let ct = f.env.ledger().timestamp();
                    let avail = f.client.get_balance(user);
                    if *amount > 0 && *amount <= avail && *unlock_time > ct {
                        f.client.lock_funds(user, amount, unlock_time);
                    }
                }
                Op::SetTime(ts) => set_ledger_timestamp(&f.env, *ts),
            }
            assert_all_conserved(&f);
            check_custody(&f);
        }
    }
}
