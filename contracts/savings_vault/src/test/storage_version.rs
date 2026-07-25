use crate::test::test_helpers::*;
use crate::{DataKey, SavingsVault, SavingsVaultClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_initialize_sets_storage_version_1() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);

    let stored_version: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap_or(0)
    });

    assert_eq!(stored_version, 1);
}

#[test]
fn test_legacy_missing_storage_version_works() {
    let env = test_env();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::PauseExpiry, &0_u64);
    });

    let version = client.get_version();
    assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));

    let stored_version: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap_or(0)
    });
    assert_eq!(stored_version, 1);
}

#[test]
#[should_panic(expected = "Unsupported storage version")]
fn test_invalid_storage_version_fails_safely() {
    let env = test_env();
    let (contract_id, client) = init_contract(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::StorageVersion, &2_u64);
    });

    client.get_version();
}
