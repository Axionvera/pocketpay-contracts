# Test Coverage Matrix

This matrix maps our existing test suite (unit, integration, and fuzzing/property tests) directly against the identified threat vectors, providing concrete code evidence that every critical path is covered.

For the full security evaluation, please refer to the main **[Audit Evidence Index](../docs/audit-evidence-index.md)**.

## Unit & Property Tests (`../contracts/savings_vault/src/test/`)

| Threat Vector | Addressed In (Code Evidence) | Description |
| --- | --- | --- |
| **Unauthorized Admin Access** | `unauthorized_access.rs` | Verifies non-admins cannot invoke `pause` or `initialize`. |
| **Admin Invariant Violations** | `admin_invariant_guard.rs` | Checks that admins cannot bypass balance checks or extract funds. |
| **Accounting / Ledger Mismatch** | `balance_conservation.rs`, `property_vault_accounting.rs` | Ensures `total_balance >= locked_balance + withdrawable_balance` is never broken under any operation. |
| **Token Transfer Rollback** | `token_transfer_rollback.rs` | Ensures failures in external token transfers rollback the vault state. |
| **Cross-User Data Leaks** | `cross_user_isolation.rs` | Prevents User A from affecting User B's locks or balances. |
| **Malicious Lock Exploits** | `lock_maturity_boundary.rs`, `invalid_lock_id.rs` | Validates lock IDs and strict boundary checks for unlock times. |
| **Replay Attacks** | `replay_protection.rs` | Ensures lock IDs are monotonic and cannot be reused or replayed. |
| **Emergency Pause Bypassing** | `pause.rs`, `pause_transition.rs` | Verifies `deposit` and `lock` revert during pause, while `withdraw` succeeds. |

## Integration & Atomicity Tests (`atomicity/`)

| Threat Vector | Addressed In (Code Evidence) | Description |
| --- | --- | --- |
| **Partial State Transitions** | `transfer-atomicity.test.ts` | End-to-end integration test verifying that a token transfer (SAC) and internal ledger update execute atomically or fail entirely without leaving orphaned balances. |
