use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, token, Address, Env};

#[test]
fn cancel_lock_fails_closed_without_mutating_active_lock() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, token_client, token_admin) =
        test_token(env, contract_id.clone(), client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &1_000);
    let lock_id = client.lock_funds(&user, &400, &2_000);

    let before_lock = client.get_lock(&user, &lock_id).expect("lock exists");
    let before_balance = client.get_balance(&user);
    let before_locked = client.get_locked_balance(&user);
    let before_user_tokens = token_client.balance(&user);
    let before_vault_tokens = token_client.balance(&contract_id);

    let result = client.try_cancel_lock(&user, &lock_id);
    assert!(result.is_err(), "cancellation must fail closed");

    assert_eq!(client.get_lock(&user, &lock_id).expect("lock remains"), before_lock);
    assert_eq!(client.get_balance(&user), before_balance);
    assert_eq!(client.get_locked_balance(&user), before_locked);
    assert_eq!(token_client.balance(&user), before_user_tokens);
    assert_eq!(token_client.balance(&contract_id), before_vault_tokens);
}

#[test]
fn cancel_lock_remains_unsupported_after_maturity() {
    let (env, contract_id, client) = setup();
    let (env, _admin, client, _token_client, token_admin) =
        test_token(env, contract_id, client);
    let user = new_user(&env);

    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    client.deposit(&user, &1_000);
    let lock_id = client.lock_funds(&user, &400, &2_000);
    let before = client.get_lock(&user, &lock_id).expect("lock exists");

    set_ledger_timestamp(&env, 2_000);
    assert!(client.can_withdraw(&user));

    let result = client.try_cancel_lock(&user, &lock_id);
    assert!(result.is_err(), "maturity must not turn cancellation into withdrawal");
    assert_eq!(client.get_lock(&user, &lock_id).expect("lock remains"), before);
    assert!(client.can_withdraw(&user), "normal matured-withdrawal path remains available");
}

#[test]
#[should_panic]
fn cancel_lock_requires_owner_authorization() {
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
    client.mock_all_auths().deposit(&user, &1_000);
    set_ledger_timestamp(&env, 1_000);
    let lock_id = client.mock_all_auths().lock_funds(&user, &400, &2_000);

    // No auth mocking here: user.require_auth() must reject the call before
    // the explicit unsupported-cancellation policy is evaluated.
    client.cancel_lock(&user, &lock_id);
}

#[test]
fn cancel_lock_rejects_unknown_lock_without_creating_state() {
    let (env, contract_id, client) = setup();
    let (_env, _admin, client, _token_client, _token_admin) =
        test_token(env, contract_id, client);
    let user = new_user(&client.env);

    let result = client.try_cancel_lock(&user, &999);
    assert!(result.is_err(), "unknown lock id must fail");
    assert!(client.get_lock(&user, &999).is_none());
}
