# Savings Vault — Contract API Reference

This document provides a detailed API reference for the Stellar PocketPay Savings Vault smart contract. It includes function signatures, parameters, auth requirements, state changes, return values, failure cases, and usage examples.

---

## Table of Contents

- [initialize](#initialize)
- [deposit](#deposit)
- [withdraw](#withdraw)
- [get_balance](#get_balance)
- [lock_funds](#lock_funds)
- [get_locked_balance](#get_locked_balance)
- [can_withdraw](#can_withdraw)

---

## Functions

### `initialize`

Initializes the contract with an admin address and the token address (asset contract) to be used for withdrawals. This function can only be called once.

#### Signature
```rust
pub fn initialize(env: Env, admin: Address, token: Address)
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `admin` | `Address` | The address to record as the contract admin. |
| `token` | `Address` | The token/asset contract address (e.g., SAC) used by the vault. |

#### Auth Requirements
- `admin` must sign the transaction (`admin.require_auth()`).

#### State Changes
- Sets `DataKey::Admin` to the `admin` address in instance storage.
- Sets `DataKey::Token` to the `token` address in instance storage.
- Sets `DataKey::Initialized` to `true` (boolean) in instance storage.

#### Return Value
- `()` (None)

#### Failure Cases / Panics
- Panics with `"Contract is already initialized"` if the contract has already been initialized.
- Panics if `admin` signature/authorization is missing.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --source admin_identity \
  --network testnet \
  -- \
  initialize \
  --admin GB... \
  --token CA...
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
client.initialize(&admin_address, &token_address);
```

---

### `deposit`

Deposits a specified amount of funds into the caller's vault, updating their available (unlocked) balance.

#### Signature
```rust
pub fn deposit(env: Env, user: Address, amount: i128)
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The depositor's address. |
| `amount` | `i128` | The amount to deposit. Must be greater than zero. |

#### Auth Requirements
- `user` must sign the transaction (`user.require_auth()`).

#### State Changes
- Reads current available balance from `DataKey::Balance(user)` in persistent storage (defaults to `0`).
- Adds `amount` to the current available balance and writes the new balance to `DataKey::Balance(user)`.

#### Return Value
- `()` (None)

#### Failure Cases / Panics
- Panics with `"Deposit amount must be greater than zero"` if `amount` is less than or equal to zero.
- Panics if `user` signature/authorization is missing.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --source user_identity \
  --network testnet \
  -- \
  deposit \
  --user GB... \
  --amount 1000
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
client.deposit(&user_address, &1000_i128);
```

---

### `withdraw`

Withdraws a specified amount of funds from the user's available balance and transfers them back to the user's Stellar address.

#### Signature
```rust
pub fn withdraw(env: Env, user: Address, amount: i128)
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The withdrawer's address. |
| `amount` | `i128` | The amount to withdraw. Must be greater than zero. |

#### Auth Requirements
- `user` must sign the transaction (`user.require_auth()`).

#### State Changes
- Reads current available balance from `DataKey::Balance(user)` in persistent storage.
- Decrements the available balance by `amount` and writes the new balance to `DataKey::Balance(user)`.
- Invokes the token contract's `transfer` function to send `amount` from the vault contract to the `user` address.

#### Return Value
- `()` (None)

#### Failure Cases / Panics
- Panics with `"Withdrawal amount must be greater than zero"` if `amount` is less than or equal to zero.
- Panics with `"Insufficient balance"` if `amount` exceeds the user's available balance.
- Panics if the token transfer fails.
- Panics if `user` signature/authorization is missing.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --source user_identity \
  --network testnet \
  -- \
  withdraw \
  --user GB... \
  --amount 500
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
client.withdraw(&user_address, &500_i128);
```

---

### `get_balance`

Queries the available (unlocked) balance for a specified user address.

#### Signature
```rust
pub fn get_balance(env: Env, user: Address) -> i128
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The address to query the balance of. |

#### Auth Requirements
- None. This is a read-only query and does not require signatures.

#### State Changes
- None (read-only query).

#### Return Value
- `i128`: The user's available balance. Returns `0` if the user has never deposited.

#### Failure Cases / Panics
- None under normal conditions.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --network testnet \
  -- \
  get_balance \
  --user GB...
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
let balance = client.get_balance(&user_address);
```

---

### `lock_funds`

Locks a specified amount of the user's available balance until a given Unix timestamp. Locked funds are moved into a separate locked balance pool and cannot be withdrawn until the lock expires.

> [!WARNING]
> Locking funds multiple times overwrites the previous unlock timestamp.

#### Signature
```rust
pub fn lock_funds(env: Env, user: Address, amount: i128, unlock_time: u64)
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The user's address. |
| `amount` | `i128` | The amount to lock. Must be greater than zero. |
| `unlock_time` | `u64` | Unix epoch timestamp (in seconds) when the funds become withdrawable. Must be in the future. |

#### Auth Requirements
- `user` must sign the transaction (`user.require_auth()`).

#### State Changes
- Decreases `DataKey::Balance(user)` by `amount` in persistent storage.
- Increases `DataKey::LockedBalance(user)` by `amount` in persistent storage (defaults to `0`).
- Sets `DataKey::UnlockTime(user)` to `unlock_time` in persistent storage.

#### Return Value
- `()` (None)

#### Failure Cases / Panics
- Panics with `"Lock amount must be greater than zero"` if `amount` is less than or equal to zero.
- Panics with `"Unlock time must be in the future"` if `unlock_time` is less than or equal to the current ledger timestamp.
- Panics with `"Insufficient balance to lock"` if `amount` exceeds the user's available balance.
- Panics if `user` signature/authorization is missing.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --source user_identity \
  --network testnet \
  -- \
  lock_funds \
  --user GB... \
  --amount 300 \
  --unlock_time 1784234567
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
client.lock_funds(&user_address, &300_i128, &1784234567_u64);
```

---

### `get_locked_balance`

Queries the locked balance for a specified user address.

#### Signature
```rust
pub fn get_locked_balance(env: Env, user: Address) -> i128
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The address to query. |

#### Auth Requirements
- None. This is a read-only query and does not require signatures.

#### State Changes
- None (read-only query).

#### Return Value
- `i128`: The user's locked balance. Returns `0` if the user has no locked funds.

#### Failure Cases / Panics
- None under normal conditions.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --network testnet \
  -- \
  get_locked_balance \
  --user GB...
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
let locked_balance = client.get_locked_balance(&user_address);
```

---

### `can_withdraw`

Checks whether a user can withdraw their locked funds (i.e. the lock has expired).

#### Signature
```rust
pub fn can_withdraw(env: Env, user: Address) -> bool
```

#### Parameters
| Parameter | Type | Description |
| :--- | :--- | :--- |
| `env` | `Env` | The Soroban environment context. |
| `user` | `Address` | The address to check. |

#### Auth Requirements
- None. This is a read-only query and does not require signatures.

#### State Changes
- None (read-only query).

#### Return Value
- `bool`: `true` if the user has a locked balance greater than zero AND the current ledger timestamp is greater than or equal to their configured `unlock_time`. Returns `false` otherwise.

#### Failure Cases / Panics
- None under normal conditions.

#### Example Invocation

**Soroban CLI:**
```bash
soroban contract invoke \
  --id CD... \
  --network testnet \
  -- \
  can_withdraw \
  --user GB...
```

**Rust SDK:**
```rust
let client = SavingsVaultClient::new(&env, &contract_id);
let is_withdrawable = client.can_withdraw(&user_address);
```
