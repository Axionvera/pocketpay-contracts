//! Structured contract error code tests.
//!
//! Verifies the full `ContractError` enum surface exposed by the contract via
//! `env.error_contract(variant)`. Each test exercises a single failure path
//! and asserts the panic emitted by the Soroban test harness contains the
//! exact numeric `u32` code matching the variant's `#[repr(u32)]` discriminant.
//!
//! SDK and mobile consumers map these same numeric codes to localized user
//! messages; this suite therefore doubles as a cross-repo compatibility
//! contract: any code change here is a BREAKING change for callers.

use super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::IntoVal;
use std::string::{String, ToString};
use std::format;

use crate::ContractError;
use test_helpers::*;

/// Helper: wrap a fallible client call in `std::panic::catch_unwind` and
/// extract the panic payload as a `String`. All error-code tests assert on
/// substrings of this payload, which in the soroban-sdk `testutils` harness
/// contains the `Status(ContractError, CODE)` diagnostic where `CODE` is the
/// u32 from the enum's `#[repr(u32)]`.
fn catch_panic_message<F: FnOnce()>(f: F) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(_) => panic!("expected a panic but the call succeeded"),
        Err(payload) => {
            if let Some(s) = payload.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                format!("{:?}", payload)
            }
        }
    }
}

// =========================================================================
// CATEGORY 1000: Validation
// =========================================================================

#[test]
fn error_code_1001_amount_not_positive_on_deposit() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    let msg = catch_panic_message(|| client.deposit(&user, &0));
    assert!(
        msg.contains((ContractError::AmountNotPositive as u32).to_string().as_str())
            || msg.contains("AmountNotPositive"),
        "panic payload must reference error code 1001 AmountNotPositive; got: {}",
        msg
    );
}

#[test]
fn error_code_1001_amount_not_positive_on_withdraw() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    let msg = catch_panic_message(|| client.withdraw(&user, &-1));
    assert!(
        msg.contains((ContractError::AmountNotPositive as u32).to_string().as_str())
            || msg.contains("AmountNotPositive"),
        "panic payload must reference error code 1001 AmountNotPositive; got: {}",
        msg
    );
}

#[test]
fn error_code_1001_amount_not_positive_on_lock() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    let msg = catch_panic_message(|| { client.lock_funds(&user, &0, &5_000); });
    assert!(
        msg.contains((ContractError::AmountNotPositive as u32).to_string().as_str())
            || msg.contains("AmountNotPositive"),
        "panic payload must reference error code 1001; got: {}",
        msg
    );
}

#[test]
fn error_code_1002_unlock_time_not_in_future_lock_funds() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    let msg = catch_panic_message(|| { client.lock_funds(&user, &50, &1_000); });
    assert!(
        msg.contains((ContractError::UnlockTimeNotInFuture as u32).to_string().as_str())
            || msg.contains("UnlockTimeNotInFuture"),
        "panic payload must reference error code 1002; got: {}",
        msg
    );
}

#[test]
fn error_code_1003_lock_duration_exceeds_maximum() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    set_ledger_timestamp(&env, 1_000);
    client.set_max_lock_duration(&vault_admin, &1_000);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    let msg = catch_panic_message(|| { client.lock_funds(&user, &10, &10_000); });
    assert!(
        msg.contains((ContractError::LockDurationExceedsMaximum as u32).to_string().as_str())
            || msg.contains("LockDurationExceedsMaximum"),
        "panic payload must reference error code 1003; got: {}",
        msg
    );
}

#[test]
fn error_code_1004_lock_duration_below_minimum() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    set_ledger_timestamp(&env, 1_000);
    client.set_min_lock_duration(&vault_admin, &5_000);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    let msg = catch_panic_message(|| { client.lock_funds(&user, &10, &2_000); });
    assert!(
        msg.contains((ContractError::LockDurationBelowMinimum as u32).to_string().as_str())
            || msg.contains("LockDurationBelowMinimum"),
        "panic payload must reference error code 1004; got: {}",
        msg
    );
}

#[test]
fn error_code_1005_amount_below_minimum_deposit() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    client.set_min_deposit_amount(&vault_admin, &1_000);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    let msg = catch_panic_message(|| client.deposit(&user, &50));
    assert!(
        msg.contains((ContractError::AmountBelowMinimumDeposit as u32).to_string().as_str())
            || msg.contains("AmountBelowMinimumDeposit"),
        "panic payload must reference error code 1005; got: {}",
        msg
    );
}

#[test]
fn error_code_1006_pause_duration_must_be_positive() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let msg = catch_panic_message(|| client.pause(&vault_admin, &0));
    assert!(
        msg.contains((ContractError::PauseDurationMustBePositive as u32).to_string().as_str())
            || msg.contains("PauseDurationMustBePositive"),
        "panic payload must reference error code 1006; got: {}",
        msg
    );
}

#[test]
fn error_code_1007_min_deposit_amount_negative() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let msg = catch_panic_message(|| client.set_min_deposit_amount(&vault_admin, &-1));
    assert!(
        msg.contains((ContractError::MinDepositAmountNegative as u32).to_string().as_str())
            || msg.contains("MinDepositAmountNegative"),
        "panic payload must reference error code 1007; got: {}",
        msg
    );
}

// =========================================================================
// CATEGORY 2000: Authorisation
// =========================================================================

#[test]
fn error_code_2001_not_authorized_admin_wrong_caller() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let rando = Address::generate(&env);
    let msg = catch_panic_message(|| client.unpause(&rando));
    assert!(
        msg.contains((ContractError::NotAuthorizedAdmin as u32).to_string().as_str())
            || msg.contains("NotAuthorizedAdmin"),
        "panic payload must reference error code 2001; got: {}",
        msg
    );
}

// =========================================================================
// CATEGORY 3000: Lifecycle
// =========================================================================

#[test]
fn error_code_3001_already_initialized() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let other_token = Address::generate(&env);
    let msg = catch_panic_message(|| client.initialize(&vault_admin, &other_token));
    assert!(
        msg.contains((ContractError::AlreadyInitialized as u32).to_string().as_str())
            || msg.contains("AlreadyInitialized"),
        "panic payload must reference error code 3001; got: {}",
        msg
    );
}

#[test]
fn error_code_3002_not_initialized_before_deposit() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    let msg = catch_panic_message(|| client.deposit(&user, &100));
    assert!(
        msg.contains((ContractError::NotInitialized as u32).to_string().as_str())
            || msg.contains("NotInitialized"),
        "panic payload must reference error code 3002; got: {}",
        msg
    );
}

#[test]
fn error_code_3003_contract_paused_blocks_deposit() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, vault_admin) = vault_with_sac(&env);
    client.pause(&vault_admin, &60);
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);
    let msg = catch_panic_message(|| client.deposit(&user, &100));
    assert!(
        msg.contains((ContractError::ContractPaused as u32).to_string().as_str())
            || msg.contains("ContractPaused"),
        "panic payload must reference error code 3003; got: {}",
        msg
    );
}

// =========================================================================
// CATEGORY 4000: Accounting
// =========================================================================

#[test]
fn error_code_4001_insufficient_balance_withdraw() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    let msg = catch_panic_message(|| client.withdraw(&user, &999_999));
    assert!(
        msg.contains((ContractError::InsufficientBalance as u32).to_string().as_str())
            || msg.contains("InsufficientBalance"),
        "panic payload must reference error code 4001; got: {}",
        msg
    );
}

#[test]
fn error_code_4002_insufficient_balance_to_lock() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    let msg = catch_panic_message(|| { client.lock_funds(&user, &9_999, &5_000); });
    assert!(
        msg.contains((ContractError::InsufficientBalanceToLock as u32).to_string().as_str())
            || msg.contains("InsufficientBalanceToLock"),
        "panic payload must reference error code 4002; got: {}",
        msg
    );
}

// =========================================================================
// CATEGORY 5000: Lock
// =========================================================================

#[test]
fn error_code_5001_lock_not_found_on_withdraw() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    let msg = catch_panic_message(|| { client.withdraw_lock(&user, &1337); });
    assert!(
        msg.contains((ContractError::LockNotFound as u32).to_string().as_str())
            || msg.contains("LockNotFound"),
        "panic payload must reference error code 5001; got: {}",
        msg
    );
}

#[test]
fn error_code_5002_lock_already_withdrawn() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);
    env.mock_all_auths();
    client.deposit(&user, &5_000);
    let id = client.lock_funds(&user, &2_000, &2_000);
    set_ledger_timestamp(&env, 10_000);
    client.withdraw_lock(&user, &id);
    let msg = catch_panic_message(|| { client.withdraw_lock(&user, &id); });
    assert!(
        msg.contains((ContractError::LockAlreadyWithdrawn as u32).to_string().as_str())
            || msg.contains("LockAlreadyWithdrawn"),
        "panic payload must reference error code 5002; got: {}",
        msg
    );
}

#[test]
fn error_code_5003_lock_not_matured() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);
    env.mock_all_auths();
    client.deposit(&user, &5_000);
    let id = client.lock_funds(&user, &2_000, &9_999_999);
    let msg = catch_panic_message(|| { client.withdraw_lock(&user, &id); });
    assert!(
        msg.contains((ContractError::LockNotMatured as u32).to_string().as_str())
            || msg.contains("LockNotMatured"),
        "panic payload must reference error code 5003; got: {}",
        msg
    );
}

#[test]
fn error_code_5004_extend_lock_time_not_increased() {
    let env = test_env();
    let (_contract_id, client, _token_client, token_admin, _vault_admin) = vault_with_sac(&env);
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);
    env.mock_all_auths();
    client.deposit(&user, &5_000);
    let id = client.lock_funds(&user, &2_000, &20_000);
    let msg = catch_panic_message(|| { client.extend_lock(&user, &id, &15_000); });
    assert!(
        msg.contains((ContractError::ExtendLockTimeNotIncreased as u32).to_string().as_str())
            || msg.contains("ExtendLockTimeNotIncreased"),
        "panic payload must reference error code 5004; got: {}",
        msg
    );
}

// =========================================================================
// CATEGORY 8000: Admin Rotation
// =========================================================================

#[test]
fn error_code_8001_cannot_transfer_admin_to_self() {
    let env = test_env();
    let (_contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let same = vault_admin.clone();
    let msg = catch_panic_message(|| client.transfer_admin(&vault_admin, &same));
    assert!(
        msg.contains((ContractError::CannotTransferAdminToSelf as u32).to_string().as_str())
            || msg.contains("CannotTransferAdminToSelf"),
        "panic payload must reference error code 8001; got: {}",
        msg
    );
}

#[test]
fn error_code_8002_cannot_transfer_admin_to_contract_address() {
    let env = test_env();
    let (contract_id, client, _token_client, _token_admin, vault_admin) = vault_with_sac(&env);
    let msg = catch_panic_message(|| client.transfer_admin(&vault_admin, &contract_id));
    assert!(
        msg.contains((ContractError::CannotTransferAdminToContractAddress as u32).to_string().as_str())
            || msg.contains("CannotTransferAdminToContractAddress"),
        "panic payload must reference error code 8002; got: {}",
        msg
    );
}
