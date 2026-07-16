// Reusable test helpers for SavingsVault contract tests.

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

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
pub fn deposit_balance(client: &SavingsVaultClient, user: &Address, amount: i128) {
    client.deposit(user, &amount);
}

/// Seeds multiple balances for a user.
pub fn seed_balances(client: &SavingsVaultClient, user: &Address, amounts: &[i128]) {
    for amt in amounts {
        client.deposit(user, amt);
    }
}

/// Sets the ledger timestamp.
pub fn set_ledger_timestamp(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|li| li.timestamp = timestamp);
}
