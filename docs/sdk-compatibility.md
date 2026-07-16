# SDK Compatibility Reference — Savings Vault

This document defines the compatibility surface of the Savings Vault smart contract for SDK maintainers and client developers. It details how the contract's Rust functions map to automatically generated JavaScript/TypeScript SDK helpers (using `soroban contract bindings`), including parameters, return types, authorization requirements, and key limitations.

---

## SDK Overview

When generating client bindings using the Soroban CLI:
```bash
soroban contract bindings typescript --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm --output-dir ./sdk
```
The output is a client class containing async helper methods mapping to the contract's public interface. All input types like `Address` are represented as strings (public keys) in JavaScript/TypeScript, and `i128`/`u64` are mapped to JS `bigint`.

---

## Function Mappings

### 1. `initialize`
Initializes the contract with an admin address. This must be invoked exactly once post-deployment.

- **Rust Signature**: 
  ```rust
  pub fn initialize(env: Env, admin: Address)
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async initialize(args: { admin: string }, options?: { signAndSubmit?: boolean }): Promise<void>
  ```
- **Parameters**:
  * `admin` (`string`): The Stellar public key of the account to register as the contract admin.
- **Return Value**: `void` (`Promise<void>`)
- **Authorization**: Requires the signature of the `admin` address (`admin.require_auth()`).
- **Error Conditions**: 
  * Panics with `"Contract is already initialized"` if called more than once.

---

### 2. `deposit`
Deposits funds (internal balance tracking) into the user's available vault balance.

- **Rust Signature**:
  ```rust
  pub fn deposit(env: Env, user: Address, amount: i128)
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async deposit(args: { user: string, amount: bigint }, options?: { signAndSubmit?: boolean }): Promise<void>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the depositor.
  * `amount` (`bigint`): The amount to deposit. Must be greater than zero.
- **Return Value**: `void` (`Promise<void>`)
- **Authorization**: Requires the signature of the `user` address (`user.require_auth()`).
- **Error Conditions**:
  * Panics with `"Deposit amount must be greater than zero"` if `amount <= 0`.

---

### 3. `withdraw`
Withdraws available (unlocked) funds from the user's vault.

- **Rust Signature**:
  ```rust
  pub fn withdraw(env: Env, user: Address, amount: i128)
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async withdraw(args: { user: string, amount: bigint }, options?: { signAndSubmit?: boolean }): Promise<void>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the withdrawer.
  * `amount` (`bigint`): The amount to withdraw. Must be greater than zero.
- **Return Value**: `void` (`Promise<void>`)
- **Authorization**: Requires the signature of the `user` address (`user.require_auth()`).
- **Error Conditions**:
  * Panics with `"Withdrawal amount must be greater than zero"` if `amount <= 0`.
  * Panics with `"Insufficient balance"` if `amount` exceeds the user's available balance.

---

### 4. `get_balance`
Queries the available (unlocked and withdrawable) balance for a specific user.

- **Rust Signature**:
  ```rust
  pub fn get_balance(env: Env, user: Address) -> i128
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async get_balance(args: { user: string }): Promise<bigint>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the user.
- **Return Value**: `bigint` representing the available balance. Returns `0` if the user has no record.
- **Authorization**: None (Read-only query).

---

### 5. `lock_funds`
Locks a portion of the user's available balance until a specified Unix timestamp. Locked funds are moved from the available balance to the locked balance.

- **Rust Signature**:
  ```rust
  pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64)
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async lock_funds(args: { user: string, amount: bigint, unlock_time: bigint }, options?: { signAndSubmit?: boolean }): Promise<void>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the user locking funds.
  * `amount` (`bigint`): The amount of available balance to lock. Must be greater than zero.
  * `unlock_time` (`bigint` / `number`): The Unix epoch timestamp (in seconds) when the locked funds unlock. Must be in the future.
- **Return Value**: `void` (`Promise<void>`)
- **Authorization**: Requires the signature of the `user` address (`user.require_auth()`).
- **Error Conditions**:
  * Panics with `"Lock amount must be greater than zero"` if `amount <= 0`.
  * Panics with `"Unlock time must be in the future"` if `unlock_time` is less than or equal to the current ledger timestamp.
  * Panics with `"Insufficient balance to lock"` if `amount` is greater than the user's available balance.

---

### 6. `get_locked_balance`
Queries the total locked balance for a specific user.

- **Rust Signature**:
  ```rust
  pub fn get_locked_balance(env: Env, user: Address) -> i128
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async get_locked_balance(args: { user: string }): Promise<bigint>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the user.
- **Return Value**: `bigint` representing the locked balance. Returns `0` if the user has no locked funds.
- **Authorization**: None (Read-only query).

---

### 7. `can_withdraw`
Checks whether a user's locked funds have unlocked and are available for withdrawal.

- **Rust Signature**:
  ```rust
  pub fn can_withdraw(env: Env, user: Address) -> bool
  ```
- **SDK Helper (TypeScript)**:
  ```typescript
  async can_withdraw(args: { user: string }): Promise<boolean>
  ```
- **Parameters**:
  * `user` (`string`): The Stellar public key of the user.
- **Return Value**: `boolean`. Returns `true` if the user has locked funds and the current ledger timestamp is greater than or equal to the unlock time. Otherwise, returns `false`.
- **Authorization**: None (Read-only query).

---

## Important Integration Notes & Limitations

> [!WARNING]
> **No External Token / XLM Transfer**
> The current contract version tracks balances internally using internal ledger state variables. It does **not** transfer actual XLM or other Stellar Asset Contract (SAC) tokens. Integration with the SAC (via `token.transfer`) is a known limitation to be addressed in subsequent releases.

> [!IMPORTANT]
> **Single Unlock Time Overwrite**
> Calling `lock_funds` multiple times updates/overwrites the single `UnlockTime` stored for the user, rather than maintaining multiple independent lock schedules. If a user already has locked funds, locking more funds will extend or shorten the unlock time for the *entire* locked balance to the new timestamp.

> [!NOTE]
> **No Admin Recovery & Upgrades**
> There are currently no admin functions exposed to the SDK for pausing the contract, recovering user funds, or upgrading the WASM bytecode. The admin address set during `initialize` is purely informational at this stage.
