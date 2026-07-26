# Audit Readiness Review — Savings Vault

> **Status:** Pre-audit readiness assessment — **not production-ready or mainnet-ready.**
>
> This document identifies audit blockers, high-risk areas, missing tests, risky
> assumptions, unresolved design questions, and documentation gaps for the
> Savings Vault contract (`contracts/savings_vault/src/lib.rs`).
>
> **Review date:** 2026-07-24  
> **Contract version:** `0.1.0` (storage version `1`)  
> **Scope:** Token custody, storage, accounting, authorization, events,
> migrations, and documentation.

For the behavioural test map, see [test-coverage.md](test-coverage.md). For the
failure-mode catalogue, see [failure-mode-catalogue.md](failure-mode-catalogue.md).
For the pre-audit checklist (legacy), see [audit-preparation.md](audit-preparation.md).

---

## Executive Summary

The Savings Vault has moved to **token-backed custody** via the Stellar Asset
Contract (SAC), with internal accounting, time locks, emergency pause, on-chain
events, and a large unit/property test suite. That is meaningful progress toward
audit readiness, but **several items remain open before an external auditor or
mainnet deployment should be treated as appropriate**.

| Readiness area | Assessment | Notes |
|----------------|------------|-------|
| Token custody | **Mostly ready** | SAC transfers in `deposit`, `withdraw`, `withdraw_lock` |
| Accounting invariants | **Mostly ready** | Conservation + proptest coverage; O(n) lock scans remain |
| Authorization | **Mostly ready** | `require_auth` on mutating calls; single-key admin |
| Storage layout | **Mostly ready** | Typed `DataKey`; TTL ops external; dead `Locks` key |
| Events | **Partial** | Emitted in code; schema/doc drift on some topics |
| Error handling | **Not ready** | Panic strings only — no `#[contracterror]` enum |
| Upgradability | **Not ready** | No logic upgrade path; storage migration stub only |
| Operations / CI | **Not ready** | No in-repo build/test CI; TTL not tested on-chain |
| External audit | **Not started** | No third-party audit report in repository |

**Bottom line:** The contract is suitable for **testnet and development review**,
not for claims of production readiness. Address the audit blockers below before
engaging an external auditor or expanding SDK/mobile integration.

---

## 1. Audit Blockers

These items should be resolved (or explicitly accepted as audit findings with
documented risk) before a formal external audit is commissioned.

### 1.1 No structured error enum — **Blocker**

**Current behaviour:** All failure paths use `panic!()` with string messages
(e.g. `"Insufficient balance"`, `"Contract is paused"`).

**Why it blocks audit readiness:**

- Off-chain callers (SDK, mobile UI) cannot reliably map failures to typed codes.
- Panic strings are fragile — wording changes break clients that parse them.
- [error-codes.md](error-codes.md) and [sdk-error-mapping-guide.md](sdk-error-mapping-guide.md)
  describe desired behaviour that the contract does not yet implement.

**Required action:** Introduce a `#[contracterror]` enum and replace panic strings;
update tests from `#[should_panic(expected = "...")]` to typed error assertions.

**Reference:** `contracts/savings_vault/src/lib.rs` — all public entry points.

---

### 1.2 No contract logic upgrade path — **Blocker for mainnet**

**Current behaviour:** Deployed WASM is immutable. `try_migrate()` only bumps
storage layout version markers (v0→v1); it does not upgrade contract logic.

**Why it blocks mainnet readiness:**

- Critical bugs cannot be patched in place.
- Users must manually migrate to a new contract ID.

**Required action before mainnet:** Choose and implement a strategy from
[upgrade-strategy.md](upgrade-strategy.md), or document immutability as an
explicit, accepted product decision with a manual migration playbook.

**Acceptable for testnet audit:** Yes — but auditors must be told the contract
is intentionally non-upgradeable today.

---

### 1.3 Single-key admin with pause authority — **Blocker for mainnet**

**Current behaviour:** One admin address can `pause`, `unpause`, and
`transfer_admin`. Pause blocks new deposits and locks but not withdrawals.

**Why it blocks mainnet readiness:**

- Compromised admin key can indefinitely disrupt new deposits (until pause
  expiry, then re-pause).
- No multi-sig, timelock, or guardian role.

**Required action before mainnet:** Multi-sig admin or equivalent; see
[admin-role.md](admin-role.md) and [admin-pause-threat-model.md](admin-pause-threat-model.md).

**Acceptable for testnet audit:** Yes — document as a known trust assumption.

---

### 1.4 Documentation drift — **Blocker for auditor onboarding**

Several docs contradict the current implementation and will confuse auditors:

| Document | Stale claim | Actual behaviour |
|----------|-------------|------------------|
| [README.md](../README.md) Known Limitations | "No on-chain events" | Events emitted for all major state changes |
| [README.md](../README.md) Known Limitations | Repeated "No custom error enum" | Still true for errors; duplicated six times |
| [deployment-environments.md](deployment-environments.md) | "internal balance tracking rather than real token custody" | SAC custody implemented |
| [sdk-contract-sequence.md](sdk-contract-sequence.md) | Deposit updates internal accounting only | Deposit performs SAC transfer |
| [audit-preparation.md](audit-preparation.md) | Deposit does not transfer tokens; no events | Outdated — superseded by this review |
| [events.md](events.md) vs code | `transfer_admin` topic naming | Code emits `xferadmin`; some tests expect `transfer_admin` |

**Required action:** Reconcile docs with `lib.rs` before sending materials to
an external auditor. Treat this review as the current source of truth until
docs are synced.

---

### 1.5 No external security audit — **Blocker for production**

There is no published third-party audit report, bug bounty, or formal verification
artifact in this repository.

**Required action:** Engage an auditor after blockers 1.1–1.4 are addressed;
use [audit-preparation.md](audit-preparation.md) as a supplementary checklist.

---

## 2. High-Risk Areas

These are not necessarily audit blockers for testnet review, but auditors and
maintainers should scrutinize them closely.

### 2.1 Token transfer before storage update (transfer-first pattern)

**Location:** `deposit`, `withdraw`, `withdraw_lock` in `lib.rs`.

**Pattern:** SAC `transfer` runs before persistent storage is updated.

**Risk:** On EVM this violates CEI and enables reentrancy. On Soroban, reentrancy
in the same invocation is not permitted; failed transfers roll back the entire
transaction atomically (verified in `token_transfer_rollback.rs`).

**Residual risk:** If Soroban semantics or token implementation change, ordering
assumptions should be revisited.

**Recommendation:** Add an inline comment in `lib.rs` documenting Soroban-safe
ordering rationale for auditors.

---

### 2.2 Global token custody invariant

**Invariant:** Sum of all user available + locked balances ≤ SAC balance held by
the contract address.

**Enforcement:** Property tests in `property_vault_accounting.rs`
(`prop_global_token_custody`).

**Risk:** A logic bug that credits internal balances without a matching deposit
transfer would allow draining other users' tokens on withdraw. Deposit-side SAC
integration mitigates this; regression tests must stay in place.

---

### 2.3 O(n) lock iteration per operation

**Location:** `withdraw`, `get_balance`, `get_locked_balance`, `can_withdraw`,
`lock_funds` iterate from lock ID `1` to `NextLockId - 1`.

**Risk:** Users with many locks pay increasing compute cost; potential DoS via
gas/resource limits on heavy accounts.

**Mitigation today:** Locks stored under individual keys (not one giant vector);
`list_locks` caps page size at 50.

**Unresolved:** No hard cap on lock count per user.

---

### 2.4 Storage TTL expiry

**Location:** Persistent entries (`Balance`, `Lock`, `NextLockId`).

**Risk:** If TTL is not extended operationally, entries may expire and reads
return default values (e.g. zero balance) while SAC tokens remain in the contract.

**Test gap:** No unit or integration test simulates TTL expiry (see §3.4).

**Reference:** [storage-ttl.md](storage-ttl.md)

---

### 2.5 SAC token compliance assumption

**Assumption:** Configured token is a standard SAC — `transfer` moves full amount,
no fees, no transfer hooks, no blacklist.

**Risk:** Non-standard tokens break 1:1 internal accounting.

**Reference:** [authorization-boundaries.md](authorization-boundaries.md),
[vault-custody-assumptions.md](vault-custody-assumptions.md)

---

### 2.6 Emergency pause trust model

**Design:** Admin can pause deposits/locks; users can always withdraw.

**Risk:** Malicious or compromised admin can block new inflows during an incident
window; cannot steal user funds directly via pause.

**Reference:** [admin-pause-threat-model.md](admin-pause-threat-model.md),
[pause-design.md](pause-design.md)

---

### 2.7 Dead storage key: `DataKey::Locks(Address)`

**Location:** `DataKey` enum defines `Locks(Address)` but all lock data is stored
under `Lock(Address, u64)`. The `load_locks` helper reconstructs from individual keys.

**Risk:** Low — no collision, but confusing for auditors reviewing storage layout.

**Reference:** [storage-audit.md](storage-audit.md)

---

## 3. Missing Tests

Based on `contracts/savings_vault/src/test/` and [test-coverage.md](test-coverage.md).

### 3.1 Covered since earlier reviews (no longer gaps)

| Area | Tests |
|------|-------|
| SAC deposit transfer | `test_deposit`, `test_deposit_fails_when_token_transfer_fails`, `token_transfer_rollback.rs` |
| SAC withdraw round-trip | `test_withdraw_returns_tokens_to_user`, `token_backed_withdrawals.rs` |
| Auth rejection without mock | `unauthorized_access.rs`, `test_*_requires_user_authorization` in `mod.rs` |
| Pause gating | `pause.rs` — deposit/lock blocked; withdraw/withdraw_lock allowed |
| Storage migration v0→v1 | `storage_version.rs` — legacy missing version, invalid version panic |
| Event schemas | `event_schema.rs`, `event_compatibility.rs` |
| `list_locks` pagination edges | `lock_read_helpers.rs` — `limit=0`, offset past end, max page size |
| Property / fuzz accounting | `property_vault_accounting.rs`, `property_fee_invariants.rs` |

### 3.2 Remaining gaps

| Gap | Severity | Notes |
|-----|----------|-------|
| **Withdraw SAC transfer failure after balance check** | Medium | Rollback tests cover insufficient *internal* balance; no test where internal accounting passes but SAC transfer fails (e.g. contract SAC balance drained externally) |
| **Withdraw_lock SAC transfer failure** | Medium | Same as above for `withdraw_lock` |
| **`pause` / `unpause` require_auth without mock** | Low | Admin auth failures implied; no explicit no-mock test like user ops |
| **TTL expiry behaviour** | High (ops) | FM-STG-03 in failure-mode catalogue — not tested |
| **Large lock count stress** | Medium | No test with hundreds of locks measuring resource limits |
| **On-chain integration tests** | Medium | All tests use SDK testutils; no CI job against testnet/Futurenet |
| **Event topic consistency** | Low | `event_schema.rs` expects `transfer_admin`; `lib.rs` emits `xferadmin` — tests may not catch doc/code drift uniformly |
| **In-repo CI** | Medium (process) | No GitHub Actions workflow running `cargo test`, `clippy`, `fmt` |

### 3.3 Recommended tests before external audit

1. Simulate contract SAC insolvency during `withdraw` / `withdraw_lock` and assert
   full transaction rollback (no internal balance mutation).
2. Add TTL expiry simulation test if Soroban testutils support it, or document
   as out-of-scope for unit tests with an integration test plan.
3. Add `#[should_panic]` / typed-error tests for `pause`/`unpause`/`transfer_admin`
   when caller is not admin without `mock_all_auths`.
4. Resolve and test canonical event topic for admin transfer (`xferadmin` vs
   `transfer_admin`).

---

## 4. Risky Assumptions

| # | Assumption | Impact if wrong | Mitigation status |
|---|------------|-----------------|-------------------|
| A1 | Token is a compliant SAC | Accounting desync, failed transfers | Documented; no on-chain token whitelist |
| A2 | Ledger timestamp is honest | Lock maturity / pause expiry wrong | Safe on Soroban (validator-set timestamp) |
| A3 | Admin key is honest | Pause abuse, admin transfer | Documented; no multi-sig |
| A4 | No protocol fees | 1:1 deposit/withdraw | Verified by `property_fee_invariants.rs` |
| A5 | Operators extend storage TTL | User state appears zero; funds stranded in contract | Manual ops only — [storage-ttl.md](storage-ttl.md) |
| A6 | SDK does not parse panic strings | Brittle error UX | **Violated today** — SDK guide exists but contract lacks typed errors |
| A7 | Contract WASM is immutable post-deploy | No in-place bug fixes | By design — [upgrade-strategy.md](upgrade-strategy.md) |
| A8 | Users migrate manually on new deployments | Stranded funds on old contract ID | No migration tooling |

---

## 5. Unresolved Design Questions

These require product/security decisions before mainnet, not just implementation.

| # | Question | Options | References |
|---|----------|---------|------------|
| Q1 | Should the contract be upgradeable? | Proxy, migration contract, redeploy-only | [upgrade-strategy.md](upgrade-strategy.md) |
| Q2 | What admin model for mainnet? | Multi-sig, timelock, DAO | [admin-role.md](admin-role.md) |
| Q3 | Who pays for storage TTL extensions? | User-pays-on-action, relayer, admin cron | [storage-ttl.md](storage-ttl.md) |
| Q4 | Can the token address ever change? | Immutable (today) vs admin rotate | `initialize` sets token once |
| Q5 | Will fees be introduced? | Breaks current 1:1 model | [vault-fee-model.md](vault-fee-model.md) |
| Q6 | Max locks per user? | Uncapped (today) vs hard limit | Performance / DoS tradeoff |
| Q7 | Should `withdraw` consume matured locks FIFO by ID or by unlock time? | Today: ascending lock ID order | Document for users/SDK |
| Q8 | Canonical event topic naming | Short symbols (`xferadmin`) vs descriptive (`transfer_admin`) | [events.md](events.md), `event_schema.rs` |

---

## 6. Area-by-Area Review

### 6.1 Token custody

**Implementation:** `deposit` transfers user→contract; `withdraw` and
`withdraw_lock` transfer contract→user via `token::Client`.

**Strengths:**

- Real SAC integration (not internal-only bookkeeping).
- Failed deposits roll back (`test_deposit_fails_when_token_transfer_fails`,
  `token_transfer_rollback.rs`).
- Global custody invariant property-tested.

**Weaknesses:**

- No handling for non-SAC or fee-on-transfer tokens.
- No on-chain reconciliation view — indexers must compare SAC balance to summed
  internal state off-chain.
- Insolvency path (contract SAC balance < summed liabilities) not unit-tested.

**Docs:** [balance-reconciliation.md](balance-reconciliation.md),
[vault-custody-assumptions.md](vault-custody-assumptions.md)

---

### 6.2 Storage

**Implementation:** Instance storage for admin, token, init flag, pause, storage
version. Persistent storage for per-user balances, locks, lock ID counter.

**Strengths:**

- Typed `DataKey` enum prevents key collisions.
- Per-lock keys avoid unbounded vector serialization.
- Storage version migration hook exists.

**Weaknesses:**

- Unused `DataKey::Locks` variant.
- No in-contract TTL bump on user actions.
- TTL expiry untested.

**Docs:** [storage-audit.md](storage-audit.md), [storage-migration.md](storage-migration.md),
[storage-versioning.md](storage-versioning.md)

---

### 6.3 Accounting

**Implementation:** Available balance in `Balance(user)`; locks in
`Lock(user, id)`; maturity evaluated at read time via ledger timestamp.

**Strengths:**

- Balance conservation table-driven and property tests.
- `i128` boundary tests in `maximum_amount_boundary.rs`.
- Partial withdraw with mixed available + matured locks tested.

**Weaknesses:**

- `get_balance` includes matured lock amounts still stored as lock entries until
  withdrawn — SDK must document semantics.
- Lock iteration cost scales with lock count.

**Docs:** [accounting-invariants.md](accounting-invariants.md),
[state-machine.md](state-machine.md)

---

### 6.4 Authorization

**Implementation:** `require_auth()` on `initialize`, `deposit`, `withdraw`,
`lock_funds`, `withdraw_lock`, `pause`, `unpause`, `transfer_admin`. Read-only
queries do not require auth.

**Strengths:**

- Broad test coverage in `unauthorized_access.rs` and auth-specific tests in
  `mod.rs`.
- Admin cannot mutate user balances or locks (`admin_invariant_guard.rs`).

**Weaknesses:**

- Single admin key.
- No role separation (pauser vs upgrader vs treasurer).

**Docs:** [authorization-boundaries.md](authorization-boundaries.md),
[admin-role.md](admin-role.md)

---

### 6.5 Events

**Implementation:** `env.events().publish` on initialize, deposit, withdraw,
withdraw_lock, lock, pause, unpause, transfer_admin.

**Strengths:**

- Events exist for all major state transitions.
- Schema tests in `event_schema.rs` and `event_compatibility.rs`.
- Formal schema doc in [events.md](events.md).

**Weaknesses:**

- Topic naming inconsistency (`xferadmin` in code vs `transfer_admin` in some
  test/doc references).
- SDK consumers must not rely on undocumented topic aliases.
- README still claims no events.

---

### 6.6 Migrations

**Implementation:** `STORAGE_VERSION = 1`; `try_migrate()` handles v0→v1;
`assert_supported_storage_version` on mutating paths; future versions panic.

**Strengths:**

- Migration tests in `storage_version.rs`.
- Documented migration guide.

**Weaknesses:**

- No logic upgrade — storage migration only.
- No test for idempotent `try_migrate` on already-v1 contract beyond implicit
  coverage through normal calls.

**Docs:** [storage-migration.md](storage-migration.md), [upgrade-strategy.md](upgrade-strategy.md)

---

### 6.7 Documentation

**Strengths:** 40+ docs covering architecture, threat models, API naming, deployment,
failure modes, walkthroughs, and security checklists.

**Weaknesses:**

- Stale sections in README, deployment guide, SDK sequence diagrams, and
  audit-preparation checklist.
- No single integration-test runbook against live testnet in CI.
- [test-coverage.md](test-coverage.md) lists gaps that are partially stale
  (withdraw_lock event and pagination gaps are now covered in other modules).

---

## 7. Pre-Audit Checklist for Maintainers

Use this before sharing the repo with an external auditor:

- [ ] Resolve or document acceptance of §1 audit blockers
- [ ] Sync README and deployment docs with token-backed behaviour and events
- [ ] Implement `#[contracterror]` enum (blocker 1.1)
- [ ] Reconcile event topic naming (`xferadmin` vs `transfer_admin`)
- [ ] Add SAC insolvency rollback tests (§3.2)
- [ ] Confirm `cargo test --workspace`, `cargo clippy`, `cargo fmt --check` pass locally
- [ ] Prepare deployed testnet contract ID and invocation transcript ([cli-smoke-test.md](cli-smoke-test.md))
- [ ] Provide threat model docs: [admin-pause-threat-model.md](admin-pause-threat-model.md), [SECURITY_REVIEW.md](SECURITY_REVIEW.md)
- [ ] State explicitly: **testnet/educational scope only** — do not overstate readiness

---

## 8. Related Documents

| Document | Purpose |
|----------|---------|
| [audit-preparation.md](audit-preparation.md) | Legacy checklist — verify against this review |
| [security-checklist.md](security-checklist.md) | PR-level security checklist |
| [test-coverage.md](test-coverage.md) | Behaviour-to-test map |
| [failure-mode-catalogue.md](failure-mode-catalogue.md) | Failure modes and coverage |
| [storage-audit.md](storage-audit.md) | Storage keys and invariants |
| [events.md](events.md) | Event schema for indexers |
| [upgrade-strategy.md](upgrade-strategy.md) | Upgrade options research |

---

*This review reflects the codebase as of 2026-07-24. Re-run this assessment after
material contract, test, or documentation changes.*
