# Contributor Self-Review Template

This is a copy-paste template for contributors to fill in and run through
**before** opening a pull request or requesting review. It exists so that a
contributor checks their own work against the issue's acceptance criteria —
behaviour, tests, CI, security, edge cases, and docs — the same way every
time, instead of relying on a reviewer to catch missing requirements first.

This is not a new review process. It packages checks that already exist
elsewhere in this repo (the [Contribution Quality Gate](contribution-quality-gate.md),
the [Traceability Table Guide](traceability-table.md), the
[Contract Contributor Security Checklist](security-checklist.md), and the
[Invariant Test Checklist](invariant-test-checklist.md)) into one worksheet you
can fill in locally, then paste into your PR description.

## How to use this template

1. Copy the [Self-review checklist](#self-review-checklist) section below into
   a scratch file, or directly into your PR description.
2. Fill in every section against the issue you're resolving. Use `—` or "not
   applicable" for sections that genuinely don't apply (for example, a
   documentation-only PR has no contract functions to list) — don't delete a
   section just because it's inconvenient.
3. Run `make verify` and paste the result in the CI section.
4. Only open the PR, or move it out of draft, once every checkbox is either
   checked or explicitly marked not applicable with a reason.
5. Leave the filled-in checklist in the PR description alongside the
   [traceability table](traceability-table.md) — reviewers use both together.

If an item doesn't pass, fix it before requesting review rather than noting it
as a known gap. See [Examples of Incomplete Work](contribution-quality-gate.md#3-examples-of-incomplete-work)
for the kind of PR this template is meant to catch before it reaches a
reviewer.

---

## Self-review checklist

### 1. Behaviour

- [ ] I can restate, in my own words, what the linked issue asks for.
- [ ] Every acceptance criterion in the issue is addressed by this PR (or I've
      explained in the PR description which ones are deferred and why).
- [ ] I've filled in the [traceability table](traceability-table.md) mapping
      each acceptance criterion to the function(s), test(s), and edge cases
      that satisfy it.
- [ ] No `TODO`, `FIXME`, or `HACK` markers remain in the code this PR touches.
- [ ] The change is scoped to one issue — unrelated refactors or fixes are
      split into a separate PR.

### 2. Tests

- [ ] Every new or changed function has tests for both the happy path and
      failure paths (unauthorized caller, invalid input, insufficient
      balance, etc.), per the [test naming convention](testing.md).
- [ ] Changes to accounting logic are covered by the relevant
      [property tests](../contracts/savings_vault/src/test/property_vault_accounting.rs)
      and checked against the [Invariant Test Checklist](invariant-test-checklist.md).
- [ ] Event schema changes have updated
      [snapshots](../contracts/savings_vault/test_snapshots/).
- [ ] `cargo test --workspace` passes locally — not just the tests I added.

### 3. CI

- [ ] `make verify` passes locally (format, Clippy, workspace tests, release
      WASM build). If you ran the steps individually instead, all of
      `cargo fmt --check`, `cargo clippy --tests -- -D warnings`, and
      `cargo test --workspace` pass.
- [ ] `make build-release` succeeds and the WASM size is reported; I checked
      for unexpected artifact size growth.
- [ ] I pushed the branch and confirmed CI is green on GitHub before
      requesting review, not just locally.

### 4. Security

- [ ] I've gone through the applicable sections of the
      [Contract Contributor Security Checklist](security-checklist.md) for
      this change (balances, lock state, token transfer atomicity,
      authorisation, storage, events, error codes) and noted "not applicable"
      for sections this PR doesn't touch, with a reason.
- [ ] Every new or changed state-changing function calls `require_auth()` on
      the correct address.
- [ ] New error paths use `ContractError` variants with `panic_with_error!`,
      not bare `panic!`.
- [ ] Storage layout changes follow the
      [storage change checklist](storage-change-checklist.md); event changes
      follow the [event compatibility policy](event-compatibility-policy.md).
- [ ] No secrets, private keys, seed phrases, or populated credential files
      are committed, logged, or included in test fixtures.

### 5. Edge cases

- [ ] Boundary amounts are tested: `0`, `1`, the maximum representable
      amount, and values just above/below any configured limit.
- [ ] For time-locked behaviour, tests cover exactly at, just before, and just
      after the relevant maturity or expiry timestamp.
- [ ] Failure paths leave storage byte-for-byte unchanged — no partial
      mutation before the failure point (see
      [Balance Reconciliation Design Note](balance-reconciliation.md)).
- [ ] Multi-entry state (multiple locks, multiple users) is tested for
      independence — acting on one entry doesn't affect another.

### 6. Docs

- [ ] Behaviour or architectural changes are reflected in the relevant
      `docs/` file(s), not just in code comments.
- [ ] If this PR changes a function signature or add/removes a function, the
      [API reference](api-reference.md) and README
      [Features](../README.md#features) table are updated to match.
- [ ] New docs are linked from the README
      [Documentation](../README.md#documentation) list, per the
      [Documentation Style Guide](docs-style-guide.md#linking-related-docs).
- [ ] Wording follows the [Documentation Style Guide](docs-style-guide.md)
      (testnet-only framing, no production claims, placeholder values only).

---

## Relationship to other docs

This template doesn't replace any existing PR requirement — it's the
checklist you personally run through before the PR template and traceability
table get filled in for reviewers.

- [PR Template](../.github/PULL_REQUEST_TEMPLATE.md) — the contributor fills
  this in for the reviewer; this self-review template is what you complete
  first, on your own, before that.
- [Contribution Quality Gate](contribution-quality-gate.md) — the fuller
  narrative explanation of "payment-ready" work, with examples of what
  incomplete work looks like. This template is the condensed, fill-in-the-blank
  version of that gate.
- [Traceability Table Guide](traceability-table.md) — the standard format for
  mapping acceptance criteria to functions and tests; fill this in as part of
  the Behaviour section above.
- [Contract Contributor Security Checklist](security-checklist.md) and
  [Invariant Test Checklist](invariant-test-checklist.md) — the detailed
  checklists the Security and Edge Cases sections above summarize.
