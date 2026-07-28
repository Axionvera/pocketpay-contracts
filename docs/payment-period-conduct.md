# Payment-Period Conduct Guidance

This document sets expectations for how contributors communicate about payment status on GrantFox-sourced issues in this repository. It exists because community channels become harder to use for everyone when the same payment question is repeated across multiple threads instead of being resolved through the normal review process.

## Before asking about payment status

Check your own submission first:

1. Re-read the issue's **Acceptance Criteria** and confirm each box is actually satisfied by your PR, not just attempted.
2. Run through the **[Contribution Quality Gate](contribution-quality-gate.md)** — implementation, testing, documentation, and CI sections all included. A PR that fails this gate is not yet ready to be evaluated, and evaluation delays traceable to an incomplete PR are not a maintainer or GrantFox responsiveness issue.
3. Confirm your **[PR template](../.github/PULL_REQUEST_TEMPLATE.md)** is filled in completely, including the `Closes #N` reference, the traceability table, and confirmation that `make verify` passes.
4. Check whether CI is green. A red CI run on your PR will hold up review regardless of how the underlying work looks.

If all of the above are true and the PR has been open for a while with no maintainer response, a single, specific comment on the PR or issue — referencing the PR number and what's outstanding — is the right way to follow up.

## What to avoid

- **Repeating the same payment question across multiple issues, PRs, or channels.** One clear follow-up is enough; the review and payment queue is processed on its own schedule, not accelerated by repetition.
- **Escalating publicly before checking your own work.** If the quality gate above hasn't been fully worked through, raise questions about the review itself, not about payment timing.
- **Treating a pending or "Maybe Rewarded" label as an entitlement dispute.** These labels reflect GrantFox's own evaluation process (see below), not a commitment made by this repository's maintainers.
- **Off-topic complaints about unrelated contributors' payment status.** Keep discussion scoped to your own submission.

## How GrantFox evaluation works, from this repo's side

This repository's maintainers review pull requests for correctness, test coverage, and adherence to the standards in [CONTRIBUTING.md](../CONTRIBUTING.md) and the [Contribution Quality Gate](contribution-quality-gate.md). Maintainer approval and merge are necessary steps, but GrantFox — not this repository — owns the campaign's evaluation, scoring, and payment process for each issue. Questions specifically about payment amount, timing, or eligibility belong with GrantFox's own support channels, not this repository's issue tracker.

## Tone

Disagreements about scope or review feedback are a normal part of open source. Keep them professional: describe what you did, point to the specific acceptance criterion or checklist item in question, and avoid repeated or escalating messages on the same topic. Maintainers and other contributors are more likely to respond quickly to a single, well-scoped comment than to a thread of follow-ups.
