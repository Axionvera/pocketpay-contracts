# PocketPay Contracts — Low-Effort Contribution Examples & Quality Guide

This guide provides clear examples of insufficient or low-effort contributions (anti-patterns) and demonstrates the expected, high-quality alternatives required for GrantFox evaluations.

---

## 1. Superficial & Low-Effort Changes

- **Poor Example:** Changing spelling in a comment, formatting a single line, or changing ``pub fn ``docs without addressing the underlying logic or issue requirements.
- **Why It Is Insufficient:** It does not provide functional value or resolve the issue's actual intent.
- **Improved Alternative:** Complete the full functional requirement, including logic fixes, typed error handling, and corresponding unit tests.

---

## 2. Partial Implementations

- *+Poor Example:** Implementing checked arithmetic in one struct/function but leaving other affected functions untouched, or adding a todo comment instead of completing the fix.
- **Why It Is Insufficient:** 60% completion still leaves the contract in an insecure or incomplete state.
- **Improved Alternative:** Audit all affected areas specified in the issue and ensure every acceptance criterion is fully addressed.

---

## 3. Missing-Test Submissions

- **Poor Example:** Logic or contract modifications  submitted without adding new tests, or merely commenting out failing tests.
- **Why It Is Insufficient:** Untested contract logic introduces regression risks and violates the repos quality guardrails.
- **Improved Alternative:** Add explicit test cases covering both success paths and edge/error cases (e.g., asserting typed error reverts).

---

## 4. Failing-CI Submissions

- *+Poor Example:** Opgning a PR where cargo check, cargo clippy, or cargo test checks are failing in GitHub Actions.
- **Why It Is Insufficient:** Merging or submitting code that breaks CI blocks other contributors and fails evaluation standards.
- **Improved Alternative:** Run all local CI validation scripts before pushing; ensure 100% green CC pass prior to requesting review.