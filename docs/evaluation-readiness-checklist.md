# Contract Evaluation-Readiness Checklist

Use this checklist before submitting contract work for GrantFox evaluation and
before the applicable payment period. It is a contributor self-review aid, not
an approval or payment guarantee.

> **A merged pull request is not automatically approved for payment.** GrantFox
> evaluates the completed work separately. Approval depends on satisfying the
> issue requirements, providing adequate tests, passing the required checks,
> and demonstrating that the change is safe and complete.

Copy this checklist into the pull request description, mark each applicable
item, and include evidence such as test names, command output, and file links.
If an item does not apply, mark it `N/A` and briefly explain why.

## Issue requirements and acceptance criteria

- [ ] I re-read the issue description and its acceptance criteria after
      completing the implementation.
- [ ] Every acceptance criterion is mapped to the relevant implementation,
      documentation, and test evidence in the pull request description.
- [ ] The change is complete and limited to the issue's scope; it does not rely
      on unstated follow-up work.
- [ ] Any deferred requirement or known limitation is clearly identified,
      justified, and linked to a follow-up issue.
- [ ] No issue-related `TODO`, `FIXME`, placeholder, or commented-out
      implementation remains.

Use the [Acceptance Criteria Audit](ACCEPTANCE_CRITERIA_AUDIT.md) when recording
the evidence for each criterion.

## Contract tests

- [ ] New or changed behaviour has focused tests for each acceptance criterion.
- [ ] Tests cover successful calls as well as expected failures and boundary
      conditions; happy-path tests alone are not adequate.
- [ ] Authorization tests cover the intended signer and unauthorized callers.
- [ ] State and accounting changes preserve the relevant invariants, including
      rollback behaviour when an operation fails.
- [ ] Time-dependent behaviour is tested before, at, and after important ledger
      timestamps where applicable.
- [ ] Event changes include assertions or updated snapshots where applicable.
- [ ] The full workspace test suite passes locally with
      `cargo test --workspace`.

See [Testing](testing.md) and the
[Invariant Test Checklist](invariant-test-checklist.md) for the repository's
detailed contract-testing expectations.

## Required checks and CI

- [ ] `make verify` passes locally. This runs formatting, Clippy, workspace
      tests, and the release WASM build.
- [ ] The pull request includes the verification result and enough output for a
      reviewer to identify what was run.
- [ ] All checks reported on the pull request pass; any failure is investigated
      rather than assumed to be unrelated.
- [ ] The release WASM size is reported, and unexpected growth is explained.

The repository may rely on contributor-run checks where an equivalent hosted
CI check is unavailable. A merge does not waive a failed or missing check. See
[Local Development](local-development.md) for individual commands.

## Security review

- [ ] State-changing entry points enforce the correct authorization boundary.
- [ ] Inputs, arithmetic, storage lifetimes, and error paths are validated.
- [ ] Failed calls cannot leave partial state changes or inconsistent balances.
- [ ] Token transfers, external calls, and re-entrancy or callback assumptions
      are reviewed where applicable.
- [ ] Sensitive information is not exposed through events, errors, logs, or
      committed fixtures.
- [ ] New trust assumptions, privileges, or security limitations are documented.

Use the [Contributor Security Checklist](security-checklist.md) for the full
contract-specific review.

## Edge cases

- [ ] Amount handling covers zero, one, maximum values, and values immediately
      below or above relevant limits where applicable.
- [ ] Empty, missing, repeated, and already-processed states are considered.
- [ ] Timestamp and ledger-boundary behaviour is covered where applicable.
- [ ] Repeated calls, initialization, pause/freeze state, and storage expiry are
      considered when the change touches those behaviours.
- [ ] Failure cases return the expected contract error and do not corrupt state.

## Documentation and self-review

- [ ] Public API, event, architecture, or security documentation is updated for
      changed behaviour.
- [ ] The README and documentation links remain accurate.
- [ ] I reviewed the final diff for accidental changes, debug code, secrets,
      stale comments, and unrelated formatting.
- [ ] I completed the [Contributor Self-Review Template](self-review-template.md)
      and recorded known limitations honestly.
- [ ] I confirmed the pull request description explains what changed, why it is
      correct, and how reviewers can verify it.

## Evaluation acknowledgement

- [ ] I understand that review and merge are repository-maintenance actions;
      neither guarantees GrantFox evaluation approval or payment.
- [ ] I understand that GrantFox may determine after merge that tests are
      inadequate, checks are missing, or issue requirements are not fully met.
- [ ] I understand that payment approval depends on the evaluation outcome and
      compliance with the applicable campaign and payment-period rules.
- [ ] Before asking about payment, I will follow the
      [Payment-Period Communication Policy](PAYMENT_POLICY.md) and
      [Payment-Period Conduct Guidance](payment-period-conduct.md).

For a final abbreviated pass immediately before requesting evaluation, also
see the [Approval Readiness Checklist](approval-readiness-checklist.md).
