//! Test suite for unsafe and unauthorized admin rotation attempts.
//!
//! Verifies authorization boundaries, invalid input guards, repeated rotation chains,
//! event emission, and security revocation rules for `transfer_admin`.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Events, symbol_short, Address, Env, Symbol};
use test_helpers::*;

/// Read the current admin address directly from contract instance storage.
fn read_stored_admin(env: &Env, contract_id: &Address) -> Address {
    env.as_contract(contract_id, || {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    })
}

// =========================================================================
// 1. Wrong-Caller & Authorization Boundary Tests
// =========================================================================

/// An unauthorized caller (non-admin) attempting to transfer admin role is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_transfer_admin_unauthorized_caller_panics() {
    let env = test_env();
    let (_contract_id, client) = init_contract(&env);

    let attacker = new_user(&env);
    let new_admin = new_user(&env);

    // Non-admin caller passing their own address must panic with "Not authorized"
    client.transfer_admin(&attacker, &new_admin);
}

/// A revoked old admin attempting to transfer admin control after a successful rotation is rejected.
#[test]
#[should_panic(expected = "Not authorized")]
fn test_transfer_admin_revoked_old_admin_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_stored_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    // Verify state updated
    assert_eq!(client.get_admin(), new_admin);

    // Original admin trying to rotate again must fail
    let third_party = new_user(&env);
    client.transfer_admin(&original_admin, &third_party);
}

/// Admin rotation requires cryptographic signature authorization (`require_auth`).
#[test]
#[should_panic]
fn test_transfer_admin_requires_signature_fails() {
    let env = Env::default(); // Note: mock_all_auths() is intentionally omitted
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

    // Initialize with mock auth enabled for setup only
    env.mock_all_auths();
    client.initialize(&admin, &token);

    // Re-create un-mocked env context or call without auth
    let unauth_env = Env::default();
    let unauth_contract = unauth_env.register(SavingsVault, ());
    let unauth_client = SavingsVaultClient::new(&unauth_env, &unauth_contract);
    let new_admin = Address::generate(&unauth_env);

    unauth_client.transfer_admin(&admin, &new_admin);
}

/// Calling `transfer_admin` before contract initialization is rejected.
#[test]
#[should_panic(expected = "Contract is not initialized")]
fn test_transfer_admin_uninitialized_contract_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    let caller = new_user(&env);
    let new_admin = new_user(&env);

    client.transfer_admin(&caller, &new_admin);
}

/// Revoked old admin cannot invoke pause or unpause functions after rotation.
#[test]
fn test_revoked_admin_cannot_pause_or_unpause() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let original_admin = read_stored_admin(&env, &contract_id);

    let new_admin = new_user(&env);
    client.transfer_admin(&original_admin, &new_admin);

    // Verify original admin cannot pause
    let res_pause = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.pause(&original_admin, &3600);
    }));
    assert!(res_pause.is_err(), "Old admin must not be able to pause contract");

    // New admin pauses contract
    client.pause(&new_admin, &3600);
    assert!(client.is_paused());

    // Verify original admin cannot unpause
    let res_unpause = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.unpause(&original_admin);
    }));
    assert!(res_unpause.is_err(), "Old admin must not be able to unpause contract");

    // New admin can unpause
    client.unpause(&new_admin);
    assert!(!client.is_paused());
}

// =========================================================================
// 2. Invalid Admin Input Tests
// =========================================================================

/// Attempting to transfer admin role to the existing admin (self-rotation) is rejected.
#[test]
#[should_panic(expected = "Invalid new admin: cannot transfer to self")]
fn test_transfer_admin_to_self_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let admin = read_stored_admin(&env, &contract_id);

    // Transferring admin to the current admin address must be rejected
    client.transfer_admin(&admin, &admin);
}

/// Attempting to transfer admin role to the vault contract address is rejected.
#[test]
#[should_panic(expected = "Invalid new admin: cannot set contract address as admin")]
fn test_transfer_admin_to_contract_address_panics() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let admin = read_stored_admin(&env, &contract_id);

    // Setting contract's own address as admin must be rejected
    client.transfer_admin(&admin, &contract_id);
}

// =========================================================================
// 3. Repeated & Cyclic Rotation Tests
// =========================================================================

/// Verifies a multi-step chain of admin rotations (Admin A -> B -> C -> D).
#[test]
fn test_repeated_admin_rotation_chain() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);

    let admin_a = read_stored_admin(&env, &contract_id);
    let admin_b = new_user(&env);
    let admin_c = new_user(&env);
    let admin_d = new_user(&env);

    // Step 1: A -> B
    client.transfer_admin(&admin_a, &admin_b);
    assert_eq!(client.get_admin(), admin_b);

    // Step 2: B -> C
    client.transfer_admin(&admin_b, &admin_c);
    assert_eq!(client.get_admin(), admin_c);

    // Step 3: C -> D
    client.transfer_admin(&admin_c, &admin_d);
    assert_eq!(client.get_admin(), admin_d);

    // Assert prior admins (A, B, C) are all revoked and cannot rotate
    let res_a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.transfer_admin(&admin_a, &new_user(&env))));
    assert!(res_a.is_err());

    let res_b = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.transfer_admin(&admin_b, &new_user(&env))));
    assert!(res_b.is_err());

    let res_c = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.transfer_admin(&admin_c, &new_user(&env))));
    assert!(res_c.is_err());

    // Only Admin D can perform operations
    client.pause(&admin_d, &1000);
    assert!(client.is_paused());
}

/// Verifies cyclic rotation (Admin A -> Admin B -> Admin A).
#[test]
fn test_cyclic_admin_rotation() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);

    let admin_a = read_stored_admin(&env, &contract_id);
    let admin_b = new_user(&env);

    // Step 1: A transfers to B
    client.transfer_admin(&admin_a, &admin_b);
    assert_eq!(client.get_admin(), admin_b);

    // Step 2: B transfers back to A
    client.transfer_admin(&admin_b, &admin_a);
    assert_eq!(client.get_admin(), admin_a);

    // Step 3: Verify Admin A regains power and Admin B is revoked
    client.pause(&admin_a, &500);
    assert!(client.is_paused());

    let res_b_pause = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| client.unpause(&admin_b)));
    assert!(res_b_pause.is_err(), "Admin B must be revoked after transferring back to A");
}

// =========================================================================
// 4. Event Schema Verification
// =========================================================================

/// Verifies that `transfer_admin` emits the expected event schema.
#[test]
fn test_transfer_admin_emits_event_schema() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);
    let admin_a = read_stored_admin(&env, &contract_id);
    let admin_b = new_user(&env);

    client.transfer_admin(&admin_a, &admin_b);

    let events = env.events().all();
    let event = events.last().expect("event must be emitted on transfer_admin");

    // Verify contract address
    assert_eq!(event.0, contract_id);

    use soroban_sdk::IntoVal;
    let topic_symbol: Symbol = event.1.get_unchecked(0).into_val(&env);
    let topic_admin: Address = event.1.get_unchecked(1).into_val(&env);

    assert_eq!(topic_symbol, symbol_short!("xferadmin"));
    assert_eq!(topic_admin, admin_a);

    // Verify data payload: new_admin
    let payload_admin: Address = event.2.into_val(&env);
    assert_eq!(payload_admin, admin_b);
}

