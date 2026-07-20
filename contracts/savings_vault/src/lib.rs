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

use soroban_sdk::{contract, contractimpl, contracttype, log, token, Address, Env, Vec};

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEntry {
    pub id: u64,
    pub amount: i128,
    pub unlock_time: u64,
}

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
    /// Stores lock entries for a user.
    Locks(Address),
    /// Stores next lock ID for a user.
    NextLockId(Address),
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

        // Get token address
        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        // Perform real token transfer from user to contract
        token_client.transfer(&user, &contract_address, &amount);

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
    /// - If `amount` exceeds the user's available balance (including matured locks).
    pub fn withdraw(env: Env, user: Address, amount: i128) {
        // Authorization
        user.require_auth();

        // Validate amount
        if amount <= 0 {
            panic!("Withdrawal amount must be greater than zero");
        }

        // Read current deposited balance
        let mut current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        let mut locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        let mut total_matured: i128 = 0;
        for lock in locks.iter() {
            if current_time >= lock.unlock_time {
                total_matured += lock.amount;
            }
        }

        // Ensure sufficient funds across available balance and matured locks
        if amount > current_balance + total_matured {
            panic!("Insufficient balance");
        }

        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &user, &amount);

        // Deduct from deposited balance first, then matured locks
        let mut remaining_to_deduct = amount;
        if remaining_to_deduct <= current_balance {
            current_balance -= remaining_to_deduct;
            remaining_to_deduct = 0;
        } else {
            remaining_to_deduct -= current_balance;
            current_balance = 0;
        }

        if remaining_to_deduct > 0 {
            let mut new_locks = Vec::new(&env);
            for lock in locks.iter() {
                if current_time >= lock.unlock_time && remaining_to_deduct > 0 {
                    if lock.amount <= remaining_to_deduct {
                        remaining_to_deduct -= lock.amount;
                    } else {
                        let updated_lock = LockEntry {
                            id: lock.id,
                            amount: lock.amount - remaining_to_deduct,
                            unlock_time: lock.unlock_time,
                        };
                        remaining_to_deduct = 0;
                        new_locks.push_back(updated_lock);
                    }
                } else {
                    new_locks.push_back(lock);
                }
            }
            locks = new_locks;
        }

        // Update balance and locks
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);
        env.storage()
            .persistent()
            .set(&DataKey::Locks(user.clone()), &locks);

        log!(
            &env,
            "Withdraw: user={}, amount={}, new_balance={}",
            user,
            amount,
            current_balance
        );
    }

    // -----------------------------------------------------------------------
    // Balance Queries
    // -----------------------------------------------------------------------

    /// Get the available (unlocked) balance for a user.
    /// Available balance includes regular deposited balance plus matured locks.
    ///
    /// Returns `0` if the user has never deposited.
    pub fn get_balance(env: Env, user: Address) -> i128 {
        let deposited_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        let locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        let mut matured_amount: i128 = 0;
        for lock in locks.iter() {
            if current_time >= lock.unlock_time {
                matured_amount += lock.amount;
            }
        }

        deposited_balance + matured_amount
    }

    // -----------------------------------------------------------------------
    // Fund Locking
    // -----------------------------------------------------------------------

    /// Lock a portion of the user's balance until a specified time.
    ///
    /// Locked funds are moved from the available balance into a separate
    /// lock entry. They cannot be withdrawn until the
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
    pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64) -> u64 {
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
        let mut current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        if amount > current_balance {
            panic!("Insufficient balance to lock");
        }

        // Assign a new lock ID
        let next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        env.storage()
            .persistent()
            .set(&DataKey::NextLockId(user.clone()), &(next_id + 1));

        // Read existing locks
        let mut locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        // Create new lock entry
        let new_lock = LockEntry {
            id: next_id,
            amount,
            unlock_time,
        };

        locks.push_back(new_lock);

        // Move funds: available -> locked
        current_balance -= amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);
        env.storage()
            .persistent()
            .set(&DataKey::Locks(user.clone()), &locks);

        log!(
            &env,
            "Lock: user={}, amount={}, unlock_time={}, available={}, lock_id={}",
            user,
            amount,
            unlock_time,
            current_balance,
            next_id
        );

        next_id
    }

    /// Get the locked balance for a user.
    ///
    /// Returns the sum of all active (unmatured) locks.
    pub fn get_locked_balance(env: Env, user: Address) -> i128 {
        let locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        let mut total_locked: i128 = 0;
        for lock in locks.iter() {
            if current_time < lock.unlock_time {
                total_locked += lock.amount;
            }
        }
        total_locked
    }

    /// Check whether a user can withdraw their locked funds.
    ///
    /// Returns `true` if:
    /// - The user has locked funds, AND
    /// - The current ledger timestamp is >= the unlock time.
    ///
    /// Returns `false` otherwise (including when there are no locked funds).
    pub fn can_withdraw(env: Env, user: Address) -> bool {
        let locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user))
            .unwrap_or_else(|| Vec::new(&env));

        let current_time = env.ledger().timestamp();
        for lock in locks.iter() {
            if current_time >= lock.unlock_time {
                return true;
            }
        }

        false
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test;
