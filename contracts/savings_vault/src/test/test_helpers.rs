// Reusable test helpers for SavingsVault contract tests.

use super::*;
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

/// Returns a default test environment with all auth calls mocked.
pub fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Registers the SavingsVault contract, initializes it, and returns its id and a client.
pub fn init_contract(env: &Env) -> (Address, SavingsVaultClient<'static>) {
    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let token = Address::generate(env);
    client.initialize(&admin, &token);
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

/// Sets the ledger's current timestamp (in unix seconds).
pub fn set_ledger_timestamp(env: &Env, timestamp: u64) {
    env.ledger().set_timestamp(timestamp);
}

pub fn setup() -> (Env, Address, SavingsVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(SavingsVault, ());
    let client = SavingsVaultClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.initialize(&admin, &token);

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

    // Removed redundant client.initialize(&admin, &contract_address); because contract is already initialized in setup()

    let token_client = token::Client::new(&env, &contract_address);
    let token_admin = token::StellarAssetClient::new(&env, &contract_address);
    (env.clone(), admin, client, token_client, token_admin)
}
}

/// A sequence helper to perform a mock deposit combined with a dummy SAC transfer.
/// This groups repetitive boilerplate that sets up an initial funded state.
pub fn deposit_with_sac(
    user: &Address,
    amount: i128,
    vault_client: &SavingsVaultClient<'static>,
    token_admin: &token::StellarAssetClient<'static>,
    token_client: &token::Client<'static>,
    contract_address: &Address,
) {
    // 1. Mint to user
    token_admin.mint(user, &10000);
    // 2. Perform internal accounting deposit
    vault_client.deposit(user, &amount);
    // 3. Mimic SAC transfer for custody
    token_client.transfer(user, contract_address, &amount);
}
