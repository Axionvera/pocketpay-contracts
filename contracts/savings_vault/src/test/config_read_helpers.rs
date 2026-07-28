//! Vault configuration read-helper tests (issue #353).
//!
//! The contract exposes read-only configuration helpers so SDK and mobile
//! clients can read the accepted token, admin, version, pause state, and the
//! configurable deposit/lock rules without exposing private operational data.
//! These tests verify each read helper returns the expected value.

use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn init_with_admin(env: &Env) -> (Address, crate::SavingsVaultClient<'static>, Address) {
    let contract_id = env.register(crate::SavingsVault, ());
    let client = crate::SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = {
        let issuer = Address::generate(env);
        env.register_stellar_asset_contract_v2(issuer).address()
    };
    client.initialize(&admin, &token);
    (admin, client, token)
}

/// The accepted token read helper returns the configured SAC address.
#[test]
fn test_get_token_returns_configured_token() {
    let env = test_env();
    let (_admin, client, token) = init_with_admin(&env);
    assert_eq!(client.get_token(), token);
}

/// The admin read helper returns the configured admin address.
#[test]
fn test_get_admin_returns_configured_admin() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);
    assert_eq!(client.get_admin(), admin);
}

/// The version read helper returns a non-empty semantic version string.
#[test]
fn test_get_version_returns_semver() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    let version = client.get_version();
    assert!(version.len() > 0, "version must be a non-empty string");
}

/// Pause state defaults to not-paused after initialization.
#[test]
fn test_is_paused_defaults_false() {
    let env = test_env();
    let (_admin, client, _token) = init_with_admin(&env);
    assert!(!client.is_paused());
}

/// The minimum deposit amount read helper reflects the configured value.
#[test]
fn test_min_deposit_amount_read_helper() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);
    client.set_min_deposit_amount(&admin, &250);
    assert_eq!(client.get_min_deposit_amount(), 250_i128);
}

/// The maximum lock duration read helper reflects the configured value.
#[test]
fn test_max_lock_duration_read_helper() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);
    client.set_max_lock_duration(&admin, &86_400);
    assert_eq!(client.get_max_lock_duration(), 86_400_u64);
}

/// The minimum lock duration read helper reflects the configured value.
#[test]
fn test_min_lock_duration_read_helper() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);
    client.set_min_lock_duration(&admin, &3_600);
    assert_eq!(client.get_min_lock_duration(), 3_600_u64);
}

// =========================================================================
// get_config — unified configuration read API (issue #456)
// =========================================================================

/// Freshly initialized contract returns all default configuration values.
#[test]
fn test_get_config_returns_defaults() {
    let env = test_env();
    let (admin, client, token) = init_with_admin(&env);
    let config = client.get_config();

    assert_eq!(config.token, token);
    assert_eq!(config.admin, admin);
    assert_eq!(config.version, soroban_sdk::String::from_str(&env, "0.1.0"));
    assert!(!config.paused);
    assert_eq!(config.pause_expiry, 0_u64);
    assert_eq!(config.min_deposit_amount, 0_i128);
    assert_eq!(config.max_lock_duration, 0_u64);
    assert_eq!(config.min_lock_duration, 0_u64);
}

/// After configuring limits, get_config returns the updated values.
#[test]
fn test_get_config_reflects_configured_limits() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);

    client.set_min_deposit_amount(&admin, &500);
    client.set_max_lock_duration(&admin, &2_592_000);
    client.set_min_lock_duration(&admin, &86_400);

    let config = client.get_config();
    assert_eq!(config.min_deposit_amount, 500_i128);
    assert_eq!(config.max_lock_duration, 2_592_000_u64);
    assert_eq!(config.min_lock_duration, 86_400_u64);
}

/// get_config reports active pause state and expiry.
#[test]
fn test_get_config_reflects_pause_state() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);

    env.ledger().set_timestamp(1_000);
    client.pause(&admin, &600);

    let config = client.get_config();
    assert!(config.paused);
    assert_eq!(config.pause_expiry, 1_600_u64);
}

/// get_config reports not-paused after explicit unpause.
#[test]
fn test_get_config_after_unpause() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);

    env.ledger().set_timestamp(1_000);
    client.pause(&admin, &600);
    client.unpause(&admin);

    let config = client.get_config();
    assert!(!config.paused);
    assert_eq!(config.pause_expiry, 0_u64);
}

/// get_config reports not-paused when pause has expired (expiry respected).
#[test]
fn test_get_config_expired_pause_reports_not_paused() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);

    env.ledger().set_timestamp(1_000);
    client.pause(&admin, &600); // expires at 1_600

    env.ledger().set_timestamp(2_000); // past expiry
    let config = client.get_config();
    assert!(!config.paused);
    // expiry timestamp is still stored even though pause has lapsed
    assert_eq!(config.pause_expiry, 1_600_u64);
}

/// get_config is non-mutating: calling it twice returns identical results.
#[test]
fn test_get_config_is_non_mutating() {
    let env = test_env();
    let (admin, client, _token) = init_with_admin(&env);

    client.set_min_deposit_amount(&admin, &100);
    let first = client.get_config();
    let second = client.get_config();
    assert_eq!(first, second);
}
