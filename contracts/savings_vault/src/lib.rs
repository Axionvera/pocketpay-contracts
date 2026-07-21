//! # Savings Vault Contract
//!
//! A Soroban smart contract that provides a savings vault for the
//! Stellar PocketPay mobile wallet. Users can deposit, withdraw,
//! and lock funds with a time-based unlock mechanism.
//!
//! ## Overview
//!
//! The Savings Vault enables users to:
//! - Deposit funds into their personal vault
//! - Withdraw available funds at any time
//! - Lock funds until a specified Unix timestamp (in seconds)
//! - Query their available and locked balances
//! - Check lock maturity status
//!
//! ## Key Features
//! - **Deposits**: Add funds to vault (internal accounting only, no real token custody yet)
//! - **Withdrawals**: Remove available (unlocked) funds from vault
//! - **Locks**: Time-based fund locking with Unix timestamp unlock times
//! - **Balance Queries**: Check available (unlocked) and locked balances separately
//! - **Authorization**: All state-changing operations require the user to authorize
//!
//! ## Storage & State
//!
//! The contract uses Soroban persistent storage to maintain:
//! - User balances (available, unlocked funds)
//! - Lock entries for each user (amount, unlock_time)
//! - Admin address (set during initialization)
//! - Token address (set during initialization)
//!
//! ## Important Notes
//!
//! - **Internal Accounting**: Currently, deposits and withdrawals update internal contract
//!   balances but do not transfer actual tokens. Real token custody and transfers are planned
//!   for future SAC (Stellar Asset Contract) integration.
//! - **Authorization**: The user's address must authorize all deposit, withdrawal, and lock operations.
//! - **Lock Overwrite**: Locking funds does not create separate lock entries per operation;
//!   each user has a vector of lock entries that can be managed independently.
//! - **Unix Timestamps**: All time values use Unix timestamps in seconds (ledger.timestamp()).
//!
//! ## Examples
//!
//! ### Initialize the contract
//! ```ignore
//! let admin_addr = Address::from_account_id(&env, &account_id);
//! let token_addr = Address::from_contract_id(&env, &token_contract_id);
//! SavingsVault::initialize(&env, admin_addr, token_addr);
//! ```
//!
//! ### Deposit and lock funds
//! ```ignore
//! let user = Address::from_account_id(&env, &user_account_id);
//! SavingsVault::deposit(&env, user.clone(), 1000);
//! let unlock_time = env.ledger().timestamp() + 86400; // 1 day from now
//! SavingsVault::lock_funds(&env, user, 500, unlock_time);
//! ```
//!
//! ### Query balances
//! ```ignore
//! let available = SavingsVault::get_balance(&env, user.clone());
//! let locked = SavingsVault::get_locked_balance(&env, user);
//! ```

#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, log, token, Address, Env, Vec};

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Represents a time-locked entry in a user's vault.
///
/// A lock entry tracks a portion of funds that are frozen until a specified
/// Unix timestamp. Multiple lock entries can exist for the same user.
///
/// # Fields
/// * `id` - Unique identifier for this lock entry (generated sequentially per user)
/// * `amount` - The amount of funds locked (in contract units)
/// * `unlock_time` - Unix timestamp (seconds) when these funds become available
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
/// * `Token` - The token contract address for real token transfers (future integration)
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

/// The main Savings Vault contract implementation.
///
/// This contract provides the public interface for managing user savings vaults
/// on the Stellar blockchain via Soroban. All state is stored on-chain and
/// all state-changing operations require authorization from the acting user.
///
/// # Authorization Model
///
/// - **Initialization**: Only the designated admin can initialize the contract (one-time only)
/// - **Deposits/Withdrawals/Locks**: Each operation requires the user to authorize it via `require_auth()`
/// - **Queries**: Read-only operations do not require authorization
///
/// # Storage Layers
///
/// The contract uses two storage layers:
/// - **Instance Storage**: Stores admin and initialization flag (tied to contract lifetime)
/// - **Persistent Storage**: Stores user balances and locks (survives longer with TTL management)
///
/// See [docs/storage-ttl.md](../../docs/storage-ttl.md) for TTL management guidelines.
#[contract]
pub struct SavingsVault;

#[contractimpl]
impl SavingsVault {
    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Initialize the contract with an admin address and token address.
    ///
    /// This function must be called exactly once before any other contract operations.
    /// It sets up the initial state and records the admin address for future reference
    /// (e.g., future admin-only features, upgrades, or emergency controls).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The address to be recorded as the contract admin. This address must
    ///   authorize the transaction via `require_auth()`.
    /// * `token` - The address of the token contract to be used for real token transfers
    ///   (future SAC integration). Currently stored but not used in deposit/withdrawal logic.
    ///
    /// # Authorization
    ///
    /// The `admin` address must sign the transaction. This ensures only an authorized party
    /// can initialize the contract.
    ///
    /// # State Changes
    ///
    /// - Sets the admin address in instance storage
    /// - Sets the token address in instance storage
    /// - Sets an initialization flag (prevents re-initialization)
    /// - Emits a log event with the admin address
    ///
    /// # Panics
    ///
    /// - If the contract has already been initialized (re-initialization attempt)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let admin = Address::from_account_id(&env, &admin_account_id);
    /// let token = Address::from_contract_id(&env, &token_contract_id);
    /// SavingsVault::initialize(&env, admin, token);
    /// ```
    ///
    /// # Future Notes
    ///
    /// The token address parameter is reserved for future SAC (Stellar Asset Contract)
    /// integration to support real token transfers and custody-backed balances.
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
    /// Increases the user's recorded balance in the contract. Currently, this updates
    /// internal contract accounting only and does not transfer real tokens. Future
    /// SAC integration will enable actual token transfers and custody.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address (must authorize this transaction via `require_auth()`)
    /// * `amount` - The amount to deposit (in contract units, must be > 0)
    ///
    /// # Authorization
    ///
    /// The `user` address must sign the transaction. Only the user can deposit on their own behalf.
    ///
    /// # State Changes
    ///
    /// - Increases the user's available balance
    /// - Emits a log event with deposit details
    ///
    /// # Panics
    ///
    /// - If `amount` is zero or negative
    ///
    /// # Notes
    ///
    /// - **Internal Accounting Only**: The user's balance is a bookkeeping entry maintained
    ///   by the contract. It is not backed by real token transfers until SAC integration is implemented.
    /// - **No Double-Lock Prevention**: If funds are already locked, calling deposit does not
    ///   affect the locked amount. Locked funds remain locked until their unlock_time.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// SavingsVault::deposit(&env, user, 1000);
    /// ```
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
    /// Removes available funds from the user's vault. Available funds include:
    /// - The user's deposited balance (not locked)
    /// - Any matured lock entries (current_time >= unlock_time)
    ///
    /// Locked funds that have not yet matured cannot be withdrawn.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The withdrawer's address (must authorize this transaction via `require_auth()`)
    /// * `amount` - The amount to withdraw (in contract units, must be > 0)
    ///
    /// # Authorization
    ///
    /// The `user` address must sign the transaction. Only the user can withdraw their own funds.
    ///
    /// # State Changes
    ///
    /// - Decreases the user's available balance
    /// - Removes matured locks as needed to satisfy the withdrawal
    /// - Transfers the amount via the token contract
    /// - Emits a log event with withdrawal details
    ///
    /// # Panics
    ///
    /// - If `amount` is zero or negative
    /// - If `amount` exceeds the user's total available funds (deposited + matured locks)
    ///   with the error message "Insufficient balance"
    ///
    /// # Withdrawal Order
    ///
    /// Funds are withdrawn in this order:
    /// 1. From the user's available (non-locked) balance first
    /// 2. From matured locks (oldest/earliest unlock times first) if needed
    ///
    /// # Notes
    ///
    /// - **Internal Accounting**: Currently, withdrawals update internal accounting but
    ///   real token transfers require SAC integration.
    /// - **Partial Lock Deduction**: If a matured lock has more funds than needed,
    ///   only the required amount is deducted from that lock.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// SavingsVault::withdraw(&env, user, 500);
    /// ```
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
    ///
    /// The available balance includes:
    /// - The user's deposited balance (funds not in any lock)
    /// - Any matured locks (where current_time >= unlock_time)
    ///
    /// Locked funds that have not yet matured are NOT included in this balance.
    /// Use [`get_locked_balance`](Self::get_locked_balance) to query unmatured locks.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address
    ///
    /// # Returns
    ///
    /// The total available balance in contract units. Returns `0` if the user has never
    /// deposited or has withdrawn all their funds.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// let available = SavingsVault::get_balance(&env, user);
    /// println!("Available balance: {}", available);
    /// ```
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
    /// Locked funds are moved from the available balance into a separate lock entry.
    /// They cannot be withdrawn until the `unlock_time` has passed (current_time >= unlock_time).
    /// Once a lock matures, its funds can be withdrawn like any other available balance.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address (must authorize this transaction via `require_auth()`)
    /// * `amount` - The amount to lock (in contract units, must be > 0)
    /// * `unlock_time` - Unix timestamp (seconds) when the funds unlock. Must be in the future
    ///   relative to the current ledger timestamp.
    ///
    /// # Authorization
    ///
    /// The `user` address must sign the transaction. Only the user can lock their own funds.
    ///
    /// # Returns
    ///
    /// The lock ID assigned to this new lock entry. Lock IDs are unique per user and
    /// can be used for future reference (e.g., in an extended API to unlock early).
    ///
    /// # State Changes
    ///
    /// - Creates a new lock entry with a unique ID
    /// - Moves the amount from available balance to the lock
    /// - Increments the next lock ID counter for this user
    /// - Emits a log event with lock details
    ///
    /// # Panics
    ///
    /// - If `amount` is zero or negative
    /// - If `amount` exceeds the user's available balance with error "Insufficient balance to lock"
    /// - If `unlock_time` is not in the future (current_time >= unlock_time)
    ///   with error "Unlock time must be in the future"
    ///
    /// # Multiple Locks
    ///
    /// A user can have multiple active locks at different unlock times. Each lock is
    /// tracked independently. When withdrawing, matured locks are consumed first.
    ///
    /// # Notes
    ///
    /// - **Overwrite Behavior**: Unlike some vault designs, locking funds multiple times
    ///   does not overwrite a previous lock; instead, a new lock entry is created.
    /// - **Lock ID Uniqueness**: Lock IDs are unique per user (not globally unique).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// let unlock_time = env.ledger().timestamp() + 86400; // 1 day from now (86400 seconds)
    /// let lock_id = SavingsVault::lock_funds(&env, user, 500, unlock_time);
    /// println!("Created lock with ID: {}", lock_id);
    /// ```
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
    /// Returns the sum of all active (unmatured) locks. Active locks are those where
    /// the current ledger timestamp is still before the unlock_time.
    ///
    /// Matured locks (where current_time >= unlock_time) are not included in this balance.
    /// They are instead available as part of the user's total available balance via
    /// [`get_balance`](Self::get_balance).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address
    ///
    /// # Returns
    ///
    /// The total amount in active (unmatured) locks, in contract units.
    /// Returns `0` if the user has no locks or all locks have matured.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// let locked = SavingsVault::get_locked_balance(&env, user);
    /// println!("Locked balance: {}", locked);
    /// ```
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
    /// This is a convenience query function that indicates whether the user has
    /// any matured locks (locks where current_time >= unlock_time).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address
    ///
    /// # Returns
    ///
    /// `true` if:
    /// - The user has at least one lock entry, AND
    /// - At least one lock has reached its unlock_time (current_time >= unlock_time)
    ///
    /// `false` if:
    /// - The user has no locks, OR
    /// - All locks are still active (current_time < unlock_time)
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    ///
    /// # Notes
    ///
    /// - This function does not check the user's deposited (non-locked) balance.
    /// - It only checks whether matured locks exist.
    /// - To check the actual amount available for withdrawal, use [`get_balance`](Self::get_balance)
    ///   to include deposited funds plus matured locks.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let user = Address::from_account_id(&env, &user_account_id);
    /// if SavingsVault::can_withdraw(&env, user.clone()) {
    ///     println!("User has matured locks available for withdrawal");
    /// }
    /// let available = SavingsVault::get_balance(&env, user);
    /// if available > 0 {
    ///     SavingsVault::withdraw(&env, user, available);
    /// }
    /// ```
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
