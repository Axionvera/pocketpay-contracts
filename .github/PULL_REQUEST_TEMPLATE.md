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

<!-- Paste the commands you ran locally and confirm each passed. -->

```
cargo fmt --check
cargo clippy --tests -- -D warnings
cargo test --workspace
```

- [ ] `cargo fmt --check` — passed
- [ ] `cargo clippy --tests -- -D warnings` — passed
- [ ] `cargo test --workspace` — passed

---

## CI Status

<!-- Confirm CI is green before requesting review. -->

- [ ] All CI checks pass on this branch

---

## Acceptance Criteria Coverage

<!-- Reference the acceptance criteria from the linked issue and confirm each one is met.
     Delete this section for straightforward bug fixes with no separate acceptance criteria. -->

| Criterion | Met? | Notes |
|---|---|---|
| | | |
