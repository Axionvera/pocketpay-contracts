# Contribution Quality Gate

This document defines the objective criteria for "payment-ready" or "production-ready" work in the PocketPay Contracts repository. To maintain high security and reliability, every Pull Request (PR) must pass through this quality gate before being considered for final approval.

## 1. Quality Gate Checklist

Every PR must satisfy the following checklist. If any item is missing or incomplete, the PR will be flagged for further work.

### Implementation
- [ ] **Completeness**: The feature or fix is fully implemented according to the issue description.
- [ ] **No Placeholders**: All `TODO`, `FIXME`, and `HACK` comments related to the change have been resolved.
- [ ] **Storage Integrity**: Storage usage follows the [Storage Audit Map](storage-audit.md) and correctly manages Persistent vs. Instance storage.
- [ ] **Authorization**: All state-changing functions correctly enforce `require_auth()` for the appropriate parties.
- [ ] **Error Handling**: Uses structured `ContractError` codes instead of generic panics where appropriate.

### Testing
- [ ] **Unit Tests**: Every new or modified function has unit tests for both success paths and failure paths (e.g., unauthorized access, invalid inputs).
- [ ] **Invariant Verification**: Changes to accounting logic are verified by new or existing [Property Tests](../contracts/savings_vault/src/test/property_vault_accounting.rs).
- [ ] **Event Snapshots**: Any changes to event schemas have updated [Snapshots](../contracts/savings_vault/test_snapshots/).
- [ ] **Local Verification**: All tests pass locally using `cargo test`.

### Documentation
- [ ] **README/Docs**: New features or architectural changes are documented in the relevant `docs/` files.
- [ ] **Acceptance Criteria**: The PR description explicitly lists how each Acceptance Criterion from the original issue was met.

### CI & Tooling
- [ ] **Formatting**: Code is formatted via `cargo fmt`.
- [ ] **Lints**: `cargo clippy --tests` passes with no warnings.
- [ ] **Build**: `make build-release` succeeds and the WASM size remains within acceptable limits.
- [ ] **Local verification**: `make verify` passes (format, Clippy, tests, and release WASM build).

---

## 2. Contract-Specific Testing Expectations

Soroban smart contracts require rigorous testing due to their immutable nature once deployed. We expect:

1.  **Authorization Boundaries**: Tests must explicitly verify that a function fails when called by an unauthorized address.
2.  **State Rollbacks**: Tests must verify that if a transaction fails (e.g., token transfer failure), no storage mutations occur.
3.  **Maturity Checks**: For time-locked funds, tests must verify behavior exactly at, before, and after the maturity timestamp.
4.  **Edge Case Amounts**: Test with `0`, `1`, `MAX`, and amounts just below/above configured limits.

---

## 3. Examples of Incomplete Work

To avoid common pitfalls, here are examples of PRs that do **not** pass the quality gate:

- **"Happy Path Only"**: A PR that adds a `deposit` feature but only tests a successful deposit, skipping unauthorized or over-limit tests.
- **"Logic without Invariants"**: A PR that changes how balances are calculated but doesn't update the property-based tests that ensure total funds remain constant.
- **"Stale Docs"**: A PR that changes a function signature in the contract but leaves the `api-reference.md` or `README.md` outdated.
- **"Ignoring Clippy"**: A PR where `cargo clippy` emits warnings that the contributor describes as "unrelated" or "minor".
- **"WIP in Main"**: PRs marked as "Work In Progress" or containing half-implemented logic should not be requested for final review.

---

## 4. How to Use This Gate

1.  **Before Opening a PR**: Review your work against this checklist.
2.  **In Your PR Description**: Reference this quality gate and confirm that all items are checked.
3.  **Reviewers**: Use this checklist as the primary framework for your review. If the gate isn't met, request changes immediately.

For more details on local reproduction of tests, see the [Test Reproduction Guide](reproducing-test-failures.md).
