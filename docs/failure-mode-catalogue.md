# Failure Mode Catalogue: Savings Vault Contract
This document lists all known failure modes for the Savings Vault contract, organized by category, with expected behavior, affected functions, and links to tests (where available).

---

## Table of Contents
1. [Initialization Failures](#1-initialization-failures)
2. [Invalid Input Failures](#2-invalid-input-failures)
3. [Insufficient Balance Failures](#3-insufficient-balance-failures)
4. [Time Lock Failures](#4-time-lock-failures)
5. [Authorization Failures](#5-authorization-failures)
6. [Storage/Versioning Failures](#6-storageversioning-failures)
7. [Token Transfer Failures](#7-token-transfer-failures)
8. [Other Failures](#8-other-failures)

---

## 1. Initialization Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-INIT-01 | Calling `initialize` after contract is already initialized | Panics with `Contract is already initialized` | `initialize` | ✅ [test_initialize_twice_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs) |
| FM-INIT-02 | Calling public functions before `initialize` | Panics with `Contract is not initialized` | All functions except `initialize` | ✅ [test_deposit_before_initialization_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs), [test_withdraw_before_initialization_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs), etc. |
| FM-INIT-03 | Admin fails to authorize `initialize` | Soroban host authorization failure | `initialize` | Implied (same mechanism as other auth failures) |

---

## 2. Invalid Input Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-IN-01 | Deposit with amount ≤ 0 | Panics with `Deposit amount must be greater than zero` | `deposit` | ✅ [test_deposit_zero_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs), [conservation_invalid_deposits_do_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| FM-IN-02 | Withdraw with amount ≤ 0 | Panics with `Withdrawal amount must be greater than zero` | `withdraw` | ✅ [test_withdraw_zero_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs), [conservation_invalid_withdrawals_do_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| FM-IN-03 | Lock with amount ≤ 0 | Panics with `Lock amount must be greater than zero` | `lock_funds` | ✅ [conservation_invalid_locks_do_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| FM-IN-04 | Lock with unlock_time ≤ current ledger time | Panics with `Unlock time must be in the future` | `lock_funds` | ✅ [test_zero_duration_lock_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/zero_duration_lock.rs) |
| FM-IN-05 | `withdraw_lock` with non-existent lock_id | Panics with `Lock not found` | `withdraw_lock` | ✅ [withdraw_lock.rs](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/withdraw_lock.rs) |

---

## 3. Insufficient Balance Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-BAL-01 | Withdraw amount > available balance (deposit + matured locks) | Panics with `Insufficient balance` | `withdraw` | ✅ [conservation_withdraw_exceeds_available_while_locked_does_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| FM-BAL-02 | Lock amount > available balance | Panics with `Insufficient balance to lock` | `lock_funds` | ✅ [conservation_invalid_locks_do_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) |
| FM-BAL-03 | Failed withdraw/lock does not change balances | No state change; available/locked balances remain the same | `withdraw`, `lock_funds` | ✅ [conservation_invalid_deposits_do_not_mutate](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs), etc. |

---

## 4. Time Lock Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-TIME-01 | `withdraw_lock` called on immature lock | Panics with `Lock has not matured yet` | `withdraw_lock` | ✅ [withdraw_lock.rs](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/withdraw_lock.rs) |
| FM-TIME-02 | `can_withdraw` returns false before unlock time | Returns false; no panic | `can_withdraw` | ✅ [lock_read_helpers.rs](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/lock_read_helpers.rs) |
| FM-TIME-03 | `can_withdraw` returns true at or after unlock time | Returns true; no panic | `can_withdraw` | ✅ [lock_read_helpers.rs](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/lock_read_helpers.rs) |

---

## 5. Authorization Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-AUTH-01 | Calling `initialize` without admin authorization | Soroban host authorization failure | `initialize` | Implied |
| FM-AUTH-02 | Calling `deposit`/`withdraw`/`lock_funds` without user authorization | Soroban host authorization failure | `deposit`, `withdraw`, `lock_funds` | ✅ [test_unauthorized_deposit_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs), [test_unauthorized_withdraw_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs), [test_unauthorized_lock_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs) |
| FM-AUTH-03 | Calling `withdraw_lock` without user authorization | Soroban host authorization failure | `withdraw_lock` | ✅ [test_unauthorized_withdraw_lock_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs) |
| FM-AUTH-04 | Calling `transfer_admin` as non-admin | Panics with `Not authorized` | `transfer_admin` | ✅ [test_non_admin_cannot_transfer_admin](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/admin_invariant_guard.rs) |

---

## 6. Storage/Versioning Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-STG-01 | Contract initialized with unsupported storage version | Panics with `Unsupported storage version` | All public functions | Implied |
| FM-STG-02 | Legacy contract (no `StorageVersion`) | Treats as version `1`; functions normally | All public functions | Implied |
| FM-STG-03 | Persistent storage entry expires (TTL) | Contract sees 0 balance; no state change unless action taken | All functions | Not tested (requires TTL setup) |

---

## 7. Token Transfer Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-TKN-01 | Token transfer fails during deposit | SAC transfer failure propagated; no internal balance change | `deposit` | Not tested (requires SAC failure simulation) |
| FM-TKN-02 | Token transfer fails during withdrawal | SAC transfer failure propagated; no internal balance/lock change | `withdraw` | Not tested (requires SAC failure simulation) |
| FM-TKN-03 | Token transfer fails during `withdraw_lock` | SAC transfer failure propagated; no lock change | `withdraw_lock` | Not tested (requires SAC failure simulation) |
| FM-TKN-04 | Out-of-band token transfer to vault address | Tokens held by vault but not credited to any user; cannot be withdrawn via normal operations | None (external) | Not tested |

---

## 8. Other Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-OTH-01 | Read-only functions fail if contract not initialized | Panics with `Contract is not initialized` | `get_balance`, `get_locked_balance`, `can_withdraw`, `get_lock`, `list_locks`, `get_version`, `get_admin` | ✅ [test_read_functions_before_initialization](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs) |
