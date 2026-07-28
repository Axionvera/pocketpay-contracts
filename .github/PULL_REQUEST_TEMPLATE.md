# Pull Request

## Issue Reference

Closes #<!-- issue number -->

<!-- Every PR must reference an open issue. If one does not exist, create it before opening this PR. -->

---

## Summary

<!-- What changed and why? Keep this concise — one or two sentences. -->

## Contract Functions Changed

<!-- List every contract function added, modified, or removed. Write "none" for documentation-only PRs. -->

| Function | Change type (added / modified / removed) | Notes |
|---|---|---|
| | | |

---

## Tests Added or Updated

<!-- Describe the tests that cover this change. Include file paths and test names where relevant.
     Every logic change requires tests for the happy path and for failure/boundary conditions. -->

- [ ] Happy-path tests added/updated
- [ ] Failure and boundary-condition tests added/updated
- [ ] Test naming follows the convention in [`docs/testing.md`](docs/testing.md)
- [ ] [`docs/test-coverage.md`](docs/test-coverage.md) updated to reflect new or changed tests

---

## Security Considerations

<!-- Describe any security impact this change has. Write "no security impact" only if the change
     is truly non-functional (e.g. comment or doc fix). For anything that touches balances,
     access control, storage, events, or external calls, work through the relevant sections of
     docs/security-checklist.md and paste the results below. -->

### Security checklist (contract changes only)

- [ ] Balance & accounting invariants preserved — see [`docs/accounting-invariants.md`](docs/accounting-invariants.md)
- [ ] Lock state and timed-operation rules unchanged (or documented if changed)
- [ ] Token transfer atomicity maintained — see [`docs/atomicity.md`](docs/atomicity.md)
- [ ] `require_auth()` called on the correct address in every state-changing function
- [ ] Storage layout change checklist followed (if applicable) — see [`docs/storage-change-checklist.md`](docs/storage-change-checklist.md)
- [ ] Event backward-compatibility policy followed (if applicable) — see [`docs/event-compatibility-policy.md`](docs/event-compatibility-policy.md)
- [ ] New error codes use `ContractError` variants with `panic_with_error!`, not bare `panic!`
- [ ] No secrets, private keys, or credentials committed or logged

---

## Commands Run

<!-- Prefer `make verify` (format, Clippy, workspace tests, release WASM build).
     Paste the command output summary, or list the individual checks below. -->

```
make verify
```

- [ ] `make verify` — passed
- [ ] (or individually) `cargo fmt --check` — passed
- [ ] (or individually) `cargo clippy --tests -- -D warnings` — passed
- [ ] (or individually) `cargo test --workspace` — passed

---

## CI Status

<!-- Confirm CI is green before requesting review. -->

- [ ] All CI checks pass on this branch

---

## Traceability Table

<!-- Map every acceptance criterion from the linked issue to the contract functions, tests,
     and edge cases that satisfy it. See docs/traceability-table.md for the full guide.

     For documentation-only PRs with no acceptance criteria, replace the table below with:
     "No acceptance criteria — documentation-only change."

     Example row (delete this comment block before submitting):

     | AC-1 | `extend_lock` rejects extensions exceeding `MaxLockDurationSecs` | `extend_lock` | `test_extend_lock_exceeds_max_duration` (`src/test/lock_extension.rs`) | Extension exactly at max (accepted); 1s over max (rejected) | ✅ Met |
-->

| Criterion ID | Criterion Text | Changed Function(s) | Test(s) | Edge Cases Covered | Status |
|---|---|---|---|---|---|
| | | | | | |
