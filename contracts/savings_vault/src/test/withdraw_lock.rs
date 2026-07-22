use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

#[test]
fn test_withdraw_matured_lock_success() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    // Initial state
    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    // Lock funds
    let lock_id = client.lock_funds(&user, &400, &2000);
    assert_eq!(client.get_balance(&user), 600);
    assert_eq!(client.get_locked_balance(&user), 400);

    // Advance time to maturity
    set_ledger_timestamp(&env, 2000);

    // Withdraw lock
    client.withdraw_lock(&user, &lock_id);

    // Balance checks
    assert_eq!(client.get_balance(&user), 600);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(token_client.balance(&user), 400);
}

#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_withdraw_immature_lock_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2000);

    // Try to withdraw before maturity
    set_ledger_timestamp(&env, 1500);
    client.withdraw_lock(&user, &lock_id);
}

#[test]
#[should_panic(expected = "Lock not found")]
fn test_withdraw_nonexistent_lock_fails() {
    let (env, contract_id, client) = setup();
    let user = new_user(&env);

    // Try to withdraw a random lock ID
    client.withdraw_lock(&user, &999);
}

#[test]
#[should_panic(expected = "Lock not found")]
fn test_withdraw_repeated_lock_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &1000);

    let lock_id = client.lock_funds(&user, &400, &2000);

    // Advance time to maturity
    set_ledger_timestamp(&env, 2000);

    // Withdraw once
    client.withdraw_lock(&user, &lock_id);

    // Try to withdraw again
    client.withdraw_lock(&user, &lock_id);
}

#[test]
#[should_panic(expected = "Lock not found")]
fn test_withdraw_wrong_user_lock_fails() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) = test_token(env, contract_id, client);
    let user_a = new_user(&env);
    let user_b = new_user(&env);

    set_ledger_timestamp(&env, 1000);
    token_admin.mint(&user_a, &1000);
    client.deposit(&user_a, &1000);

    let lock_id = client.lock_funds(&user_a, &400, &2000);

    // Advance time to maturity
    set_ledger_timestamp(&env, 2000);

    // User B tries to withdraw User A's lock
    client.withdraw_lock(&user_b, &lock_id);
}

#[test]
#[should_panic]
fn test_unauthorized_withdraw_lock_fails() {
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
    client.mock_all_auths().lock_funds(&user, &50, &2_000);
    
    set_ledger_timestamp(&env, 3_000);
    
    // Call without auth mocking: require_auth() must reject this withdrawal.
    client.withdraw_lock(&user, &1);
}
