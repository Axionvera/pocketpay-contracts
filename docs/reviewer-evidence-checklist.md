# Reviewer Evidence Checklist

Use this checklist when reviewing a PocketPay Contracts pull request. It helps
maintainers decide whether a contribution is complete and evaluation-ready by
checking the evidence attached to the exact proposed change.

This checklist complements the [Contribution Quality
Gate](contribution-quality-gate.md), the [Traceability Table
Guide](traceability-table.md), and the [Contributor Self-Review
Template](self-review-template.md). Those documents remain the detailed sources
of truth; this page organizes the maintainer's review.

## 1. Issue and implementation scope

- [ ] The PR links one open issue and its summary matches that issue's
  requested behavior.
- [ ] The diff is focused on the agreed scope and does not mix unrelated
  refactors, formatting, or dependency changes.
- [ ] Every changed file has a clear role in the implementation or its
  evidence.
- [ ] The implementation is wired into the actual contract or contributor
  workflow rather than left as an unused helper, placeholder, `TODO`, or
  parallel source of truth.
- [ ] The PR identifies every contract function added, modified, or removed;
  documentation-only PRs explicitly say `none`.

When the issue is ambiguous or the diff is broader than the agreed scope, ask
for clarification before evaluating completeness.

## 2. Implementation quality and risk

- [ ] The implementation evidence points to the exact functions, logic,
  configuration, or documentation sections that satisfy the issue.
- [ ] Authorization, accounting, storage, event, and error-code impacts are
  described when applicable.
- [ ] Security-sensitive changes are checked against the [Contract Contributor
  Security Checklist](security-checklist.md).
- [ ] Dependency, storage-layout, and event-compatibility changes use their
  repository checklists and do not introduce unexplained risk.
- [ ] No secrets, private keys, credentials, real wallet material, or
  production claims appear in the diff or evidence.

For documentation, apply the [Documentation Style Guide](docs-style-guide.md)
and preserve the repository's educational/testnet boundaries.

## 3. Test evidence

- [ ] Each behavior change has named tests with file paths.
- [ ] Tests cover the happy path plus relevant failure, authorization,
  boundary, and regression cases.
- [ ] Assertions prove observable behavior rather than only mirroring internal
  implementation details.
- [ ] Documentation-only or non-executable changes include a specific no-test
  justification instead of leaving the test section blank.
- [ ] Any skipped, flaky, or failing test is explained and is not described as
  passing.

Use the [Testing Guide](testing.md) for naming and coverage expectations and
the [Test Reproduction Guide](reproducing-test-failures.md) when a failure must
be reproduced locally.

## 4. CI and command evidence

- [ ] The PR lists the commands run and their results, preferably `make
  verify` for executable changes.
- [ ] Relevant hosted jobs belong to the current PR head commit, are complete,
  and are green.
- [ ] A queued, skipped, cancelled, `action_required`, or zero-job workflow is
  not counted as passing CI.
- [ ] Infrastructure or permission failures are distinguished from
  patch-related failures and have a clear owner/next action.
- [ ] The PR remains a draft or review is deferred while relevant checks are
  missing or unresolved.

Do not infer patch readiness from an automation-dispatch check alone. Inspect
the job purpose and confirm that the checks which validate the affected files
actually ran.

## 5. Acceptance criteria and documentation impact

- [ ] Every issue acceptance criterion appears in the PR's [Traceability
  Table](traceability-table.md).
- [ ] Each row identifies implementation evidence, test evidence, edge cases,
  and status.
- [ ] Documentation impact is recorded for every criterion, including an
  explicit reason when no documentation change is needed.
- [ ] Partial or deferred criteria explain what remains and link a follow-up
  issue when appropriate.
- [ ] The PR is not represented as complete while a required criterion is
  Partial or Not Met.

Cross-check the table against the diff and test results; a checked box without
matching evidence is not sufficient.

## 6. Reviewer outcome

Choose one outcome and leave evidence-specific feedback:

- **Approve** only when all applicable checks above pass and the exact-head CI
  required for the patch is green.
- **Request changes** with the missing criterion, file, test, risk control, or
  documentation impact named precisely.
- **Defer review** when the blocker is external (for example, maintainer-only
  workflow approval or infrastructure failure) and record the owner and next
  action.

Technical approval or merge does not by itself establish an external reward or
payment decision. Keep review feedback about repository quality and leave any
platform evaluation to the platform's documented process.

## Copy-ready review record

```markdown
### Reviewer evidence record

- Issue/scope: Pass / Changes requested — evidence
- Implementation/risk: Pass / Changes requested — evidence
- Tests: Pass / Changes requested / N/A — evidence
- Exact-head CI: Pass / Blocked — run links and head commit
- Acceptance criteria: Complete / Partial — traceability notes
- Documentation impact: Complete / Changes requested / N/A — evidence
- Outcome: Approve / Request changes / Defer
- Next action and owner: ...
```
