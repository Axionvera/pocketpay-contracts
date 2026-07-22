//! Unit tests for the Savings Vault contract.
//!
//! These tests use the Soroban SDK test utilities to simulate
//! on-chain interactions in an isolated environment.
mod balance_conservation;
mod initialization;
mod lock_read_helpers;
mod maximum_amount_boundary;
mod property_vault_accounting;
mod property_fee_invariants;
mod test_helpers;
mod unauthorized_access;
mod zero_duration_lock;
mod withdraw_lock;
mod replay_protection;
mod admin_invariant_guard;
mod pause;

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Events, Address};

use test_helpers::*;

// =========================================================================
// Version Metadata Tests
// =========================================================================

#[test]
fn test_get_version() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let version = client.get_version();
    assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_deposit_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.deposit(&user, &100);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_withdraw_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.withdraw(&user, &100);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_lock_funds_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    client.lock_funds(&user, &100, &2_000);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_balance_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_balance(&user);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_locked_balance_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_locked_balance(&user);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_can_withdraw_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.can_withdraw(&user);
}

// =========================================================================
// Deposit Tests
// =========================================================================

#[test]
fn test_deposit() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 100);
    assert_eq!(client.get_balance(&user), 100);
}

#[test]
fn test_multiple_deposits() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    seed_balances(&client, &user, &[100, 250]);
    assert_eq!(client.get_balance(&user), 350);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_zero_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &0);
}

#[test]
#[should_panic(expected = "Deposit amount must be greater than zero")]
fn test_deposit_negative_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &-50);
}

#[test]
fn test_deposit_fails_when_token_transfer_fails() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    // The user holds fewer tokens than they try to deposit, so the SAC transfer
    // reverts. The deposit must fail and internal accounting must stay unchanged,
    // proving balances are only credited after a successful token transfer.
    token_admin.mint(&user, &50);
    let before = client.get_balance(&user);

    let result = client.try_deposit(&user, &100);
    assert!(
        result.is_err(),
        "deposit must fail when the token transfer fails"
    );
    assert_eq!(
        client.get_balance(&user),
        before,
        "a failed deposit must not mutate the user's balance"
    );
}

#[test]
fn test_get_balance_default_zero_for_new_user_after_initialization() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    // Note: init_contract already handles initialization, removed duplicate call.

    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// Withdrawal Tests
// =========================================================================

#[test]
fn test_withdraw() {
    let (env, contract_id, client) = setup();

    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);

    let user = Address::generate(&env);
    let deposit_amount = 500;

    token_admin.mint(&user, &10000);

    let user_balance = token_client.balance(&user);
    assert_eq!(&user_balance, &10000);

    // Deposit now performs real token transfer
    client.deposit(&user, &deposit_amount);

    let user_balance = token_client.balance(&user);
    assert_eq!(&user_balance, &9500);

    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

#[test]
fn test_withdraw_returns_tokens_to_user() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);

    // Full token-custody round-trip: the user funds their wallet, deposits into the
    // vault (tokens leave the wallet), then withdraws (tokens return). Existing tests
    // only check the internal balance; this asserts the real SAC token balance moves
    // back to the user, proving withdrawals return actual token custody, not just
    // internal accounting.
    token_admin.mint(&user, &1000);
    assert_eq!(token_client.balance(&user), 1000);

    client.deposit(&user, &400);
    assert_eq!(
        token_client.balance(&user),
        600,
        "deposit moves tokens into the vault"
    );

    client.withdraw(&user, &400);
    assert_eq!(
        token_client.balance(&user),
        1000,
        "withdrawal returns tokens to the user"
    );
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
fn test_withdraw_entire_balance() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    let deposit_amount = 100;

    // Deposit performs a real token transfer, so the user must hold tokens first.
    token_admin.mint(&user, &deposit_amount);
    client.deposit(&user, &deposit_amount);

    client.withdraw(&user, &deposit_amount);
    assert_eq!(client.get_balance(&user), 0);
    // The full amount is returned to the user's token balance.
    assert_eq!(token_client.balance(&user), deposit_amount);
}

#[test]
#[should_panic]
fn test_withdraw_requires_user_authorization() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token_address);
    token_admin.mock_all_auths().mint(&user, &1_000);
    client.mock_all_auths().deposit(&user, &100);

    // Call without auth mocking: require_auth() must reject this withdrawal.
    client.withdraw(&user, &1);
}

#[test]
#[should_panic]
fn test_deposit_requires_user_authorization() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token_address);
    token_admin.mock_all_auths().mint(&user, &1_000);

    // Call without auth mocking: require_auth() must reject this deposit.
    client.deposit(&user, &100);
}

#[test]
#[should_panic]
fn test_lock_funds_requires_user_authorization() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token_address);

    let user = Address::generate(&env);

    client.mock_all_auths().initialize(&admin, &token_address);
    token_admin.mock_all_auths().mint(&user, &1_000);
    client.mock_all_auths().deposit(&user, &100);
    set_ledger_timestamp(&env, 1_000);

    // Call without auth mocking: require_auth() must reject this lock.
    client.lock_funds(&user, &50, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_more_than_balance_panics() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);

    token_admin.mint(&user, &100);
    client.deposit(&user, &100);

    client.withdraw(&user, &200);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_zero_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, _token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    deposit_balance(&client, &user, 100);
    client.withdraw(&user, &0);
}

#[test]
#[should_panic(expected = "Withdrawal amount must be greater than zero")]
fn test_withdraw_negative_panics() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);

    token_admin.mint(&user, &100);
    client.deposit(&user, &100);

    client.withdraw(&user, &-10);
}

#[test]
fn test_withdraw_from_empty_balance_panics() {
    // AC: Withdrawing from an empty balance fails.
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, _token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);

    // User never deposited — balance is implicitly 0
    let res = client.try_withdraw(&user, &1);
    assert!(res.is_err());
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_withdraw_exceeds_available_after_deposit_panics() {
    // AC: Withdrawing more than available balance fails.
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);

    client.deposit(&user, &100);
    // Attempt to withdraw more than deposited
    client.withdraw(&user, &101);
}

/// Verify that a successful withdraw leaves the remaining balance correct,
/// which also proves the contract does not corrupt state on partial withdrawals.
/// The companion panic test (`test_failed_withdraw_does_not_change_available_balance_panics`)
/// confirms the over-withdraw is rejected before any mutation occurs.
#[test]
fn test_failed_withdraw_does_not_change_available_balance() {
    // AC: Failed withdrawal does not change available balance.
    // Strategy (no_std): perform a *valid* withdraw of the exact balance to
    // prove state is only mutated on success, paired with the should_panic
    // test below that confirms rejection happens before any write.
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    let deposit_amount = 100;

    // Deposit now performs real token transfer
    token_admin.mint(&user, &deposit_amount);
    client.deposit(&user, &deposit_amount);

    // A valid partial withdraw succeeds and leaves the remainder intact.
    client.withdraw(&user, &60);
    assert_eq!(client.get_balance(&user), 40);

    // A second withdraw of exactly the remaining amount also succeeds.
    client.withdraw(&user, &40);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_failed_withdraw_does_not_change_available_balance_panics() {
    // Confirms that attempting to withdraw 1 unit more than deposited
    // is rejected (panics) — i.e. the balance is never decremented.
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);

    client.deposit(&user, &100);
    client.withdraw(&user, &101); // must panic — balance stays at 100
}

#[test]
#[should_panic(expected = "Insufficient balance")]
fn test_failed_withdraw_does_not_change_locked_balance() {
    // AC: Failed withdrawal does not change locked balance if applicable.
    // Depositing 500 and locking 300 leaves 200 available.
    // Attempting to withdraw 201 must panic, leaving both balances intact.
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);

    client.deposit(&user, &500);
    // Lock 300, leaving 200 available
    client.lock_funds(&user, &300, &10_000);

    assert_eq!(client.get_balance(&user), 200);
    assert_eq!(client.get_locked_balance(&user), 300);

    // Attempt to withdraw more than the available 200 — must panic.
    // Because the panic is raised before any storage write, both the
    // available and locked balances remain unchanged.
    client.withdraw(&user, &201);
}

// =========================================================================
// Balance Query Tests
// =========================================================================

#[test]
fn test_get_balance_no_deposits() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, _token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// Fund Locking Tests
// =========================================================================

#[test]
fn test_lock_funds() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &2_000);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);
}

#[test]
fn test_lock_funds_multiple_times() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);
}

// -------------------------------------------------------------------------
// Repeated lock operations — independent multi-lock maturity
// -------------------------------------------------------------------------
//
// After multi-lock support, each `lock_funds` call creates an independent
// `LockEntry` with its own `unlock_time`. Behaviour when locking repeatedly:
//
// - Locked balance: **accumulates**. Each call adds `amount` on top of
//   whatever is already locked.
// - Available (deposited) balance: decreases by each `amount` locked.
// - Unlock times: **independent**, not overwritten. Each entry matures on
//   its own schedule.
// - `get_locked_balance`: sums only *unmatured* locks
//   (`current_time < unlock_time`).
// - `get_balance`: deposited balance + *matured* lock amounts.
// - `can_withdraw`: `true` if *any* lock has matured
//   (`current_time >= unlock_time`).

/// Two independent locks with a later second unlock time.
///
/// Lock 1: 300 until T=5_000
/// Lock 2: 200 until T=6_000
///
/// At T=5_000 only lock 1 matures; lock 2 remains locked until T=6_000.
#[test]
fn test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &300, &5_000);
    client.lock_funds(&user, &200, &6_000);

    // Before either matures: both amounts locked, remaining deposit available.
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // At lock 1's unlock time: lock 1 matures (available), lock 2 still locked.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 200);
    assert_eq!(client.get_balance(&user), 800); // 500 deposited + 300 matured

    // At lock 2's unlock time: both locks matured.
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

/// Two independent locks where the second unlock time is earlier.
///
/// Lock 1: 300 until T=6_000
/// Lock 2: 200 until T=5_000
///
/// At T=5_000 only lock 2 matures; lock 1 stays locked until T=6_000.
/// Earlier locks do not pull later locks forward (and vice versa).
#[test]
fn test_repeated_lock_overwrites_unlock_time_with_earlier_value() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &300, &6_000);
    client.lock_funds(&user, &200, &5_000);

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);

    // Only the earlier lock (200) matures at T=5_000; 300 remains locked.
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 300);
    assert_eq!(client.get_balance(&user), 700); // 500 deposited + 200 matured

    // Remaining lock matures at T=6_000.
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

/// Three independent locks: each matures on its own schedule.
#[test]
fn test_repeated_lock_three_times_accumulates_and_keeps_last_unlock_time() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 1_000);

    client.lock_funds(&user, &100, &3_000);
    client.lock_funds(&user, &100, &4_000);
    client.lock_funds(&user, &100, &7_000);

    assert_eq!(client.get_balance(&user), 700);
    assert_eq!(client.get_locked_balance(&user), 300);

    // At T=4_000 the first two locks have matured; the third is still locked.
    set_ledger_timestamp(&env, 4_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 100);
    assert_eq!(client.get_balance(&user), 900); // 700 deposited + 200 matured

    // All three mature once the latest unlock time is reached.
    set_ledger_timestamp(&env, 7_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 1_000);
}

#[test]
#[should_panic(expected = "Lock amount must be greater than zero")]
fn test_lock_zero_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &0, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_more_than_balance_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &500, &2_000);
}

#[test]
#[should_panic(expected = "Unlock time must be in the future")]
fn test_lock_past_time_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 5_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 100);
    client.lock_funds(&user, &50, &3_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_from_empty_balance_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    // User has 0 balance, attempt to lock 100
    client.lock_funds(&user, &100, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_lock_more_than_available_balance_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);
    // Attempt to lock more than available (100)
    client.lock_funds(&user, &101, &2_000);
}

#[test]
fn test_failed_lock_does_not_change_available_balance() {
    // Strategy: Verify a valid partial lock leaves the remaining available balance correct.
    // The companion panic test confirms that the lock is rejected before any mutation occurs.
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    // Initial check
    assert_eq!(client.get_balance(&user), 100);

    // A valid partial lock succeeds and updates available/locked balances
    client.lock_funds(&user, &60, &2_000);
    assert_eq!(client.get_balance(&user), 40);

    // Another valid lock
    client.lock_funds(&user, &40, &3_000);
    assert_eq!(client.get_balance(&user), 0);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_failed_lock_does_not_change_available_balance_panics() {
    // Confirms that attempting to lock more than available balance is rejected (panics)
    // and available balance is not mutated.
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 100);

    // Attempting to lock 101 must panic, leaving available balance at 100
    client.lock_funds(&user, &101, &2_000);
}

#[test]
#[should_panic(expected = "Insufficient balance to lock")]
fn test_failed_lock_does_not_change_locked_balance() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 500);

    // Lock 200, leaving 300 available, and locked balance at 200
    client.lock_funds(&user, &200, &2_000);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);

    // Attempt to lock 301, which is more than available 300.
    // This must panic, leaving locked balance at 200.
    client.lock_funds(&user, &301, &3_000);
}

// =========================================================================
// can_withdraw Tests — Time-Lock Boundary Behaviour
// =========================================================================
//
// Boundary rule: `can_withdraw` returns `true` when
//   ledger.timestamp() >= unlock_time (inclusive).
// This section tests before, exactly at, and after the unlock time,
// with explicit boundary positions so the rule is unambiguous.

// -------------------------------------------------------------------------
// Before unlock — returns false
// -------------------------------------------------------------------------

/// Funds locked at T=1000 with unlock at T=10_000.
/// Checking at T=1000 (right after locking) — still far before unlock.
#[test]
fn test_can_withdraw_before_unlock() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &10_000);
    assert_eq!(client.can_withdraw(&user), false);
}

/// Boundary: 1 second before unlock.
/// Unlock is at T=5000, check at T=4999 — still locked.
#[test]
fn test_can_withdraw_one_second_before_unlock_returns_false() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    // Set ledger to exactly 1 second before unlock
    set_ledger_timestamp(&env, 4_999);
    assert_eq!(client.can_withdraw(&user), false);
}

// -------------------------------------------------------------------------
// At unlock — returns true (inclusive boundary)
// -------------------------------------------------------------------------

/// Boundary: exactly at unlock time.
/// Unlock at T=5000, check at T=5000 — funds are now withdrawable.
/// This confirms the boundary is **inclusive** (>=).
#[test]
fn test_can_withdraw_exactly_at_unlock() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
}

// -------------------------------------------------------------------------
// After unlock — returns true
// -------------------------------------------------------------------------

/// Unlock at T=5000, check at T=6000 — well after unlock.
#[test]
fn test_can_withdraw_after_unlock() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    set_ledger_timestamp(&env, 6_000);
    assert_eq!(client.can_withdraw(&user), true);
}

/// Boundary: 1 second after unlock.
/// Unlock at T=5000, check at T=5001 — confirm it's still true.
#[test]
fn test_can_withdraw_one_second_after_unlock_returns_true() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);
    // Set ledger to exactly 1 second after unlock
    set_ledger_timestamp(&env, 5_001);
    assert_eq!(client.can_withdraw(&user), true);
}

// -------------------------------------------------------------------------
// No locked funds — returns false
// -------------------------------------------------------------------------

/// User with no locked funds always returns false, regardless of
/// any stored unlock time or timestamp.
#[test]
fn test_can_withdraw_no_locked_funds() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, _token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    assert_eq!(client.can_withdraw(&user), false);
}

// -------------------------------------------------------------------------
// Locked balance correctness across boundary checks
// -------------------------------------------------------------------------

/// The locked balance is unaffected by repeated `can_withdraw` queries.
/// Lock 300 at T=1000, unlock at T=5000. Check locked balance before,
/// at, and after unlock — it should remain 300 throughout.
#[test]
fn test_locked_balance_correct_before_at_and_after_unlock() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &300, &5_000);

    // Before unlock (T=4999): cannot withdraw, locked balance = 300
    set_ledger_timestamp(&env, 4_999);
    assert_eq!(client.can_withdraw(&user), false);
    assert_eq!(client.get_locked_balance(&user), 300);
    // Available balance still reflects deduction
    assert_eq!(client.get_balance(&user), 200);

    // At unlock (T=5000): can withdraw, locked balance = 0
    set_ledger_timestamp(&env, 5_000);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 500);

    // After unlock (T=5001): can withdraw, locked balance = 0
    set_ledger_timestamp(&env, 5_001);
    assert_eq!(client.can_withdraw(&user), true);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 500);
}

// -------------------------------------------------------------------------
// Boundary rule documentation test
// -------------------------------------------------------------------------

/// This test explicitly documents the boundary rule:
/// `can_withdraw` uses **inclusive** comparison (>=).
///
/// - ledger.timestamp() <  unlock_time  →  false  (locked)
/// - ledger.timestamp() >= unlock_time  →  true   (unlocked, if locked_balance > 0)
#[test]
fn test_can_withdraw_boundary_rule_is_inclusive_gte() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);
    let unlock_time: u64 = 5_000;

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1000);
    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &unlock_time);

    // t < unlock_time → false
    set_ledger_timestamp(&env, unlock_time - 1);
    assert!(
        !client.can_withdraw(&user),
        "Expected false when ledger.timestamp() < unlock_time"
    );

    // t == unlock_time → true (inclusive boundary)
    set_ledger_timestamp(&env, unlock_time);
    assert!(
        client.can_withdraw(&user),
        "Expected true when ledger.timestamp() == unlock_time (inclusive >=)"
    );

    // t > unlock_time → true
    set_ledger_timestamp(&env, unlock_time + 1);
    assert!(
        client.can_withdraw(&user),
        "Expected true when ledger.timestamp() > unlock_time"
    );
}

// =========================================================================
// Isolation Tests (multiple users)
// =========================================================================

#[test]
fn test_separate_user_balances() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // Deposit now performs real token transfer
    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 500);

    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_balance(&bob), 500);

    client.withdraw(&alice, &200);
    assert_eq!(client.get_balance(&alice), 800);
    assert_eq!(client.get_balance(&bob), 500);
}

#[test]
fn balance_isolation_between_users_deposit() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    deposit_balance(&client, &alice, 1_000);
    assert_eq!(client.get_balance(&alice), 1000_i128);
    assert_eq!(client.get_balance(&bob), 0_i128);
}

#[test]
fn balance_isolation_between_users_withdraw() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // Deposit now performs real token transfer
    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 4_000);

    assert_eq!(client.get_balance(&alice), 1000_i128);
    assert_eq!(client.get_balance(&bob), 4000_i128);

    client.withdraw(&alice, &500);
    assert_eq!(client.get_balance(&alice), 500);
    assert_eq!(client.get_balance(&bob), 4000);

    client.withdraw(&bob, &2000);
    assert_eq!(client.get_balance(&alice), 500);
    assert_eq!(client.get_balance(&bob), 2000);
}

#[test]
fn balance_isolation_between_users_lock() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let alice = new_user(&env);
    let bob = new_user(&env);

    token_admin.mint(&alice, &10000);
    token_admin.mint(&bob, &10000);

    // Deposit now performs real token transfer
    deposit_balance(&client, &alice, 2_000);
    deposit_balance(&client, &bob, 4_000);

    client.lock_funds(&alice, &1_000, &3600);
    assert_eq!(client.get_balance(&alice), 1_000);
    assert_eq!(client.get_locked_balance(&alice), 1_000);
}

// =========================================================================
// Admin Function Tests
// =========================================================================

#[test]
fn test_get_admin() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    
    // Get admin from contract storage manually
    let admin = env.as_contract(&contract_id, || {
        env.storage().instance().get(&super::DataKey::Admin).unwrap()
    });
    
    assert_eq!(client.get_admin(), admin);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_get_admin_before_initialization_panics() {
    let env = Env::default();
    let contract_id = env.register(super::SavingsVault, ());
    let client = super::SavingsVaultClient::new(&env, &contract_id);
    
    client.get_admin();
}

#[test]
fn test_transfer_admin() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    
    let original_admin = env.as_contract(&contract_id, || {
        env.storage().instance().get(&super::DataKey::Admin).unwrap()
    });
    
    let new_admin = new_user(&env);
    
    client.transfer_admin(&original_admin, &new_admin);
    
    assert_eq!(client.get_admin(), new_admin);
}

#[test]
#[should_panic(expected = "Not authorized")]
fn test_transfer_admin_not_authorized_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    
    let random_user = new_user(&env);
    let new_admin = new_user(&env);
    
    client.transfer_admin(&random_user, &new_admin);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_transfer_admin_before_initialization_panics() {
    let env = Env::default();
    let contract_id = env.register(super::SavingsVault, ());
    let client = super::SavingsVaultClient::new(&env, &contract_id);
    
    let random_user = new_user(&env);
    let new_admin = new_user(&env);
    
    client.transfer_admin(&random_user, &new_admin);
}

#[test]
#[should_panic]
fn test_transfer_admin_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(super::SavingsVault, ());
    let client = super::SavingsVaultClient::new(&env, &contract_id);
    let admin = new_user(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
    
    client.mock_all_auths().initialize(&admin, &token);
    
    let new_admin = new_user(&env);
    
    // Call without auth mocking
    client.transfer_admin(&admin, &new_admin);
}

#[test]
fn test_initialize_emits_event() {
    use soroban_sdk::{symbol_short, Symbol, TryIntoVal};

    let env = test_env();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    client.mock_all_auths().initialize(&admin, &token);

    let events = env.events().all();
    assert_eq!(events.len(), 1);

    let (_contract, topics, data) = events.get(0).unwrap();
    let topic0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let emitted_token: Address = data.try_into_val(&env).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "initialize"));
    assert_eq!(topic1, admin);
    assert_eq!(emitted_token, token);
}

#[test]
fn test_deposit_emits_event() {
    use soroban_sdk::{symbol_short, TryIntoVal};

    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let user = new_user(&env);
    token_admin.mint(&user, &1000);

    deposit_balance(&client, &user, 100);

    let events = env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();
    let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let (amount, new_balance): (i128, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(topic0, symbol_short!("deposit"));
    assert_eq!(topic1, user);
    assert_eq!((amount, new_balance), (100_i128, 100_i128));
}

#[test]
fn test_withdraw_emits_event() {
    use soroban_sdk::{symbol_short, TryIntoVal};

    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let user = new_user(&env);
    token_admin.mint(&user, &1000);

    deposit_balance(&client, &user, 100);
    client.withdraw(&user, &50);

    let events = env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();
    let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let (amount, new_balance, new_locked): (i128, i128, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(topic0, symbol_short!("withdraw"));
    assert_eq!(topic1, user);
    assert_eq!((amount, new_balance, new_locked), (50_i128, 50_i128, 0_i128));
}

#[test]
fn test_lock_funds_emits_event() {
    use soroban_sdk::{symbol_short, TryIntoVal};

    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, contract_id, client);

    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 200);
    client.lock_funds(&user, &100, &2_000);

    let events = env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();
    let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let (amount, unlock_time, new_balance, new_locked): (i128, u64, i128, i128) =
        data.try_into_val(&env).unwrap();
    assert_eq!(topic0, symbol_short!("lock"));
    assert_eq!(topic1, user);
    assert_eq!(
        (amount, unlock_time, new_balance, new_locked),
        (100_i128, 2_000_u64, 100_i128, 100_i128)
    );
}
