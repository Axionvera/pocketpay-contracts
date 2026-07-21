# Advanced Audit Preparation Checklist — Savings Vault

This document provides a comprehensive security review and audit preparation checklist tailored specifically to the Savings Vault smart contract architecture. It identifies security goals, invariants, storage layouts, authorization boundaries, events, error handling, known limitations, and test coverage/gaps that must be addressed prior to any production-like use or external third-party audit.

> [!IMPORTANT]
> This checklist documents the required state for audit readiness. Many items highlight discrepancies between the current implementation and production-grade security standards.

---

## 1. Authentication & Authorization Checklist

The contract relies on the Soroban authorization framework to verify caller identity. The following checks must be verified:

- [ ] **State-Changing Auth Enforcement**: Ensure that every state-modifying function invokes `require_auth()` on the target address.
  - [x] Verified for `initialize` on `admin`.
  - [x] Verified for `deposit` on `user`.
  - [x] Verified for `withdraw` on `user`.
  - [x] Verified for `lock_funds` on `user`.
- [ ] **Read-Only Operations**: Read-only queries (`get_balance`, `get_locked_balance`, `can_withdraw`) must not call `require_auth` to avoid unnecessary transaction signing and costs.
- [ ] **Admin Roles & Privileges**: The contract records an `admin` address in instance storage.
  - [ ] **inert Admin**: There are currently no admin-only operations (e.g. upgrade, pause, emergency recovery). The admin key is only checked during initialization. This must be highlighted to auditors.
- [ ] **Authorization Test Coverage**:
  - [x] Covered: Basic happy-path tests execute with auth mocked (`mock_all_auths` in [test_helpers.rs](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/test_helpers.rs#L9)).
  - [ ] **CRITICAL TEST GAP**: No tests currently verify rejection when an unauthorized user attempts to deposit, withdraw, or lock funds on behalf of another user *without* mocking auth. Tests must assert that calling these functions without mock signatures triggers the host's authorization failure.

---

## 2. Accounting & Invariants Checklist

The contract maintains internal accounts for user deposits and locks, alongside a Stellar Asset Contract (SAC) token integration.

- [ ] **Deposit/Withdrawal Asymmetry**:
  - [ ] **CRITICAL FLAW**: `deposit` only updates internal bookkeeping storage:
    ```rust
    let new_balance = current_balance + amount;
    env.storage().persistent().set(&DataKey::Balance(user.clone()), &new_balance);
    ```
    It does **not** transfer tokens from the user to the contract. Conversely, `withdraw` attempts to transfer real tokens from the contract to the user:
    ```rust
    token_client.transfer(&contract_address, &user, &amount);
    ```
    This asymmetry means that a user can credit their balance arbitrarily in `deposit`, then drain the contract's actual token holdings via `withdraw`.
- [ ] **Total Balance Conservation Invariant**:
  - [x] The contract must maintain the invariant: `available_balance + locked_balance == net_deposited` for every user.
  - [x] This invariant is property-tested under [balance_conservation.rs](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs#L77-L94).
  - [ ] **Gaps**: The property tests mock the deposit token transfer by manually executing a transfer from the user to the contract in the test runner ([balance_conservation.rs:L117-L118](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/balance_conservation.rs#L117-L118)). A real contract audit requires the contract itself to handle token transfers during deposit.
- [ ] **Overflow & Underflow Protection**:
  - [x] Soroban's `i128` handles large values, but checks are needed to ensure balances cannot overflow if users deposit amounts near `i128::MAX`.
  - [ ] **Gaps**: No tests assert behavior when a user's balance is extremely large or checks for overflow when accumulating balances.
- [ ] **Withdrawal Ordering**:
  - [x] Matured locks must be consumed in chronological order (oldest first). This is implemented in [lib.rs:L397-L415](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L397-L415).
  - [x] Matured locks and available balances are correctly summed in `get_balance`.

---

## 3. Storage & State Checklist

Soroban contracts use a state storage model consisting of Persistent, Instance, and Temporary storage tiers, each with associated TTLs (Time To Live).

- [ ] **Storage Key Separation**:
  - [x] Instance storage is used for metadata: `Admin`, `Initialized`, `Token` in [lib.rs:L207-L209](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L207-L209).
  - [x] Persistent storage is used for user state: `Balance(Address)`, `Locks(Address)`, `NextLockId(Address)`.
- [ ] **Unbounded Vector Growth (DoS Vector)**:
  - [ ] **CRITICAL RISK**: User locks are stored in a single vector (`Vec<LockEntry>`) under `DataKey::Locks(Address)`. Every deposit/withdrawal/lock operation reads and writes the entire vector:
    ```rust
    let mut locks: Vec<LockEntry> = env.storage().persistent().get(&DataKey::Locks(user.clone())).unwrap_or_else(|| Vec::new(&env));
    // ... operations ...
    env.storage().persistent().set(&DataKey::Locks(user.clone()), &locks);
    ```
    If a user accumulates a large number of locks (e.g. hundreds of small deposits/locks), the cost to serialize, deserialize, and write this vector will grow linearly. This can exceed the transaction CPU or memory limits, permanently freezing the user's funds.
  - [ ] **Mitigation Plan**: Implement a maximum limit on the number of active locks per user, or use separate storage keys for individual locks (e.g., `DataKey::Lock(Address, u64)`).
- [ ] **TTL Extension Procedures**:
  - [x] Persistent and Instance entries must be extended regularly using the host's TTL controls.
  - [ ] **Gaps**: The codebase outlines manual commands in [storage-ttl.md](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/docs/storage-ttl.md) but lacks an automated or programmatic trigger within the contract logic to extend the TTL during user operations (e.g., calling `extend_ttl` inside `deposit` / `withdraw`).

---

## 4. Events Checklist

Events are the primary mechanism for off-chain indexers and user interfaces to monitor smart contract state changes.

- [ ] **Event Schema Integrity**:
  - [x] The event schema is defined in [events.md](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/docs/events.md) for `initialize`, `deposit`, `withdraw`, `lock`, and `unlock`.
- [ ] **Contract Implementation Status**:
  - [ ] **CRITICAL GAPS**: The contract does **not** emit any events. Only host log statements (`log!`) are used. Off-chain systems cannot reliably monitor deposits, withdrawals, or locks.
  - [ ] **Action Required**: Replace internal `log!` macros with `env.events().publish(...)` using the defined schemas before submitting to audit.
- [ ] **Event Test Coverage**:
  - [ ] No tests verify event emission or event payloads. Once events are implemented, tests must assert event topics and values.

---

## 5. Error Handling Checklist

Robust error handling ensures the contract fails gracefully and provides debuggable contexts to callers.

- [ ] **Panic Strings vs. Typed Errors**:
  - [ ] **Issue**: The contract currently handles errors via `panic!` strings (e.g. `panic!("Insufficient balance")` in [lib.rs:L377](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L377)). Panic strings are expensive to transmit, cannot be easily parsed by client SDKs, and are not localized.
  - [ ] **Action Required**: Implement a Soroban `#[contracterror]` enum (e.g., `Error`) mapping error states to stable `u32` codes, as outlined in [error-codes.md](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/docs/error-codes.md).
- [ ] **Test Assertion on Error Output**:
  - [x] Tests verify expected panic strings using `#[should_panic(expected = "...")]` in [mod.rs](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L38).
  - [ ] **Gaps**: Switching to `#[contracterror]` will require updating these test assertions to check for typed error results instead of panic string matches.

---

## 6. Test Suite and Verification Coverage

Review the overall test suite to identify untested execution paths.

### Existing Tests Reference

- **Initialization Tests**:
  - [test_initialize_success](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs#L6): Verifies happy path of initialization.
  - [test_initialize_fails_on_second_call](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs#L18): Verifies that re-initialization panics.
- **Deposit & Withdraw Tests**:
  - [test_deposit](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L20): Confirms basic deposit increases available balance.
  - [test_withdraw](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L71): Confirms withdrawal of unlocked funds works with token client mock transfers.
  - [test_failed_withdraw_does_not_change_available_balance](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L188): Proves state is not corrupted on rejected withdrawals.
- **Locking Tests**:
  - [test_lock_funds](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L267): Verifies basic lock action.
  - [test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later](file:///Users/boufdaddy/Documents/web3%20projects/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L316): Checks multi-lock chronological maturities.

### Missing Test Areas (Must be Added Before Audit)

1. [ ] **Authorization Signature Tests**: Verify behavior when executing operations without setting up mocks to confirm signature validation works on-chain.
2. [ ] **Token Transfer Failure**: Mock a failing token transfer (e.g. transfer returns error or has insufficient allowance/balance) during withdrawal and assert that the internal accounting does not decrement.
3. [ ] **Unbounded Vector Stress Test**: Add a test that generates a large number of locks (e.g., 200) for a single user and verify resource usage/gas exhaustion limits.
4. [ ] **Zero / Negative Time Locks**: Test locking funds with an `unlock_time` set to 0 or negative relative to the current timestamp (should be caught by the future-timestamp check).
5. [ ] **Event Verification**: Once event emission is added, write assertions using `env.events().all()` to check correctness of emitted events.

---

## 7. Summary of Known Limitations

| Component | Limitation / Issue | Risk / Impact | Mitigation / Status |
|---|---|---|---|
| **Accounting** | Internal accounting only; no token custody on `deposit`. | Users can mint internal balances without backing funds; token drain threat. | **Critical Block**: Must integrate SAC token transfer in `deposit` before mainnet. |
| **Storage** | Unbounded lock list (`Vec<LockEntry>`). | Gas exhaustion / contract lockup for users with many active locks. | **High Risk**: Limit max active locks or split locks into separate keys. |
| **Errors** | String panics. | High fee consumption, difficult SDK parsing. | **Medium Risk**: Implement `#[contracterror]` enum. |
| **Events** | No event emissions. | Off-chain infrastructure cannot sync or read state changes. | **Medium Risk**: Implement event publishing in all state-changing actions. |
| **Admin** | Inert Admin Role. | Admin address is recorded but has no administrative capability. | **Low Risk**: Document as intentional or implement upgradeability/pause mechanisms. |

---

*Last updated: 2026-07-20*
