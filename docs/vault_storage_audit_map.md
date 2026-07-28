# Vault Storage Audit Map

This document maps all storage entries for the Savings Vault contract, including keys, value types, mutating functions, and invariants. This map supports audit preparation by providing a clear overview of state management.

## 1. Storage Keys & Value Types

| Storage Key | Layer | Rust Type | Description |
|---|---|---|---|
| `DataKey::Admin` | Instance | `Address` | Contract administrator address. |
| `DataKey::Initialized` | Instance | `bool` | Flag indicating contract initialization. |
| `DataKey::Token` | Instance | `Address` | Token contract (SAC) address for transfers. |
| `DataKey::Balance(Address)` | Persistent | `i128` | Available (unlocked) balance of a user. |
| `DataKey::Locks(Address)` | Persistent | `Vec<LockEntry>` | Active or matured lock records for a user. |
| `DataKey::NextLockId(Address)` | Persistent | `u64` | Counter for generating unique lock IDs per user. |

## 2. Mutating Functions

| Function | Storage Keys Modified |
|---|---|
| `initialize` | `Admin`, `Initialized`, `Token` |
| `deposit` | `Balance(user)` |
| `withdraw` | `Balance(user)`, `Locks(user)` |
| `withdraw_lock` | `Locks(user)` |
| `lock_funds` | `Balance(user)`, `Locks(user)`, `NextLockId(user)` |

## 3. Storage Invariants

- **Admin**: Set exactly once during initialization. Cannot be changed. Must be a valid `Address`.
- **Initialized**: Set to `true` during initialization. Prevents re-initialization.
- **Token**: Set exactly once during initialization. Must be a valid token contract address.
- **Balance**: Must always be $\ge 0$. Modifications must be authorized by `user.require_auth()`.
- **Locks**:
  - Each `LockEntry` must have `amount > 0`.
  - When a lock is created, its `unlock_time` must be strictly greater than the current ledger timestamp.
- **NextLockId**: Must monotonically increase starting from `1` for each user.
- **Global Accounting**: The total contract token balance must equal the sum of all users' `Balance` plus the sum of all users' locked amounts.
