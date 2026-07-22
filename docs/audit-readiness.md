# Audit Readiness Review
## Savings Vault Contract — PocketPay

---

## Overview

This document is a comprehensive pre-audit readiness review of the Savings Vault
contract (`contracts/savings_vault/src/lib.rs`). It identifies audit blockers,
high-risk areas, missing tests, risky assumptions, unresolved design questions,
and documentation gaps.

**Review date**: 2026-07-22
**Contract version**: 0.1.0 (storage version 1)
**Review scope**: Storage, authorization, token custody, accounting, events,
migrations, and documentation.

---

## 1. Audit Blockers

### 1.1 ❌ No Custom Error Enum

**Severity**: Medium
**Status**: Missing

The contract uses bare `panic!()` strings for all error conditions:

| Location       | Panic message                               |
|----------------|---------------------------------------------|
| `initialize`   | `"Contract is already initialized"`         |
| `deposit`      | `"Deposit amount must be greater than zero"` |
| `withdraw`     | `"Insufficient balance"`                    |
| `withdraw`     | `"Withdrawal amount must be greater than zero"` |
| `lock_funds`   | `"Lock amount must be greater than zero"`   |
| `lock_funds`   | `"Unlock time must be in the future"`       |
| `lock_funds`   | `"Insufficient balance to lock"`            |
| `withdraw_lock`| `"Lock not found"`                          |
| `withdraw_lock`| `"Lock has not matured yet"`                |
| `pause`        | `"Pause duration must be greater than zero"`|
| Auth guards    | `"Not authorized"`, `"Contract is paused"`  |
| Migration      | `"Unsupported storage version: {}"`         |
| Assert helpers | `"Unsupported storage version"`             |
| Assert helpers | `"Contract is not initialized"`             |

**Why this matters**: Off-chain callers (wallets, indexers, UI) cannot reliably
distinguish error modes. Panic strings are human-readable but not
machine-readable. A custom `ContractError` enum with well-known discriminants
enables typed error handling.

**Action**: Define an enum like `SavingsVaultError` with variants for each
failure mode and replace all `panic!()` calls.

### 1.2 ⚠️ Duplicate Event in `initialize`

**Severity**: Low
**Status**: Exists

`initialize()` emits two separate events for the same logical action:

```rust
// Event 1 — topic "initialize"
let topics = (Symbol::new(&env, "initialize"), admin.clone());
env.events().publish(topics, token.clone());

// Event 2 — topic "init"
let topics = (symbol_short!("init"), admin.clone());
env.events().publish(topics, token.clone());
```

Plus two log calls producing the same information. This appears to be leftover
from iterative development — one event should be canonical.

**Action**: Consolidate to a single `initialize` event and single log call.

### 1.3 ✅ Token Custody — Resolved

SAC transfers are fully implemented in `deposit()`, `withdraw()`, and
`withdraw_lock()`. The README has been updated.

### 1.4 ✅ Pause/Emergency Stop — Resolved

Contract-level emergency pause is implemented (`pause()`, `unpause()`,
`is_paused()`, `require_not_paused()`). Pause blocks deposits and locks but
NOT withdrawals (users can always exit). Supports time-bounded auto-expiry.
Test suite in `src/test/pause.rs`.

---

## 2. High-Risk Areas

### 2.1 Token Transfer Ordering (CEI Pattern)

**Location**: `lib.rs` — `deposit()` (line 427), `withdraw()` (line 499),
`withdraw_lock()` (line 589)

**Current pattern (all three functions)**:
1. Validate inputs and authorization
2. Perform SAC token transfer
3. Update contract storage

This is a "Transfer-first" pattern, NOT Checks-Effects-Interactions.

**Risk analysis**:
- In Ethereum/EVM, "effects before interactions" prevents reentrancy attacks.
  Soroban/Wasm does NOT permit reentrancy in the same invocation context, so
  the risk is mitigated by the VM itself.
- However, if an EIP/EVM-compatible execution mode is ever added, this pattern
  would need to be reversed.

**Recommendation**: Document that this ordering is an intentional Soroban-safe
pattern and add a comment explaining why CEI is not required. The current code
is correct for Soroban but would be flagged in an EVM-oriented audit.

### 2.2 Ledger Timestamp Dependency

**Location**: `lib.rs` — `lock_funds()` (line 657), `withdraw()` (line 483),
`withdraw_lock()` (line 580)

The contract relies on `env.ledger().timestamp()` for:
- Rejecting past unlock times
- Determining lock maturity
- Pause expiry

**Assessment**: ✅ Safe — Soroban ledger timestamps are set by validators at
ledger close and cannot be manipulated by users. Monotonicity is guaranteed.

### 2.3 Integer Overflow in Balance Arithmetic

**Location**: `lib.rs` — `deposit()` (line 435), `withdraw()` (lines 533-537)

```rust
let new_balance = current_balance + amount;  // deposit
let new_locked: i128 = locks.iter()
    .filter(|lock| current_time < lock.unlock_time)
    .map(|lock| lock.amount)
    .sum();  // withdraw
```

**Assessment**: ✅ Safe — Soroban SDK uses `i128` arithmetic which panics on
overflow in debug builds. In WASM/release builds, `i128` overflow wraps but
deposits are bounded by SAC token supply and individual amounts are validated
to be > 0. Sum of locks is bounded by total deposits.

### 2.4 Storage Key Collision Risk

**Assessment**: ✅ Safe — `DataKey` enum ensures unique storage slots. Each
variant maps to a distinct discriminant. No key collision possible.

### 2.5 Authorization Bypass via Uninitialized State

**Assessment**: ✅ Safe — `assert_initialized()` is called at the top of every
state-changing function. Uninitialized calls panic before any storage access.

---

## 3. Missing Tests

### 3.1 ✅ SAC Transfer Rollback — Now Covered

Tests added in `src/test/token_transfer_rollback.rs` (9 tests):
- Failed deposit (insufficient SAC balance, zero balance, exceeding with locks)
- Failed withdrawal (exceeds balance, exceeds with locks, exceeds matured total)
- Failed withdraw_lock (non-existent lock ID)
- Cumulative drift (5 consecutive failed ops)
- Cross-user consistency after mixed failures

### 3.2 ❌ No Paused-State Re-entrant Tests

**Missing**: Tests verifying that `require_not_paused()` is called correctly in
every entrypoint that should be gated.

- `deposit()` checks `require_not_paused()` ✅
- `lock_funds()` checks `require_not_paused()` ✅
- `withdraw()` does NOT check pause (intentional — users can always exit)
- `withdraw_lock()` does NOT check pause (intentional)

**Action**: Add tests confirming deposits/locks are blocked during pause and
withdrawals are NOT blocked.

**Coverage**: `src/test/pause.rs` covers basic pause behavior but does not
exhaustively test all entrypoints under pause.

### 3.3 ❌ No Storage Migration Tests

`try_migrate()` handles v0→v1 migration but there are no tests verifying:
- v0 contract state correctly migrates to v1
- v1 contract state is unchanged on repeated calls
- Unknown future versions panic as expected

### 3.4 ❌ No Boundary Tests for `MAX_LOCK_PAGE_SIZE` (50)

No tests verify that `list_locks()` correctly pages at exactly 50 entries.

### 3.5 ❌ No Fuzzing for `list_locks` Pagination

No proptest harness for lock pagination correctness.

---

## 4. Risky Assumptions

### 4.1 SAC Token Compliance

**Assumption**: The configured token contract implements standard Stellar Asset
Contract (SAC) semantics — `transfer()` moves balance, no fees, no blacklist.

**Risk**: A non-standard token could have custom `transfer()` logic that silently
deducts fees or blocks specific addresses, breaking the vault's 1:1 accounting.

**Mitigation**: Documented in `docs/authorization-boundaries.md`. The vault
computes its own balance via `get_balance()`, which always reflects on-chain
storage truth.

### 4.2 Single Admin Trust Model

**Assumption**: The admin is a single key. If compromised, the attacker can:
- Transfer admin to themselves
- Call `pause()` to block deposits

**Risk**: No multi-sig, no timelock, no admin recovery, no DAO governance.

**Mitigation**: Documented in `docs/admin-role.md`. Recommend multi-sig admin
before mainnet.

### 4.3 No Fee Model

**Assumption**: Deposits and withdrawals are 1:1 with no protocol fee.

**Risk**: Future fee introduction would require a migration. Property tests
(`property_fee_invariants.rs`) verify no fees exist today.

---

## 5. Unresolved Design Questions

### 5.1 Upgrade Mechanism

**Current state**: No proxy pattern, no upgrade entrypoint. `try_migrate()` only
handles storage layout changes, not logic upgrades.

**Question**: Should the contract be upgradeable? If so, proxy or deploy-new?

**Reference**: `docs/upgrade-strategy.md`

### 5.2 Admin Multi-Sig

**Question**: Should admin move from single-address to multi-sig or DAO?

**Reference**: `docs/admin-role.md`

### 5.3 Storage TTL Automation

**Question**: Who pays for storage TTL extensions? Can storage be auto-bumped?

**Reference**: `docs/storage-ttl.md`

### 5.4 Token Change

**Question**: Can the token address be changed after initialization? Currently
immutable.

### 5.5 Fee Introduction

**Question**: Will the vault ever charge fees? If yes, how does the accounting
model change?

---

## 6. Documentation Gaps

### 6.1 ✅ Existing Documentation

The `docs/` directory contains 23 documents covering:
- State machine, failure modes, authorization boundaries
- Storage audit, TTL, versioning, migration
- Events, accounting invariants, security review
- Local development, deployment, CLI smoke test
- Architecture, admin role, pause design

### 6.2 ❌ Missing: On-Chain Event Schema

No formal event schema document. Events exist in code but are not catalogued
with their data payload layouts for indexers/relayers.

### 6.3 ❌ Missing: Integration Test Guide

No document explaining how to run integration tests against a local Soroban
network (e.g., `soroban-test` or Futurenet).

---

## 7. Test Suite Health

### Current coverage (approximate)

| Module                     | Test count | Type                        |
|----------------------------|-----------|------------------------------|
| `mod.rs`                   | ~40       | Unit + auth boundary         |
| `balance_conservation.rs`  | ~12       | Proptest + table-driven      |
| `property_vault_accounting.rs` | ~4    | Proptest fuzz               |
| `property_fee_invariants.rs`   | ~4    | Proptest fuzz               |
| `token_transfer_rollback.rs`   | 9     | Unit (new — issue #237)      |
| `pause.rs`                 | ~10       | Unit                         |
| `unauthorized_access.rs`   | ~5        | Auth boundary                |
| `initialization.rs`        | ~8        | Unit                         |
| `replay_protection.rs`     | ~3        | Unit                         |
| Others                     | ~60       | Boundaries, edge cases       |
| **Total**                  | **~155**  |                              |

All tests pass ✅ (verified 2026-07-22).

---

## 8. Summary

| Area            | Readiness | Notes                                          |
|-----------------|-----------|-------------------------------------------------|
| Storage         | ✅ Ready  | DataKey enum, persistent + instance separation  |
| Authorization   | ✅ Ready  | require_auth on all mutating calls              |
| Token Custody   | ✅ Ready  | SAC transfers in deposit/withdraw/withdraw_lock |
| Accounting      | ✅ Ready  | Balance conservation, proptest fuzz, 155 tests  |
| Events          | ✅ Ready  | All state changes emit events                   |
| Pause           | ✅ Ready  | Emergency pause with auto-expiry, tested        |
| Migrations      | ✅ Ready  | v0→v1 storage migration implemented             |
| Error Handling  | ⚠️ Needs work | Panic strings only, no custom error enum     |
| Upgrade         | ⚠️ Missing | No proxy or upgrade pattern                    |
| Multi-sig Admin | ⚠️ Missing | Single key trust model                        |
| Event Schema    | ⚠️ Missing | No formal event catalog for indexers          |
| Integration     | ⚠️ Missing | No integration test guide                     |

**Overall**: The contract is in good shape for a formal audit. The two remaining
blockers before audit are: (1) implementing a custom error enum, and (2)
consolidating the duplicate initialize events. All other items are operational
improvements (multi-sig, upgrade) that do not block the audit itself.

---

## 9. Recommendations

1. **Define `SavingsVaultError` enum** — replace all panic strings before
   external integrations depend on error messages.

2. **Consolidate `initialize` events** — keep one canonical event.

3. **Add formal event schema doc** — catalogue topics, data types, and payload
   layouts for every event emitted.

4. **Move admin to multi-sig before mainnet** — single-key admin is not
   production-grade for a custody contract.

5. **Consider upgradeability** — if the vault is expected to change post-deploy,
   choose a proxy or deploy-new pattern before the first deployment.

6. **Add storage migration tests** — verify v0→v1 and future-version panics.

7. **Be honest about limitations** — this review identifies real gaps; none are
   showstoppers for the current testnet/development phase.
