# Authorisation Rules & Security Matrix — Savings Vault

This document provides a comprehensive security reference and audit matrix detailing the authorisation rules, expected callers, authentication primitives, and security assumptions for all public functions in the **Savings Vault Contract** (`contracts/savings_vault`).

---

## 1. Public Function Authorisation Matrix

| Function Name | Authorisation Required | Enforced By | State Mutating? | Expected Caller | Risk Level |
|---|---|---|---|---|---|
| `initialize(admin, token)` | `admin.require_auth()` | Soroban Host | Yes | Deployer / Contract Admin | **HIGH** |
| `deposit(user, amount)` | `user.require_auth()` | Soroban Host | Yes | Depositing Account Owner | **MEDIUM** |
| `withdraw(user, amount)` | `user.require_auth()` | Soroban Host | Yes | Account Owner Only | **HIGH** |
| `lock_funds(user, amount, unlock_time)` | `user.require_auth()` | Soroban Host | Yes | Account Owner Only | **MEDIUM** |
| `withdraw_lock(user, lock_id)` | `user.require_auth()` | Soroban Host | Yes | Account Owner Only | **HIGH** |
| `extend_lock(user, lock_id, new_unlock_time)` | `user.require_auth()` | Soroban Host | Yes | Account Owner Only | **MEDIUM** |
| `pause(admin, duration)` | `admin.require_auth()` | Soroban Host | Yes | Contract Admin Only | **MEDIUM** |
| `unpause(admin)` | `admin.require_auth()` | Soroban Host | Yes | Contract Admin Only | **MEDIUM** |
| `transfer_admin(admin, new_admin)` | `admin.require_auth()` | Soroban Host | Yes | Current Contract Admin | **HIGH** |
| `get_balance(user)` | None (Public Query) | N/A | No (Read-only) | Any Account / Indexer / Frontend | **LOW** |
| `get_locked_balance(user)` | None (Public Query) | N/A | No (Read-only) | Any Account / Indexer / Frontend | **LOW** |
| `can_withdraw(user)` | None (Public Query) | N/A | No (Read-only) | Any Account / Indexer / Frontend | **LOW** |

---

## 2. Function-by-Function Authorisation Details

### 2.1 `initialize(env: Env, admin: Address, token: Address)`
* **Authorisation Rule:** Requires a valid cryptographic signature from the `admin` address.
* **Mechanism:** Explicit call to `admin.require_auth()`.
* **Single-Invocation Protection:** Enforces instance storage validation. Re-invocation panics with `ContractError::AlreadyInitialized`.
* **Known Assumptions:** The deployer provides a valid SAC `token` address. The recorded `admin` address does not have special privileges over user vaults after initialization.

### 2.2 `deposit(env: Env, user: Address, amount: i128)`
* **Authorisation Rule:** Requires authorization from `user`.
* **Mechanism:** `user.require_auth()`.
* **Caller Expectation:** The vault owner depositing funds on their own behalf.
* **Protection Against Misuse:** An arbitrary third party cannot invoke `deposit` for `user` without `user`'s signature, preventing unauthorized account creation or token-locking scenarios.

### 2.3 `withdraw(env: Env, user: Address, amount: i128)`
* **Authorisation Rule:** Requires authorization strictly from `user`.
* **Mechanism:** `user.require_auth()`.
* **Caller Expectation:** Only the account owner who owns the underlying deposited balance or matured time-locks can authorize a withdrawal.
* **Cross-User Protection:** If `attacker` calls `withdraw(alice, 500)` signed by `attacker`, the Soroban host rejects the invocation because `alice` did not authorize the transaction tree.
* **Admin Limitation:** The contract `admin` **cannot** withdraw funds from `user`'s vault. `withdraw` checks `user.require_auth()`, not `admin.require_auth()`.

### 2.4 `lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64)`
* **Authorisation Rule:** Requires authorization from `user`.
* **Mechanism:** `user.require_auth()`.
* **Caller Expectation:** Account owner locking a portion of their liquid balance until `unlock_time`.
* **Protection Against Misuse:** Third parties cannot lock another user's liquid funds to grief them or restrict their liquidity.

### 2.5 `withdraw_lock(env: Env, user: Address, lock_id: u64)`
* **Authorisation Rule:** Requires authorization from `user`.
* **Mechanism:** `user.require_auth()`.
* **Caller Expectation:** Account owner claiming a matured time-locked position.
* **Protection Against Misuse:** Third parties cannot trigger premature lock withdrawals or withdraw matured locks on behalf of another user to execute them into an arbitrary destination.

### 2.6 `extend_lock(env: Env, user: Address, lock_id: u64, new_unlock_time: u64)`
* **Authorisation Rule:** Requires authorization from `user`.
* **Mechanism:** `user.require_auth()`.
* **Caller Expectation:** Account owner increasing the lock duration of an existing lock entry.
* **Protection Against Misuse:** Unauthorised parties cannot force lock extensions on user vaults.

### 2.7 `pause(env: Env, admin: Address, duration: u64)`
* **Authorisation Rule:** Requires authorization from the configured `admin`.
* **Mechanism:** `admin.require_auth()` paired with storage validation checking `admin == stored_admin`.
* **Caller Expectation:** The contract administrator freezing new deposits and lock actions in an emergency.

### 2.8 `unpause(env: Env, admin: Address)`
* **Authorisation Rule:** Requires authorization from the configured `admin`.
* **Mechanism:** `admin.require_auth()` paired with storage validation checking `admin == stored_admin`.
* **Caller Expectation:** The contract administrator resuming contract operations.

### 2.9 `transfer_admin(env: Env, admin: Address, new_admin: Address)`
* **Authorisation Rule:** Requires authorization from the current `admin`.
* **Mechanism:** `admin.require_auth()` paired with verification that `admin == stored_admin`.
* **Caller Expectation:** The current administrator rotating the admin role to a new address.

### 2.10 Read-Only Queries (`get_balance`, `get_locked_balance`, `can_withdraw`)
* **Authorisation Rule:** None.
* **Mechanism:** Unauthenticated view functions reading persistent storage (`DataKey::Balance`, `DataKey::Locks`).
* **Caller Expectation:** Publicly accessible for off-chain mobile wallets, explorers, and indexers. State cannot be modified through read-only queries.

---

## 3. Multi-Tenant Balance Isolation & Storage Keys

Security and multi-tenant isolation are preserved by pairing Soroban's Host authentication with address-derived storage keys:

```rust
DataKey::Balance(user: Address)
DataKey::Lock(user: Address, lock_id: u64)
DataKey::NextLockId(user: Address)
```

1. **Storage Isolation:** User $A$'s balance is key-partitioned under `Balance(Address_A)` and cannot collide with User $B$'s entry under `Balance(Address_B)`.
2. **Auth Binding:** Any state change targeting `Balance(Address_A)` MUST present a valid host authorization payload signed by `Address_A`.

---

## 4. Known Security Assumptions & Boundaries

1. **Soroban Host Cryptographic Verification:** The contract relies on the Soroban host implementation for signature parsing, replay protection, and address authentication via `require_auth()`.
2. **Inert Admin Role:** The recorded `admin` address has zero administrative override powers over user vaults (no sweep, no force-unlock, no proxy-drain).
3. **SAC Token Contract Security:** `withdraw` delegates real token transfers to the SAC token address configured during `initialize`. The SAC token contract must adhere to the standard Soroban token interface.

---

## 5. Misuse Test Verification

The unit test suite in [`contracts/savings_vault/src/test/unauthorized_access.rs`](../contracts/savings_vault/src/test/unauthorized_access.rs) explicitly verifies authorization and cross-user misuse protections:

- `test_unauthorized_deposit_fails`: Verifies unauthenticated deposits are rejected by the Host.
- `test_unauthorized_withdraw_fails`: Verifies unauthenticated withdrawals are rejected.
- `test_unauthorized_lock_fails`: Verifies unauthenticated lock attempts are rejected.
- `test_unauthorized_withdraw_lock_fails`: Verifies unauthenticated lock withdrawal attempts are rejected.
- `test_unauthorized_extend_lock_fails`: Verifies unauthenticated lock extension attempts are rejected.
- `test_transfer_admin_unauthorized_caller_panics`: Verifies non-admin accounts cannot execute admin rotations.
- `test_unauthorized_pause_fails`: Verifies non-admin accounts cannot pause the vault.
- `test_unauthorized_unpause_fails`: Verifies non-admin accounts cannot unpause the vault.
