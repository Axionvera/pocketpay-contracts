# Savings Vault Security Review

## Overview
This document provides a security review of the Savings Vault smart contract, covering authorization, fund custody assumptions, misuse scenarios, and failure modes.

## Table of Contents
1. [Public Contract Functions Analysis](#1-public-contract-functions-analysis)
2. [Authorization Rules](#2-authorization-rules)
3. [Current Security Posture](#3-current-security-posture)
4. [Misuse Scenarios](#4-misuse-scenarios)
5. [Recommendations](#5-recommendations)
6. [Assumptions & Limitations](#6-assumptions--limitations)

---

## 1. Public Contract Functions Analysis
All functions are defined in [src/lib.rs](../contracts/savings_vault/src/lib.rs).

### `initialize(env, admin, token)`
- **Purpose**: Configure the contract with an admin address and token address
- **Authorization**: Admin must sign (via `admin.require_auth()`)
- **State Changes**:
  - Sets `Admin` in instance storage
  - Sets `Initialized = true` in instance storage
  - Sets `Token` (SAC address) in instance storage
- **Restrictions**: Can only be called **once** (panics if already initialized)
- **Assumptions**:
  - `admin` is a secure address
  - `token` is a valid Stellar Asset Contract (SAC) address

### `deposit(env, user, amount)`
- **Purpose**: Pull tokens from the caller and credit the user's internal available balance
- **Authorization**: User must sign (via `user.require_auth()`)
- **State Changes**:
  - Transfers `amount` tokens from `user` to the contract via SAC
  - Increases user's `Balance(Address)` by `amount`
- **Restrictions**:
  - Contract must already be initialized
  - Amount must be > 0
- **Assumptions**:
  - `user` is authorized to deposit for themselves
  - The configured token contract behaves as expected and honors transfers

### `withdraw(env, user, amount)`
- **Purpose**: Debit a user's balance and transfer tokens to them
- **Authorization**: User must sign (via `user.require_auth()`)
- **State Changes**:
  - Decreases user's `Balance(Address)` and/or matured `LockEntry`s
  - Transfers `amount` tokens from the contract to `user` via SAC
- **Restrictions**:
  - Contract must already be initialized
  - Amount must be > 0
  - Total available (deposited + matured locks) must be >= `amount`
- **Assumptions**:
  - The contract has sufficient token balance to cover the withdrawal
  - The configured `Token` is a valid, functional SAC

### `get_balance(env, user)`
- **Purpose**: Query a user's available balance (deposited + matured locks)
- **Authorization**: None (public read-only)
- **State Changes**: None
- **Restrictions**: Contract must already be initialized
- **Assumptions**: None (safe for public access)

### `lock_funds(env, user, amount, unlock_time)`
- **Purpose**: Move available balance into a time-locked entry
- **Authorization**: User must sign (via `user.require_auth()`)
- **State Changes**:
  - Decreases user's `Balance(Address)` by `amount`
  - Creates a new `LockEntry` with unique ID, `amount`, and `unlock_time`
  - Increments `NextLockId(Address)`
- **Restrictions**:
  - Contract must already be initialized
  - Amount must be > 0
  - Available balance must be >= `amount`
  - `unlock_time` must be > current ledger timestamp
- **Assumptions**:
  - Ledger timestamps are secure and monotonic (guaranteed by Soroban)

### `get_locked_balance(env, user)`
- **Purpose**: Query a user's total locked (unmatured) balance
- **Authorization**: None (public read-only)
- **State Changes**: None
- **Restrictions**: Contract must already be initialized
- **Assumptions**: None (safe for public access)

### `can_withdraw(env, user)`
- **Purpose**: Check if a user has any matured locks
- **Authorization**: None (public read-only)
- **State Changes**: None
- **Restrictions**: Contract must already be initialized
- **Assumptions**: None (safe for public access)

---

## 2. Authorization Rules
| Function          | Authorized Caller       |
|-------------------|-------------------------|
| `initialize`      | Admin (only once)       |
| `deposit`         | User (self)             |
| `withdraw`        | User (self)             |
| `get_balance`     | Anyone (read-only)      |
| `lock_funds`      | User (self)             |
| `get_locked_balance` | Anyone (read-only) |
| `can_withdraw`    | Anyone (read-only)      |

All user-specific operations require the user to authorize via `require_auth()`, which is good!

---

## 3. Current Security Posture
1. **Authorization boundaries are simple and explicit**
   - `initialize` requires `admin` authorization.
   - `deposit`, `withdraw`, and `lock_funds` require the target `user` to authorize the call.
   - Read-only functions do not require authorization.

2. **Custody is token-backed at deposit time**
   - `deposit` now performs a SAC transfer from the user into the vault before crediting internal balance.
   - `withdraw` transfers from the vault back to the user after balance checks pass.

3. **Initialization is enforced**
   - All public functions except `initialize` require the initialized flag to be present.
   - Calls made before initialization fail fast with `Contract is not initialized`.

4. **State transitions remain all-or-nothing**
   - Invalid deposits, withdrawals, and lock attempts fail before state is committed.
   - Failed operations are covered by tests that assert balances do not mutate.

5. **Remaining limitations**
   - The contract uses panic strings rather than a custom error enum, so callers should not depend on stable machine-readable error codes.
   - There is no admin recovery, pause, or emergency withdrawal path.
   - The contract assumes the configured SAC token is trustworthy and remains compatible with expected transfer semantics.

---

## 4. Misuse Scenarios
| Scenario | Expected Behavior |
|----------|------------------|
| Withdraw another user's funds | Panics because `user.require_auth()` is called, and an attacker cannot authorize as another user |
| Bypass lock duration (withdraw before unlock time) | Panics because `withdraw` checks available balance (deposited + matured locks); unmatured locks are not counted as available |
| Replay operations | Not possible in Soroban (transactions are unique and validated by the network) |
| Deposit zero/negative amount | Panics |
| Withdraw zero/negative amount | Panics |
| Lock zero/negative amount | Panics |
| Lock more than available balance | Panics |
| Lock with past unlock time | Panics |
| Interact with uninitialized contract | Panics with `Contract is not initialized` before any balance logic runs |
| Initialize contract twice | Panics |

---

## 5. Recommendations
1. **Keep auth-focused tests**: Preserve targeted tests that do not rely on `mock_all_auths()` for misuse scenarios.
2. **Add a custom error enum**: Replace panic strings with contract errors before external integrations depend on failure modes.
3. **Review token assumptions**: Confirm the chosen SAC and asset admin policy match custody expectations.
4. **Add operational safeguards**: Consider pause, multi-sig admin, and event coverage before production use.
5. **Audit before production**: Get a formal audit before deploying to mainnet

---

## 6. Assumptions & Limitations
- The contract uses Soroban's built-in `require_auth()` for authorization, which is considered secure
- Ledger timestamps are monotonic and secure (provided by Soroban)
- The configured `Token` is a valid Stellar Asset Contract
- The contract admin address is secure
- Users are responsible for managing their own private keys
- The contract does not currently support pausing, upgrading, or admin-controlled recovery of funds
