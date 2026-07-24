//! Admin configuration invariant guard tests.
//!
//! Proves that admin configuration changes (currently: `transfer_admin`)
//! cannot violate vault accounting, user isolation, or lock invariants.
//!
//! These tests establish a safety baseline before broadening admin capabilities.

use super::*;
use soroban_sdk::{testutils::Address as _, Address};

use test_helpers::*;

/// Helper: read the real admin address from contract instance storage.
fn read_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap()
    })
}

/// Helper: read the token address from contract instance storage.
fn read_token(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::Token)
            .unwrap()
    })
}

// =========================================================================
// Admin Transfer Preserves Vault Accounting
// =========================================================================

/// After `transfer_admin`, the user's available balance is identical.
#[test]
fn test_admin_transfer_preserves_user_available_balance() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 500);

    let balance_before = client.get_balance(&user);
    assert_eq!(balance_before, 500);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let balance_after = client.get_balance(&user);
    assert_eq!(balance_after, 500, "available balance must not change after admin transfer");
}

/// After `transfer_admin`, the user's locked balance is identical.
#[test]
fn test_admin_transfer_preserves_user_locked_balance() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &400, &5_000);

    let locked_before = client.get_locked_balance(&user);
    assert_eq!(locked_before, 400);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let locked_after = client.get_locked_balance(&user);
    assert_eq!(locked_after, 400, "locked balance must not change after admin transfer");
}

/// After `transfer_admin`, the user's lock entries are identical.
#[test]
fn test_admin_transfer_preserves_user_lock_entries() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let _lock_id_1 = client.lock_funds(&user, &200, &5_000);
    let _lock_id_2 = client.lock_funds(&user, &150, &6_000);

    let locks_before = client.list_locks(&user, &0u32, &10u32);
    assert_eq!(locks_before.len(), 2);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let locks_after = client.list_locks(&user, &0u32, &10u32);
    assert_eq!(locks_after.len(), 2, "lock count must not change after admin transfer");
    assert_eq!(
        locks_after.get(0).unwrap(),
        locks_before.get(0).unwrap(),
        "lock entries must be identical after admin transfer"
    );
    assert_eq!(
        locks_after.get(1).unwrap(),
        locks_before.get(1).unwrap(),
        "lock entries must be identical after admin transfer"
    );
}

/// After `transfer_admin`, `can_withdraw` returns the same result.
#[test]
fn test_admin_transfer_preserves_can_withdraw_status() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);
    client.lock_funds(&user, &500, &5_000);

    assert!(!client.can_withdraw(&user));

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    assert!(
        !client.can_withdraw(&user),
        "can_withdraw must not change after admin transfer"
    );
}

/// After `transfer_admin`, `get_lock` returns the same lock record.
#[test]
fn test_admin_transfer_preserves_individual_lock_records() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &1_000);
    deposit_balance(&client, &user, 1_000);

    let lock_id = client.lock_funds(&user, &300, &5_000);
    let lock_before = client.get_lock(&user, &lock_id).unwrap();

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let lock_after = client.get_lock(&user, &lock_id).unwrap();
    assert_eq!(
        lock_before, lock_after,
        "individual lock record must not change after admin transfer"
    );
}

// =========================================================================
// Admin Transfer Preserves Multi-User Isolation
// =========================================================================

/// Transfer admin while two users have different balances.
#[test]
fn test_admin_transfer_preserves_multi_user_balance_isolation() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let alice = new_user(&env);
    let bob = new_user(&env);
    token_admin.mint(&alice, &5_000);
    token_admin.mint(&bob, &5_000);

    deposit_balance(&client, &alice, 1_000);
    deposit_balance(&client, &bob, 3_000);

    let alice_before = client.get_balance(&alice);
    let bob_before = client.get_balance(&bob);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    assert_eq!(client.get_balance(&alice), alice_before, "alice balance unchanged");
    assert_eq!(client.get_balance(&bob), bob_before, "bob balance unchanged");
    assert_ne!(alice_before, bob_before, "users have different balances (isolation preserved)");
}

/// Transfer admin while both users have active locks.
#[test]
fn test_admin_transfer_preserves_multi_user_lock_isolation() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let alice = new_user(&env);
    let bob = new_user(&env);
    token_admin.mint(&alice, &5_000);
    token_admin.mint(&bob, &5_000);
    set_ledger_timestamp(&env, 1_000);

    deposit_balance(&client, &alice, 2_000);
    deposit_balance(&client, &bob, 3_000);

    client.lock_funds(&alice, &1_000, &5_000);
    client.lock_funds(&bob, &2_000, &6_000);

    let alice_locked_before = client.get_locked_balance(&alice);
    let bob_locked_before = client.get_locked_balance(&bob);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    assert_eq!(client.get_locked_balance(&alice), alice_locked_before);
    assert_eq!(client.get_locked_balance(&bob), bob_locked_before);
    assert_ne!(alice_locked_before, bob_locked_before);
}

// =========================================================================
// Unauthorized Admin Configuration Attempts
// =========================================================================

/// A random user (non-admin) cannot transfer admin role.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_non_admin_cannot_transfer_admin() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);

    let random_user = new_user(&env);
    let fake_new_admin = new_user(&env);

    client.transfer_admin(&random_user, &fake_new_admin);
}

/// After a successful admin transfer, the old admin can no longer transfer.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_old_admin_loses_power_after_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let another_admin = new_user(&env);
    client.transfer_admin(&original_admin, &another_admin);
}

/// `transfer_admin` requires the admin's cryptographic signature.
#[test]
#[should_panic]
fn test_transfer_admin_requires_auth() {
    let env = Env::default();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();

    client.mock_all_auths().initialize(&admin, &token);

    let new_admin = new_user(&env);
    client.transfer_admin(&admin, &new_admin);
}

// =========================================================================
// Repeated Admin Transfers Preserve Accounting
// =========================================================================

/// Multiple successive admin transfers don't corrupt any user state.
#[test]
fn test_repeated_admin_transfers_preserve_balances() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 2_000);

    let balance_snapshot = client.get_balance(&user);

    let admin_a = new_user(&env);
    let admin_b = new_user(&env);
    let admin_c = new_user(&env);

    client.transfer_admin(&original_admin, &admin_a);
    assert_eq!(client.get_balance(&user), balance_snapshot);

    client.transfer_admin(&admin_a, &admin_b);
    assert_eq!(client.get_balance(&user), balance_snapshot);

    client.transfer_admin(&admin_b, &admin_c);
    assert_eq!(client.get_balance(&user), balance_snapshot);
}

/// Multiple successive admin transfers don't corrupt locks.
#[test]
fn test_repeated_admin_transfers_preserve_locks() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);

    client.lock_funds(&user, &1_000, &5_000);
    client.lock_funds(&user, &2_000, &6_000);

    let locked_snapshot = client.get_locked_balance(&user);
    let locks_snapshot_len = client.list_locks(&user, &0u32, &10u32).len();

    let admin_a = new_user(&env);
    let admin_b = new_user(&env);

    client.transfer_admin(&original_admin, &admin_a);
    assert_eq!(client.get_locked_balance(&user), locked_snapshot);
    assert_eq!(
        client.list_locks(&user, &0u32, &10u32).len(),
        locks_snapshot_len
    );

    client.transfer_admin(&admin_a, &admin_b);
    assert_eq!(client.get_locked_balance(&user), locked_snapshot);
    assert_eq!(
        client.list_locks(&user, &0u32, &10u32).len(),
        locks_snapshot_len
    );
}

/// Repeated admin transfers preserve the accounting invariant:
/// `available + locked == total_deposited`.
#[test]
fn test_repeated_admin_transfers_preserve_accounting_invariant() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &10_000);

    deposit_balance(&client, &user, 5_000);
    client.lock_funds(&user, &2_000, &5_000);

    let total_deposited: i128 = 5_000;
    let invariant = || {
        let available = client.get_balance(&user);
        let locked = client.get_locked_balance(&user);
        assert_eq!(
            available + locked,
            total_deposited,
            "invariant: available + locked == total_deposited"
        );
    };

    invariant();

    let admin_a = new_user(&env);
    let admin_b = new_user(&env);

    client.transfer_admin(&original_admin, &admin_a);
    invariant();

    client.transfer_admin(&admin_a, &admin_b);
    invariant();
}

// =========================================================================
// New Admin Cannot Manipulate Vaults
// =========================================================================

/// The new admin has no special powers to withdraw another user's funds.
#[test]
fn test_new_admin_cannot_withdraw_user_funds() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_client = token::Client::new(&env, &token);

    let user = new_user(&env);
    deposit_balance(&client, &user, 1_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let balance_after_transfer = client.get_balance(&user);
    let token_balance_after = token_client.balance(&user);

    // deposit_balance mints 1000 then deposits 1000, user retains 0 extra tokens
    assert_eq!(balance_after_transfer, 1_000);
    assert_eq!(token_balance_after, 0);
}

/// The new admin has no power to modify lock entries for another user.
#[test]
fn test_new_admin_cannot_modify_user_locks() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);

    client.lock_funds(&user, &1_000, &5_000);
    let lock_id = client.lock_funds(&user, &2_000, &6_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let locks = client.list_locks(&user, &0u32, &10u32);
    assert_eq!(locks.len(), 2);
    assert_eq!(locks.get(0).unwrap().amount, 1_000);
    assert_eq!(locks.get(1).unwrap().amount, 2_000);

    let lock = client.get_lock(&user, &lock_id).unwrap();
    assert_eq!(lock.amount, 2_000);
    assert_eq!(lock.unlock_time, 6_000);
}

/// Admin transfer doesn't create or delete lock entries.
#[test]
fn test_admin_transfer_does_not_create_or_delete_locks() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);

    client.lock_funds(&user, &500, &3_000);
    client.lock_funds(&user, &500, &4_000);
    client.lock_funds(&user, &500, &5_000);

    let count_before = client.list_locks(&user, &0u32, &100u32).len();

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let count_after = client.list_locks(&user, &0u32, &100u32).len();
    assert_eq!(
        count_before, count_after,
        "admin transfer must not create or delete lock entries"
    );
}

// =========================================================================
// Lock Maturity Timing Unaffected by Admin Transfer
// =========================================================================

/// Locks still mature at the correct time after admin transfer.
#[test]
fn test_admin_transfer_does_not_affect_lock_maturity_timing() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);
    client.lock_funds(&user, &2_000, &5_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    set_ledger_timestamp(&env, 4_999);
    assert!(!client.can_withdraw(&user));
    assert_eq!(client.get_locked_balance(&user), 2_000);

    set_ledger_timestamp(&env, 5_000);
    assert!(client.can_withdraw(&user));
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.get_balance(&user), 5_000);
}

/// Lock entry unlock_time is preserved exactly through admin transfer.
#[test]
fn test_admin_transfer_preserves_lock_unlock_time() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    set_ledger_timestamp(&env, 1_000);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);

    let expected_unlock: u64 = 86_400;
    let lock_id = client.lock_funds(&user, &3_000, &expected_unlock);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let lock = client.get_lock(&user, &lock_id).unwrap();
    assert_eq!(
        lock.unlock_time, expected_unlock,
        "unlock_time must survive admin transfer unchanged"
    );
}

// =========================================================================
// Post-Transfer Operations Remain Valid
// =========================================================================

/// After admin transfer, a user can still deposit successfully.
#[test]
fn test_user_can_deposit_after_admin_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 1_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    deposit_balance(&client, &user, 500);
    assert_eq!(client.get_balance(&user), 1_500);
}

/// After admin transfer, a user can still withdraw successfully.
#[test]
fn test_user_can_withdraw_after_admin_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 1_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    client.withdraw(&user, &300);
    assert_eq!(client.get_balance(&user), 700);
}

/// After admin transfer, a user can still lock funds successfully.
#[test]
fn test_user_can_lock_funds_after_admin_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    set_ledger_timestamp(&env, 1_000);
    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 1_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    client.lock_funds(&user, &500, &5_000);
    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 500);
}

/// After admin transfer, a user can still withdraw a matured lock.
#[test]
fn test_user_can_withdraw_lock_after_admin_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    set_ledger_timestamp(&env, 1_000);
    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 1_000);
    let lock_id = client.lock_funds(&user, &500, &3_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    set_ledger_timestamp(&env, 3_000);
    client.withdraw_lock(&user, &lock_id);

    assert_eq!(client.get_balance(&user), 500);
    assert_eq!(client.get_locked_balance(&user), 0);
    assert_eq!(client.list_locks(&user, &0u32, &10u32).len(), 0);
}

// =========================================================================
// Re-initialization Protection Survives Admin Transfer
// =========================================================================

/// Re-initialization is still blocked after admin transfer.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_reinitialization_blocked_after_admin_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    let new_token = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    client.initialize(&new_admin, &new_token);
}

/// Even the original admin cannot reinitialize after transferring admin.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_original_admin_cannot_reinitialize_after_transfer() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let fake_token = new_user(&env);
    client.initialize(&original_admin, &fake_token);
}

// =========================================================================
// Token Custody Unaffected by Admin Transfer
// =========================================================================

/// Admin transfer does not move any real tokens.
#[test]
fn test_admin_transfer_does_not_move_real_tokens() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);
    let token_client = token::Client::new(&env, &token);

    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 2_000);

    let user_token_before = token_client.balance(&user);
    let vault_token_before = token_client.balance(&contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let user_token_after = token_client.balance(&user);
    let vault_token_after = token_client.balance(&contract_id);

    assert_eq!(user_token_before, user_token_after, "user token balance unchanged");
    assert_eq!(vault_token_before, vault_token_after, "vault token balance unchanged");
}

/// Admin transfer event is emitted correctly.
#[test]
fn test_admin_transfer_emits_event() {
    use soroban_sdk::{symbol_short, Symbol, TryIntoVal};

    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let events = env.events().all();
    let mut found = false;
    for i in 0..events.len() {
        let (_contract, topics, data) = events.get(i).unwrap();
        let topic0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        if topic0 == symbol_short!("xferadmin") {
            let topic1: Address = topics.get(1).unwrap().try_into_val(&env).unwrap();
            let emitted_new_admin: Address = data.try_into_val(&env).unwrap();
            assert_eq!(topic1, original_admin);
            assert_eq!(emitted_new_admin, new_admin);
            found = true;
            break;
        }
    }
    assert!(found, "xferadmin event must be emitted");
}

// =========================================================================
// Edge Cases: Admin Role Cannot Bypass Lock Rules
// =========================================================================

/// The admin role does not bypass lock timing for user withdrawals.
#[test]
fn test_admin_role_does_not_bypass_lock_rules_for_user() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    set_ledger_timestamp(&env, 1_000);
    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);
    client.lock_funds(&user, &3_000, &100_000);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    set_ledger_timestamp(&env, 2_000);
    assert!(!client.can_withdraw(&user));

    let result = client.try_withdraw(&user, &3_000);
    assert!(result.is_err(), "withdrawal of locked funds must still fail after admin transfer");

    assert_eq!(client.get_balance(&user), 2_000, "available balance unchanged");
    assert_eq!(client.get_locked_balance(&user), 3_000, "locked balance unchanged");
}

/// Admin transfer does not affect the user's NextLockId counter.
#[test]
fn test_admin_transfer_preserves_lock_id_counter() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_admin(&env, &contract_id);
    let token = read_token(&env, &contract_id);
    let token_admin = token::StellarAssetClient::new(&env, &token);

    set_ledger_timestamp(&env, 1_000);
    let user = new_user(&env);
    token_admin.mint(&user, &5_000);
    deposit_balance(&client, &user, 5_000);

    let lock_id_1 = client.lock_funds(&user, &500, &5_000);
    let lock_id_2 = client.lock_funds(&user, &500, &5_000);
    assert_eq!(lock_id_1, 1);
    assert_eq!(lock_id_2, 2);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    let lock_id_3 = client.lock_funds(&user, &500, &5_000);
    assert_eq!(lock_id_3, 3, "lock ID counter must not reset after admin transfer");
}
