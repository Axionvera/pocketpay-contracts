# PocketPay Contracts Contributor Evaluation Policy

This policy is the repository-level evaluation standard for PocketPay Contracts contributions, including issues tracked by the GrantFox OSS campaign. It brings the existing readiness, testing, traceability, review, and payment-period guidance into one contributor-facing policy.

Related guidance:

- [Contract Evaluation-Readiness Checklist](./evaluation-readiness-checklist.md)
- [Contribution Quality Gate](./contribution-quality-gate.md)
- [Acceptance Criteria Audit](./ACCEPTANCE_CRITERIA_AUDIT.md)
- [Contributor Self-Review Template](./self-review-template.md)
- [Payment-Period Conduct Guidance](./payment-period-conduct.md)
- [Payment-Period Communication Policy](./PAYMENT_POLICY.md)

## Merge does not guarantee payment approval

A merged pull request means the repository accepted the contribution into its codebase. It does **not** mean that GrantFox has approved a reward or payment.

GrantFox evaluation happens after merge and is separate from repository maintenance. From this repository's side, reward evaluation depends on the completed issue scope, contribution quality, applicable contract tests, required verification/CI, and acceptance-criteria completion. A merge, closed issue, approving GitHub review, or `Maybe Rewarded` label must not be treated as a payout confirmation.

GrantFox owns the campaign's evaluation, scoring, and payment decision. Repository maintainers own code review, merge decisions, and repository quality standards.

## Contributor self-review

Before requesting maintainer review, contributors should complete a final self-review of the actual diff:

1. Re-read the entire issue and every acceptance criterion.
2. Map each criterion to the implementation, documentation, and test evidence that satisfies it.
3. Confirm there are no placeholders, hidden follow-up requirements, unrelated changes, or stale documentation.
4. Complete the [Contributor Self-Review Template](./self-review-template.md).
5. Confirm the pull-request description identifies the issue with `Closes #N` and explains the completed scope.
6. Record any known limitation or genuinely pre-existing infrastructure failure explicitly rather than presenting partial work as complete.

Self-review is preparation for maintainer review; it is not GrantFox approval.

## Contract testing and verification expectations

Contract changes must follow the repository's existing verification standard.

- Run `make verify` for applicable code changes. It covers formatting, Clippy, workspace tests, and the release WASM build.
- Behaviour changes should include relevant success, failure, authorization, state/accounting, boundary, and regression coverage described by the repository's testing guidance.
- Pull-request checks should be green before approval. A failing check must be investigated; it is not waived by merge intent.
- Tests, lint rules, or checks must not be removed, skipped, or weakened merely to make a contribution appear green.
- Security-sensitive changes should follow the [Contributor Security Checklist](./security-checklist.md), including authorization, token transfer, storage, event, and invariant review.

Documentation-only contributions do not need invented contract tests when runtime behaviour is untouched. They still must satisfy their issue's complete documentation acceptance criteria and keep links, commands, terminology, and cross-references accurate.

## Acceptance-criteria completion

Issue acceptance criteria are part of the deliverable.

Every pull request should make it easy to trace each criterion to concrete evidence. Use the repository's [Traceability Table Guide](./traceability-table.md) and [Acceptance Criteria Audit](./ACCEPTANCE_CRITERIA_AUDIT.md) rather than leaving criteria implicit.

A criterion is not complete because a nearby file changed. If the requirement cannot be completed as written, raise the mismatch on the issue or pull request and get it resolved explicitly. Do not silently defer required scope while presenting the issue as finished.

## Maintainer review

Maintainers evaluate the real contribution, not merely the existence or size of a diff. Review should cover:

- correctness and completeness against the issue;
- authorization, accounting, storage, token-transfer, and other contract safety properties when applicable;
- required tests and negative paths;
- `make verify` / CI state;
- documentation impact;
- acceptance-criteria traceability; and
- the contributor's self-review and disclosed limitations.

Use the [PR Reviewer Evidence Checklist](./REVIEWER_EVIDENCE_CHECKLIST.md) and [Contribution Quality Gate](./contribution-quality-gate.md) as the detailed maintainer references.

Maintainer approval and merge are necessary repository actions, but neither is a GrantFox reward decision.

## GrantFox post-merge evaluation

For an issue tracked by the GrantFox OSS campaign, the expected sequence is:

1. The contributor completes the issue and repository review requirements.
2. The repository maintainer reviews and, when acceptable, merges the contribution.
3. GrantFox performs its post-merge campaign evaluation.
4. Reward/payment status comes from that GrantFox evaluation, not from the GitHub merge state.

Campaign payment windows may be processed after campaign closure or during designated review cycles as described in the [Payment-Period Communication Policy](./PAYMENT_POLICY.md). Contributors should keep their campaign claim and payout identity information accurate on the appropriate GrantFox surface.

## Payment-period conduct

During evaluation or payment periods:

- Do **not** spam community channels with repeated payment-status questions.
- Do not repeat the same complaint or ping across issues, pull requests, chat channels, or unrelated maintainer threads.
- Check your own acceptance criteria, self-review, verification, and CI state before asking about payment.
- When follow-up is appropriate, make one professional, specific message on the canonical PR, issue, or GrantFox support surface with the PR number and the information needed to resolve the question.
- Follow up again only when there is new information or a maintainer/evaluator asks for it.

Technical scope or review disagreements belong on the relevant issue/PR. GrantFox-specific payment amount, timing, scoring, or eligibility questions belong with GrantFox's campaign/support process.

## Ready for post-merge evaluation

Before treating a merged contribution as evaluation-ready, confirm:

- [ ] Every acceptance criterion is satisfied or explicitly resolved with the maintainer.
- [ ] Contributor self-review is complete.
- [ ] Applicable contract tests and `make verify` expectations are satisfied.
- [ ] Pull-request checks are green, or an accepted pre-existing/infrastructure exception is documented.
- [ ] Security and documentation impacts are addressed.
- [ ] The PR clearly links the issue and provides acceptance-criteria traceability.
- [ ] GrantFox claim/payout identity information is present on the appropriate campaign surface.
- [ ] Payment-period communication follows the repository's no-spam guidance.

Passing this checklist means the contribution is ready to be evaluated. It does not itself approve a reward.
