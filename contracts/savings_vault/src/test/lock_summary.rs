use super::*;

// =========================================================================
// get_lock_summary — Empty / New User
// =========================================================================

#[test]
fn test_lock_summary_empty_user() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 0);
    assert_eq!(summary.total_locked_amount, 0);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 0);
    assert_eq!(summary.latest_unlock, 0);
}

// =========================================================================
// get_lock_summary — Deposit Only (no locks)
// =========================================================================

#[test]
fn test_lock_summary_deposit_only() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);

    deposit_balance(&client, &user, 500);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 0);
    assert_eq!(summary.total_locked_amount, 0);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 0);
    assert_eq!(summary.latest_unlock, 0);
}

// =========================================================================
// get_lock_summary — Single Active Lock
// =========================================================================

#[test]
fn test_lock_summary_single_active_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 1);
    assert_eq!(summary.total_locked_amount, 200);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 5_000);
    assert_eq!(summary.latest_unlock, 5_000);
}

// =========================================================================
// get_lock_summary — Multiple Active Locks
// =========================================================================

#[test]
fn test_lock_summary_multiple_active_locks() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &3_000);
    client.lock_funds(&user, &200, &8_000);
    client.lock_funds(&user, &100, &5_000);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 3);
    assert_eq!(summary.total_locked_amount, 600);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 3_000);
    assert_eq!(summary.latest_unlock, 8_000);
}

// =========================================================================
// get_lock_summary — Matured Locks
// =========================================================================

#[test]
fn test_lock_summary_matured_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &3_000);

    // Advance past unlock time
    set_ledger_timestamp(&env, 3_000);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 1);
    assert_eq!(summary.total_locked_amount, 200);
    assert_eq!(summary.matured_count, 1);
    assert_eq!(summary.withdrawable_amount, 200);
    // No immature locks, so earliest/latest should be 0
    assert_eq!(summary.earliest_unlock, 0);
    assert_eq!(summary.latest_unlock, 0);
}

// =========================================================================
// get_lock_summary — Mixed (matured + immature)
// =========================================================================

#[test]
fn test_lock_summary_mixed_matured_and_immature() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &3_000); // matures at 3_000
    client.lock_funds(&user, &200, &8_000); // matures at 8_000
    client.lock_funds(&user, &100, &5_000); // matures at 5_000

    // Advance: first and third locks mature, second still active
    set_ledger_timestamp(&env, 5_000);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 3);
    assert_eq!(summary.total_locked_amount, 600);
    assert_eq!(summary.matured_count, 2);
    assert_eq!(summary.withdrawable_amount, 400);
    // Only lock 2 (unlock_time=8_000) is immature
    assert_eq!(summary.earliest_unlock, 8_000);
    assert_eq!(summary.latest_unlock, 8_000);
}

// =========================================================================
// get_lock_summary — After Withdrawing a Lock
// =========================================================================

#[test]
fn test_lock_summary_after_withdraw_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    let lock_id_1 = client.lock_funds(&user, &300, &3_000);
    client.lock_funds(&user, &200, &8_000);

    // Mature lock 1 and withdraw it
    set_ledger_timestamp(&env, 3_000);
    client.withdraw_lock(&user, &lock_id_1);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 1);
    assert_eq!(summary.total_locked_amount, 200);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 8_000);
    assert_eq!(summary.latest_unlock, 8_000);
}

// =========================================================================
// get_lock_summary — Cross-User Isolation
// =========================================================================

#[test]
fn test_lock_summary_user_isolation() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let alice = new_user(&env);
    let bob = new_user(&env);
    token_admin.mint(&alice, &5000);
    token_admin.mint(&bob, &5000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &alice, 1_000);
    client.lock_funds(&alice, &400, &5_000);
    client.lock_funds(&alice, &100, &6_000);

    deposit_balance(&client, &bob, 500);
    client.lock_funds(&bob, &200, &4_000);

    let alice_summary = client.get_lock_summary(&alice);
    let bob_summary = client.get_lock_summary(&bob);

    assert_eq!(alice_summary.active_count, 2);
    assert_eq!(alice_summary.total_locked_amount, 500);
    assert_eq!(alice_summary.earliest_unlock, 5_000);
    assert_eq!(alice_summary.latest_unlock, 6_000);

    assert_eq!(bob_summary.active_count, 1);
    assert_eq!(bob_summary.total_locked_amount, 200);
    assert_eq!(bob_summary.earliest_unlock, 4_000);
    assert_eq!(bob_summary.latest_unlock, 4_000);
}

// =========================================================================
// get_lock_summary — All Locks Matured
// =========================================================================

#[test]
fn test_lock_summary_all_matured() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &3_000);
    client.lock_funds(&user, &200, &4_000);

    set_ledger_timestamp(&env, 4_000);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 2);
    assert_eq!(summary.total_locked_amount, 500);
    assert_eq!(summary.matured_count, 2);
    assert_eq!(summary.withdrawable_amount, 500);
    assert_eq!(summary.earliest_unlock, 0);
    assert_eq!(summary.latest_unlock, 0);
}

// =========================================================================
// get_lock_summary — All Locks Withdrawn
// =========================================================================

#[test]
fn test_lock_summary_all_withdrawn() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    let id1 = client.lock_funds(&user, &300, &3_000);
    let id2 = client.lock_funds(&user, &200, &4_000);

    set_ledger_timestamp(&env, 4_000);
    client.withdraw_lock(&user, &id1);
    client.withdraw_lock(&user, &id2);

    let summary = client.get_lock_summary(&user);
    assert_eq!(summary.active_count, 0);
    assert_eq!(summary.total_locked_amount, 0);
    assert_eq!(summary.matured_count, 0);
    assert_eq!(summary.withdrawable_amount, 0);
    assert_eq!(summary.earliest_unlock, 0);
    assert_eq!(summary.latest_unlock, 0);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_lock_summary_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_lock_summary(&user);
}
