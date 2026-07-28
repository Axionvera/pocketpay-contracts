//! Unit tests for deposit amount normalisation, precision assumptions, minimum values, and invalid inputs.

use super::*;
use test_helpers::*;

#[test]
fn test_minimum_valid_deposit() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    // Minimum valid deposit is 1 atomic base unit (e.g., 1 stroop)
    let min_amount: i128 = 1;
    client.deposit(&user, &min_amount);

    assert_eq!(client.get_balance(&user), 1);
}

#[test]
fn test_typical_stroop_precision_deposit() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    // 1.5 XLM = 15_000_000 stroops (7 decimal places)
    let stroop_amount: i128 = 15_000_000;
    client.deposit(&user, &stroop_amount);

    assert_eq!(client.get_balance(&user), 15_000_000);
}

#[test]
#[should_panic]
fn test_deposit_zero_amount_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &0);
}

#[test]
#[should_panic]
fn test_deposit_negative_one_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &-1);
}

#[test]
#[should_panic]
fn test_deposit_min_i128_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &i128::MIN);
}

#[test]
#[should_panic]
fn test_deposit_overflow_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    // Initial deposit of max i128
    client.deposit(&user, &i128::MAX);

    // Attempting to deposit 1 more must panic with overflow
    client.deposit(&user, &1);
}

#[test]
fn test_minimum_lock_funds_amount() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &100);
    let unlock_time = env.ledger().timestamp() + 3600;

    // Minimum lock amount is 1 atomic unit
    let lock_id = client.lock_funds(&user, &1, &unlock_time);
    assert_eq!(lock_id, 1);
    assert_eq!(client.get_balance(&user), 99);
    assert_eq!(client.get_locked_balance(&user), 1);
}

#[test]
#[should_panic]
fn test_lock_funds_zero_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &100);
    let unlock_time = env.ledger().timestamp() + 3600;

    client.lock_funds(&user, &0, &unlock_time);
}

#[test]
#[should_panic]
fn test_lock_funds_negative_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = new_user(&env);

    client.deposit(&user, &100);
    let unlock_time = env.ledger().timestamp() + 3600;

    client.lock_funds(&user, &-5, &unlock_time);
}
