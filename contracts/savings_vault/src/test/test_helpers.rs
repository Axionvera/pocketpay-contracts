// Reusable test helpers for SavingsVault contract tests.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

/// Returns a default test environment with all auth calls mocked.
pub fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Registers the SavingsVault contract and returns its id and a client.
pub fn init_contract(env: &Env) -> (Address, SavingsVaultClient<'static>) {
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(env, &contract_id);
    (contract_id, client)
}

/// Generates a new user address.
pub fn new_user(env: &Env) -> Address {
    Address::generate(env)
}

/// Deposits a balance for a user.
/// Note: Contract must already be initialized.
pub fn deposit_balance(client: &SavingsVaultClient, user: &Address, amount: i128) {
    client.deposit(user, &amount);
}

/// Seeds multiple balances for a user.
/// Note: Contract must already be initialized.
pub fn seed_balances(client: &SavingsVaultClient, user: &Address, amounts: &[i128]) {
    for amt in amounts {
        client.deposit(user, amt);
    }
}

/// Sets the ledger's current timestamp (in unix seconds) for tests that
/// simulate time-based behaviour, such as lock/unlock schedules.
///
/// Use this instead of mutating `env.ledger()` directly so that every
/// time-based test makes its ledger time assumption explicit and easy
/// to spot at a glance.
pub fn set_ledger_timestamp(env: &Env, timestamp: u64) {
    env.ledger().set_timestamp(timestamp);
}

pub fn setup() -> (Env, Address, SavingsVaultClient<'static>) {
    let env = Env::default();
    // Allow all auth calls in test mode so we can focus on logic
    env.mock_all_auths();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);

    (env, contract_id, client)
}

pub fn test_token(
    env: Env,
    client: SavingsVaultClient<'static>,
) -> (
    Env,
    Address,
    SavingsVaultClient<'static>,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let admin = Address::generate(&env);

    let contract = env.register_stellar_asset_contract_v2(admin.clone());
    let contract_address = contract.address();

    client.initialize(&admin, &contract_address);

    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    (env.clone(), admin, client, token_client, token_admin)
}
