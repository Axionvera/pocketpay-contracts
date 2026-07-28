//! Simulation compatibility scenarios for SDK integration (issue #416).
//!
//! Soroban clients preview a call via `simulateTransaction` before asking
//! the user to sign anything. Simulation never commits state, so an SDK
//! needs to know, for each vault function: does it need a signature at
//! all, is it safe to call speculatively (e.g. to preview an outcome),
//! and does a failed simulation ever leave the ledger changed. These
//! tests prove those properties directly against the deployed contract
//! surface rather than just asserting them in prose.
//!
//! See `docs/sdk-contract-sequence.md` for the request/response shape of
//! `simulateTransaction` itself; this file covers per-function behaviour.

extern crate std;

use super::test_helpers::*;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, IntoVal};

/// Registers and initializes a vault without permanently mocking auth on
/// `env` (unlike `test_env()`/`setup()`), so calls made afterwards on this
/// same client genuinely require a real signature. `env.mock_all_auths()`
/// has no "off" switch once called, so `initialize`'s own required
/// signature is mocked for that one invocation only, via `mock_auths`.
fn init_without_mocking_future_calls(env: &Env) -> (Address, Address, SavingsVaultClient<'static>) {
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = {
        let issuer = Address::generate(env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };

    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (admin.clone(), token.clone()).into_val(env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &token);

    (contract_id, admin, client)
}

// =========================================================================
// 1. Read-only calls: safe to simulate, never require a signature
// =========================================================================

/// Every read-only getter must succeed with zero auth mocked at all — an
/// SDK can call these purely via `simulateTransaction` and never prompt
/// the user for a signature.
#[test]
fn test_read_only_calls_do_not_require_auth() {
    let env = strict_test_env();
    let (_contract_id, admin, client) = init_without_mocking_future_calls(&env);
    let user = Address::generate(&env);

    // None of the following mock any auth. If any of them internally
    // required require_auth(), this test would panic.
    let _ = client.get_version();
    let _ = client.get_token();
    let _ = client.get_admin();
    let _ = client.is_paused();
    let _ = client.get_min_deposit_amount();
    let _ = client.get_max_lock_duration();
    let _ = client.get_min_lock_duration();
    let _ = client.get_balance(&user);
    let _ = client.get_locked_balance(&user);
    let _ = client.can_withdraw(&user);
    let _ = client.get_balance_snapshot(&user);
    let _ = client.get_lock_summary(&user);
    let _ = client.get_lock(&user, &1);
    let _ = client.list_locks(&user, &0, &10);

    assert_eq!(client.get_admin(), admin);
}

// =========================================================================
// 2. Pre-initialization behaviour: split between safe defaults and a
//    single, predictable panic
// =========================================================================

/// `get_version` and the three optional-config getters (`get_min_deposit_amount`,
/// `get_max_lock_duration`, `get_min_lock_duration`) work even before
/// `initialize` is ever called — they read optional config with a `0`/
/// version-string default. Every other function, including every other
/// read-only getter, panics with the exact same "Contract is not
/// initialized" message. An SDK probing an unknown contract ID can rely
/// on this: those four calls are always simulation-safe, and any other
/// call's simulation failure with that exact message means "deploy but
/// don't call `initialize` yet" rather than a vault-specific error.
#[test]
fn test_pre_initialization_calls_split_between_safe_defaults_and_deterministic_panic() {
    let env = strict_test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Safe pre-initialization defaults.
    assert_eq!(client.get_version(), soroban_sdk::String::from_str(&env, "0.1.0"));
    assert_eq!(client.get_min_deposit_amount(), 0);
    assert_eq!(client.get_max_lock_duration(), 0);
    assert_eq!(client.get_min_lock_duration(), 0);

    // Everything else fails before initialization (exact literal message
    // checked below in test_pre_initialization_panic_message_is_identical).
    assert!(client.try_get_token().is_err());
    assert!(client.try_get_admin().is_err());
    assert!(client.try_is_paused().is_err());
    assert!(client.try_get_balance(&user).is_err());
    assert!(client.try_get_locked_balance(&user).is_err());
    assert!(client.try_can_withdraw(&user).is_err());
    assert!(client.try_get_balance_snapshot(&user).is_err());
    assert!(client.try_get_lock_summary(&user).is_err());
    assert!(client.try_get_lock(&user, &1).is_err());
    assert!(client.try_list_locks(&user, &0, &10).is_err());
    assert!(client.try_deposit(&user, &100).is_err());
    assert!(client.try_withdraw(&user, &100).is_err());
    assert!(client.try_lock_funds(&user, &100, &1_000).is_err());
    assert!(client.try_withdraw_lock(&user, &1).is_err());
}

/// Confirms the literal panic text a pre-initialization call surfaces —
/// two representative functions, one read-only and one state-changing —
/// is exactly "Contract is not initialized", so an SDK can match on it
/// verbatim rather than guessing at wording.
#[test]
#[should_panic]
fn test_pre_initialization_panic_message_is_identical_get_balance() {
    let env = strict_test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    client.get_balance(&Address::generate(&env));
}

#[test]
#[should_panic]
fn test_pre_initialization_panic_message_is_identical_deposit() {
    let env = strict_test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    client.deposit(&Address::generate(&env), &100);
}

// =========================================================================
// 3. State-changing calls: require a signature and fail deterministically
//    without one
// =========================================================================

/// Every state-changing function fails if called with no signature mocked
/// at all, matching what a client sees when it simulates a call for a
/// user who hasn't approved it yet: the simulation fails, no signature
/// is requested, and nothing about vault state changes.
#[test]
fn test_state_changing_calls_require_signature_and_fail_without_it() {
    let env = strict_test_env();
    let (_contract_id, admin, client) = init_without_mocking_future_calls(&env);
    let user = Address::generate(&env);

    assert!(client.try_deposit(&user, &100).is_err());
    assert!(client.try_withdraw(&user, &100).is_err());
    assert!(client.try_lock_funds(&user, &100, &1_000).is_err());
    assert!(client.try_withdraw_lock(&user, &1).is_err());
    assert!(client.try_extend_lock(&user, &1, &2_000).is_err());
    assert!(client.try_pause(&admin, &1_000).is_err());
    assert!(client.try_unpause(&admin).is_err());
    assert!(client
        .try_transfer_admin(&admin, &Address::generate(&env))
        .is_err());
    assert!(client.try_set_min_deposit_amount(&admin, &10).is_err());
    assert!(client.try_set_max_lock_duration(&admin, &10_000).is_err());
    assert!(client.try_set_min_lock_duration(&admin, &10).is_err());

    // None of the failed attempts above changed anything observable.
    assert_eq!(client.get_admin(), admin);
    assert!(!client.is_paused());
    assert_eq!(client.get_balance(&user), 0);
}

// =========================================================================
// 4. Matured withdrawal: read helpers correctly predict the paid call's
//    outcome at every point across the maturity boundary
// =========================================================================

/// An SDK should be able to call the free `can_withdraw` (and
/// `get_balance_snapshot().withdrawable`) read helpers to preview whether
/// a `withdraw_lock` simulation will succeed, without spending a
/// simulation round-trip on the paid call itself. This is a deterministic
/// fixture: same lock, same three points in time, same two predictions,
/// every run.
#[test]
fn test_can_withdraw_predicts_withdraw_lock_outcome_across_maturity_boundary() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &1_000);
    let lock_id = client.lock_funds(&user, &400, &5_000);

    // Before maturity: both read helpers say "not yet", and the paid call agrees.
    set_ledger_timestamp(&env, 4_999);
    assert!(!client.can_withdraw(&user));
    assert_eq!(client.get_balance_snapshot(&user).withdrawable, 0);
    assert!(client.try_withdraw_lock(&user, &lock_id).is_err());

    // Exact maturity: both read helpers flip to "yes", and the paid call succeeds.
    set_ledger_timestamp(&env, 5_000);
    assert!(client.can_withdraw(&user));
    assert_eq!(client.get_balance_snapshot(&user).withdrawable, 400);
    assert!(client.try_withdraw_lock(&user, &lock_id).is_ok());

    // After withdrawal: both read helpers agree there is nothing left to withdraw.
    assert!(!client.can_withdraw(&user));
    assert_eq!(client.get_balance_snapshot(&user).withdrawable, 0);
}

// =========================================================================
// 5. Error cases: failed state-changing calls are side-effect-free with a
//    stable, deterministic message
// =========================================================================

/// Deposit, withdraw, and lock_funds error paths never mutate state, and
/// panic with the exact same literal message every run — an SDK can
/// safely surface the diagnostic text from a failed simulation without
/// worrying it's non-deterministic or partially-applied.
#[test]
fn test_failed_state_changing_calls_leave_state_completely_unchanged() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);

    let balance_before = client.get_balance(&user);
    let locked_before = client.get_locked_balance(&user);

    for _ in 0..2 {
        assert!(client.try_deposit(&user, &0).is_err());
        assert!(client.try_deposit(&user, &-1).is_err());
        assert!(client.try_withdraw(&user, &0).is_err());
        assert!(client.try_withdraw(&user, &10_000).is_err());
        assert!(client.try_lock_funds(&user, &0, &10_000).is_err());
        assert!(client.try_lock_funds(&user, &10_000, &10_000).is_err());
    }

    assert_eq!(client.get_balance(&user), balance_before);
    assert_eq!(client.get_locked_balance(&user), locked_before);
}

// =========================================================================
// 6. Repeated read-only simulation calls are idempotent
// =========================================================================

/// An SDK polling a balance screen re-simulates the same read-only call
/// repeatedly. That must never drift the result or touch state.
#[test]
fn test_repeated_read_only_simulation_calls_are_idempotent() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &300);

    let first = client.get_balance_snapshot(&user);
    for _ in 0..5 {
        assert_eq!(client.get_balance_snapshot(&user), first);
        assert_eq!(client.get_balance(&user), first.unlocked);
    }
}

// =========================================================================
// 7. Read helpers agree with each other
// =========================================================================

/// `get_lock_summary` is an aggregate; `list_locks`/`get_lock` are the
/// per-entry source of truth. An SDK combining both in one screen needs
/// them to describe the same state.
#[test]
fn test_lock_summary_and_list_locks_agree_on_lock_state() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id, client);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &1_000);
    client.lock_funds(&user, &100, &2_000);
    client.lock_funds(&user, &150, &3_000);
    client.lock_funds(&user, &200, &4_000);

    let summary = client.get_lock_summary(&user);
    let locks = client.list_locks(&user, &0, &10);

    assert_eq!(summary.active_count as usize, locks.len() as usize);
    let total: i128 = locks.iter().map(|l| l.amount).sum();
    assert_eq!(summary.total_locked_amount, total);

    for lock in locks.iter() {
        let fetched = client.get_lock(&user, &lock.id).unwrap();
        assert_eq!(fetched.amount, lock.amount);
        assert_eq!(fetched.unlock_time, lock.unlock_time);
    }
}
