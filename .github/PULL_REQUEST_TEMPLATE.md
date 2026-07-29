# Pull Request

<!-- Before filling this in, complete the Contributor Self-Review Template
     (docs/self-review-template.md) — behaviour, tests, CI, security, edge
     cases, and docs. Fixing gaps it finds is faster than a reviewer finding
     them. -->

- [ ] I completed the [Contributor Self-Review Template](../docs/self-review-template.md) before opening this PR.

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

## Test Evidence

<!-- Provide clear evidence of testing for this change. Every logic change requires tests
     for the happy path and for failure/boundary conditions. -->

### Tests added or updated

- [ ] Happy-path tests added/updated
- [ ] Failure and boundary-condition tests added/updated
- [ ] Test naming follows the convention in [`docs/testing.md`](docs/testing.md)
- [ ] [`docs/test-coverage.md`](docs/test-coverage.md) updated to reflect new or changed tests

### No-test justification

<!-- If no tests were added, explain why. Examples: documentation-only change,
     refactor with no behaviour change, CI-only change. -->

- [ ] N/A — tests added (or)
- [ ] Justification provided below:

**Justification for no tests:**

### Contract-specific test examples

<!-- Review and reference existing contract test patterns relevant to this change.
     Common test categories in this repo:

     | Category | Example files |
     |---|---|
     | Balance & accounting invariants | `test/balance_conservation.rs`, `test/total_vault_balance.rs`, `test/property_vault_accounting.rs` |
     | Lock operations | `test/lock_amount_validation.rs`, `test/lock_extension.rs`, `test/lock_maturity_boundary.rs`, `test/multi_lock_invariants.rs` |
     | Authorization & isolation | `test/unauthorized_access.rs`, `test/cross_user_isolation.rs` |
     | Edge cases & boundaries | `test/zero_duration_lock.rs`, `test/maximum_amount_boundary.rs`, `test/independent_lock_creation.rs` |
     | Negative paths | `test/negative_paths.rs` |
     | Event correctness | `test/event_schema.rs`, `test/event_compatibility.rs`, `test/event_ordering.rs` |
     | Pause & admin | `test/pause.rs`, `test/pause_state_read.rs`, `test/pause_transition.rs`, `test/admin_rotation.rs` |
     | Replay protection | `test/replay_protection.rs`, `test/lock_maturity_replay.rs` |
     | Invariant checklist | `test/invariant_checklist_examples.rs` |

     See [`docs/testing.md`](docs/testing.md) for the full testing guide. -->

- [ ] Existing contract test examples reviewed and relevant patterns referenced

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

<!-- Complete this canonical acceptance criteria audit template by mapping every
     criterion from the linked issue to implementation, tests, documentation
     impact, and edge cases. See docs/traceability-table.md for the full guide.

     For documentation-only PRs with no acceptance criteria, replace the table below with:
     "No acceptance criteria — documentation-only change."

     Example row (delete this comment block before submitting):

     | AC-1 | `extend_lock` rejects extensions exceeding `MaxLockDurationSecs` | `extend_lock` in `contracts/savings_vault/src/lib.rs` | `test_extend_lock_exceeds_max_duration` (`src/test/lock_extension.rs`) | N/A — behavior already documented | Extension exactly at max (accepted); 1s over max (rejected) | ✅ Met |
-->

| Criterion ID | Criterion Text | Implementation Evidence | Test Evidence | Documentation Impact | Edge Cases Covered | Status |
|---|---|---|---|---|---|---|
| | | | | | | |
