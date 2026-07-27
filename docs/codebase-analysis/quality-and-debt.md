# Quality And Debt Assessment

This document assesses the codebase's quality, operational readiness, and
remaining technical debt based on the current repository state.

## 1. Overall Assessment

The repository is in a strong **development/testnet** state, but not in a
fully production-ready state.

### Major strengths

- Clear business domain: token-backed savings vault with time locks
- Stable typed error model using `#[contracterror]`
- Consistent auth boundaries on mutating paths
- Good event coverage across major state transitions
- Strong local Rust test coverage, including property-based invariants
- Explicit storage-versioning support and migration hook
- Good separation between global config and per-user persistent state

### Major weaknesses

- Runtime implementation remains concentrated in a single large `lib.rs`
- Several docs no longer match the actual code
- Automation/CI is too thin for the size of the test and docs surface
- Some read paths scale linearly with historical lock count
- Storage TTL management depends on operations outside the contract
- Mainnet governance and upgrade posture remain unresolved

## 2. Validation And Tooling Posture

### Verified during this analysis

- `cargo test` completed successfully from the workspace root
- The registered Rust suite covers initialization, auth, accounting, pause,
  events, storage versioning, rollback behavior, and property-based invariants

### Tooling/process concerns

- The only GitHub workflow in the repo triggers an external automation dispatch.
- There is no in-repo CI workflow for:
  - `cargo test`
  - `cargo fmt --check`
  - `cargo clippy`
  - WASM build validation
- The local `Makefile` only provides:
  - `build-release`
  - `wasm-size`
- Root docs still mention targets such as `make test`, `make build-wasm`, and
  `make clean`, but those targets do not exist.

## 3. Code Quality Findings

### 3.1 Monolithic implementation file

**Observation**

`contracts/savings_vault/src/lib.rs` contains the full runtime implementation:
state types, storage keys, errors, helpers, admin logic, user flows, and read
models.

**Impact**

- Harder navigation for new contributors
- Higher chance of merge conflicts
- More difficult targeted review when changing one concern

**Risk level**

Medium maintainability risk, low immediate correctness risk.

### 3.2 Documentation drift

**Observation**

Multiple docs still describe old behavior or abandoned designs.

### Main drift patterns observed

- Panic-string-only errors are described even though `ContractError` now exists.
- Several docs still describe `DataKey::Locks(user)` as the main storage model.
- Some docs still mention missing pause/event support although both are present.
- Root/project structure docs still reference `test.rs` instead of `src/test/`.
- Some doc links and absolute file references point to old local machine paths.

**Impact**

- Misleads auditors and new maintainers
- Makes architectural review slower than necessary
- Increases chance of implementation mistakes based on outdated guidance

**Risk level**

High maintainability/documentation risk.

### 3.3 Orphaned TypeScript test

**Observation**

`tests/atomicity/transfer-atomicity.test.ts` references services under
`src/services/...`, but the repository contains no matching Node project,
package manifest, or service implementation.

**Impact**

- Confusing for maintainers
- Creates a false impression of broader cross-language coverage
- Adds noise during codebase comprehension

**Risk level**

Low runtime risk, medium maintainability risk.

## 4. Performance Assessment

### 4.1 Linear scans over historical lock ids

**Observation**

Several read and write paths iterate `1..next_lock_id`:

- `get_balance_snapshot`
- `get_lock_summary`
- `get_locked_balance`
- `can_withdraw`
- `load_locks`
- `lock_funds` when recomputing `new_locked` for event payload

**Impact**

- Cost grows with the number of locks a user has ever created
- Historical withdrawn locks still contribute to iteration cost
- Heavy users can become expensive to query on-chain

**Risk level**

Medium-to-high performance risk if per-user lock count grows substantially.

### 4.2 No lock-count cap

**Observation**

`list_locks` caps page size at 50, but the contract does not cap the total
number of locks a user may create.

**Impact**

- Read helpers remain exposed to unbounded historical growth
- Gas/resource consumption can become uneven across accounts

**Risk level**

Medium performance and anti-DoS risk.

### 4.3 Event payload recomputation cost

**Observation**

After creating a lock, `lock_funds` recomputes the user's immature locked total
by scanning every historical lock id in order to emit `new_locked`.

**Impact**

- Extra work on every lock creation
- Event convenience comes at the cost of growing write-time complexity

**Risk level**

Medium performance inefficiency, not a correctness bug.

## 5. Security Assessment

### 5.1 Strong areas

- User-facing mutations require `user.require_auth()`
- Admin-facing mutations require both signature and stored-admin match
- Withdrawals remain open during emergency pause
- Deposit/withdraw/withdraw_lock rollback behavior is tested
- Stable contract error codes improve SDK-side handling and analytics
- Token custody invariants are backed by property tests

### 5.2 Main security risks and trust assumptions

#### Single-admin control

- One admin address controls pause, unpause, config setters, and admin transfer
- Fine for testnet/development
- Weak for mainnet governance without multi-sig or role separation

#### TTL expiry risk

- Persistent entries rely on Soroban TTL mechanics
- If operators fail to extend TTL, storage can disappear while SAC-held funds
  still exist at the contract address
- This is primarily an operational safety risk, not a logic bug

#### Token-behavior assumption

- The contract assumes the configured token behaves like a normal SAC
- Fee-on-transfer or policy-heavy token behavior could break 1:1 accounting

#### No upgrade path

- Logic is effectively immutable once deployed
- Storage migration exists, but it is not a logic-upgrade framework

#### No external audit

- The repo explicitly targets development/testnet use
- No third-party audit artifact is published in the repository

### 5.3 Policy gap worth clarifying

**Observation**

Admin-configured `MinLockDurationSecs` and `MaxLockDurationSecs` are enforced in
`lock_funds`, but `extend_lock` only checks:

- lock exists
- lock is not withdrawn
- new time is in the future
- new time is strictly greater than the current unlock time

**Why this matters**

If the intended policy is "all active lock durations must respect current admin
limits", then `extend_lock` currently bypasses that policy. If the intended
policy is "limits apply only at creation time", the docs should state that more
explicitly.

**Risk level**

Medium policy/requirements risk. This is not automatically a bug, but it is a
behavioral ambiguity that should be resolved.

## 6. Maintainability Assessment

### 6.1 Dead or transitional storage concepts

`DataKey::Locks(Address)` still exists, but live lock state is keyed by
`DataKey::Lock(user, lock_id)`.

**Consequences**

- Readers must mentally separate historical design from current design
- Docs easily drift when they rely on the obsolete vector model

### 6.2 Large documentation surface

The `docs/` directory is extensive and valuable, but it has become hard to keep
fully synchronized with the code.

**Consequences**

- Excellent breadth of topic coverage
- Elevated long-term maintenance cost
- Higher chance of contradictory statements across documents

### 6.3 Mixed sources of truth

For many topics, the actual source of truth is currently split between:

- `lib.rs`
- test modules
- newer docs such as `error-codes.md`
- older docs that still describe superseded behavior

That makes onboarding harder than it needs to be.

## 7. Technical Debt Inventory

### High priority debt

1. Sync stale docs to the current contract implementation
2. Add real CI for test/lint/build verification
3. Decide mainnet posture for admin governance and upgrades
4. Clarify TTL operational ownership and failure handling

### Medium priority debt

1. Break `lib.rs` into internal modules without changing the public interface
2. Remove or formally deprecate `DataKey::Locks(Address)`
3. Clarify whether lock duration limits should apply to `extend_lock`
4. Revisit lock-scan costs for high-history users
5. Remove or relocate the orphaned TypeScript test

### Low priority debt

1. Expand task-runner convenience targets if docs continue to reference them
2. Consolidate overlapping docs with similar subject matter

## 8. Recommended Next Steps

### Short term

1. Make `docs/comprehensive-analysis.md` and this package the canonical review entry point
2. Update stale high-traffic docs:
   - `README.md`
   - `architecture.md`
   - `storage-audit.md`
   - `SECURITY_REVIEW.md`
   - any docs still centered on `Locks(user)` or panic strings
3. Add a GitHub Actions workflow for fmt, clippy, test, and release build

### Medium term

1. Split runtime code into internal modules such as:
   - `state.rs`
   - `errors.rs`
   - `admin.rs`
   - `funds.rs`
   - `locks.rs`
   - `reads.rs`
2. Decide whether lock-duration config should constrain lock extension
3. Introduce either:
   - a lock-count cap, or
   - more scalable aggregate accounting for large lock histories

### Long term

1. Choose a real mainnet governance model
2. Choose an upgrade or migration strategy
3. Add a formal operational runbook for TTL maintenance and custody monitoring
4. Pursue an external security audit after the above are stabilized

## 9. Bottom Line

The codebase demonstrates thoughtful contract design and unusually strong local
test coverage for a project of this size. Its main weaknesses are not basic
logic hygiene, but rather:

- scaling characteristics of lock-history reads,
- incomplete operational hardening,
- governance/upgrade gaps for serious deployment,
- and a large amount of documentation drift created by rapid iteration.

For development, learning, and testnet usage, the repository is in good shape.
For production or auditor handoff, the highest-return work is now on process,
documentation synchronization, and operational hardening rather than on
fundamental feature completeness.
