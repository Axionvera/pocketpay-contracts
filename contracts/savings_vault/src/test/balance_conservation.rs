//! Property-style / table-driven balance conservation tests (issue #37).
//!
//! Invariant under test:
//!   `get_balance(user) + get_locked_balance(user) == net_deposited`
//! where `net_deposited = sum(successful deposits) - sum(successful withdrawals)`.
//!
//! Additional guarantees checked after every step:
//! - available balance is never negative
//! - locked balance is never negative
//! - failed (invalid) operations leave both balances unchanged

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// Shared harness: initialized vault + SAC token so successful withdrawals can
/// transfer funds out of the contract.
struct Fixture {
    env: Env,
    contract_id: Address,
    client: SavingsVaultClient<'static>,
    token_client: token::Client<'static>,
    user: Address,
}

fn new_fixture() -> Fixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);
    let user = Address::generate(&env);

    // Large mint so sequences never run out of external token supply.
    token_admin.mint(&user, &1_000_000_000);
    set_ledger_timestamp(&env, 1_000);

    Fixture {
        env,
        contract_id,
        client,
        token_client,
        user,
    }
}

// ---------------------------------------------------------------------------
// Multi-user fixture
// ---------------------------------------------------------------------------

/// Harness with multiple users, tracking expected totals per user for conservation checks.
struct MultiUserFixture {
    env: Env,
    client: SavingsVaultClient<'static>,
    token_admin: token::StellarAssetClient<'static>,
    users: Vec<Address>,
    expected_totals: Vec<i128>, // one per user
}

fn new_multi_user_fixture(user_count: usize) -> MultiUserFixture {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, client);

    let mut users = Vec::new(&env);
    let mut expected_totals = Vec::new(&env);

    for _ in 0..user_count {
        let user = Address::generate(&env);
        token_admin.mint(&user, &1_000_000_000);
        users.push_back(user);
        expected_totals.push_back(0);
    }

    set_ledger_timestamp(&env, 1_000);

    MultiUserFixture {
        env,
        client,
        token_admin,
        users,
        expected_totals,
    }
}

/// Wraps an Op to target a specific user by index.
#[derive(Clone, Copy, Debug)]
enum UserOp {
    Op(usize, Op), // (user index, operation)
    SetTime(u64),
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// Deterministic vault operations used by table-driven sequences.
#[derive(Clone, Copy, Debug)]
enum Op {
    /// Credit internal balance and back it with a real SAC transfer into the vault.
    Deposit(i128),
    /// Withdraw from available balance (deposited + matured locks).
    Withdraw(i128),
    /// Lock `amount` until `unlock_time` (must be strictly after ledger time).
    Lock { amount: i128, unlock_time: u64 },
    /// Advance the ledger timestamp (matures locks without changing totals).
    SetTime(u64),
}

/// Outcome expected for a single step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expect {
    Ok,
    Err,
}

// ---------------------------------------------------------------------------
// Invariant helpers
// ---------------------------------------------------------------------------

/// Assert non-negativity and total conservation against the shadow model total.
fn assert_conserved(client: &SavingsVaultClient, user: &Address, expected_total: i128) {
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
        expected_total,
        "conservation failed: available ({available}) + locked ({locked}) != expected total ({expected_total})"
    );
}

fn snapshot(client: &SavingsVaultClient, user: &Address) -> (i128, i128) {
    (client.get_balance(user), client.get_locked_balance(user))
}

/// Run one operation sequence, checking conservation after every step.
///
/// Returns the final expected total so callers can make extra assertions if needed.
fn run_sequence(ops: &[(Op, Expect)]) -> i128 {
    let f = new_fixture();
    let mut expected_total: i128 = 0;

    assert_conserved(&f.client, &f.user, expected_total);

    for (step, (op, expect)) in ops.iter().enumerate() {
        let before = snapshot(&f.client, &f.user);

        match (op, expect) {
            (Op::Deposit(amount), Expect::Ok) => {
                f.client.deposit(&f.user, amount);
                expected_total += amount;
            }
            (Op::Deposit(amount), Expect::Err) => {
                let res = f.client.try_deposit(&f.user, amount);
                assert!(
                    res.is_err(),
                    "step {step}: deposit({amount}) was expected to fail"
                );
                assert_eq!(
                    snapshot(&f.client, &f.user),
                    before,
                    "step {step}: failed deposit must not mutate balances"
                );
            }
            (Op::Withdraw(amount), Expect::Ok) => {
                f.client.withdraw(&f.user, amount);
                expected_total -= amount;
            }
            (Op::Withdraw(amount), Expect::Err) => {
                let res = f.client.try_withdraw(&f.user, amount);
                assert!(
                    res.is_err(),
                    "step {step}: withdraw({amount}) was expected to fail"
                );
                assert_eq!(
                    snapshot(&f.client, &f.user),
                    before,
                    "step {step}: failed withdraw must not mutate balances"
                );
            }
            (Op::Lock { amount, unlock_time }, Expect::Ok) => {
                f.client.lock_funds(&f.user, amount, unlock_time);
                // Lock moves available → locked; total is unchanged.
            }
            (Op::Lock { amount, unlock_time }, Expect::Err) => {
                let res = f.client.try_lock_funds(&f.user, amount, unlock_time);
                assert!(
                    res.is_err(),
                    "step {step}: lock({amount} until {unlock_time}) was expected to fail"
                );
                assert_eq!(
                    snapshot(&f.client, &f.user),
                    before,
                    "step {step}: failed lock must not mutate balances"
                );
            }
            (Op::SetTime(ts), Expect::Ok) => {
                set_ledger_timestamp(&f.env, *ts);
                // Maturity only reclassifies funds; total is unchanged.
            }
            (Op::SetTime(_), Expect::Err) => {
                panic!("step {step}: SetTime cannot fail");
            }
        }

        assert_conserved(&f.client, &f.user, expected_total);
    }

    expected_total
}

// =========================================================================
// Sequences — valid operation flows
// =========================================================================

/// Deposit → partial withdraw → deposit → full remaining withdraw.
#[test]
fn conservation_deposit_withdraw_cycle() {
    let total = run_sequence(&[
        (Op::Deposit(500), Expect::Ok),
        (Op::Withdraw(200), Expect::Ok),
        (Op::Deposit(100), Expect::Ok),
        (Op::Withdraw(400), Expect::Ok),
    ]);
    assert_eq!(total, 0);
}

/// Multiple deposits and partial withdrawals interleaved.
#[test]
fn conservation_multiple_deposits_and_partial_withdrawals() {
    let total = run_sequence(&[
        (Op::Deposit(100), Expect::Ok),
        (Op::Deposit(250), Expect::Ok),
        (Op::Withdraw(50), Expect::Ok),
        (Op::Deposit(50), Expect::Ok),
        (Op::Withdraw(150), Expect::Ok),
        (Op::Withdraw(100), Expect::Ok),
    ]);
    assert_eq!(total, 100);
}

/// Lock moves funds without changing total; unlock reclassifies without changing total.
#[test]
fn conservation_lock_and_time_advance() {
    let total = run_sequence(&[
        (Op::Deposit(1_000), Expect::Ok),
        (
            Op::Lock {
                amount: 400,
                unlock_time: 5_000,
            },
            Expect::Ok,
        ),
        (
            Op::Lock {
                amount: 200,
                unlock_time: 8_000,
            },
            Expect::Ok,
        ),
        // Before any unlock: available 400, locked 600, total 1000.
        (Op::SetTime(5_000), Expect::Ok),
        // First lock matures: available 800, locked 200, total still 1000.
        (Op::SetTime(8_000), Expect::Ok),
        // Both mature: available 1000, locked 0.
        (Op::Withdraw(1_000), Expect::Ok),
    ]);
    assert_eq!(total, 0);
}

/// Withdraw matured lock funds after unlocking; remaining active locks stay locked.
#[test]
fn conservation_withdraw_after_partial_lock_maturity() {
    let total = run_sequence(&[
        (Op::Deposit(800), Expect::Ok),
        (
            Op::Lock {
                amount: 300,
                unlock_time: 3_000,
            },
            Expect::Ok,
        ),
        (
            Op::Lock {
                amount: 200,
                unlock_time: 9_000,
            },
            Expect::Ok,
        ),
        // available=300, locked=500
        (Op::Withdraw(100), Expect::Ok),
        // available=200, locked=500, total=700
        (Op::SetTime(3_000), Expect::Ok),
        // matured 300 → available=500, locked=200, total=700
        (Op::Withdraw(400), Expect::Ok),
        // available=100, locked=200, total=300
        (Op::SetTime(9_000), Expect::Ok),
        // all free: available=300, locked=0
        (Op::Withdraw(300), Expect::Ok),
    ]);
    assert_eq!(total, 0);
}

/// Deposit, lock, deposit again, then withdraw only available (not locked) funds.
#[test]
fn conservation_deposit_while_funds_locked() {
    let total = run_sequence(&[
        (Op::Deposit(500), Expect::Ok),
        (
            Op::Lock {
                amount: 500,
                unlock_time: 10_000,
            },
            Expect::Ok,
        ),
        (Op::Deposit(250), Expect::Ok),
        (Op::Withdraw(250), Expect::Ok),
        // Locked portion still held; total = 500.
        (Op::SetTime(10_000), Expect::Ok),
        (Op::Withdraw(500), Expect::Ok),
    ]);
    assert_eq!(total, 0);
}

/// Longer mixed sequence: deposits, locks, time advances, withdrawals.
#[test]
fn conservation_long_mixed_sequence() {
    let total = run_sequence(&[
        (Op::Deposit(1_000), Expect::Ok),
        (
            Op::Lock {
                amount: 100,
                unlock_time: 2_000,
            },
            Expect::Ok,
        ),
        (
            Op::Lock {
                amount: 200,
                unlock_time: 4_000,
            },
            Expect::Ok,
        ),
        (
            Op::Lock {
                amount: 300,
                unlock_time: 6_000,
            },
            Expect::Ok,
        ),
        (Op::Withdraw(50), Expect::Ok),
        (Op::Deposit(150), Expect::Ok),
        (Op::SetTime(2_000), Expect::Ok),
        (Op::Withdraw(200), Expect::Ok), // spends matured 100 + some available
        (Op::SetTime(4_000), Expect::Ok),
        (Op::Withdraw(150), Expect::Ok),
        (Op::SetTime(6_000), Expect::Ok),
        (Op::Withdraw(400), Expect::Ok),
        (Op::Deposit(25), Expect::Ok),
        (Op::Withdraw(25), Expect::Ok),
    ]);
    // Start 1000 -50 +150 -200 -150 -400 +25 -25 = 350
    assert_eq!(total, 350);
}

// =========================================================================
// Sequences — invalid operations must not mutate balances
// =========================================================================

/// Invalid deposits (zero / negative) leave state untouched; valid ones still conserve.
#[test]
fn conservation_invalid_deposits_do_not_mutate() {
    let total = run_sequence(&[
        (Op::Deposit(100), Expect::Ok),
        (Op::Deposit(0), Expect::Err),
        (Op::Deposit(-10), Expect::Err),
        (Op::Deposit(50), Expect::Ok),
    ]);
    assert_eq!(total, 150);
}

/// Over-withdraw, zero, and negative withdraws do not change balances.
#[test]
fn conservation_invalid_withdrawals_do_not_mutate() {
    let total = run_sequence(&[
        (Op::Deposit(200), Expect::Ok),
        (Op::Withdraw(0), Expect::Err),
        (Op::Withdraw(-5), Expect::Err),
        (Op::Withdraw(201), Expect::Err),
        (Op::Withdraw(50), Expect::Ok),
        (Op::Withdraw(200), Expect::Err), // only 150 left
        (Op::Withdraw(150), Expect::Ok),
    ]);
    assert_eq!(total, 0);
}

/// Over-lock, zero lock, past unlock time do not change balances.
#[test]
fn conservation_invalid_locks_do_not_mutate() {
    let total = run_sequence(&[
        (Op::Deposit(300), Expect::Ok),
        (
            Op::Lock {
                amount: 0,
                unlock_time: 5_000,
            },
            Expect::Err,
        ),
        (
            Op::Lock {
                amount: -1,
                unlock_time: 5_000,
            },
            Expect::Err,
        ),
        (
            Op::Lock {
                amount: 301,
                unlock_time: 5_000,
            },
            Expect::Err,
        ),
        (
            Op::Lock {
                amount: 100,
                unlock_time: 500, // past relative to ledger t=1000
            },
            Expect::Err,
        ),
        (
            Op::Lock {
                amount: 100,
                unlock_time: 5_000,
            },
            Expect::Ok,
        ),
        // Cannot lock more than remaining deposited balance (200).
        (
            Op::Lock {
                amount: 201,
                unlock_time: 6_000,
            },
            Expect::Err,
        ),
        (Op::Withdraw(50), Expect::Ok),
    ]);
    // 300 - 50 = 250 (100 of which is locked)
    assert_eq!(total, 250);
}

/// Withdraw that would touch only locked (unmatured) funds must fail and not mutate.
#[test]
fn conservation_withdraw_exceeds_available_while_locked_does_not_mutate() {
    let total = run_sequence(&[
        (Op::Deposit(500), Expect::Ok),
        (
            Op::Lock {
                amount: 400,
                unlock_time: 20_000,
            },
            Expect::Ok,
        ),
        // Only 100 available.
        (Op::Withdraw(101), Expect::Err),
        (Op::Withdraw(100), Expect::Ok),
        (Op::Withdraw(1), Expect::Err),
    ]);
    assert_eq!(total, 400);
}

/// Interleave valid and invalid ops across a longer sequence.
#[test]
fn conservation_mixed_valid_and_invalid_sequence() {
    let total = run_sequence(&[
        (Op::Deposit(1_000), Expect::Ok),
        (Op::Withdraw(0), Expect::Err),
        (
            Op::Lock {
                amount: 300,
                unlock_time: 5_000,
            },
            Expect::Ok,
        ),
        (Op::Withdraw(800), Expect::Err), // only 700 available
        (Op::Withdraw(700), Expect::Ok),
        (Op::Deposit(-1), Expect::Err),
        (Op::Deposit(200), Expect::Ok),
        (
            Op::Lock {
                amount: 500,
                unlock_time: 6_000,
            },
            Expect::Err, // only 200 free deposited
        ),
        (
            Op::Lock {
                amount: 100,
                unlock_time: 6_000,
            },
            Expect::Ok,
        ),
        (Op::SetTime(5_000), Expect::Ok),
        // matured 300 + free 100 = 400 available; locked 100
        (Op::Withdraw(400), Expect::Ok),
        (Op::SetTime(6_000), Expect::Ok),
        (Op::Withdraw(100), Expect::Ok),
    ]);
    // 1000 - 700 + 200 - 400 - 100 = 0
    assert_eq!(total, 0);
}

// ---------------------------------------------------------------------------
// Multi-user invariant helpers
// ---------------------------------------------------------------------------

fn assert_all_conserved(f: &MultiUserFixture) {
    for (i, user) in f.users.iter().enumerate() {
        assert_conserved(&f.client, user, f.expected_totals[i]);
    }
}

fn snapshot_all(f: &MultiUserFixture) -> Vec<(i128, i128)> {
    f.users.iter().map(|u| snapshot(&f.client, u)).collect()
}

fn run_multi_user_sequence(ops: &[(UserOp, Expect)]) {
    let mut f = new_multi_user_fixture(2); // default 2 users for tests
    assert_all_conserved(&f);

    for (step, (user_op, expect)) in ops.iter().enumerate() {
        let before = snapshot_all(&f);

        match (user_op, expect) {
            (UserOp::Op(user_idx, op), Expect::Ok) => {
                let user = &f.users[*user_idx];
                match op {
                    Op::Deposit(amount) => {
                        f.client.deposit(user, amount);
                        f.expected_totals[*user_idx] += amount;
                    }
                    Op::Withdraw(amount) => {
                        f.client.withdraw(user, amount);
                        f.expected_totals[*user_idx] -= amount;
                    }
                    Op::Lock { amount, unlock_time } => {
                        f.client.lock_funds(user, amount, *unlock_time);
                    }
                    Op::SetTime(_) => {
                        // handled via UserOp::SetTime
                    }
                }
            }
            (UserOp::Op(user_idx, op), Expect::Err) => {
                let user = &f.users[*user_idx];
                match op {
                    Op::Deposit(amount) => {
                        assert!(f.client.try_deposit(user, amount).is_err());
                    }
                    Op::Withdraw(amount) => {
                        assert!(f.client.try_withdraw(user, amount).is_err());
                    }
                    Op::Lock { amount, unlock_time } => {
                        assert!(f.client.try_lock_funds(user, amount, *unlock_time).is_err());
                    }
                    Op::SetTime(_) => {
                        panic!("step {step}: SetTime via UserOp::Op is invalid");
                    }
                }
                assert_eq!(
                    snapshot_all(&f),
                    before,
                    "step {step}: failed operation on user {user_idx} must not mutate balances"
                );
            }
            (UserOp::SetTime(ts), Expect::Ok) => {
                set_ledger_timestamp(&f.env, *ts);
            }
            (UserOp::SetTime(_), Expect::Err) => {
                panic!("step {step}: SetTime cannot fail");
            }
        }

        assert_all_conserved(&f);
    }
}

// =========================================================================
// Multi-user invariant tests
// =========================================================================

/// Verify that operations on user 0 never affect user 1's balances.
#[test]
fn conservation_cross_user_isolation() {
    run_multi_user_sequence(&[
        (UserOp::Op(0, Op::Deposit(500)), Expect::Ok),
        (UserOp::Op(1, Op::Deposit(300)), Expect::Ok),
        (UserOp::Op(0, Op::Lock { amount: 200, unlock_time: 3000 }), Expect::Ok),
        (UserOp::Op(1, Op::Withdraw(100)), Expect::Ok),
        (UserOp::Op(0, Op::Deposit(100)), Expect::Ok),
        (UserOp::Op(1, Op::Lock { amount: 150, unlock_time: 5000 }), Expect::Ok),
        (UserOp::SetTime(3000), Expect::Ok),
        (UserOp::Op(0, Op::Withdraw(300)), Expect::Ok), // available (300) + matured (200) → withdraw 300
        (UserOp::SetTime(5000), Expect::Ok),
        (UserOp::Op(1, Op::Withdraw(350)), Expect::Ok), // 200 available + 150 matured → withdraw 350
    ]);
}

/// More complex cross-user sequence with valid and invalid operations.
#[test]
fn conservation_cross_user_mixed_valid_invalid() {
    run_multi_user_sequence(&[
        (UserOp::Op(0, Op::Deposit(1000)), Expect::Ok),
        (UserOp::Op(1, Op::Deposit(500)), Expect::Ok),
        // Invalid ops don't affect anyone
        (UserOp::Op(0, Op::Withdraw(1001)), Expect::Err),
        (UserOp::Op(1, Op::Lock { amount: 0, unlock_time: 2000 }), Expect::Err),
        // Mixed valid ops
        (UserOp::Op(0, Op::Lock { amount: 400, unlock_time: 4000 }), Expect::Ok),
        (UserOp::Op(1, Op::Lock { amount: 200, unlock_time: 6000 }), Expect::Ok),
        (UserOp::SetTime(4000), Expect::Ok),
        (UserOp::Op(0, Op::Withdraw(500)), Expect::Ok), // 600 available + 400 matured → withdraw 500
        (UserOp::Op(1, Op::Withdraw(100)), Expect::Ok),
        (UserOp::SetTime(6000), Expect::Ok),
        (UserOp::Op(1, Op::Withdraw(400)), Expect::Ok),
    ]);
}

// =========================================================================
// Table-driven multi-sequence runner
// =========================================================================

/// Single entry point that exercises several independent sequences so a
/// regression in conservation shows up under one test name as well.
#[test]
fn conservation_table_driven_sequences() {
    // Each row: (label, ops, final expected total)
    let cases: &[(&str, &[(Op, Expect)], i128)] = &[
        (
            "empty",
            &[],
            0,
        ),
        (
            "deposit only",
            &[(Op::Deposit(42), Expect::Ok)],
            42,
        ),
        (
            "deposit withdraw all",
            &[
                (Op::Deposit(99), Expect::Ok),
                (Op::Withdraw(99), Expect::Ok),
            ],
            0,
        ),
        (
            "lock unlock withdraw",
            &[
                (Op::Deposit(60), Expect::Ok),
                (
                    Op::Lock {
                        amount: 60,
                        unlock_time: 2_500,
                    },
                    Expect::Ok,
                ),
                (Op::SetTime(2_500), Expect::Ok),
                (Op::Withdraw(60), Expect::Ok),
            ],
            0,
        ),
        (
            "invalid then valid",
            &[
                (Op::Withdraw(1), Expect::Err),
                (Op::Deposit(10), Expect::Ok),
                (Op::Withdraw(11), Expect::Err),
                (Op::Withdraw(10), Expect::Ok),
            ],
            0,
        ),
        (
            "two locks staggered maturity",
            &[
                (Op::Deposit(500), Expect::Ok),
                (
                    Op::Lock {
                        amount: 100,
                        unlock_time: 3_000,
                    },
                    Expect::Ok,
                ),
                (
                    Op::Lock {
                        amount: 150,
                        unlock_time: 7_000,
                    },
                    Expect::Ok,
                ),
                (Op::SetTime(3_000), Expect::Ok),
                (Op::Withdraw(250), Expect::Ok), // free 250 + matured 100 = 350 avail; take 250
                (Op::SetTime(7_000), Expect::Ok),
                (Op::Withdraw(250), Expect::Ok),
            ],
            0,
        ),
        (
            "withdraw uses available first then matured",
            &[
                (Op::Deposit(300), Expect::Ok), // available:300
                (Op::Lock { amount:100, unlock_time: 3000 }, Expect::Ok), // available: 200, locked: 100
                (Op::SetTime(3000), Expect::Ok), // available now 200 + 100, locked 0
                (Op::Withdraw(250), Expect::Ok), // first uses 200 available, then 50 from matured lock
                (Op::Deposit(100), Expect::Ok),
            ],
            150, // 300 - 250 + 100 =150
        ),
        (
            "partial withdraw part of matured lock",
            &[
                (Op::Deposit(200), Expect::Ok),
                (Op::Lock { amount: 150, unlock_time: 4000 }, Expect::Ok), // available 50, locked 150
                (Op::SetTime(4000), Expect::Ok), // available 50+150, locked 0
                (Op::Withdraw(175), Expect::Ok), // uses all 50, then 125 of matured (150-125=25 left
            ],
            25,
        ),
    ];

    for (label, ops, expected) in cases {
        let total = run_sequence(ops);
        assert_eq!(
            total, *expected,
            "sequence '{label}' ended with total {total}, expected {expected}"
        );
    }
}

#[test]
fn conservation_multi_lock_many_locks() {
    run_sequence(&[
        (Op::Deposit(1000), Expect::Ok),
        (Op::Lock { amount: 100, unlock_time: 2000 }, Expect::Ok),
        (Op::Lock { amount: 100, unlock_time: 3000 }, Expect::Ok),
        (Op::Lock { amount: 100, unlock_time: 4000 }, Expect::Ok),
        (Op::Lock { amount: 100, unlock_time: 5000 }, Expect::Ok),
        (Op::Lock { amount: 100, unlock_time: 6000 }, Expect::Ok),
        (Op::SetTime(3500), Expect::Ok), // first 3 locks mature
        (Op::Withdraw(600), Expect::Ok), // 500 available + 300 mature = 800, take 600
        (Op::SetTime(6000), Expect::Ok), // all mature
        (Op::Withdraw(400), Expect::Ok),
    ]);
}

