# Contract Contributor Security Checklist

This is the checklist to use for every pull request that touches
`contracts/savings_vault` — new functions, changed logic, storage, events, or
error handling. It exists so accounting, authorisation, storage, events, and
failure behaviour get checked the same way on every PR, instead of relying on
each reviewer to remember what to look for.

How to use it:

- **Contributors**: run through the relevant sections before opening a PR and
  note anything that doesn't apply (e.g. "no storage change") in the PR
  description.
- **Maintainers**: paste or reference this checklist in the PR review and
  confirm each applicable item before approving.

This checklist intentionally stays short and points to the canonical doc for
each topic instead of restating it — see those docs for the full rationale,
edge cases, and test references.

## 1. Balance & Accounting Invariants

- [ ] The change preserves that a user's `Balance` (available) plus the sum of
      their active `LockEntry` amounts never exceeds what they actually
      deposited, and never goes negative.
- [ ] Every code path that debits or credits a balance (`deposit`, `withdraw`,
      `withdraw_lock`, `lock_funds`, `extend_lock`) is checked against
      [Formal Accounting Invariants](accounting-invariants.md) — confirm no
      invariant in that document is broken by the change.
- [ ] A failed operation (rejected input, failed token transfer, panic) leaves
      `Balance` and locked balance byte-for-byte unchanged — no partial state
      mutation before the failure point. See
      [Balance Reconciliation Design Note](balance-reconciliation.md).
- [ ] Per-user state is isolated: one user's deposit, lock, or withdrawal
      cannot change another user's balance or locks.

## 2. Lock State & Timed Operations

- [ ] `unlock_time` is validated as strictly in the future relative to the
      ledger timestamp for both `lock_funds` and `extend_lock`
      (`ContractError::UnlockTimeNotInFuture`).
- [ ] Lock duration is checked against `MaxLockDurationSecs` /
      `MinLockDurationSecs` where the change touches lock creation or
      extension.
- [ ] Locked amount can never exceed the user's available `Balance` at lock
      time (`InsufficientBalanceToLock`).
- [ ] Maturity boundary behaviour (`can_withdraw`, `withdraw_lock`) stays
      inclusive/exclusive exactly as documented — do not change the boundary
      without updating [Ledger Time and Lock Maturity Guide](ledger-time-locks.md)
      and its tests.
- [ ] Multiple independent locks per user remain independently addressable by
      `lock_id`; extending or withdrawing one lock must not affect another.
      See [Lock Extension Design](lock-extension-design.md) and
      [Multi-Lock Storage](multi-lock-storage.md).

## 3. Token Transfer Atomicity

- [ ] Every token movement goes through the configured SAC `token` client
      (`env.invoke_contract` / token client transfer) — no implicit balance
      changes without a corresponding on-chain transfer.
- [ ] Internal accounting (`Balance`, `LockEntry`) is only updated after the
      token transfer step it depends on succeeds; a failed transfer must not
      leave internal accounting ahead of actual custody, or vice versa. See
      [Token Transfer Atomicity and Rollback Verification](atomicity.md).
- [ ] `deposit`, `withdraw`, and `withdraw_lock` are covered by a rollback
      test asserting balances are unchanged when the token transfer fails
      (see `token_transfer_rollback.rs`, `test_deposit_fails_when_token_transfer_fails`).
- [ ] Custody assumptions in [Vault Custody Assumptions](vault-custody-assumptions.md)
      still hold after the change (e.g. no code path assumes native XLM
      custody or bypasses the configured token).

## 4. Authorisation & Privilege Boundaries

- [ ] Every state-changing function calls `require_auth()` on the correct
      address — the acting `user` for user-scoped functions, `admin` for
      admin-gated ones (`pause`, `unpause`, `set_min_deposit_amount`,
      `set_max_lock_duration`, `set_min_lock_duration`, `transfer_admin`).
- [ ] The stored admin address is compared against the caller before any
      privileged effect runs (`NotAuthorizedAdmin`), not only via
      `require_auth()` on an assumed-correct address.
- [ ] New or changed functions are added to the
      [Authorisation Rules & Security Matrix](authorisation-rules.md) and
      [Authorization Boundaries](authorization-boundaries.md) tables in the
      same PR.
- [ ] Admin-privilege changes (including `transfer_admin`) are reviewed
      against the [Admin Role](admin-role.md) document and the
      [Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md)
      for new misuse or compromised-admin scenarios.

## 5. Storage Layout & Migration

- [ ] Any new or changed storage key, value shape, or `DataKey` variant
      follows the [Storage Change Checklist](storage-change-checklist.md) —
      complete every item there, not just this summary.
- [ ] `STORAGE_VERSION` is bumped and a migration path is documented in
      [Storage Migration Guide](storage-migration.md) if the on-chain layout
      changes in a way existing deployments must account for.
- [ ] Persistent vs. instance storage placement matches
      [Storage Audit](storage-audit.md) (user balances/locks: persistent;
      admin/config/version: instance) unless the PR explicitly changes that
      design and updates the doc.
- [ ] Storage TTL implications are considered for any new persistent entry —
      see [Storage TTL Guide](storage-ttl.md).

## 6. Event Compatibility

- [ ] New events, or changes to existing event topics/payloads, are checked
      against the [Event Backward-Compatibility Policy](event-compatibility-policy.md)
      — additive changes only; no renaming or removing existing topics/fields
      without a documented breaking-change process.
- [ ] The [Event Schema](event-schema.md) document is updated in the same PR
      as any event change, and `event_compatibility.rs` /
      `event_schema.rs` / `event_ordering.rs` tests are updated to lock the
      new shape in.
- [ ] Event payloads avoid leaking more user data than necessary — recheck
      against [Event Privacy Review](event-privacy-review.md).

## 7. Error Codes & Failure Behaviour

- [ ] New failure conditions use a `ContractError` variant with
      `panic_with_error!`, not a bare `panic!`, and the variant is added to
      the correct numeric range in `lib.rs` (see the category-range table
      above `enum ContractError`) rather than reusing or renumbering an
      existing code.
- [ ] The new/changed error is documented in the
      [Contract Error Reference](error-codes.md) and, if it changes
      SDK-facing behaviour, in the
      [Error Code Standard](error-code-standard.md) and
      [SDK Error Mapping Guide](sdk-error-mapping-guide.md).
- [ ] The failure is added to the
      [Failure Mode Catalogue](failure-mode-catalogue.md) with its expected
      behaviour and affected function(s).
- [ ] Existing error codes are never renumbered or reused for a different
      condition — downstream SDKs depend on code stability.

## 8. Required Test Coverage

- [ ] The change includes tests for the happy path **and** for its failure
      and boundary conditions (unauthorized caller, zero/negative amount,
      insufficient balance, uninitialized contract, paused contract, as
      applicable).
- [ ] New tests follow the [Test Naming Conventions](testing.md) so they stay
      discoverable alongside existing ones under
      `contracts/savings_vault/src/test/`.
- [ ] [Test Coverage Summary](test-coverage.md) is updated so the new
      behaviour is mapped to its test(s), and any closed test gap is removed
      from that document's gap list.
- [ ] `cargo fmt --check`, `cargo clippy --tests -- -D warnings`, and
      `cargo test --workspace` all pass locally before opening the PR.

## 9. General Hygiene

- [ ] No secrets, private keys, seed phrases, or RPC credentials are logged,
      committed, or used in examples — use placeholders per
      [Documentation Style Guide](docs-style-guide.md).
- [ ] Documentation-only changes are kept separate from contract logic
      changes, per [CONTRIBUTING.md](../CONTRIBUTING.md#pull-request-expectations).
- [ ] If the change affects release readiness, the
      [Contract Release Checklist](contract-release-checklist.md) is also
      run before merge.

*Use this checklist as part of the PR description and ensure each applicable
item is addressed before merging.*
