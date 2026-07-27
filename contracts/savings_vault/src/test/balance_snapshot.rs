use super::*;

// =========================================================================
// get_balance_snapshot — Empty / New User
// =========================================================================

#[test]
fn test_balance_snapshot_empty_user() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let user = new_user(&env);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 0);
    assert_eq!(snap.locked, 0);
    assert_eq!(snap.total, 0);
    assert_eq!(snap.withdrawable, 0);
}

// =========================================================================
// get_balance_snapshot — Deposit Only (no locks)
// =========================================================================

#[test]
fn test_balance_snapshot_deposit_only() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);

    deposit_balance(&client, &user, 500);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 500);
    assert_eq!(snap.locked, 0);
    assert_eq!(snap.total, 500);
    assert_eq!(snap.withdrawable, 0);
}

// =========================================================================
// get_balance_snapshot — Active (immature) Lock
// =========================================================================

#[test]
fn test_balance_snapshot_with_active_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 300);
    assert_eq!(snap.locked, 200);
    assert_eq!(snap.total, 500);
    assert_eq!(snap.withdrawable, 0);
}

// =========================================================================
// get_balance_snapshot — Matured Lock
// =========================================================================

#[test]
fn test_balance_snapshot_matured_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &1000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 500);
    client.lock_funds(&user, &200, &5_000);

    // Advance past unlock time
    set_ledger_timestamp(&env, 5_000);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 300);
    assert_eq!(snap.locked, 200);
    assert_eq!(snap.total, 500);
    assert_eq!(snap.withdrawable, 200);
}

// =========================================================================
// get_balance_snapshot — Mixed (matured + immature)
// =========================================================================

#[test]
fn test_balance_snapshot_mixed_matured_and_immature() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &300, &3_000); // matures at 3_000
    client.lock_funds(&user, &200, &8_000); // matures at 8_000

    // Advance to 3_000: first lock matures, second still active
    set_ledger_timestamp(&env, 3_000);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 500);
    assert_eq!(snap.locked, 500);
    assert_eq!(snap.total, 1_000);
    assert_eq!(snap.withdrawable, 300);
}

// =========================================================================
// get_balance_snapshot — After Withdrawing a Lock
// =========================================================================

#[test]
fn test_balance_snapshot_after_withdraw_lock() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &2000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 1_000);
    let lock_id = client.lock_funds(&user, &400, &3_000);

    set_ledger_timestamp(&env, 3_000);
    client.withdraw_lock(&user, &lock_id);

    let snap = client.get_balance_snapshot(&user);
    assert_eq!(snap.unlocked, 600);
    assert_eq!(snap.locked, 0);
    assert_eq!(snap.total, 600);
    assert_eq!(snap.withdrawable, 0);
}

// =========================================================================
// get_balance_snapshot — Cross-User Isolation
// =========================================================================

#[test]
fn test_balance_snapshot_user_isolation() {
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

    deposit_balance(&client, &bob, 500);

    let alice_snap = client.get_balance_snapshot(&alice);
    let bob_snap = client.get_balance_snapshot(&bob);

    assert_eq!(alice_snap.unlocked, 600);
    assert_eq!(alice_snap.locked, 400);
    assert_eq!(alice_snap.total, 1_000);

    assert_eq!(bob_snap.unlocked, 500);
    assert_eq!(bob_snap.locked, 0);
    assert_eq!(bob_snap.total, 500);
    assert_eq!(bob_snap.withdrawable, 0);
}

// =========================================================================
// get_balance_snapshot — Consistency with Individual Queries
// =========================================================================

#[test]
fn test_balance_snapshot_consistent_with_individual_queries() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);
    let (env, _admin, client, _token_client, token_admin) = test_token(env, _contract_id, client);
    let user = new_user(&env);
    token_admin.mint(&user, &5000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &user, 2_000);
    client.lock_funds(&user, &500, &3_000);
    client.lock_funds(&user, &300, &6_000);

    set_ledger_timestamp(&env, 4_000);

    let snap = client.get_balance_snapshot(&user);
    let balance = client.get_balance(&user);
    let locked = client.get_locked_balance(&user);

    assert_eq!(snap.unlocked, balance);
    assert_eq!(snap.locked, locked);
    assert_eq!(snap.total, balance + locked);
}

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_balance_snapshot_uninitialized_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = new_user(&env);
    client.get_balance_snapshot(&user);
}
