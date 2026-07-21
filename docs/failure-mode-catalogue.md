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
| FM-INIT-01 | Calling `initialize` after contract is already initialized | Panics with `Contract is already initialized` | `initialize` | ✅ [test_initialize_twice_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L26) |
| FM-INIT-02 | Calling public functions before `initialize` | Panics with `Contract is not initialized` | All functions except `initialize` | ✅ [test_deposit_uninitialized_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L28), [test_withdraw_uninitialized_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L38), etc. |
| FM-INIT-03 | Admin fails to authorize `initialize` | Soroban host authorization failure | `initialize` | Implied (same mechanism as other auth failures) |

---

## 2. Invalid Input Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-IN-01 | Deposit with amount ≤ 0 | Panics with `Deposit amount must be greater than zero` | `deposit` | ✅ [test_deposit_zero_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L110), [test_deposit_negative_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L121) |
| FM-IN-02 | Withdraw with amount ≤ 0 | Panics with `Withdrawal amount must be greater than zero` | `withdraw` | ✅ [test_withdraw_zero_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L263), [test_withdraw_negative_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L273) |
| FM-IN-03 | Lock with amount ≤ 0 | Panics with `Lock amount must be greater than zero` | `lock_funds` | ✅ [test_lock_zero_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L548) |
| FM-IN-04 | Lock with unlock_time ≤ current ledger time | Panics with `Unlock time must be in the future` | `lock_funds` | ✅ [test_lock_past_time_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L575) |

---

## 3. Insufficient Balance Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-BAL-01 | Withdraw amount > available balance (deposit + matured locks) | Panics with `Insufficient balance` | `withdraw` | ✅ [test_withdraw_more_than_balance_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L251) |
| FM-BAL-02 | Lock amount > available balance | Panics with `Insufficient balance to lock` | `lock_funds` | ✅ [test_lock_more_than_balance_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L562), [test_lock_from_empty_balance_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L588) |
| FM-BAL-03 | Failed withdraw/lock does not change balances | No state change; available/locked balances remain the same | `withdraw`, `lock_funds` | ✅ [test_failed_withdraw_does_not_change_available_balance](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L317), [test_failed_lock_does_not_change_available_balance](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L610) |

---

## 4. Time Lock Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-TIME-01 | `can_withdraw` returns false before unlock time | Returns false; no panic | `can_withdraw` | ✅ [test_can_withdraw_before_unlock](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L681) |
| FM-TIME-02 | `can_withdraw` returns true at or after unlock time | Returns true; no panic | `can_withdraw` | ✅ [test_can_withdraw_exactly_at_unlock](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L718), [test_can_withdraw_after_unlock](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L737) |

---

## 5. Authorization Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-AUTH-01 | Calling `initialize` without admin authorization | Soroban host authorization failure | `initialize` | Implied |
| FM-AUTH-02 | Calling `deposit`/`withdraw`/`lock_funds` without user authorization | Soroban host authorization failure | `deposit`, `withdraw`, `lock_funds` | ✅ [test_deposit_requires_user_authorization](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L207), [test_withdraw_requires_user_authorization](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L185), [test_lock_funds_requires_user_authorization](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L228) |

---

## 6. Storage/Versioning Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-STG-01 | Contract initialized with unsupported storage version | Panics with `Unsupported storage version: X` | All public functions | ✅ [test_invalid_storage_version_fails_safely](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L1096) |
| FM-STG-02 | Legacy contract (no `StorageVersion`) | Treats as version `1`; functions normally | All public functions | ✅ [test_legacy_missing_storage_version_works](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L1071) |
| FM-STG-03 | Persistent storage entry expires (TTL) | Contract sees 0 balance; no state change unless action taken | All functions | Not tested (requires TTL setup) |

---

## 7. Token Transfer Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-TKN-01 | Token transfer fails during deposit | SAC transfer failure propagated; no internal balance change | `deposit` | Not tested (requires SAC failure simulation) |
| FM-TKN-02 | Token transfer fails during withdrawal | SAC transfer failure propagated; no internal balance/lock change | `withdraw` | Not tested (requires SAC failure simulation) |
| FM-TKN-03 | Out-of-band token transfer to vault address | Tokens held by vault but not credited to any user; cannot be withdrawn via normal operations | None (external) | Not tested |

---

## 8. Other Failures
| ID | Failure Mode | Expected Behavior | Affected Functions | Test Coverage |
|----|--------------|-------------------|--------------------|---------------|
| FM-OTH-01 | Read-only functions fail if contract not initialized | Panics with `Contract is not initialized` | `get_balance`, `get_locked_balance`, `can_withdraw` | ✅ [test_get_balance_uninitialized_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L56), etc. |
