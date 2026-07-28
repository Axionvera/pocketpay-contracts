# Vault Contract Public API Reference

## Overview

The vault contract provides secure storage and management of user funds with lock-based access control.

## Version

**Current Version**: 1.0.0

**Status**: Stable

**Compatibility**: Backward-compatible changes only until v2.0.0

---

## Public Functions

### Initialization

#### `initialize`

Initializes the vault contract with the owner address.

**Function Signature**
```rust
pub fn initialize(env: Env, owner: Address) -> Result<(), VaultError>
event VaultInitialized {
    owner: Address,
    timestamp: u64,
}
pub fn deposit(env: Env, user: Address, amount: i128) -> Result<(), VaultError>
event Deposit {
    user: Address,
    amount: i128,
    new_balance: i128,
    timestamp: u64,
}
pub fn withdraw(env: Env, user: Address, amount: i128) -> Result<(), VaultError>
event Withdraw {
    user: Address,
    amount: i128,
    new_balance: i128,
    timestamp: u64,
}
pub fn create_lock(
    env: Env,
    user: Address,
    amount: i128,
    lock_duration: u64,
) -> Result<u64, VaultError>
event LockCreated {
    lock_id: u64,
    user: Address,
    amount: i128,
    duration: u64,
    unlock_time: u64,
    timestamp: u64,
}
pub fn withdraw_lock(env: Env, user: Address, lock_id: u64) -> Result<(), VaultError>
event LockWithdrawn {
    lock_id: u64,
    user: Address,
    amount: i128,
    timestamp: u64,
}
pub fn get_balance(env: Env, user: Address) -> Result<i128, VaultError>
pub fn get_lock(env: Env, lock_id: u64) -> Result<LockData, VaultError>
pub fn get_user_locks(env: Env, user: Address) -> Result<Vec<u64>, VaultError>
pub fn get_total_balance(env: Env) -> Result<i128, VaultError>
pub fn transfer_ownership(env: Env, new_owner: Address) -> Result<(), VaultError>
event OwnershipTransferred {
    old_owner: Address,
    new_owner: Address,
    timestamp: u64,
}
pub fn pause(env: Env) -> Result<(), VaultError>
pub fn unpause(env: Env) -> Result<(), VaultError>
struct LockData {
    user: Address,
    amount: i128,
    unlock_time: u64,
    withdrawn: bool,
    created_at: u64,
}
struct LockData {
    user: Address,
    amount: i128,
    unlock_time: u64,
    withdrawn: bool,
    created_at: u64,
}
pub fn deposit(env: Env, amount: i128) -> Result<(), VaultError>
pub fn deposit(env: Env, user: Address, amount: i128) -> Result<(), VaultError>
// v1.0.0
deposit(env, amount);

// v2.0.0
deposit(env, user, amount);
pub fn deposit(env: Env, amount: i128) -> Result<(), VaultError>
pub fn deposit(env: Env, amount: i128, memo: Option<String>) -> Result<(), VaultError>
