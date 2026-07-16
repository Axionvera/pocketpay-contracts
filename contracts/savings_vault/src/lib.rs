//! # Savings Vault Contract
//!
//! A Soroban smart contract that provides a savings vault for the
//! Stellar PocketPay mobile wallet. Users can deposit, withdraw,
//! and lock funds with a time-based unlock mechanism.
//!
//! ## Features
//! - Deposit and withdraw XLM (or any Stellar asset)
//! - Lock funds until a specified timestamp
//! - Query balances and lock status
//! - Admin-controlled initialization

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, log, token, Address, Env};

// ---------------------------------------------------------------------------
// Storage Keys
// ---------------------------------------------------------------------------

/// All keys used to store data on-chain.
/// Using an enum keeps storage organized and easy to extend.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Stores the admin address (set once during initialization).
    Admin,
    /// Stores the available (unlocked) balance for a user.
    Balance(Address),
    /// Stores the locked balance for a user.
    LockedBalance(Address),
    /// Stores the unlock timestamp (Unix epoch seconds) for a user.
    UnlockTime(Address),
    /// Flag indicating the contract has been initialized.
    Initialized,
    /// Token Address
    Token,
}

// ---------------------------------------------------------------------------
// Contract Definition
// ---------------------------------------------------------------------------

#[contract]
pub struct SavingsVault;

#[contractimpl]
impl SavingsVault {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialize the contract with an admin address.
    ///
    /// This can only be called once. The admin address is stored for future
    /// reference (e.g. upgradeability or admin-only features).
    ///
    /// # Arguments
    /// * `admin` - The address that will be recorded as the contract admin.
    ///
    /// # Panics
    /// Panics if the contract has already been initialized.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        // Ensure we haven't already initialized
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("Contract is already initialized");
        }

        // Require the admin to have signed this transaction
        admin.require_auth();

        // Persist admin & initialization flag
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Token, &token);

        log!(&env, "Savings Vault initialized with admin: {}", admin);
    }

    // -----------------------------------------------------------------------
    // Deposits
    // -----------------------------------------------------------------------

    /// Deposit funds into the caller's vault.
    ///
    /// # Arguments
    /// * `user`   - The depositor's address (must authorize the call).
    /// * `amount` - The amount to deposit (must be > 0).
    ///
    /// # Panics
    /// Panics if `amount` is zero or negative.
    pub fn deposit(env: Env, user: Address, amount: i128) {
        // Authorization: only the user can deposit on their own behalf
        user.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Deposit amount must be greater than zero");
        }

        // Read current balance (default to 0 if none exists)
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        // Update balance
        let new_balance = current_balance + amount;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &new_balance);

        log!(
            &env,
            "Deposit: user={}, amount={}, new_balance={}",
            user,
            amount,
            new_balance
        );
    }

    // -----------------------------------------------------------------------
    // Withdrawals
    // -----------------------------------------------------------------------

    /// Withdraw funds from the caller's vault.
    ///
    /// # Arguments
    /// * `user`   - The withdrawer's address (must authorize the call).
    /// * `amount` - The amount to withdraw (must be > 0).
    ///
    /// # Panics
    /// - If `amount` is zero or negative.
    /// - If `amount` exceeds the user's available balance.
    pub fn withdraw(env: Env, user: Address, amount: i128) {
        // Authorization
        user.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Withdrawal amount must be greater than zero");
        }

        // Read current balance
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        // Ensure sufficient funds
        if amount > current_balance {
            panic!("Insufficient balance");
        }
        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &user, &amount);

        // Update balance
        let new_balance = current_balance - amount;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &new_balance);

        log!(
            &env,
            "Withdraw: user={}, amount={}, new_balance={}",
            user,
            amount,
            new_balance
        );
    }

    // -----------------------------------------------------------------------
    // Balance Queries
    // -----------------------------------------------------------------------

    /// Get the available (unlocked) balance for a user.
    ///
    /// Returns `0` if the user has never deposited.
    pub fn get_balance(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(user))
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Fund Locking
    // -----------------------------------------------------------------------

    /// Lock a portion of the user's balance until a specified time.
    ///
    /// Locked funds are moved from the available balance into a separate
    /// locked balance bucket. They cannot be withdrawn until the
    /// `unlock_time` has passed.
    ///
    /// # Arguments
    /// * `user`        - The user's address (must authorize the call).
    /// * `amount`      - The amount to lock (must be > 0).
    /// * `unlock_time` - Unix timestamp (seconds) when the funds unlock.
    ///
    /// # Panics
    /// - If `amount` is zero or negative.
    /// - If `amount` exceeds the user's available balance.
    /// - If `unlock_time` is in the past.
    pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64) {
        // Authorization
        user.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Lock amount must be greater than zero");
        }

        // Validate unlock time is in the future
        let current_time = env.ledger().timestamp();
        if unlock_time <= current_time {
            panic!("Unlock time must be in the future");
        }

        // Read available balance
        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        if amount > current_balance {
            panic!("Insufficient balance to lock");
        }

        // Read existing locked balance (may already have some locked)
        let current_locked: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::LockedBalance(user.clone()))
            .unwrap_or(0);

        // Move funds: available -> locked
        let new_balance = current_balance - amount;
        let new_locked = current_locked + amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &new_balance);
        env.storage()
            .persistent()
            .set(&DataKey::LockedBalance(user.clone()), &new_locked);
        env.storage()
            .persistent()
            .set(&DataKey::UnlockTime(user.clone()), &unlock_time);

        log!(
            &env,
            "Lock: user={}, amount={}, unlock_time={}, available={}, locked={}",
            user,
            amount,
            unlock_time,
            new_balance,
            new_locked
        );
    }

    /// Get the locked balance for a user.
    ///
    /// Returns `0` if the user has no locked funds.
    pub fn get_locked_balance(env: Env, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::LockedBalance(user))
            .unwrap_or(0)
    }

    /// Check whether a user can withdraw their locked funds.
    ///
    /// Returns `true` if:
    /// - The user has locked funds, AND
    /// - The current ledger timestamp is >= the unlock time.
    ///
    /// Returns `false` otherwise (including when there are no locked funds).
    pub fn can_withdraw(env: Env, user: Address) -> bool {
        // Check if user has any locked funds
        let locked_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::LockedBalance(user.clone()))
            .unwrap_or(0);

        if locked_balance <= 0 {
            return false;
        }

        // Check if unlock time has passed
        let unlock_time: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::UnlockTime(user))
            .unwrap_or(0);

        let current_time = env.ledger().timestamp();

        current_time >= unlock_time
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_helpers;
