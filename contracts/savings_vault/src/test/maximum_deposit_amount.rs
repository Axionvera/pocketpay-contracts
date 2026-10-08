//! Fund-safety regression coverage for configurable per-deposit ceilings (#452).
//!
//! The maximum bounds *each incoming transfer*, not cumulative holdings;
//! failed checks must happen before any SAC token movement.

use soroban_sdk::{testutils::Address as _, token, Address, Env};

fn fixture() -> (
    Env,
    Address,
    crate::SavingsVaultClient<'static>,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    let (vault, client, token_client, token_admin, admin) =
        super::test_helpers::vault_with_sac(&env);
    (env, vault, client, admin, token_client, token_admin)
}

#[test]
fn test_deposit_min_and_max_inclusive_with_rejected_transfer_atomicity() {
    let (env, vault, client, admin, token_client, token_admin) = fixture();
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);

    assert_eq!(client.get_max_deposit_amount(), 0);
    assert_eq!(client.get_config().max_deposit_amount, 0);

    client.set_min_deposit_amount(&admin, &100);
    client.set_max_deposit_amount(&admin, &200);
    assert_eq!(client.get_max_deposit_amount(), 200);
    assert_eq!(client.get_config().max_deposit_amount, 200);

    // Both configured bounds are inclusive.
    client.deposit(&user, &100);
    client.deposit(&user, &200);
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(token_client.balance(&vault), 300);

    let user_before = token_client.balance(&user);
    let vault_before = token_client.balance(&vault);
    assert!(client.try_deposit(&user, &99).is_err());
    assert!(client.try_deposit(&user, &201).is_err());

    // Rejected deposits cannot debit the user's SAC wallet or credit custody.
    assert_eq!(client.get_balance(&user), 300);
    assert_eq!(token_client.balance(&user), user_before);
    assert_eq!(token_client.balance(&vault), vault_before);
}

#[test]
fn test_maximum_is_per_deposit_and_can_be_disabled() {
    let (env, _vault, client, admin, _token_client, token_admin) = fixture();
    let user = Address::generate(&env);
    token_admin.mint(&user, &1_000);

    client.set_max_deposit_amount(&admin, &200);
    client.deposit(&user, &200);
    client.deposit(&user, &200);
    assert_eq!(client.get_balance(&user), 400);

    // Zero disables the ceiling; the separate positive-amount guard remains.
    client.set_max_deposit_amount(&admin, &0);
    assert_eq!(client.get_max_deposit_amount(), 0);
    client.deposit(&user, &500);
    assert_eq!(client.get_balance(&user), 900);
    assert!(client.try_deposit(&user, &0).is_err());
}

#[test]
fn test_conflicting_configuration_is_rejected_without_mutation() {
    let (_env, _vault, client, admin, _token_client, _token_admin) = fixture();

    client.set_min_deposit_amount(&admin, &100);
    client.set_max_deposit_amount(&admin, &200);

    assert!(client.try_set_max_deposit_amount(&admin, &99).is_err());
    assert!(client.try_set_min_deposit_amount(&admin, &201).is_err());

    let config = client.get_config();
    assert_eq!(config.min_deposit_amount, 100);
    assert_eq!(config.max_deposit_amount, 200);

    // Inclusive equality remains legal when the bounds meet exactly.
    client.set_max_deposit_amount(&admin, &100);
    assert_eq!(client.get_max_deposit_amount(), 100);
}

#[test]
fn test_negative_and_non_admin_max_updates_do_not_mutate_limit() {
    let (env, _vault, client, admin, _token_client, _token_admin) = fixture();
    let other = Address::generate(&env);

    client.set_max_deposit_amount(&admin, &250);
    assert!(client.try_set_max_deposit_amount(&other, &300).is_err());
    assert!(client.try_set_max_deposit_amount(&admin, &-1).is_err());
    assert_eq!(client.get_max_deposit_amount(), 250);
}
