use crate::test::test_helpers::*;
use crate::{SavingsVault, SavingsVaultClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Successful initialisation
// ---------------------------------------------------------------------------

/// The first initialisation with valid admin and token addresses must
/// succeed without panicking and must persist both values.
#[test]
fn test_initialize_success() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Should not panic.
    client.initialize(&admin, &token);
}

/// The token address passed to `initialize` must be exactly the value
/// returned by `get_token`. This verifies it is stored correctly.
#[test]
fn test_token_is_stored_and_returned_correctly() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    // get_token must return the exact address that was passed in.
    assert_eq!(
        client.get_token(),
        token,
        "stored token address must match the one passed to initialize"
    );
}

/// Initialising with a different token address on a fresh contract also
/// stores and returns that specific address — the storage key is not
/// hard-coded to a particular value.
#[test]
fn test_different_token_addresses_are_each_stored_correctly() {
    // First contract instance.
    let env1 = test_env();
    let contract_id1 = env1.register(SavingsVault, ());
    let client1 = SavingsVaultClient::new(&env1, &contract_id1);
    let admin1 = Address::generate(&env1);
    let token1 = Address::generate(&env1);
    client1.initialize(&admin1, &token1);
    assert_eq!(client1.get_token(), token1);

    // Second, independent contract instance with a different token.
    let env2 = test_env();
    let contract_id2 = env2.register(SavingsVault, ());
    let client2 = SavingsVaultClient::new(&env2, &contract_id2);
    let admin2 = Address::generate(&env2);
    let token2 = Address::generate(&env2);
    client2.initialize(&admin2, &token2);
    assert_eq!(client2.get_token(), token2);

    // The two instances are fully independent.
    assert_ne!(
        token1, token2,
        "test setup must use distinct token addresses"
    );
}

// ---------------------------------------------------------------------------
// get_token read helper
// ---------------------------------------------------------------------------

/// `get_token` returns the configured token address after initialisation.
#[test]
fn test_get_token_after_initialization() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    assert_eq!(client.get_token(), token);
}

/// `get_token` must panic before the contract is initialised.
#[test]
#[should_panic]
fn test_get_token_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    client.get_token();
}

// ---------------------------------------------------------------------------
// Repeated-initialisation guard
// ---------------------------------------------------------------------------

/// The second call to `initialize` must panic regardless of the arguments.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_twice_panics() {
    let env = test_env();
    // init_contract registers and initializes with a generated admin + token.
    let (_id, client) = init_contract(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    // Second call must be rejected.
    client.initialize(&admin, &token);
}

/// Even passing the same admin and token on the second call must be rejected —
/// the guard fires unconditionally.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_initialize_same_params_twice_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);
    // Repeating with identical arguments must still panic.
    client.initialize(&admin, &token);
}

/// An attacker cannot overwrite the admin by calling `initialize` again
/// with a different admin address.
#[test]
#[should_panic(expected = "Contract is already initialized")]
fn test_reinitialize_with_different_admin_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    // Attacker's address must not replace the stored admin.
    let attacker = Address::generate(&env);
    client.initialize(&attacker, &token);
}

/// After a rejected second initialisation the token address must not change.
#[test]
fn test_token_unchanged_after_rejected_reinitialisation() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    // Attempt a second initialisation with a different token (should fail).
    let result = client.try_initialize(&admin, &Address::generate(&env));
    assert!(
        result.is_err(),
        "second initialisation must be rejected"
    );

    // Original token must still be intact.
    assert_eq!(
        client.get_token(),
        token,
        "token address must not change after a rejected re-initialisation"
    );
}

// ---------------------------------------------------------------------------
// Pre-initialisation guards for all entry points
// ---------------------------------------------------------------------------

/// `deposit` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_deposit_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.deposit(&user, &100);
}

/// `withdraw` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_withdraw_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.withdraw(&user, &100);
}

/// `lock_funds` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_lock_funds_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.lock_funds(&user, &100, &1000);
}

/// `get_balance` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_get_balance_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.get_balance(&user);
}

/// `get_locked_balance` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_get_locked_balance_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.get_locked_balance(&user);
}

/// `can_withdraw` panics when the contract has not been initialised.
#[test]
#[should_panic]
fn test_can_withdraw_before_initialization_panics() {
    let env = test_env();
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);
    client.can_withdraw(&user);
}
