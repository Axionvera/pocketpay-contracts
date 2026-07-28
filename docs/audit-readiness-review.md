# Audit Readiness Review — Savings Vault Contract

> **Status:** Completed Readiness Assessment  
> **Target Crate:** `contracts/savings_vault`  
> **Overall Audit Readiness Rating:** **NOT AUDIT READY** (Critical blockers identified)  

This audit readiness review provides a comprehensive security and architectural evaluation of the **Savings Vault Contract**. It identifies audit blockers, high-risk assumptions, missing test coverage, and unresolved design questions prior to engaging external security auditors.

> [!CAUTION]
> **Production & Audit Warning:** The contract currently contains fundamental architectural discrepancies between asset custody and internal balance accounting. It **must not** be submitted for external security audit or deployed to production/mainnet in its present state.

---

## 1. Audit Blockers & High-Risk Areas

The following items are critical blockers that will cause immediate audit findings or failures if presented to an auditor today:

### 1.1 Critical Discrepancy: Internal Accounting vs. Asset Custody
* **Location:** [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs#L104-L133) (`deposit`) and [`withdraw`](../contracts/savings_vault/src/lib.rs#L152-L240)
* **Risk Level:** **CRITICAL (Audit Blocker)**
* **Description:** The `deposit` function increases a user's internal storage balance (`DataKey::Balance(user)`) without executing a token transfer from the user into the contract's custodial address. However, `withdraw` attempts to transfer real SAC tokens (`token_client.transfer(&contract_address, &user, &amount)`).
* **Impact:** Any user can call `deposit(user, 1_000_000)` without holding or transferring tokens, and subsequently invoke `withdraw(user, 1_000_000)` to drain all real tokens held by the vault contract.

### 1.2 Unbounded Storage Vectors ($O(N)$ Gas & Execution Risk)
* **Location:** [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs#L168-L223) (`withdraw`), [`lock_funds`](../contracts/savings_vault/src/lib.rs#L331-L345), [`get_balance`](../contracts/savings_vault/src/lib.rs#L257-L270)
* **Risk Level:** **HIGH**
* **Description:** User time-locks are stored in a single `Vec<LockEntry>` per user under `DataKey::Locks(user)`. Every invocation of `withdraw`, `get_balance`, `get_locked_balance`, `can_withdraw`, or `lock_funds` reads, iterates over, or rewrites the entire vector.
* **Impact:** As the number of lock entries for a user grows, the CPU and transaction size/resource consumption will scale linearly ($O(N)$). Eventually, transaction execution will hit Soroban CPU or memory limit bounds, permanently locking user funds.

### 1.3 Risk of Storage Key Expiration (TTL Expiry)
* **Location:** Storage operations throughout [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs)
* **Risk Level:** **HIGH**
* **Description:** The contract does not invoke `extend_ttl()` on persistent or instance storage keys. Instance storage holds `Admin`, `Initialized`, and `Token`.
* **Impact:** If instance storage expires, all vault operations (`withdraw`, `deposit`) panic when attempting to read `DataKey::Token`. If persistent storage (`Balance`, `Locks`) expires, `unwrap_or(0)` logic will evaluate balances as zero, temporarily or permanently denying access to user funds until restored.

### 1.4 Unstructured Failure Diagnostics (Panics vs. Contract Error Enums)
* **Location:** Validation guards in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs)
* **Risk Level:** **MEDIUM**
* **Description:** Input validations use raw Rust `panic!("...")` strings instead of a structured `#[contracterror]` enum.
* **Impact:** Calling SDKs cannot parse machine-readable error codes. String panics introduce fragility across contract releases and make client-side handling error-prone.

### 1.5 Missing On-Chain Event Publications
* **Location:** State-changing methods in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs)
* **Risk Level:** **MEDIUM**
* **Description:** State changes emit debug logs via `log!(&env, ...)` but do not publish standard Soroban events via `env.events().publish(...)`.
* **Impact:** Off-chain indexers, auditors, and mobile wallet apps cannot reliably index or audit deposits, withdrawals, or fund locks on-chain.

---

## 2. Domain-by-Domain Analysis

### 2.1 Asset Custody
* **Current Implementation:** Token address is configured during `initialize(admin, token)`. Withdrawals call `token::Client::transfer`. Deposits do not execute transfers.
* **Audit Assessment:** Custody mechanism is incomplete. Token contract trust assumptions (e.g. standard Stellar Asset Contract behavior vs custom token behavior) are unverified.

### 2.2 Storage & State Management
* **Current Implementation:** Key-value schema using `DataKey` enum with persistent (`Balance`, `Locks`, `NextLockId`) and instance (`Admin`, `Initialized`, `Token`) storage.
* **Audit Assessment:** Clean key separation, but vulnerable to unbounded vector growth and lack of automated TTL maintenance.

### 2.3 Accounting Logic
* **Current Implementation:** Internal ledger tracks available balance and lock vector entries. Deductions during `withdraw` consume available balance first, then mature lock entries sequentially.
* **Audit Assessment:** Invariant `available + locked == net_deposited` is well-tested in `balance_conservation.rs`, but execution math relies on linear iteration over lock vectors.

### 2.4 Authorisation & Permissions
* **Current Implementation:** `require_auth()` is properly invoked on user addresses for `deposit`, `withdraw`, and `lock_funds`. Admin address requires auth during `initialize`.
* **Audit Assessment:** Permissions are correctly checked for user operations. Admin role is inert after initialization.

### 2.5 Events & Observability
* **Current Implementation:** Debug `log!` calls only.
* **Audit Assessment:** Does not comply with Soroban event standards. Proposed schema in `docs/events.md` remains unimplemented.

### 2.6 Migrations & Upgradeability
* **Current Implementation:** Immutable deployment. No `upgrade()` or proxy pattern implemented.
* **Audit Assessment:** If a security bug is discovered in production, the contract cannot be patched. User funds would have to be manually migrated or drained depending on the exploit.

### 2.7 Documentation Integrity
* **Current Implementation:** Documentation across `docs/` is accurate regarding internal accounting limitations and future designs, but README and docs must link to this formal readiness review.

---

## 3. Missing Test Coverage

To achieve audit readiness, the test suite in `contracts/savings_vault/src/test/` must be expanded with the following tests:

1. **SAC Token Deposit Transfer Tests:** Tests validating that `deposit()` pulls tokens from user balance to contract address and fails if user has insufficient allowance/balance.
2. **Vector Scaling / Gas Stress Tests:** Tests creating 100+ `LockEntry` items for a single user to test iteration overhead and resource limit bounds during `withdraw()` and `get_balance()`.
3. **Storage TTL Expiration Simulation:** Tests simulating ledger TTL expiry on `DataKey::Token`, `DataKey::Balance`, and `DataKey::Locks` to verify contract error handling and recovery behavior.
4. **Unauthorized Invocation Assertion Tests:** Direct tests calling `deposit`, `withdraw`, and `lock_funds` without mocking auth or with mismatched signers to verify Host `require_auth` enforcement.
5. **Re-entrancy / Cross-Contract Call Safety Tests:** Verification of state consistency during nested host function calls or unexpected token contract callbacks.

---

## 4. Unresolved Design Questions

Before scheduling an audit, the following design decisions must be resolved and documented:

1. **Deposit Mechanism Choice:** Should deposits use Soroban token `transfer` (requiring caller to transfer to contract) or `transfer_from` (requiring prior approval)?
2. **Lock Data Structure Refactoring:** Should individual locks be stored under separate persistent keys (e.g., `DataKey::Lock(Address, u64)`) to achieve $O(1)$ lookups instead of a single $O(N)$ `Vec<LockEntry>`?
3. **Error Representation:** Will the contract adopt a `#[contracterror]` enum with standard error codes before audit?
4. **Administrative Governance & Pause:** Will an emergency pause (`docs/pause-design.md`) or upgrade strategy (`docs/upgrade-strategy.md`) be implemented before mainnet deployment?

---

## 5. Summary Checklist for Audit Readiness

- [ ] Implement token transfer logic inside `deposit()`.
- [ ] Refactor lock storage to eliminate unbounded `Vec<LockEntry>` iteration.
- [ ] Implement explicit TTL extension calls (`extend_ttl`) for persistent and instance keys.
- [ ] Define and implement `#[contracterror]` enum.
- [ ] Publish standard Soroban events (`env.events().publish`).
- [ ] Add missing test cases (SAC deposit, vector scaling, authorization checks).
- [ ] Resolve upgradeability and emergency pause decisions.
