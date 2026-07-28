# Traceability Table Guide

This document defines a standard traceability table format for PocketPay
contract pull requests. The table maps every acceptance criterion from the
linked issue to the contract functions, tests, and edge cases that satisfy it.

## Why traceability matters

Smart contract changes are high-stakes: once deployed, logic is effectively
immutable. A traceability table gives reviewers and auditors a single place to
verify that:

- Every acceptance criterion has a concrete implementation.
- Every implementation change has test coverage.
- Boundary and failure scenarios are explicitly called out, not assumed.
- Nothing from the issue was silently dropped or left for a future PR.

This is especially important for evaluation readiness, where assessors need to
confirm that each deliverable maps cleanly to the original requirements.

---

## Table format

Every contract PR that references an issue with acceptance criteria must include
a **Traceability Table** in the PR description. The table uses six columns:

| Criterion ID | Criterion Text | Changed Function(s) | Test(s) | Edge Cases Covered | Status |
|---|---|---|---|---|---|

### Column definitions

| Column | What to write |
|---|---|
| **Criterion ID** | A short label matching the issue's acceptance criteria, e.g. `AC-1`, `AC-2`. Number them in the order they appear in the issue. |
| **Criterion Text** | The exact wording of the acceptance criterion copied from the issue. Keep it verbatim so reviewers can cross-reference without switching tabs. |
| **Changed Function(s)** | The contract function(s) added, modified, or removed to satisfy this criterion. Use the Rust function name (e.g. `extend_lock`, `lock_funds`). Write `—` for documentation-only criteria. |
| **Test(s)** | The test function name(s) and file path(s) that prove this criterion is met. Use the format `test_name` (`path/to/file.rs`). List multiple tests on separate lines within the cell. |
| **Edge Cases Covered** | Boundary conditions, failure paths, and adversarial scenarios explicitly tested for this criterion. Examples: "zero amount rejected", "expired pause auto-clears", "unauthorized caller blocked". Write `—` if no edge cases apply (e.g. pure documentation criteria). |
| **Status** | One of: ✅ Met, ⚠️ Partial (with explanation), ❌ Not Met (with justification for deferral). Every row should be ✅ before the PR is marked ready for review. |

---

## Worked example

The following example shows how a contributor would fill in the traceability
table for a hypothetical issue: *"Enforce admin-configured lock duration limits
during `extend_lock`"*.

### Issue acceptance criteria (hypothetical)

> 1. `extend_lock` rejects extensions that would exceed `MaxLockDurationSecs`.
> 2. `extend_lock` rejects extensions that would result in a duration shorter than `MinLockDurationSecs`.
> 3. Existing tests for `extend_lock` continue to pass.
> 4. Error codes use `ContractError` variants, not bare panics.
> 5. `docs/lock-extension-design.md` is updated to reflect the new enforcement rules.

### Traceability table

| Criterion ID | Criterion Text | Changed Function(s) | Test(s) | Edge Cases Covered | Status |
|---|---|---|---|---|---|
| AC-1 | `extend_lock` rejects extensions exceeding `MaxLockDurationSecs` | `extend_lock` | `test_extend_lock_exceeds_max_duration` (`src/test/lock_extension.rs`) | Extension exactly at max (accepted); extension 1 second over max (rejected); extension when no max is configured (accepted) | ✅ Met |
| AC-2 | `extend_lock` rejects durations shorter than `MinLockDurationSecs` | `extend_lock` | `test_extend_lock_below_min_duration` (`src/test/lock_extension.rs`) | Extension exactly at min (accepted); extension 1 second below min (rejected) | ✅ Met |
| AC-3 | Existing `extend_lock` tests continue to pass | — | All existing tests in `src/test/lock_extension.rs` | No regressions in happy-path or unauthorized-caller tests | ✅ Met |
| AC-4 | Error codes use `ContractError`, not bare panics | `extend_lock` | `test_extend_lock_exceeds_max_duration`, `test_extend_lock_below_min_duration` | Verify error variant matches `ContractError::LockDurationOutOfRange` | ✅ Met |
| AC-5 | `docs/lock-extension-design.md` updated | — | — | — | ✅ Met |

---

## Rules and tips

### When to include the table

- **Required** for any PR that references an issue with explicit acceptance
  criteria.
- **Optional but encouraged** for bug-fix PRs where the issue describes
  expected behavior. Treat each "expected behavior" statement as a criterion.
- **Skip** for trivial PRs (typo fixes, comment updates) where the issue has
  no acceptance criteria. Write "No acceptance criteria — documentation-only
  change" in the Traceability Table section of the PR template.

### Documentation-only criteria

Use `—` in the Changed Function(s), Test(s), and Edge Cases columns. Set
Status to ✅ and note the doc file path in the Criterion Text or a brief note.

### Multiple tests per criterion

List each test on a separate line within the same table cell using `<br>` or a
line break inside the markdown cell:

```markdown
| AC-1 | ... | `deposit` | `test_deposit_happy_path` (`src/test/deposit.rs`)<br>`test_deposit_minimum_amount` (`src/test/deposit.rs`) | ... | ✅ Met |
```

### Multiple functions per criterion

Similarly, list each function name separated by commas or line breaks:

```markdown
| AC-2 | ... | `lock_funds`, `extend_lock` | ... | ... | ✅ Met |
```

### Partial or deferred criteria

If a criterion cannot be fully met in the current PR, set Status to ⚠️ Partial
and explain what remains. Link to a follow-up issue if one has been created:

```markdown
| AC-3 | ... | ... | ... | ... | ⚠️ Partial — max-lock enforcement deferred to #42 |
```

### Relationship to other PR template sections

The traceability table does **not** replace the existing PR template sections:

- **Contract Functions Changed** — still lists every function touched,
  including those unrelated to acceptance criteria (e.g. refactors).
- **Tests Added or Updated** — still confirms happy-path, failure, and naming
  conventions at a high level.
- **Security Considerations** — still covers security-specific analysis.

The traceability table is the **cross-cutting map** that ties functions and
tests back to the *why* — the issue's acceptance criteria.

### Numbering criteria

If the issue uses checkboxes (`- [ ]`), number them top-to-bottom as `AC-1`,
`AC-2`, etc. If the issue uses numbered lists, match those numbers.

---

## Cross-references

- [PR Template](../.github/PULL_REQUEST_TEMPLATE.md) — the traceability table
  section in the PR template.
- [Contribution Quality Gate](contribution-quality-gate.md) — the broader
  quality checklist that the traceability table supports.
- [Contract Release Checklist](contract-release-checklist.md) — release-level
  verification that complements per-PR traceability.
