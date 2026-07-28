# Vault Contract: Audit Readiness Review

**Date:** July 28, 2026  
**Status:** Pre-Audit Internal Review  
**Version:** v1.0.0 (Pre-production)

---

## 1. Summary of Vault Architecture
The Vault contract serves as a token-backed custody solution for the PocketPay ecosystem. It allows users to lock assets, supports multi-lock storage, and manages time-based or condition-based withdrawals.

## 2. High-Risk Areas & Security Blockers

### 2.1 Token Custody & Internal Accounting
*   **Invariants:** The total sum of internal user balances *must* be less than or equal to the actual token balance held by the contract address.
*   **Risk:** If the contract uses internal state to track balances (e.g., a `Map<Address, i128>`), any bug in the `deposit` or `withdraw` logic could allow more tokens to be withdrawn than exist.
*   **Check:** Ensure all math uses `checked_add`, `checked_sub`, and `checked_mul`.

### 2.2 Authorization & Admin Controls
*   **Authorization:** Every state-changing function (`withdraw`, `change_admin`, `lock`) must call `sender.require_auth()`. 
*   **Admin Risk:** If the admin has "God Mode" (the ability to withdraw any user's funds), this must be explicitly documented as a centralized risk.
*   **Blocker:** Verify that `require_auth` is correctly scoped to the specific arguments being passed to prevent cross-contract replay.

### 2.3 Multi-Lock Storage & TTL (Time To Live)
*   **Storage Invariants:** In Soroban, ledger entries have a TTL. If the vault's storage (Persistent/Instance) expires, user funds could be "trapped" until the entry is bumped.
*   **Blocker:** The contract must implement a mechanism to bump TTL on every user interaction to prevent data loss.

## 3. Missing Test Coverage
Based on a review of `tests/`, the following scenarios are currently missing or under-tested:
- [ ] **Unauthorized Withdrawal:** Attempting to withdraw funds using a valid address but without a signature from the owner.
- [ ] **Integer Overflow:** Testing deposits that exceed `i128` limits.
- [ ] **Re-entrancy/Logic Race:** Rapid succession of deposit/withdraw calls in a single ledger.
- [ ] **Storage Expiry Simulation:** Mocking a scenario where a persistent lock entry reaches its minimum TTL.

## 4. Unresolved Design Decisions
- **Fee Structure:** It is currently undecided if the vault should take a protocol fee on withdrawal or if fees are handled at the SDK level.
- **Contract Upgradability:** Should the contract be upgradeable via `Wasm` upload, or should it be "frozen" in production?
- **Emergency Pause:** Currently, there is no "Pause" function to stop withdrawals in the event of a detected exploit.

## 5. Cross-Repo Risks (SDK & Mobile)

### 5.1 SDK Compatibility
*   **BigInt Precision:** The SDK (JavaScript/TypeScript) may experience precision loss if `i128` values from the contract are not handled using the native `BigInt` type.
*   **Event Polling:** The vault relies on events for the mobile UI to update. If the SDK misses an event, the UI might show a "Locked" state when the funds have already been released.

### 5.2 Mobile Readiness
*   **Signing Latency:** Mobile biometrics/signing can be slow. If the contract has a "window" for execution, slow signing could cause transaction expiration.
*   **Validation:** The mobile app must validate the `VaultID` before attempting a deposit to prevent sending funds to an uninitialized lock.

## 6. Error Code Registry
| Error Code | Name | Description |
|---|---|---|
| 100 | NotAuthorized | Triggered when `require_auth` fails. |
| 101 | InsufficientBalance | Attempting to withdraw more than is locked. |
| 102 | LockStillActive | Attempting to withdraw before the timelock expires. |
| 103 | VaultAlreadyExists | Attempting to initialize an existing VaultID. |

---
**Review Conclusion:** The vault is currently **NOT audit-ready**. High-risk accounting invariants and storage TTL management require implementation/hardening.
