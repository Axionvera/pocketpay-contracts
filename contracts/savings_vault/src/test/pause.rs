use super::*;
use soroban_sdk::{testutils::Address as _, Address};

/// Helper: initialize contract and return (env, admin, contract_id, client, token_admin).
fn setup_with_admin() -> (
    Env,
    Address,
    Address,
    SavingsVaultClient<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = {
        let issuer = Address::generate(&env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    let token_admin = token::StellarAssetClient::new(&env, &token);
    client.initialize(&admin, &token);

    (env, admin, contract_id, client, token_admin)
}

// =========================================================================
// Basic Pause / Unpause
// =========================================================================

#[test]
fn test_admin_can_pause() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);
    client.pause(&admin, &600);
    assert!(client.is_paused());
}

#[test]
fn test_admin_can_unpause() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);
    client.pause(&admin, &600);
    assert!(client.is_paused());

    client.unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
fn test_pause_sets_correct_expiry() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);
    client.pause(&admin, &600);

    // 100 seconds later — still paused (expiry = 1600)
    set_ledger_timestamp(&env, 1_100);
    assert!(client.is_paused());

    // Exactly at expiry — is_paused returns false (expired)
    set_ledger_timestamp(&env, 1_600);
    assert!(!client.is_paused());
}

// =========================================================================
// Deposits Blocked During Pause
// =========================================================================

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_deposit_blocked_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.pause(&admin, &600);

    client.deposit(&user, &100);
}

// =========================================================================
// Locks Blocked During Pause
// =========================================================================

#[test]
#[should_panic(expected = "Contract is paused")]
fn test_lock_blocked_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);

    client.pause(&admin, &600);

    client.lock_funds(&user, &200, &2_000);
}

// =========================================================================
// Withdrawals Allowed During Pause
// =========================================================================

#[test]
fn test_withdraw_allowed_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);

    client.pause(&admin, &600);
    assert!(client.is_paused());

    // Withdrawal should succeed even during pause
    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

#[test]
fn test_withdraw_lock_allowed_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    let lock_id = client.lock_funds(&user, &200, &2_000);

    client.pause(&admin, &600);

    // Advance past unlock time
    set_ledger_timestamp(&env, 2_000);

    // withdraw_lock should succeed during pause
    client.withdraw_lock(&user, &lock_id);
}

// =========================================================================
// Read-Only Queries Unaffected
// =========================================================================

#[test]
fn test_read_queries_work_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &5_000);

    client.pause(&admin, &600);

    // All read queries must still work
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);
    assert!(!client.can_withdraw(&user));
    assert_eq!(
        client.get_version(),
        soroban_sdk::String::from_str(&env, "0.1.0")
    );
}

// =========================================================================
// Auto-Unpause on Expiry
// =========================================================================

#[test]
fn test_auto_unpause_on_expiry() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);

    client.pause(&admin, &600);
    assert!(client.is_paused());

    // Deposit is blocked during pause
    let result = client.try_deposit(&user, &100);
    assert!(result.is_err());

    // Advance past expiry
    set_ledger_timestamp(&env, 1_600);

    // is_paused now returns false
    assert!(!client.is_paused());

    // Deposit succeeds again — the pause was auto-cleared by require_not_paused
    client.deposit(&user, &100);
    assert_eq!(client.get_balance(&user), 600);
}

#[test]
fn test_auto_unpause_clears_storage() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);

    client.pause(&admin, &600);

    // Advance past expiry
    set_ledger_timestamp(&env, 1_600);

    // Trigger require_not_paused via a deposit
    client.deposit(&user, &100);

    // Verify storage was actually cleared
    let paused: bool = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&super::DataKey::Paused)
            .unwrap_or(false)
    });
    assert!(!paused, "Paused flag should be cleared after auto-unpause");

    let expiry: u64 = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get(&super::DataKey::PauseExpiry)
            .unwrap_or(0)
    });
    assert_eq!(
        expiry, 0,
        "PauseExpiry should be cleared after auto-unpause"
    );
}

// =========================================================================
// Authorization
// =========================================================================

#[test]
#[should_panic(expected = "Not authorized")]
fn test_pause_requires_admin() {
    let (env, _admin, _contract_id, client, _token_admin) = setup_with_admin();
    let random_user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    client.pause(&random_user, &600);
}

#[test]
#[should_panic(expected = "Not authorized")]
fn test_unpause_requires_admin() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);
    client.pause(&admin, &600);

    let random_user = Address::generate(&env);
    client.unpause(&random_user);
}

#[test]
#[should_panic]
fn test_pause_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = {
        let issuer = Address::generate(&env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.mock_all_auths().initialize(&admin, &token);

    set_ledger_timestamp(&env, 1_000);

    // Call without auth mocking: require_auth() must reject this pause.
    client.pause(&admin, &600);
}

#[test]
#[should_panic]
fn test_unpause_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = {
        let issuer = Address::generate(&env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.mock_all_auths().initialize(&admin, &token);

    set_ledger_timestamp(&env, 1_000);
    client.mock_all_auths().pause(&admin, &600);

    // Call without auth mocking: require_auth() must reject this unpause.
    client.unpause(&admin);
}

// =========================================================================
// Pause Duration Validation
// =========================================================================

#[test]
#[should_panic(expected = "Pause duration must be greater than zero")]
fn test_pause_zero_duration_panics() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);
    client.pause(&admin, &0);
}

// =========================================================================
// Deposit After Unpause Resumes
// =========================================================================

#[test]
fn test_deposit_resumes_after_unpause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &100);

    client.pause(&admin, &600);

    // Deposit blocked
    let result = client.try_deposit(&user, &100);
    assert!(result.is_err());

    // Unpause
    client.unpause(&admin);

    // Deposit succeeds
    client.deposit(&user, &100);
    assert_eq!(client.get_balance(&user), 200);
}

// =========================================================================
// Lock Resumes After Unpause
// =========================================================================

#[test]
fn test_lock_resumes_after_unpause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);

    client.pause(&admin, &600);

    // Lock blocked
    let result = client.try_lock_funds(&user, &200, &2_000);
    assert!(result.is_err());

    // Unpause
    client.unpause(&admin);

    // Lock succeeds
    client.lock_funds(&user, &200, &2_000);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(client.get_locked_balance(&user), 200);
}

// =========================================================================
// Event Emissions
// =========================================================================

#[test]
fn test_pause_emits_event() {
    use soroban_sdk::{symbol_short, TryIntoVal};

    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);

    client.pause(&admin, &600);

    let events = env.events().all();
    let (_contract, topics, data) = events.get(events.len() - 1).unwrap();
    let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let expiry: u64 = data.try_into_val(&env).unwrap();
    assert_eq!(topic0, symbol_short!("pause"));
    assert_eq!(topic1, admin);
    assert_eq!(expiry, 1_600);
}

#[test]
fn test_unpause_emits_event() {
    use soroban_sdk::{symbol_short, TryIntoVal};

    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);

    client.pause(&admin, &600);
    client.unpause(&admin);

    let events = env.events().all();
    let (_contract, topics, _data) = events.get(events.len() - 1).unwrap();
    let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic0, symbol_short!("unpause"));
    assert_eq!(topic1, admin);
}

// =========================================================================
// is_paused Before Initialization
// =========================================================================

#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_is_paused_uninitialized_panics() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    client.is_paused();
}

// =========================================================================
// Pause Does Not Affect Existing Locked Funds
// =========================================================================

#[test]
fn test_locked_funds_mature_normally_during_pause() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);
    client.deposit(&user, &500);
    client.lock_funds(&user, &200, &2_000);

    client.pause(&admin, &600);

    // Lock maturity is time-based, not pause-based
    set_ledger_timestamp(&env, 2_000);

    // The lock has matured — get_balance includes it, can_withdraw returns true
    assert_eq!(client.get_balance(&user), 500);
    assert!(client.can_withdraw(&user));

    // Withdrawal of matured funds works during pause
    client.withdraw(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
}

// =========================================================================
// Double Pause Is Allowed (Refreshes Expiry)
// =========================================================================

#[test]
fn test_double_pause_refreshes_expiry() {
    let (env, admin, _contract_id, client, token_admin) = setup_with_admin();
    let user = Address::generate(&env);
    set_ledger_timestamp(&env, 1_000);

    token_admin.mint(&user, &1_000);

    // First pause: 600 seconds (expires at 1600)
    client.pause(&admin, &600);
    assert!(client.is_paused());

    // Advance to T=1500 — still paused
    set_ledger_timestamp(&env, 1_500);
    assert!(client.is_paused());

    // Second pause: 300 seconds from T=1500 (new expiry = 1800)
    client.pause(&admin, &300);

    // At T=1600 — originally expired, but the refresh extends to 1800
    set_ledger_timestamp(&env, 1_600);
    assert!(client.is_paused());

    // At T=1800 — now expired
    set_ledger_timestamp(&env, 1_800);
    assert!(!client.is_paused());
}

// =========================================================================
// Unpause When Already Unpaused Is No-Op
// =========================================================================

#[test]
fn test_unpause_when_not_paused_is_noop() {
    let (env, admin, _contract_id, client, _token_admin) = setup_with_admin();
    set_ledger_timestamp(&env, 1_000);

    // Contract is not paused — unpause should not panic
    client.unpause(&admin);
    assert!(!client.is_paused());
}
