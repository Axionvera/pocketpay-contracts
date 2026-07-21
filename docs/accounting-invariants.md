# Formal Accounting Invariants
## Savings Vault Contract

---
## Overview
This document defines the formal accounting invariants that must *always* hold true for the Savings Vault contract, serving as audit preparation and a reference for developers.

---
## 1. Core Balance Invariants

### Invariant 1.1: Individual User Balance Conservation
For every user address U at any time T:
```
available_balance(U, T) + locked_balance(U, T) = net_deposited(U, T)
```
Where:
- `available_balance(U, T)`: [get_balance(env, U)](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L272) – available balance (deposited + matured locks)
- `locked_balance(U, T)`: [get_locked_balance(env, U)](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L400) – sum of all unmatured lock amounts
- `net_deposited(U, T)`: total amount deposited by user U minus total amount withdrawn by user U up to time T

**Tests Covering This Invariant**:
- [balance_conservation.rs](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs) (all property-driven tests!)

---
### Invariant 1.2: Global Token Custody Invariant
For all times T:
```
contract_token_balance(T) = sum_over_all_users(available_balance(U, T) + locked_balance(U, T))
```
Where `contract_token_balance(T)` is the SAC token balance of the contract address at time T.

**Note**: No automated tests yet for this invariant! (Requires SAC token balance checks in tests!)

---
## 2. Lock Entry Invariants

### Invariant 2.1: Lock Entry Amounts Are Positive
Every lock entry for every user has an amount > 0.

### Invariant 2.2: Unlock Times Are Strictly Increasing (Per Lock ID)
Lock IDs are monotonically increasing (per user), and unlock times are independent (not overwritten).

**Tests**: [test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L417), [test_repeated_lock_three_times_accumulates_and_keeps_last_unlock_time](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L483)

---
## 3. User Isolation Invariants

### Invariant 3.1: User Balance Isolation
Operations on user A's balance have *no effect* on any other user's balance, locks, or next lock ID.

**Tests**:
- [test_separate_user_balances](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L756)
- [balance_isolation_between_users_deposit](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L778)
- [balance_isolation_between_users_withdraw](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L794)
- [balance_isolation_between_users_lock](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L822)

---
## 4. Failed Operations Invariants

### Invariant 4.1: Failed Operations Are No‑Ops
For any operation that fails (panics or errors), the state of the contract (all user balances, locks, storage) remains *exactly the same* as before the operation was invoked.

**Tests**:
- [test_deposit_zero_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L119)
- [test_withdraw_zero_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L228)
- [test_failed_withdraw_does_not_change_available_balance](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L283)
- [conservation_invalid_deposits_do_not_mutate](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs#L336)
- [conservation_invalid_withdrawals_do_not_mutate](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs#L348)
- [conservation_invalid_locks_do_not_mutate](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs#L363)

---
## 5. Withdrawal Invariants

### Invariant 5.1: Withdrawal Amounts Are Bounded
A withdrawal of amount A from user U at time T is only allowed if:
```
0 < A ≤ available_balance(U, T)
```

**Tests**: [test_withdraw_more_than_balance_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L216), [test_withdraw_exceeds_available_after_deposit_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L265)

---
## 6. Initialization Invariants

### Invariant 6.1: Initialization Is Idempotent
The contract can be initialized *exactly once*; subsequent calls to `initialize` panic without changing state.

**Tests**: [test_initialize_twice_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L26)

### Invariant 6.2: No Operations Before Initialization
All functions except `initialize` panic if called before the contract is initialized.

**Tests**: [test_deposit_uninitialized_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L38), [test_withdraw_uninitialized_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L47)
