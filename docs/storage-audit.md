# Storage Audit Map and Mutation Trace

This document provides a comprehensive audit of all storage usage in the PocketPay Savings Vault contract. It maps storage keys, value types, mutation points, invariants, and test coverage to ensure a high-security posture for fund custody and accounting.

## 1. Storage Key Map

The contract uses Soroban's **Instance** storage for global configuration and **Persistent** storage for user-specific data to optimize for resource usage and scalability.

| DataKey Variant | Storage Type | Value Type | Ownership | Description |
| :--- | :--- | :--- | :--- | :--- |
| `Admin` | Instance | `Address` | Global | The current administrator address for configuration and pause controls. |
| `Initialized` | Instance | `bool` | Global | Flag indicating if the contract has been successfully initialized. |
| `Token` | Instance | `Address` | Global | The address of the SAC token managed by this vault. |
| `StorageVersion` | Instance | `u64` | Global | Current schema version for the contract storage. |
| `Paused` | Instance | `bool` | Global | Current pause status of the vault. |
| `PauseExpiry` | Instance | `u64` | Global | Timestamp (ledger seconds) when the current pause automatically expires. |
| `MinDepositAmount` | Instance | `i128` | Global | Minimum amount required for a single deposit operation. |
| `MaxLockDurationSecs` | Instance | `u64` | Global | Maximum duration a lock can be active. |
| `MinLockDurationSecs` | Instance | `u64` | Global | Minimum duration required for a new lock. |
| `Balance(Address)` | Persistent | `i128` | User | Total balance (available + locked) for a specific user. |
| `Lock(Address, u64)` | Persistent | `LockEntry` | User | Individual lock record keyed by user and a monotonic ID. |
| `NextLockId(Address)` | Persistent | `u64` | User | Monotonic counter used to generate unique IDs for a user's locks. |

*Note: The `Locks(Address)` variant is defined in the enum but is currently unused in favor of individual `Lock(Address, u64)` entries.*

---

## 2. Mutation Trace

The following table tracks which functions mutate specific storage keys. Read-only operations are omitted for brevity.

| Function | Mutated Keys | State Change Description |
| :--- | :--- | :--- |
| `initialize` | `Admin`, `Initialized`, `Token`, `StorageVersion` | Sets global config; locks initialization. |
| `try_migrate` | `StorageVersion` | Increments schema version during upgrades. |
| `pause` | `Paused`, `PauseExpiry` | Enables emergency pause with an expiry timestamp. |
| `unpause` | `Paused`, `PauseExpiry` | Manually clears pause state. |
| `require_not_paused` | `Paused`, `PauseExpiry` | **Lazy Mutation**: Clears pause if `ledger.timestamp() >= expiry`. |
| `set_min_deposit_amount`| `MinDepositAmount` | Updates global minimum deposit threshold. |
| `set_max_lock_duration` | `MaxLockDurationSecs` | Updates global maximum lock time. |
| `set_min_lock_duration` | `MinLockDurationSecs` | Updates global minimum lock time. |
| `deposit` | `Balance(user)` | Increments user balance after successful token transfer. |
| `withdraw` | `Balance(user)` | Decrements user balance after successful token transfer. |
| `lock_funds` | `Balance(user)`, `Lock(user, id)`, `NextLockId(user)` | Debits available balance, writes lock entry, increments ID counter. |
| `extend_lock` | `Lock(user, id)` | Updates the `unlock_time` of an existing lock entry. |
| `withdraw_lock` | `Lock(user, id)` | Zeroes out the lock entry amount after maturity and token transfer. |
| `transfer_admin` | `Admin` | Rotates the administrative address. |

---

## 3. Failure-Path State Expectations

The contract employs a **"transfer-then-write"** pattern to ensure atomicity. If a transaction fails at any point, all storage changes are rolled back by the Soroban host environment.

| Failure Scenario | Expected State | Implementation Detail |
| :--- | :--- | :--- |
| **Token Transfer Fails** | No mutation to `Balance` or `Lock`. | Token transfer is attempted *before* storage writes in `deposit` and `withdraw`. |
| **Auth Failure** | No mutation to any key. | `require_auth()` is called at the start of all protected methods. |
| **Invariant Violation** | Transaction panics; all state reverted. | Invariants are checked at the end of functions or via guards. |
| **Vault Paused** | `require_not_paused` panics (unless lazy clearing). | Mutations are blocked early in the call stack. |

---

## 4. Invariants and Test Coverage

Core accounting and security invariants are verified through a combination of unit tests and property-based tests.

| Invariant | Description | Test Reference |
| :--- | :--- | :--- |
| **Balance Conservation** | `Available + Locked == Total` for all users at all times. | [balance_conservation.rs](file:///c:/Users/abbat/.trae/GrantFox/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| **Token Custody** | `Contract SAC Balance == Σ(User Balances)`. | [property_fee_invariants.rs](file:///c:/Users/abbat/.trae/GrantFox/pocketpay-contracts/contracts/savings_vault/src/test/property_fee_invariants.rs) |
| **Atomic Rollback** | Failed transfers must not credit/debit internal accounting. | [token_transfer_rollback.rs](file:///c:/Users/abbat/.trae/GrantFox/pocketpay-contracts/contracts/savings_vault/src/test/token_transfer_rollback.rs) |
| **Lock Integrity** | `Locked Balance == Σ(Active Lock Entries)`. | [multi_lock_invariants.rs](file:///c:/Users/abbat/.trae/GrantFox/pocketpay-contracts/contracts/savings_vault/src/test/multi_lock_invariants.rs) |
| **ID Uniqueness** | `NextLockId` must never produce a duplicate ID for a user. | [multi_lock_invariants.rs](file:///c:/Users/abbat/.trae/GrantFox/pocketpay-contracts/contracts/savings_vault/src/test/multi_lock_invariants.rs) |

---

## 5. Technical Debt and Audit Notes

- **Unused Storage Variant**: `DataKey::Locks(Address)` should be removed or implemented to avoid confusion during audits.
- **TTL Management**: The audit currently assumes standard Soroban TTL management for Persistent/Instance storage. A dedicated TTL extension strategy should be documented if custom intervals are required.
- **Linear Scan Risk**: Functions like `get_balance_snapshot` and `list_locks` iterate over lock IDs. While capped by `MAX_LOCK_PAGE_SIZE`, high lock counts per user could impact gas costs for complex read-aggregations.
