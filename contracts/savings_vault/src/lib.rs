//! Savings Vault — Soroban smart contract for the PocketPay mobile wallet.
//!
//! Users deposit tokens, withdraw available funds, and lock funds with a
//! time-based unlock mechanism. Balances are tracked on-chain and all
//! state-changing operations require the user's authorization.
//!
//! See [`docs/state-machine.md`](../../docs/state-machine.md) for the
//! contract's state transitions and error paths.

#![no_std]
extern crate alloc;
#[cfg(test)]
extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, log, symbol_short, token, Address, Env, Symbol, Vec,
};

const MAX_LOCK_PAGE_SIZE: u32 = 50;

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A time-locked entry in a user's vault. Multiple locks can exist per user.
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
/// Storage keys for contract state.
///
/// This enum defines all persistent and instance storage locations used by the contract.
/// Using an enum keeps storage organized, prevents key collisions, and makes the storage
/// model easy to review and extend.
///
/// # Variants
/// * `Admin` - The address of the contract admin (set once during initialization)
/// * `Balance(Address)` - Available (unlocked) balance for a specific user
/// * `Locks(Address)` - Vector of lock entries for a specific user
/// * `NextLockId(Address)` - Counter for generating unique lock IDs per user
/// * `Initialized` - Boolean flag indicating contract initialization status
/// * `Token` - The token contract address used for real token transfers
/// * `Paused` - Boolean flag indicating the contract is in emergency pause
/// * `PauseExpiry` - Unix timestamp (seconds) when the pause auto-expires (0 = no expiry set)
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Balance(Address),
    Locks(Address),
    NextLockId(Address),
    Initialized,
    Token,
    StorageVersion,
    /// Global pause flag — when true, deposits and locks are blocked.
    Paused,
    /// Unix timestamp when the current pause expires and the contract auto-unpauses.
    PauseExpiry,
}

pub const STORAGE_VERSION: u64 = 1;

// ---------------------------------------------------------------------------
// Contract Definition
// ---------------------------------------------------------------------------

#[contract]
pub struct SavingsVault;

#[contractimpl]
impl SavingsVault {
    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn assert_initialized(env: &Env) {
        if !env.storage().instance().has(&DataKey::Initialized) {
            panic!("Contract is not initialized");
        }
    }

    fn assert_supported_storage_version(env: &Env) {
        let version: u64 = env
            .storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap_or(0);
        if version != STORAGE_VERSION {
            panic!("Unsupported storage version");
        }
    }

    fn assert_admin(env: &Env, admin: &Address) {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != &stored_admin {
            panic!("Not authorized");
        }
    }

    fn load_locks(env: &Env, user: Address) -> Vec<LockEntry> {
        env.storage()
            .persistent()
            .get(&DataKey::Locks(user))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Assert the contract is not paused (or that the pause has expired).
    ///
    /// If a pause is active but its expiry timestamp has been reached, the pause
    /// is automatically cleared so callers do not need to invoke `unpause`
    /// explicitly after a time-bounded pause expires.
    ///
    /// # Panics
    ///
    /// Panics with `"Contract is paused"` when the pause is active and has not
    /// expired.
    fn require_not_paused(env: &Env) {
        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);

        if paused {
            let expiry: u64 = env
                .storage()
                .instance()
                .get(&DataKey::PauseExpiry)
                .unwrap_or(0);

            if expiry != 0 && env.ledger().timestamp() >= expiry {
                env.storage().instance().set(&DataKey::Paused, &false);
                env.storage().instance().set(&DataKey::PauseExpiry, &0_u64);
                return;
            }
            panic!("Contract is paused");
        }
    }

    fn assert_supported_storage_version(env: &Env) {
        let stored_version: u64 = env
            .storage()
            .instance()
            .get(&DataKey::StorageVersion)
            .unwrap_or(0);
        if stored_version != STORAGE_VERSION {
            panic!("Unsupported storage version");
        }
    }

    fn try_migrate(env: &Env) {
        // Placeholder for future migration logic
        // When STORAGE_VERSION is incremented, implement migration here
    }

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// One-time setup. Records admin and token addresses. Panics if called twice.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("Contract is already initialized");
        }

        // Try migration before initializing
        Self::try_migrate(&env);

        // Require the admin to have signed this transaction
        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::StorageVersion, &1_u64);

        // Emit initialize event
        let topics = (symbol_short!("initialize"), admin.clone());
        env.events().publish(topics, token.clone());

        log!(&env, "Savings Vault initialized with admin: {}, storage version: {}", admin, STORAGE_VERSION);
        let topics = (symbol_short!("init"), admin.clone());
        env.events().publish(topics, token.clone());

        log!(&env, "Vault init: admin={}, version={}", admin, STORAGE_VERSION);
    }

    // -----------------------------------------------------------------------
    // Version Metadata
    // -----------------------------------------------------------------------

    /// Returns the hard-coded semantic version baked into the WASM binary.
    pub fn get_version(env: Env) -> soroban_sdk::String {
        // No need to be initialized for version check, but check storage version if possible
        if env.storage().instance().has(&DataKey::Initialized) {
            Self::try_migrate(&env);
            Self::assert_supported_storage_version(&env);
        }
        soroban_sdk::String::from_str(&env, "0.1.0")
    }

    // -----------------------------------------------------------------------
    // Token Configuration
    // -----------------------------------------------------------------------

    /// Get the configured token address.
    ///
    /// Returns the address of the Stellar Asset Contract (SAC) that the vault
    /// uses for deposits and withdrawals.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// The token address as an `Address`.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    ///
    /// # Panics
    ///
    /// - If the contract has not been initialized.
    pub fn get_token(env: Env) -> Address {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        env.storage().instance().get(&DataKey::Token).unwrap()
    }

    // -----------------------------------------------------------------------
    // Emergency Pause
    // -----------------------------------------------------------------------

    /// Activate an emergency pause on the contract.
    ///
    /// When paused, `deposit` and `lock_funds` are blocked. Withdrawals
    /// (`withdraw` and `withdraw_lock`) remain available so users can always
    /// exit. Read-only query functions are unaffected.
    ///
    /// The pause automatically expires after `duration_secs` seconds. If the
    /// pause is still active when `env.ledger().timestamp() >= expiry`, the
    /// next call to a mutating function will silently clear the pause
    /// (auto-unpause).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The current admin address (must authorize this transaction)
    /// * `duration_secs` - How long the pause lasts, in seconds. Must be > 0.
    ///
    /// # Authorization
    ///
    /// The `admin` address must sign the transaction and must match the stored
    /// admin.
    ///
    /// # State Changes
    ///
    /// - Sets `Paused` to `true` in instance storage
    /// - Sets `PauseExpiry` to `current_timestamp + duration_secs`
    /// - Emits a `pause` event with the admin address and expiry timestamp
    ///
    /// # Panics
    ///
    /// - If the contract has not been initialized
    /// - If the caller is not the admin
    /// - If `duration_secs` is zero
    pub fn pause(env: Env, admin: Address, duration_secs: u64) {
        Self::assert_initialized(&env);
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        if duration_secs == 0 {
            panic!("Pause duration must be greater than zero");
        }

        let expiry = env.ledger().timestamp() + duration_secs;
        env.storage().instance().set(&DataKey::Paused, &true);
        env
            .storage()
            .instance()
            .set(&DataKey::PauseExpiry, &expiry);

        let topics = (symbol_short!("pause"), admin.clone());
        env.events().publish(topics, expiry);

        log!(
            &env,
            "Pause: admin={}, expiry={}",
            admin,
            expiry
        );
    }

    /// Deactivate an active pause.
    ///
    /// Immediately clears the pause flag and expiry, re-enabling deposits and
    /// locks. Can be called by the admin even before the pause expires, allowing
    /// early restoration of normal operations after an incident is resolved.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The current admin address (must authorize this transaction)
    ///
    /// # Authorization
    ///
    /// The `admin` address must sign the transaction and must match the stored
    /// admin.
    ///
    /// # State Changes
    ///
    /// - Sets `Paused` to `false`
    /// - Sets `PauseExpiry` to `0`
    /// - Emits an `unpause` event with the admin address
    ///
    /// # Panics
    ///
    /// - If the contract has not been initialized
    /// - If the caller is not the admin
    pub fn unpause(env: Env, admin: Address) {
        Self::assert_initialized(&env);
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        env.storage().instance().set(&DataKey::Paused, &false);
        env.storage().instance().set(&DataKey::PauseExpiry, &0_u64);

        let topics = (symbol_short!("unpause"), admin.clone());
        env.events().publish(topics, ());

        log!(&env, "Unpause: admin={}", admin);
    }

    /// Check whether the contract is currently paused.
    ///
    /// Returns `true` when the pause flag is set **and** the pause has not yet
    /// expired. If the pause has expired, returns `false` (the flag is not
    /// cleared by this read-only call — it will be cleared lazily on the next
    /// mutating call via `require_not_paused`).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// `true` if the contract is actively paused; `false` otherwise.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    pub fn is_paused(env: Env) -> bool {
        Self::assert_initialized(&env);

        let paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::Paused)
            .unwrap_or(false);

        if !paused {
            return false;
        }

        let expiry: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PauseExpiry)
            .unwrap_or(0);

        if expiry != 0 && env.ledger().timestamp() >= expiry {
            return false;
        }

        true
    }

    // -----------------------------------------------------------------------
    // Deposits
    // -----------------------------------------------------------------------

    /// Transfers tokens from the user into the vault and credits their balance.
    /// Panics if amount <= 0.
    pub fn deposit(env: Env, user: Address, amount: i128) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        Self::require_not_paused(&env);

        user.require_auth();

        if amount <= 0 {
            panic!("Deposit amount must be greater than zero");
        }

        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&user, &contract_address, &amount);

        let current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        let new_balance = current_balance + amount;
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &new_balance);

        let topics = (symbol_short!("deposit"), user.clone());
        let payload = (amount, new_balance);
        env.events().publish(topics, payload);

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

    /// Withdraws available funds from the user's vault. Satisfies the
    /// withdrawal from the deposited balance first, then from matured locks.
    /// Panics if amount <= 0 or exceeds available balance.
    pub fn withdraw(env: Env, user: Address, amount: i128) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

        user.require_auth();

        if amount <= 0 {
            panic!("Withdrawal amount must be greater than zero");
        }

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

        let new_locked: i128 = locks
            .iter()
            .filter(|lock| current_time < lock.unlock_time)
            .map(|lock| lock.amount)
            .sum();

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);
        env.storage()
            .persistent()
            .set(&DataKey::Locks(user.clone()), &locks);

        let topics = (symbol_short!("withdraw"), user.clone());
        let payload = (amount, current_balance, new_locked);
        env.events().publish(topics, payload);

        log!(
            &env,
            "Withdraw: user={}, amount={}, new_balance={}, new_locked={}",
            user,
            amount,
            current_balance,
            new_locked
        );
    }

    /// Withdraws a specific matured lock entry by its ID.
    /// Panics if the lock doesn't exist or hasn't matured.
    pub fn withdraw_lock(env: Env, user: Address, lock_id: u64) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

        user.require_auth();

        let mut locks = Self::load_locks(&env, user.clone());

        let lock_index = locks.iter().position(|lock| lock.id == lock_id);

        let index = match lock_index {
            Some(i) => i,
            None => panic!("Lock not found"),
        };

        let lock = locks.get(index as u32).unwrap();

        let current_time = env.ledger().timestamp();
        if current_time < lock.unlock_time {
            panic!("Lock has not matured yet");
        }

        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        token_client.transfer(&contract_address, &user, &lock.amount);

        locks.remove(index as u32);

        env.storage()
            .persistent()
            .set(&DataKey::Locks(user.clone()), &locks);

        let topics = (Symbol::new(&env, "withdraw_lock"), user.clone());
        let payload = (lock_id, lock.amount);
        env.events().publish(topics, payload);

        log!(
            &env,
            "WithdrawLock: user={}, lock_id={}, amount={}",
            user,
            lock_id,
            lock.amount
        );
    }

    // -----------------------------------------------------------------------
    // Balance Queries
    // -----------------------------------------------------------------------

    /// Returns the user's available balance: deposited funds + matured locks.
    pub fn get_balance(env: Env, user: Address) -> i128 {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let deposited_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        let locks = Self::load_locks(&env, user);

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

    /// Locks a portion of the user's available balance until `unlock_time`.
    /// Returns the lock ID. Panics if amount <= 0, exceeds balance, or
    /// unlock_time is not in the future.
    pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64) -> u64 {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        Self::require_not_paused(&env);

        user.require_auth();

        if amount <= 0 {
            panic!("Lock amount must be greater than zero");
        }

        let current_time = env.ledger().timestamp();
        if unlock_time <= current_time {
            panic!("Unlock time must be in the future");
        }

        let mut current_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        if amount > current_balance {
            panic!("Insufficient balance to lock");
        }

        let next_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        env.storage()
            .persistent()
            .set(&DataKey::NextLockId(user.clone()), &(next_id + 1));

        let mut locks: Vec<LockEntry> = env
            .storage()
            .persistent()
            .get(&DataKey::Locks(user.clone()))
            .unwrap_or_else(|| Vec::new(&env));

        let new_lock = LockEntry {
            id: next_id,
            amount,
            unlock_time,
        };

        locks.push_back(new_lock);

        current_balance -= amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);
        env.storage()
            .persistent()
            .set(&DataKey::Locks(user.clone()), &locks);

        let new_locked: i128 = locks.iter().map(|l| l.amount).sum();

        let topics = (symbol_short!("lock"), user.clone());
        let payload = (amount, unlock_time, current_balance, new_locked);
        env.events().publish(topics, payload);

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

    /// Returns the sum of all active (unmatured) lock amounts.
    pub fn get_locked_balance(env: Env, user: Address) -> i128 {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let locks = Self::load_locks(&env, user);

        let current_time = env.ledger().timestamp();
        let mut total_locked: i128 = 0;
        for lock in locks.iter() {
            if current_time < lock.unlock_time {
                total_locked += lock.amount;
            }
        }
        total_locked
    }

    /// Returns true if the user has at least one matured lock.
    pub fn can_withdraw(env: Env, user: Address) -> bool {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let locks = Self::load_locks(&env, user);

        let current_time = env.ledger().timestamp();
        for lock in locks.iter() {
            if current_time >= lock.unlock_time {
                return true;
            }
        }

        false
    }

    /// Returns a single lock entry by ID, or None if not found.
    pub fn get_lock(env: Env, user: Address, lock_id: u64) -> Option<LockEntry> {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let locks = Self::load_locks(&env, user);
        locks.iter().find(|lock| lock.id == lock_id)
    }

    /// Returns a paginated list of lock entries for a user (oldest first).
    pub fn list_locks(env: Env, user: Address, offset: u32, limit: u32) -> Vec<LockEntry> {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        if limit == 0 {
            return Vec::new(&env);
        }

        let page_limit = limit.min(MAX_LOCK_PAGE_SIZE);
        let locks = Self::load_locks(&env, user);
        let total = locks.len();
        if offset >= total {
            return Vec::new(&env);
        }

        let end = offset.saturating_add(page_limit).min(total);
        let mut page = Vec::new(&env);
        for i in offset..end {
            page.push_back(locks.get(i).unwrap());
        }
        page
    }

    // -----------------------------------------------------------------------
    // Admin Functions
    // -----------------------------------------------------------------------

    /// Returns the admin address set during initialization.
    pub fn get_admin(env: Env) -> Address {
        Self::assert_initialized(&env);
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    /// Transfers admin privileges to a new address. Only the current admin
    /// can call this.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        Self::assert_initialized(&env);
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let old_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        let topics = (symbol_short!("xferadmin"), old_admin.clone());
        env.events().publish(topics, new_admin.clone());

        log!(&env, "Admin transferred from {} to {}", old_admin, new_admin);
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test;
#[cfg(test)]
#[path = "test/test_helpers.rs"]
mod test_helpers;