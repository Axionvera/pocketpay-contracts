# Meaningful Change Threshold

PocketPay Contracts evaluates a contribution by the complete outcome it
delivers, not by a minimum number of changed lines. A short diff can be the
right fix, while a large diff can still leave the issue incomplete.

This guide complements the [Contribution Quality Gate](contribution-quality-gate.md).
The quality gate remains the source of truth for contract safety, tests,
documentation, and local verification requirements.

## What makes a change meaningful

A change is meaningful when the pull request provides enough evidence for a
reviewer to conclude that it:

1. addresses the linked issue's complete agreed scope;
2. implements the intended behavior without placeholders or unrelated work;
3. includes the tests and edge cases appropriate to the risk;
4. updates documentation and traceability where the public contract or
   contributor workflow changes; and
5. passes the repository's relevant verification and CI checks.

Line count can help a reviewer notice an unexpectedly small or broad diff, but
it is only a prompt to inspect the evidence. It is not an acceptance criterion
and must not be used as a substitute for technical review.

## Small complete changes and small incomplete changes

| Change | Small but complete | Small and incomplete |
|---|---|---|
| Boundary fix | Changes the exact comparison, adds tests immediately below, at, and above the boundary, and maps the result to the issue criteria. | Changes the comparison but adds no boundary or regression test. |
| Error mapping | Adds the missing `ContractError` mapping, covers the failure path, and updates error documentation when public behavior changes. | Adds a new error variant without wiring every throw path or documenting the public result. |
| Documentation correction | Fixes the incorrect guidance at its source, updates affected links, and verifies that the links resolve. | Adds a second explanation elsewhere while leaving the incorrect source unchanged. |
| Focused refactor | Removes duplication without changing behavior and demonstrates that the existing relevant tests still pass. | Moves code while leaving dead paths, placeholders, or unexplained behavior differences. |

A small complete change should not be padded with unrelated files or cosmetic
edits. An incomplete change should not be accepted merely because it has many
lines.

## Insufficient examples

The following do not meet the meaningful-change threshold:

- a comment, rename, or formatting-only diff presented as a behavioral fix;
- a happy-path implementation when the issue requires authorization, failure,
  or boundary behavior;
- a new function with `TODO`, `FIXME`, mocked return values, or unreachable
  integration points left in the requested scope;
- tests that only restate implementation details and do not prove the issue's
  observable behavior;
- documentation that duplicates existing guidance instead of updating or
  linking the repository's source of truth;
- a broad cleanup that obscures the issue-specific change and makes its
  evidence difficult to review; or
- a pull request that omits acceptance criteria, known limitations, or relevant
  verification results.

## Reviewer assessment

Reviewers should use this sequence rather than counting lines:

1. **Confirm scope.** Compare the diff with the linked issue and its acceptance
   criteria. Identify any requested behavior that is absent.
2. **Trace the outcome.** Follow each criterion to implementation,
   documentation, and test evidence in the pull request's
   [traceability table](traceability-table.md).
3. **Check risk coverage.** Apply the contract-specific checks in the
   [Contribution Quality Gate](contribution-quality-gate.md), including
   authorization, rollback, invariant, and edge-case expectations when they
   are relevant.
4. **Check integration.** Confirm the change is wired into the actual contract
   or contributor workflow rather than added as an unused helper or isolated
   document.
5. **Classify the result.** Approve a small change when the evidence is complete;
   request the missing issue-scoped work when it is not. Ask for unrelated
   improvements in a follow-up issue rather than padding the current pull
   request.

If the issue itself is ambiguous, reviewers should clarify the intended scope
before using this guide to judge completeness.
