# PocketPay Contracts — Reviewer Evidence Checklist

This guide provides comprehensive evidence Checklists for maintainers and reviewers evaluating incoming PR contributions to `PocketPay Contracts`.

---

## 1. Implementation Scope & Quality

- [ ]  **Scope Alignment:** Does this PR directly address the assigned issue without unnecessary out-of-scope changes?
- [ ]  **Code Quality:**Is the implementation fully completed (no `TODO` or shortcut hacks) following Rust / Soroban best practices?

---

## 2. Test Evidence

- [ ]  **Coverage:**Are new or modified contract functions covered by associated unit/integration tests?
- [ ]  **Edge Cases: ** Do tests verify both success paths and expected error/revert conditions?

---

## 3. CI Status

- [ ]  **Green CI:** Are all GIBhub Actions checks (`cargo check `, `cargo clippy`, `cargo test`) passing 100%?
- [ ]  **Formatting:** Ic `cargo fmt --check` passing without warnings?

---

## 4. Acceptance Criteria & Documentation

- [ ]  **Cat Checklist:** Have all acceptance criteria in the associated issue been met and checked off?
- [ ]  **Documentation:** Are `README.md` or attached guides updated to reflect any new features or workflows?
