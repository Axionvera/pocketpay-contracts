# Approval Readiness Checklist

Use this checklist right before you consider an issue ready for maintainer
evaluation — after your own [Self-Review Template](self-review-template.md) is
filled in, and before you request review or raise a payment-period question.

Merging a PR does **not** mean the work is approved. This checklist exists so
you catch gaps yourself, but the final call on whether an issue's acceptance
criteria are actually met is a maintainer/GrantFox evaluation step, not the
merge event.

This is not a new process — it is a short, final pass over checks that already
exist elsewhere in this repo. If an item below fails, follow the linked doc
for how to fix it; don't just note it as a known gap.

## 1. Implementation completeness

- [ ] Every acceptance criterion in the linked issue is fully implemented, not
      partially — see the [Contribution Quality Gate](contribution-quality-gate.md#1-quality-gate-checklist).
- [ ] No `TODO`, `FIXME`, or `HACK` markers remain in the code this PR touches.
- [ ] The PR is scoped to this issue only — unrelated refactors or fixes are
      split into a separate PR.

## 2. Tests

- [ ] `cargo test --workspace` passes locally, and the summary (test count) is
      pasted into the PR.
- [ ] Every new or changed function has tests for the happy path and for
      failure/boundary conditions, following the
      [test naming convention](testing.md).
- [ ] Accounting or invariant changes are checked against the
      [Invariant Test Checklist](invariant-test-checklist.md) and the property
      tests in `contracts/savings_vault/src/test/property_vault_accounting.rs`.
- [ ] Event schema changes have updated snapshots under
      `contracts/savings_vault/test_snapshots/`.

## 3. CI status

> This repository has no GitHub Actions workflow that builds, lints, or tests
> the contract. The only workflow, `.github/workflows/trigger-auto-merge.yml`,
> dispatches to an external automation repo and does not run `cargo test`,
> Clippy, `fmt`, or a build. "CI status" below means the local commands run
> and reported by you — not a GitHub check.

- [ ] `make verify` passes locally (`cargo fmt --check`,
      `cargo clippy --tests -- -D warnings`, `cargo test --workspace`,
      `cargo build --release --target wasm32-unknown-unknown`), and the output
      is pasted into the PR — not just asserted.
- [ ] `make build-release` (or `make wasm-size`) output is included, and any
      WASM size growth versus the previous release is explained.

## 4. Acceptance criteria review

- [ ] The linked issue's acceptance criteria are re-read one by one, and each
      is confirmed `Met` or `Deferred` — not assumed from memory of the
      original implementation work.
- [ ] The [Acceptance Criteria Audit Table](ACCEPTANCE_CRITERIA_AUDIT.md) is
      filled in the PR description, with a file/line reference and a test name
      for every row marked `Pass`.
- [ ] Every criterion marked `Deferred` has a written justification and a
      linked tracking issue in the PR description.

## 5. Documentation updated

- [ ] Behaviour or architectural changes are reflected in the relevant
      `docs/` file(s), not only in code comments.
- [ ] If a function was added, removed, or changed, `docs/api-reference.md`
      and the README [Features](../README.md#features) table match the new
      signature.
- [ ] Event or security-relevant docs are updated when applicable
      (`docs/events.md`, `docs/event-schema.md`, `docs/security-checklist.md`).
- [ ] New docs are linked from the README
      [Documentation](../README.md#documentation) list, per the
      [Documentation Style Guide](docs-style-guide.md#linking-related-docs).

## 6. Known limitations

- [ ] Anything left out of scope, deferred, or not fully solved by this PR is
      written down explicitly in the PR description — not left implicit or
      discovered later by a reviewer.
- [ ] Each known limitation either links a tracking issue or states plainly
      that none is planned yet.

## 7. Merge is not approval

- [ ] I understand that merging this PR into `main` is not, by itself, an
      approval of the work or a confirmation that payment/evaluation criteria
      are met.
- [ ] I understand evaluation and payment happen in a separate, post-merge
      step run by GrantFox, per the
      [Payment-Period Communication Policy](PAYMENT_POLICY.md) and the
      [Payment-Period Conduct Guidance](payment-period-conduct.md), and that a
      merged PR can still be found incomplete during that review.
- [ ] I have not treated a merge, or the absence of a payment update, as
      grounds to escalate or repeat a payment-status question — see
      [Payment-Period Conduct Guidance](payment-period-conduct.md#what-to-avoid).

---

## Relationship to other docs

This checklist doesn't replace anything — it's the last, short pass before you
ask for evaluation, after the longer worksheets below are already done:

- [Self-Review Template](self-review-template.md) — the detailed, fill-in
  worksheet for behaviour, tests, CI, security, and edge cases; do this first.
- [Contribution Quality Gate](contribution-quality-gate.md) — the fuller
  narrative definition of "payment-ready" work, with examples of incomplete
  PRs.
- [Acceptance Criteria Audit Template](ACCEPTANCE_CRITERIA_AUDIT.md) — the
  table format referenced in section 4.
- [Payment-Period Conduct Guidance](payment-period-conduct.md) — what to check
  before raising a payment-status question, referenced in section 7.
