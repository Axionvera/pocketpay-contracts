# PocketPay Contracts — Acceptance Criteria Audit Template

To ensure all contributions to `pocketpay-contracts` are verified objectively, contributors must include an **Acceptance Criteria Audit Table** in their PR description prior to requesting maintainer review.

---

## Acceptance Criteria Audit Matrix

Copy and paste the table below into your Pull Request description, mapping every criterion listed in the target issue to its implementation and test evidence.

| Acceptance Criterion | Status (`Pass` / `Deferred`) | Implementation Evidence (File & Line) | Test Evidence (Command / Test Name) | Doc Impact (Files / Inline Comments) |
| :--- | :---: | :--- | :--- | :--- |
| *e.g., Add checked_add overflow protection* | `Pass` | `contracts/src/lib.rs:L42-L48` | `cargo test test_overflow_reverts` | Updated inline doc comments |
| *e.g., Update README setup guide* | `Pass` | `README.md:L12` | N/A (Documentation) | `README.md` |

---

## Handling Incomplete or Deferred Criteria

If any acceptance criterion listed in the issue cannot be fully satisfied within the current PR scope:

1. **Mark Status clearly:** Set the status in the matrix to `Deferred` or `Partial`.
2. **Document Justification:** Provide a technical justification in the PR description explaining why the criterion was deferred.
3. **Link Follow-up Issue:** Create or reference an open tracking issue on GitHub for the deferred work and link it in the table. Unjustified or undocumented incomplete criteria will prevent PR approval.
