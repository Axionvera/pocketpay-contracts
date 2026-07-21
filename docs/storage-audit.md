# Storage Audit — Savings Vault Contract

This document provides a detailed storage audit for the Savings Vault smart contract. It maps the contract's Soroban storage keys to their respective mutating functions, defines their expected types and storage layers, and lists critical storage invariants.

> **Important:** This contract is designed for educational and testnet use. It is not production-ready or mainnet-ready.

---

## 1. Storage Keys and Schema

The contract uses the [DataKey](../contracts/savings_vault/src/lib.rs#L113-L126) enum to define all storage locations. Storage is split between **Instance** storage (tied to the contract lifetime) and **Persistent** storage (associated with individual user state and requires active TTL management).

### Storage Layout Summary

| Key Variant | Storage Layer | Rust Type | Description |
|---|---|---|---|
| `DataKey::Admin` | Instance | `Address` | The address of the contract administrator. |
| `DataKey::Initialized` | Instance | `bool` | Flag indicating if the contract has been initialized. |
| `DataKey::Token` | Instance | `Address` | The address of the token contract (SAC) used for transfers. |
| `DataKey::Balance(Address)` | Persistent | `i128` | The available (unlocked) balance of a specific user. |
| `DataKey::Locks(Address)` | Persistent | `Vec<LockEntry>` | A vector of active or matured lock records for a user. |
| `DataKey::NextLockId(Address)` | Persistent | `u64` | A sequential counter for generating unique lock IDs per user. |

---

## 2. Storage Entries Detail

### `DataKey::Admin`
- **Storage Layer**: Instance
- **Value Type**: `Address`
- **Initialization**: Set once during [initialize](../contracts/savings_vault/src/lib.rs#L219-L238).
- **Mutating Functions**: None (read-only after initialization).
- **Invariants**: 
  - Must represent a valid authorized address on the network.
  - Can never be updated or overwritten once set.

### `DataKey::Initialized`
- **Storage Layer**: Instance
- **Value Type**: `bool`
- **Initialization**: Set once to `true` during [initialize](../contracts/savings_vault/src/lib.rs#L219-L238).
- **Mutating Functions**: None (read-only after initialization).
- **Invariants**:
  - Once set to `true`, any subsequent calls to [initialize](../contracts/savings_vault/src/lib.rs#L219-L238) will panic.
  - All state-changing functions and queries (except `get_version`) assert that this key is set to `true`.

### `DataKey::Token`
- **Storage Layer**: Instance
- **Value Type**: `Address`
- **Initialization**: Set once during [initialize](../contracts/savings_vault/src/lib.rs#L219-L238).
- **Mutating Functions**: None (read-only after initialization).
- **Invariants**:
  - Must represent a valid token contract address (Stellar Asset Contract) deployed on the network.
  - Can never be updated or overwritten once set.

### `DataKey::Balance(Address)`
- **Storage Layer**: Persistent
- **Value Type**: `i128`
- **Initialization**: Created on first [deposit](../contracts/savings_vault/src/lib.rs#L303-L350).
- **Mutating Functions**:
  - [deposit](../contracts/savings_vault/src/lib.rs#L303-L350): Increments the balance by the deposited amount.
  - [withdraw](../contracts/savings_vault/src/lib.rs#L384-L481): Decrements the balance (and potentially modifies matured locks).
  - [lock_funds](../contracts/savings_vault/src/lib.rs#L577-L663): Decrements the available balance by the locked amount.
- **Invariants**:
  - `Balance(user) >= 0` at all times.
  - Only modified when authorization is successfully checked via `user.require_auth()`.

### `DataKey::Locks(Address)`
- **Storage Layer**: Persistent
- **Value Type**: `Vec<LockEntry>` where [LockEntry](../contracts/savings_vault/src/lib.rs#L88-L92) is defined as:
  ```rust
  pub struct LockEntry {
      pub id: u64,
      pub amount: i128,
      pub unlock_time: u64,
  }
  ```
- **Initialization**: Created on first call to [lock_funds](../contracts/savings_vault/src/lib.rs#L577-L663).
- **Mutating Functions**:
  - [lock_funds](../contracts/savings_vault/src/lib.rs#L577-L663): Appends a new `LockEntry` to the user's locks vector.
  - [withdraw](../contracts/savings_vault/src/lib.rs#L384-L481): Consumes matured locks (deleting fully consumed ones and updating the remaining amount on partially consumed ones) if the withdrawal amount exceeds the user's basic available balance.
- **Invariants**:
  - For each `LockEntry`, `amount > 0` must hold true.
  - When created, `unlock_time` must be in the future relative to the current ledger timestamp (`unlock_time > env.ledger().timestamp()`).
  - Active and matured locks remain in this vector until a [withdraw](../contracts/savings_vault/src/lib.rs#L384-L481) transaction explicitly processes/consumes them.
  - Total funds tracked in locks for a user must equal the sum of all `amount` values in the vector.

### `DataKey::NextLockId(Address)`
- **Storage Layer**: Persistent
- **Value Type**: `u64`
- **Initialization**: Defaults to `1` if not found. Created and set to `2` during the first call to [lock_funds](../contracts/savings_vault/src/lib.rs#L577-L663).
- **Mutating Functions**:
  - [lock_funds](../contracts/savings_vault/src/lib.rs#L577-L663): Increments the ID counter by 1.
- **Invariants**:
  - Monotonically increasing counter starting at `1`.
  - For any user lock index, the generated ID will satisfy `NextLockId(user) > max(lock.id for lock in Locks(user))` (unless locks are empty).

---

## 3. Storage Mutation Mapping

The following matrix maps contract entry points to their impact on each storage key (No Access, Read, Write, or Read/Write):

| Function | `Admin` | `Initialized` | `Token` | `Balance(user)` | `Locks(user)` | `NextLockId(user)` |
|---|---|---|---|---|---|---|
| `initialize` | Write | Write (Check/Write) | Write | No Access | No Access | No Access |
| `get_version` | No Access | No Access | No Access | No Access | No Access | No Access |
| `deposit` | No Access | Read | Read | Read & Write | No Access | No Access |
| `withdraw` | No Access | Read | Read | Read & Write | Read & Write | No Access |
| `get_balance` | No Access | Read | No Access | Read | Read | No Access |
| `lock_funds` | No Access | Read | No Access | Read & Write | Read & Write | Read & Write |
| `get_locked_balance`| No Access | Read | No Access | No Access | Read | No Access |
| `can_withdraw` | No Access | Read | No Access | No Access | Read | No Access |
| `get_lock` | No Access | Read | No Access | No Access | Read | No Access |
| `list_locks` | No Access | Read | No Access | No Access | Read | No Access |

---

## 4. Key Security Invariants & Potential Risks

### 4.1 Balance Invariant
The total custody balance of the contract on-chain (the Stellar Asset Contract balance of the contract's address) must always equal the sum of all users' `Balance(user)` entries and the amounts in all users' `Locks(user)` entries.
$$\text{Contract Token Balance} = \sum_{u} \left( \text{Balance}(u) + \sum_{l \in \text{Locks}(u)} l.\text{amount} \right)$$
*Security Impact*: Since Soroban contract balances are backed by the SAC, any discrepancy where internal accounting exceeds the actual contract balance would result in transaction panics during withdrawal.

### 4.2 Storage Expiry (TTL) Hazards
Since user balances use **Persistent** storage, they are subject to expiration if not read/written (or extended) within the network's maximum TTL period.
- **Risk**: If `Balance(user)` expires, the contract defaults to `0` via `unwrap_or(0)` logic. If a user subsequently deposits or checks their balance, they might read `0` or write a new balance that does not include their prior funds, potentially leading to state loss.
- **Mitigation**: Users must interact with the contract or invoke `extend` to keep their persistent entries active. See [Storage TTL](storage-ttl.md) for detailed guidelines.

### 4.3 Lock Manipulation
Matured locks are not moved automatically back to the available balance. Instead, they remain in the `Locks` vector until a `withdraw` is triggered.
- **Invariance**: A lock is only consumed when the user requests a withdrawal of an amount that exceeds their available balance.
- **Risk**: If the `Locks` vector grows too large (e.g., thousands of lock entries per user), iterating over all locks in `withdraw`, `get_balance`, or `get_locked_balance` could exceed CPU instructions or memory limits, causing denial of service (DoS) for the user.
