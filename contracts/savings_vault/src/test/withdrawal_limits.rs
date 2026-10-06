//! Per-user withdrawal ceiling tests (issue #453).
//!
//! The ceiling is deliberately user-controlled. It is a transaction boundary,
//! not an admin custody control: a user can raise or disable it before
//! withdrawing an oversized matured lock.

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::Address;

#[test]
fn withdrawal_limit_defaults_to_disabled_and_round_trips() {
    let (env, _contract_id, client) = setup();
    let user = Address::generate(&env);

    assert_eq!(client.get_withdrawal_limit(&user), 0);

    client.set_withdrawal_limit(&user, &250);
    assert_eq!(client.get_withdrawal_limit(&user), 250);

    client.set_withdrawal_limit(&user, &0);
    assert_eq!(client.get_withdrawal_limit(&user), 0);
}

#[test]
fn available_withdrawal_enforces_limit_without_mutation() {
    let env = test_env();
    let (_contract_id, client, token_client, token_admin, _vault_admin) =
        vault_with_sac(&env);
    let user = Address::generate(&env);

    token_admin.mint(&user, &500);
    client.deposit(&user, &500);
    client.set_withdrawal_limit(&user, &100);

    let before_tokens = token_client.balance(&user);
    let result = client.try_withdraw(&user, &101);
    assert!(result.is_err());

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(token_client.balance(&user), before_tokens);

    client.set_withdrawal_limit(&user, &101);
    client.withdraw(&user, &101);
    assert_eq!(client.get_balance(&user), 399);
    assert_eq!(token_client.balance(&user), before_tokens + 101);
}

#[test]
fn matured_lock_limit_is_fail_closed_but_user_can_raise_it_to_exit() {
    let env = test_env();
    let (_contract_id, client, token_client, token_admin, _vault_admin) =
        vault_with_sac(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    let lock_id = client.lock_funds(&user, &400, &2_000);

    client.set_withdrawal_limit(&user, &300);
    env.ledger().set_timestamp(2_000);

    let before_tokens = token_client.balance(&user);
    let result = client.try_withdraw_lock(&user, &lock_id);
    assert!(result.is_err());

    let lock = client.get_lock(&user, &lock_id).expect("lock should remain present");
    assert!(!lock.withdrawn);
    assert_eq!(lock.amount, 400);
    assert_eq!(client.get_locked_balance(&user), 400);
    assert_eq!(token_client.balance(&user), before_tokens);

    // The owner controls the guard and can raise it to preserve the vault's
    // documented "users can always exit" invariant.
    client.set_withdrawal_limit(&user, &400);
    client.withdraw_lock(&user, &lock_id);

    let lock = client.get_lock(&user, &lock_id).expect("lock should remain queryable");
    assert!(lock.withdrawn);
    assert_eq!(lock.amount, 0);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(token_client.balance(&user), before_tokens + 400);
}

#[test]
fn withdrawal_limits_are_isolated_per_user() {
    let (env, _contract_id, client) = setup();
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    client.set_withdrawal_limit(&alice, &100);
    client.set_withdrawal_limit(&bob, &250);

    assert_eq!(client.get_withdrawal_limit(&alice), 100);
    assert_eq!(client.get_withdrawal_limit(&bob), 250);
}

#[test]
fn negative_limit_is_rejected_and_preserves_existing_value() {
    let (env, _contract_id, client) = setup();
    let user = Address::generate(&env);

    client.set_withdrawal_limit(&user, &100);
    let result = client.try_set_withdrawal_limit(&user, &-1);
    assert!(result.is_err());

    assert_eq!(client.get_withdrawal_limit(&user), 100);
    assert_eq!(ContractError::WithdrawalLimitNegative as u32, 1008);
    assert_eq!(ContractError::WithdrawalLimitExceeded as u32, 4003);
}
