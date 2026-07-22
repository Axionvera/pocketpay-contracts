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
//! - **Deposits**: Transfer tokens into the vault and credit the user's balance
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
//! - **Token-backed balances**: Deposits transfer tokens from the user into the vault via the
//!   configured Stellar Asset Contract (SAC), and withdrawals transfer tokens back to the user.
//!   Internal balances are updated only after the token transfer succeeds.
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
extern crate alloc;
#[cfg(test)]
extern crate std;

use soroban_sdk::{
    contract, contractimpl, contracttype, log, symbol_short, token, Address, Env, Symbol, Vec,
};

/// Maximum number of lock records returned by [`SavingsVault::list_locks`] per call.
const MAX_LOCK_PAGE_SIZE: u32 = 50;

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
    pub owner: Address,
    pub amount: i128,
    pub created_time: u64,
    pub unlock_time: u64,
    pub withdrawn: bool,
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
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Stores the admin address (set once during initialization).
    Admin,
    /// Stores the available (unlocked) balance for a user.
    Balance(Address),
    /// Stores individual lock entries for a user.
    Lock(Address, u64),
    /// Stores next lock ID for a user.
    NextLockId(Address),
    /// Flag indicating the contract has been initialized.
    Initialized,
    /// Token Address
    Token,
    /// Storage version marker
    StorageVersion,
}

/// Current storage schema version.
/// Increment this when making breaking changes to storage layout,
/// and implement a corresponding migration in `try_migrate()`.
pub const STORAGE_VERSION: u64 = 1;

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
    /// * `token` - The address of the token contract used for real token transfers. Deposits
    ///   and withdrawals move balances through this Stellar Asset Contract (SAC).
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
    /// # Notes
    ///
    /// The token address is the Stellar Asset Contract (SAC) used for real token transfers,
    /// so deposits and withdrawals are backed by actual token custody.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        // Ensure we haven't already initialized
        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("Contract is already initialized");
        }

        // Try migration before initializing
        Self::try_migrate(&env);

        // Require the admin to have signed this transaction
        admin.require_auth();

        // Persist admin, token, initialization flag, and storage version
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

    /// Get the contract version.
    ///
    /// Returns a hard-coded semantic version string that matches the contract
    /// crate version in `Cargo.toml`. Because the value is baked into the
    /// compiled WASM binary, no on-chain storage is read or written.
    ///
    /// SDKs and deployment tooling can call this to verify contract
    /// compatibility before executing state-changing operations.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// A string containing the contract version (e.g. `"0.1.0"`).
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let version = SavingsVault::get_version(&env);
    /// assert_eq!(version, "0.1.0");
    /// ```
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
    // Deposits
    // -----------------------------------------------------------------------

    /// Deposit funds into the caller's vault.
    ///
    /// Transfers `amount` tokens from the user into the vault via the configured Stellar
    /// Asset Contract (SAC) and then credits the user's recorded balance. The balance is
    /// updated only after the token transfer succeeds.
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
    /// - If the contract has not been initialized.
    /// - If `amount` is zero or negative.
    pub fn deposit(env: Env, user: Address, amount: i128) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

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

        // Emit deposit event
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
    /// - If the contract has not been initialized.
    /// - If `amount` is zero or negative.
    /// - If `amount` exceeds the user's available balance.
    pub fn withdraw(env: Env, user: Address, amount: i128) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

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

        let next_lock_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        let current_time = env.ledger().timestamp();
        let mut total_matured: i128 = 0;
        
        for i in 1..next_lock_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time >= lock.unlock_time {
                    total_matured += lock.amount;
                }
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
            for i in 1..next_lock_id {
                if remaining_to_deduct == 0 {
                    break;
                }
                if let Some(mut lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                    if !lock.withdrawn && current_time >= lock.unlock_time {
                        if lock.amount <= remaining_to_deduct {
                            remaining_to_deduct -= lock.amount;
                            lock.amount = 0;
                            lock.withdrawn = true;
                        } else {
                            lock.amount -= remaining_to_deduct;
                            remaining_to_deduct = 0;
                        }
                        env.storage().persistent().set(&DataKey::Lock(user.clone(), i), &lock);
                    }
                }
            }
        }

        // Calculate new_locked after withdrawal
        let mut new_locked: i128 = 0;
        for i in 1..next_lock_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time < lock.unlock_time {
                    new_locked += lock.amount;
                }
            }
        }

        // Update balance
        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);

        // Emit withdraw event
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

    /// Withdraw a specific matured lock entry.
    ///
    /// This function allows a user to withdraw the funds associated with a specific
    /// lock entry, provided that the lock has matured (current_time >= unlock_time).
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The owner of the lock (must authorize this transaction via `require_auth()`)
    /// * `lock_id` - The unique identifier of the lock to withdraw
    ///
    /// # Authorization
    ///
    /// The `user` address must sign the transaction.
    ///
    /// # Panics
    /// - If the contract has not been initialized.
    /// - If the lock with the given `lock_id` does not exist for the `user`.
    /// - If the lock has not yet matured (current_time < unlock_time).
    pub fn withdraw_lock(env: Env, user: Address, lock_id: u64) {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

        // Authorization
        user.require_auth();

        // Load lock
        let mut lock = match env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), lock_id)) {
            Some(l) => l,
            None => panic!("Lock not found"),
        };

        if lock.withdrawn {
            panic!("Lock already withdrawn");
        }

        // Verify maturity
        let current_time = env.ledger().timestamp();
        if current_time < lock.unlock_time {
            panic!("Lock has not matured yet");
        }

        // Get token address & client
        let token = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token);
        let contract_address = env.current_contract_address();

        // Perform token transfer to the user
        token_client.transfer(&contract_address, &user, &lock.amount);

        // Mark the lock as withdrawn and clear the amount
        lock.withdrawn = true;
        lock.amount = 0;

        // Save updated lock back to persistent storage
        env.storage()
            .persistent()
            .set(&DataKey::Lock(user.clone(), lock_id), &lock);

        // Emit withdrawal lock event
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
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let deposited_balance: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Balance(user.clone()))
            .unwrap_or(0);

        let next_lock_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        let current_time = env.ledger().timestamp();
        let mut matured_amount: i128 = 0;
        
        for i in 1..next_lock_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time >= lock.unlock_time {
                    matured_amount += lock.amount;
                }
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
    /// - If the contract has not been initialized.
    /// - If `amount` is zero or negative.
    /// - If `amount` exceeds the user's available balance.
    /// - If `unlock_time` is in the past.
    pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64) -> u64 {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);

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

        // Create new lock entry
        let new_lock = LockEntry {
            id: next_id,
            owner: user.clone(),
            amount,
            created_time: current_time,
            unlock_time,
            withdrawn: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Lock(user.clone(), next_id), &new_lock);

        // Move funds: available -> locked
        current_balance -= amount;

        env.storage()
            .persistent()
            .set(&DataKey::Balance(user.clone()), &current_balance);

        // Calculate new_locked for the event
        let mut new_locked: i128 = 0;
        for i in 1..=next_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time < lock.unlock_time {
                    new_locked += lock.amount;
                }
            }
        }

        // Emit lock event
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
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let next_lock_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        let current_time = env.ledger().timestamp();
        let mut total_locked: i128 = 0;
        for i in 1..next_lock_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time < lock.unlock_time {
                    total_locked += lock.amount;
                }
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
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let next_lock_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        let current_time = env.ledger().timestamp();
        for i in 1..next_lock_id {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                if !lock.withdrawn && current_time >= lock.unlock_time {
                    return true;
                }
            }
        }

        false
    }

    /// Get a single lock record for a user by lock ID.
    ///
    /// Returns the stored [`LockEntry`] when a matching record exists. Lock IDs are
    /// assigned by [`lock_funds`](Self::lock_funds) and are unique per user.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address
    /// * `lock_id` - The lock ID returned from `lock_funds`
    ///
    /// # Returns
    ///
    /// `Some(LockEntry)` when the lock exists; `None` when the user has no matching lock.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    pub fn get_lock(env: Env, user: Address, lock_id: u64) -> Option<LockEntry> {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        env.storage().persistent().get(&DataKey::Lock(user.clone(), lock_id))
    }

    /// List lock records for a user with offset/limit pagination.
    ///
    /// Returns a page of stored lock entries in creation order (oldest first).
    /// Both active and matured entries still present in storage are included.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `user` - The user's address
    /// * `offset` - Number of records to skip from the start of the list
    /// * `limit` - Maximum records to return (capped at [`MAX_LOCK_PAGE_SIZE`])
    ///
    /// # Returns
    ///
    /// A vector of up to `limit` lock entries (after capping). Returns an empty vector
    /// when the user has no locks, when `limit` is zero, or when `offset` is past the end.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    pub fn list_locks(env: Env, user: Address, offset: u32, limit: u32) -> Vec<LockEntry> {
        Self::assert_initialized(&env);
        Self::try_migrate(&env);
        Self::assert_supported_storage_version(&env);
        let next_lock_id: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::NextLockId(user.clone()))
            .unwrap_or(1);

        let total = (next_lock_id - 1) as usize;
        if limit == 0 || offset as usize >= total {
            return Vec::new(&env);
        }

        let page_limit = limit.min(MAX_LOCK_PAGE_SIZE);
        let end = (offset as usize).saturating_add(page_limit as usize).min(total);
        let mut page = Vec::new(&env);

        // Locks are 1-indexed (ids from 1 to next_lock_id - 1)
        // offset 0 means start at id 1
        for i in (offset as u64 + 1)..=(end as u64) {
            if let Some(lock) = env.storage().persistent().get::<_, LockEntry>(&DataKey::Lock(user.clone(), i)) {
                page.push_back(lock);
            }
        }
        page
    }

    // -----------------------------------------------------------------------
    // Admin Functions
    // -----------------------------------------------------------------------

    /// Get the admin address.
    ///
    /// Returns the address stored as admin during contract initialization.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// The admin `Address`.
    ///
    /// # Authorization
    ///
    /// No authorization required (read-only operation).
    pub fn get_admin(env: Env) -> Address {
        Self::assert_initialized(&env);
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    /// Transfer admin privileges to a new address.
    ///
    /// This function replaces the current admin address with a new one. Only the current admin
    /// can call this function.
    ///
    /// # Arguments
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The current admin address (must authorize this transaction)
    /// * `new_admin` - The new admin address to set
    ///
    /// # Authorization
    ///
    /// The `admin` address must sign the transaction.
    ///
    /// # State Changes
    ///
    /// - Updates the admin address in instance storage
    /// - Emits an event with the old and new admin addresses
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) {
        Self::assert_initialized(&env);
        admin.require_auth();
        Self::assert_admin(&env, &admin);

        let old_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        env.storage().instance().set(&DataKey::Admin, &new_admin);

        // Emit transfer_admin event
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
