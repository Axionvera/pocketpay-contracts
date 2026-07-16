# Storage Model — Savings Vault

This document describes the on-chain storage design for the Savings Vault smart contract. It details the storage keys, expected value types, and the categorization of storage layout (Instance vs. Persistent storage) in Stellar Soroban.

---

## Soroban Storage Types Overview

The Savings Vault contract utilizes two types of storage provided by the Soroban environment:

1. **Instance Storage**: Stored alongside the contract's instance bytecode. This is ideal for global contract configuration, metadata, and flags that are accessed frequently and apply to the contract as a whole.
2. **Persistent Storage**: Stored as separate ledger entries. This is ideal for user-specific data or entries that can scale dynamically. Persistent storage has separate rent/state expiry considerations from the main contract instance.

---

## Storage Key & Value Mapping

All storage entries are identified using keys from the `DataKey` enum defined in the contract.

| Enum Key Variant | Value Type | Storage Classification | Description | Default Fallback |
| :--- | :--- | :--- | :--- | :--- |
| `DataKey::Admin` | `Address` | **Instance** | The address of the contract administrator. | *None (Panics if not set)* |
| `DataKey::Initialized` | `bool` | **Instance** | A boolean flag indicating if initialization has run. | *None (Fails check)* |
| `DataKey::Balance(Address)` | `i128` | **Persistent** | The unlocked, withdrawable balance of a specific user. | `0` |
| `DataKey::LockedBalance(Address)` | `i128` | **Persistent** | The balance currently locked under a time-lock constraint. | `0` |
| `DataKey::UnlockTime(Address)` | `u64` | **Persistent** | The UNIX timestamp (seconds) when the locked funds unlock. | `0` |

---

## Detailed Key Definitions & Access Patterns

### 1. Admin Address
* **Key**: `DataKey::Admin`
* **Storage Type**: Instance Storage
* **Value Type**: `Address`
* **Access Mode**: Read / Write (Set once during initialization)
* **Usage**:
  * Set during the invocation of `initialize(env, admin)`.
  * Persisted via `env.storage().instance().set(&DataKey::Admin, &admin)`.

### 2. Initialization Flag
* **Key**: `DataKey::Initialized`
* **Storage Type**: Instance Storage
* **Value Type**: `bool`
* **Access Mode**: Read / Write (Set once during initialization)
* **Usage**:
  * Used to prevent multiple initialization calls.
  * Verified using `env.storage().instance().has(&DataKey::Initialized)`.
  * Set to `true` during `initialize(...)` via `env.storage().instance().set(&DataKey::Initialized, &true)`.

### 3. User Balance
* **Key**: `DataKey::Balance(Address)`
* **Storage Type**: Persistent Storage
* **Value Type**: `i128`
* **Access Mode**: Read / Write
* **Usage**:
  * Tracks deposits, withdrawals, and locks.
  * Retrieved using `env.storage().persistent().get(&DataKey::Balance(user))`. Defaults to `0` if not present.
  * Updated during `deposit()`, `withdraw()`, and `lock_funds()`.

### 4. Locked Balance
* **Key**: `DataKey::LockedBalance(Address)`
* **Storage Type**: Persistent Storage
* **Value Type**: `i128`
* **Access Mode**: Read / Write
* **Usage**:
  * Tracks funds that have been locked by the user.
  * Retrieved using `env.storage().persistent().get(&DataKey::LockedBalance(user))`. Defaults to `0` if not present.
  * Increased when `lock_funds()` is called.

### 5. Unlock Time
* **Key**: `DataKey::UnlockTime(Address)`
* **Storage Type**: Persistent Storage
* **Value Type**: `u64` (Unix epoch seconds)
* **Access Mode**: Read / Write
* **Usage**:
  * The release timestamp for user's locked funds.
  * Retrieved using `env.storage().persistent().get(&DataKey::UnlockTime(user))`. Defaults to `0` if not present.
  * Checked by `can_withdraw()` against `env.ledger().timestamp()`.

---

## Where to Find in Code

The storage definition is located in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs):
* Storage enum definition: [`DataKey`](../contracts/savings_vault/src/lib.rs#L23-L36)
* Initialization check: [`initialize()`](../contracts/savings_vault/src/lib.rs#L61-L75)
* User balance mutations: [`deposit()`](../contracts/savings_vault/src/lib.rs#L99-L110) & [`withdraw()`](../contracts/savings_vault/src/lib.rs#L136-L152)
* Lock mechanisms: [`lock_funds()`](../contracts/savings_vault/src/lib.rs#L205-L236)
