//! Total Vault Balance Invariant Tests
//!
//! Invariants under test:
//! 1. Global Vault Balance Invariant:
//!    `token_client.balance(&contract_id) == sum(get_balance(u) + get_locked_balance(u))` for all users `u`.
//! 2. Non-negativity:
//!    `get_balance(u) >= 0` and `get_locked_balance(u) >= 0` for all users `u`.
//! 3. Failure Non-Mutation:
//!    Failed/reverted operations do not alter global vault balance or any individual user balance.

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

// ---------------------------------------------------------------------------
// Harness & Diagnostic Helper
// ---------------------------------------------------------------------------

struct MultiUserHarness {
    env: Env,
    contract_id: Address,
    client: SavingsVaultClient<'static>,
    token_client: token::Client<'static>,
    token_admin: token::StellarAssetClient<'static>,
    users: Vec<Address>,
}

fn create_harness(user_count: usize) -> MultiUserHarness {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, client);

    let mut users = Vec::new(&env);
    for _ in 0..user_count {
        let user = Address::generate(&env);
        token_admin.mint(&user, &10_000_000);
        users.push_back(user);
    }

    set_ledger_timestamp(&env, 1_000);

    MultiUserHarness {
        env,
        contract_id,
        client,
        token_client,
        token_admin,
        users,
    }
}

/// Verifies total balance invariants across all users in the harness.
///
/// Diagnostic output details per-user balance breakdowns if an invariant check fails.
fn verify_vault_total_invariant(harness: &MultiUserHarness, step_context: &str) {
    let contract_balance = harness.token_client.balance(&harness.contract_id);
    let mut sum_available: i128 = 0;
    let mut sum_locked: i128 = 0;

    for (idx, user) in harness.users.iter().enumerate() {
        let avail = harness.client.get_balance(&user);
        let locked = harness.client.get_locked_balance(&user);

        assert!(
            avail >= 0,
            "[{step_context}] User {idx} available balance is negative: {avail}"
        );
        assert!(
            locked >= 0,
            "[{step_context}] User {idx} locked balance is negative: {locked}"
        );

        sum_available += avail;
        sum_locked += locked;
    }

    let total_user_liabilities = sum_available + sum_locked;

    assert_eq!(
        contract_balance, total_user_liabilities,
        "[{step_context}] Vault balance invariant mismatch! Contract SAC token balance = {contract_balance}, Sum of user balances (available={sum_available} + locked={sum_locked}) = {total_user_liabilities}"
    );
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[test]
fn test_total_balance_invariant_multi_user_deposits_and_withdrawals() {
    let mut harness = create_harness(3);
    let alice = harness.users.get(0).unwrap();
    let bob = harness.users.get(1).unwrap();
    let charlie = harness.users.get(2).unwrap();

    verify_vault_total_invariant(&harness, "Initial state (empty)");

    // User deposits
    harness.client.deposit(&alice, &1_000);
    harness
        .token_client
        .transfer(&alice, &harness.contract_id, &1_000);
    verify_vault_total_invariant(&harness, "After Alice deposit 1000");

    harness.client.deposit(&bob, &2_500);
    harness
        .token_client
        .transfer(&bob, &harness.contract_id, &2_500);
    verify_vault_total_invariant(&harness, "After Bob deposit 2500");

    harness.client.deposit(&charlie, &500);
    harness
        .token_client
        .transfer(&charlie, &harness.contract_id, &500);
    verify_vault_total_invariant(&harness, "After Charlie deposit 500");

    // Interleaved withdrawals
    harness.client.withdraw(&alice, &400);
    verify_vault_total_invariant(&harness, "After Alice withdraw 400");

    harness.client.withdraw(&bob, &1_000);
    verify_vault_total_invariant(&harness, "After Bob withdraw 1000");

    harness.client.deposit(&alice, &300);
    harness
        .token_client
        .transfer(&alice, &harness.contract_id, &300);
    verify_vault_total_invariant(&harness, "After Alice second deposit 300");

    harness.client.withdraw(&charlie, &500);
    verify_vault_total_invariant(&harness, "After Charlie full withdraw 500");

    assert_eq!(harness.client.get_balance(&alice), 900);
    assert_eq!(harness.client.get_balance(&bob), 1_500);
    assert_eq!(harness.client.get_balance(&charlie), 0);
}

#[test]
fn test_total_balance_invariant_with_active_and_matured_locks() {
    let harness = create_harness(2);
    let alice = harness.users.get(0).unwrap();
    let bob = harness.users.get(1).unwrap();

    harness.client.deposit(&alice, &5_000);
    harness
        .token_client
        .transfer(&alice, &harness.contract_id, &5_000);

    harness.client.deposit(&bob, &3_000);
    harness
        .token_client
        .transfer(&bob, &harness.contract_id, &3_000);

    verify_vault_total_invariant(&harness, "Initial deposits");

    // Alice locks funds in 2 stages
    harness.client.lock_funds(&alice, &1_500, &5_000);
    verify_vault_total_invariant(&harness, "Alice Lock 1 (1500 until T=5000)");

    harness.client.lock_funds(&alice, &1_000, &10_000);
    verify_vault_total_invariant(&harness, "Alice Lock 2 (1000 until T=10000)");

    // Bob locks funds
    harness.client.lock_funds(&bob, &2_000, &7_500);
    verify_vault_total_invariant(&harness, "Bob Lock 1 (2000 until T=7500)");

    // Total balances checked: available balance reduced, locked balance increased, global total preserved
    assert_eq!(harness.client.get_balance(&alice), 2_500);
    assert_eq!(harness.client.get_locked_balance(&alice), 2_500);
    assert_eq!(harness.client.get_balance(&bob), 1_000);
    assert_eq!(harness.client.get_locked_balance(&bob), 2_000);

    // Advance ledger time to T=5,000 (Alice Lock 1 matures)
    set_ledger_timestamp(&harness.env, 5_000);
    verify_vault_total_invariant(&harness, "At T=5000 (Alice Lock 1 matured)");

    assert_eq!(harness.client.get_balance(&alice), 4_000); // 2500 + 1500 matured
    assert_eq!(harness.client.get_locked_balance(&alice), 1_000);

    // Alice withdraws from matured available balance
    harness.client.withdraw(&alice, &3_500);
    verify_vault_total_invariant(&harness, "After Alice withdraw 3500");

    // Advance ledger time to T=10,000 (All locks matured)
    set_ledger_timestamp(&harness.env, 10_000);
    verify_vault_total_invariant(&harness, "At T=10000 (All locks matured)");

    assert_eq!(harness.client.get_locked_balance(&alice), 0);
    assert_eq!(harness.client.get_locked_balance(&bob), 0);
}

#[test]
fn test_total_balance_invariant_preserved_on_failed_operations() {
    let harness = create_harness(2);
    let alice = harness.users.get(0).unwrap();
    let bob = harness.users.get(1).unwrap();

    harness.client.deposit(&alice, &1_000);
    harness
        .token_client
        .transfer(&alice, &harness.contract_id, &1_000);
    verify_vault_total_invariant(&harness, "Initial setup");

    // 1. Failed deposit (0 or negative)
    let res = harness.client.try_deposit(&alice, &0);
    assert!(res.is_err(), "Deposit 0 must fail");
    verify_vault_total_invariant(&harness, "After failed deposit 0");

    let res = harness.client.try_deposit(&alice, &-50);
    assert!(res.is_err(), "Deposit negative must fail");
    verify_vault_total_invariant(&harness, "After failed deposit negative");

    // 2. Failed withdraw (exceeding balance, 0, negative)
    let res = harness.client.try_withdraw(&alice, &1_001);
    assert!(res.is_err(), "Withdraw exceeding balance must fail");
    verify_vault_total_invariant(&harness, "After failed withdraw > balance");

    let res = harness.client.try_withdraw(&alice, &0);
    assert!(res.is_err(), "Withdraw 0 must fail");
    verify_vault_total_invariant(&harness, "After failed withdraw 0");

    let res = harness.client.try_withdraw(&bob, &1);
    assert!(res.is_err(), "Withdraw from empty user must fail");
    verify_vault_total_invariant(&harness, "After failed withdraw from empty user");

    // 3. Failed lock_funds (0, negative, past unlock_time, exceeding balance)
    let res = harness.client.try_lock_funds(&alice, &0, &5_000);
    assert!(res.is_err(), "Lock amount 0 must fail");
    verify_vault_total_invariant(&harness, "After failed lock 0");

    let res = harness.client.try_lock_funds(&alice, &200, &500); // T=1000 is current ledger time
    assert!(res.is_err(), "Lock past time must fail");
    verify_vault_total_invariant(&harness, "After failed lock past time");

    let res = harness.client.try_lock_funds(&alice, &1_001, &5_000);
    assert!(res.is_err(), "Lock exceeding balance must fail");
    verify_vault_total_invariant(&harness, "After failed lock > balance");

    // Verify balances remain exact
    assert_eq!(harness.client.get_balance(&alice), 1_000);
    assert_eq!(harness.client.get_locked_balance(&alice), 0);
    assert_eq!(harness.client.get_balance(&bob), 0);
}

#[derive(Clone, Copy, Debug)]
enum TestOp {
    Deposit(usize, i128),
    Withdraw(usize, i128),
    Lock(usize, i128, u64),
    AdvanceTime(u64),
}

#[test]
fn test_total_balance_invariant_table_driven_sequences() {
    let harness = create_harness(4);

    let steps = [
        TestOp::Deposit(0, 10_000),
        TestOp::Deposit(1, 5_000),
        TestOp::Deposit(2, 2_000),
        TestOp::Lock(0, 3_000, 4_000),
        TestOp::Lock(1, 2_000, 6_000),
        TestOp::Withdraw(0, 2_000),
        TestOp::Withdraw(3, 100), // fails
        TestOp::Deposit(3, 1_000),
        TestOp::AdvanceTime(4_000), // User 0 lock matures
        TestOp::Withdraw(0, 4_000),
        TestOp::Lock(2, 2_000, 8_000),
        TestOp::AdvanceTime(6_000), // User 1 lock matures
        TestOp::Withdraw(1, 4_000),
        TestOp::AdvanceTime(8_000), // User 2 lock matures
        TestOp::Withdraw(2, 2_000),
    ];

    verify_vault_total_invariant(&harness, "Start of table-driven sequence");

    for (step_idx, op) in steps.iter().enumerate() {
        match op {
            TestOp::Deposit(u_idx, amount) => {
                let user = harness.users.get(*u_idx as u32).unwrap();
                let res = harness.client.try_deposit(&user, amount);
                if res.is_ok() && *amount > 0 {
                    harness
                        .token_client
                        .transfer(&user, &harness.contract_id, amount);
                }
            }
            TestOp::Withdraw(u_idx, amount) => {
                let user = harness.users.get(*u_idx as u32).unwrap();
                let _ = harness.client.try_withdraw(&user, amount);
            }
            TestOp::Lock(u_idx, amount, unlock_time) => {
                let user = harness.users.get(*u_idx as u32).unwrap();
                let _ = harness.client.try_lock_funds(&user, amount, unlock_time);
            }
            TestOp::AdvanceTime(ts) => {
                set_ledger_timestamp(&harness.env, *ts);
            }
        }

        let ctx = match step_idx {
            0 => "Step 0",
            1 => "Step 1",
            2 => "Step 2",
            3 => "Step 3",
            4 => "Step 4",
            5 => "Step 5",
            6 => "Step 6",
            7 => "Step 7",
            8 => "Step 8",
            9 => "Step 9",
            10 => "Step 10",
            11 => "Step 11",
            12 => "Step 12",
            13 => "Step 13",
            14 => "Step 14",
            _ => "Step X",
        };
        verify_vault_total_invariant(&harness, ctx);
    }
}
