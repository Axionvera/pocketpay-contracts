# Contract Simulation Compatibility

Soroban clients preview every call via `simulateTransaction` before asking
a user to sign anything: the RPC node runs the contract invocation against
current ledger state, returns the outcome (or error), and never commits
it. This document is a per-function reference for what an SDK should
expect from that preview, for every public `SavingsVault` function.

For the general request/response shape of `simulateTransaction` itself —
where it sits between the mobile app, the SDK, and the contract — see
[SDK ↔ Contract Sequence Diagrams](sdk-contract-sequence.md). This document
covers per-function behavior instead: does a given call need a signature
at all, is it safe to call speculatively, and what happens when it fails.

Every claim below is backed by a passing test in
[`contracts/savings_vault/src/test/simulation_compatibility.rs`](../contracts/savings_vault/src/test/simulation_compatibility.rs) —
run `cargo test simulation_compatibility` to reproduce them directly.

## 1. Read-only calls: safe to simulate, never require a signature

None of the following call `require_auth()`. An SDK can call these purely
through `simulateTransaction` — no wallet prompt, no submitted transaction,
no cost beyond the RPC round-trip:

`get_version`, `get_token`, `get_admin`, `is_paused`, `get_min_deposit_amount`,
`get_max_lock_duration`, `get_min_lock_duration`, `get_balance`,
`get_locked_balance`, `can_withdraw`, `get_balance_snapshot`,
`get_lock_summary`, `get_lock`, `list_locks`.

Repeated simulation of the same read-only call is idempotent: it returns
the identical value every time as long as no state-changing call has
landed in between (`test_repeated_read_only_simulation_calls_are_idempotent`).

`get_balance_snapshot` and `get_lock_summary` exist specifically for SDK
and mobile clients that want a single-call overview instead of composing
several read calls (`unlocked`/`locked`/`total`/`withdrawable` and
`active_count`/`total_locked_amount`/`matured_count`/`withdrawable_amount`/
`earliest_unlock`/`latest_unlock`, respectively). `get_lock_summary`'s
aggregates and `list_locks`'s per-entry data always agree with each other
for the same user (`test_lock_summary_and_list_locks_agree_on_lock_state`).

## 2. Before `initialize`: four safe defaults, everything else the same panic

Four read-only getters work even before `initialize` has ever been called,
returning a defined default rather than failing:

| Function | Pre-init return value |
| --- | --- |
| `get_version` | `"0.1.0"` (hardcoded, doesn't depend on storage) |
| `get_min_deposit_amount` | `0` (no floor configured) |
| `get_max_lock_duration` | `0` (no ceiling configured) |
| `get_min_lock_duration` | `0` (no floor configured) |

Every other function — including every other read-only getter, not just
the state-changing ones — panics with the exact literal message
`"Contract is not initialized"` if called before `initialize`. This
includes `deposit`, `withdraw`, `lock_funds`, `withdraw_lock`,
`get_balance`, `get_admin`, `is_paused`, `get_balance_snapshot`,
`get_lock_summary`, `get_lock`, and `list_locks`.

Practical implication for an SDK probing an unfamiliar contract ID: a
simulation failure with exactly that message means "deployed but not yet
initialized," not a vault-specific error — safe to match on verbatim
(`test_pre_initialization_calls_split_between_safe_defaults_and_deterministic_panic`,
`test_pre_initialization_panic_message_is_identical_get_balance`,
`test_pre_initialization_panic_message_is_identical_deposit`).

## 3. State-changing calls: require a signature

`deposit`, `withdraw`, `lock_funds`, `withdraw_lock`, `extend_lock`,
`pause`, `unpause`, `transfer_admin`, `set_min_deposit_amount`,
`set_max_lock_duration`, and `set_min_lock_duration` all call
`require_auth()` on the relevant address (the `user` for vault operations,
the `admin` for configuration). Simulating one of these without a
signature fails and requests nothing from the user — the SDK's normal flow
is simulate first (to catch obvious failures and estimate cost/footprint
without a prompt), then request a signature only once simulation succeeds.
See [Deposit](sdk-contract-sequence.md#deposit) and
[Withdraw](sdk-contract-sequence.md#withdraw) for that sequence.

None of these calls have a partial-success path: a failed attempt (missing
auth or any other panic) leaves every observable value — balances, admin,
pause state — exactly as it was before the call
(`test_state_changing_calls_require_signature_and_fail_without_it`).

## 4. Matured withdrawal: predict the outcome before spending a simulation

`can_withdraw` and `get_balance_snapshot().withdrawable` are both free,
read-only, and always agree with what `withdraw_lock` will do:

- Before a lock's `unlock_time`: both report "nothing withdrawable," and
  `withdraw_lock` fails.
- At the exact `unlock_time` second (inclusive boundary) and after: both
  flip to reporting the matured amount, and `withdraw_lock` succeeds.
- After a successful `withdraw_lock`: both report the released amount is
  gone.

An SDK building a "claim matured funds" UI should call `can_withdraw` (or
inspect `get_balance_snapshot`) to decide whether to even offer the
action, rather than simulating `withdraw_lock` speculatively — see
`test_can_withdraw_predicts_withdraw_lock_outcome_across_maturity_boundary`
for the exact boundary fixture (lock created at `unlock_time = 5_000`,
checked at `4_999`, `5_000`, and after withdrawal).

## 5. Error and unsupported cases

Simulation-time failures are side-effect-free by construction: Soroban
never applies state from a call that panics, whether that call is only
simulated or fully submitted and rejected. `deposit`, `withdraw`, and
`lock_funds` each panic with a fixed literal message for their invalid-input
cases (zero/negative amounts, insufficient balance, amount exceeding
available balance) — the same message every time, safe to display or log
directly from a failed simulation's diagnostic
(`test_failed_state_changing_calls_leave_state_completely_unchanged`).

Two panic sources exist for token-moving calls and they are
distinguishable: a vault-level rejection (e.g. `"Insufficient balance"`)
panics from the vault contract itself, while a token-transfer failure
(the depositor doesn't hold enough of the configured SAC token) panics
from the token contract the vault calls into. Both surface in the
simulation diagnostic's event log with their originating contract ID, so
an SDK that needs to tell "you don't have enough of this token" apart from
"the vault rejected this amount" can do so — see
[`token_transfer_rollback.rs`](../contracts/savings_vault/src/test/token_transfer_rollback.rs)
for the token-transfer-failure side and
[Error Response Path](sdk-contract-sequence.md#error-response-path) for
how the SDK should generally handle both without branching on panic text.

## 6. SDK integration notes

- Call the free read helpers (`can_withdraw`, `get_balance_snapshot`,
  `get_lock_summary`) to pre-validate a user action locally before ever
  simulating the paid call it depends on — it's a local computation
  against already-fetched data, not another RPC round-trip.
- A `"Contract is not initialized"` simulation failure on a contract ID
  you expect to be live means treat it as "not deployed yet," not as a
  generic error to surface to the end user.
- Don't infer "the vault is broken" from a state-changing call's
  simulation failure without checking the read helpers first — most
  rejections (insufficient balance, immature lock, invalid amount) are
  expected outcomes of normal use, not contract bugs.
- Every fixture referenced above is deterministic (fixed amounts, fixed
  timestamps, fixed expected messages) specifically so SDK-side tests can
  assert against literal values instead of "some error occurred."

## See also

- [SDK ↔ Contract Sequence Diagrams](sdk-contract-sequence.md) — end-to-end
  request/response flow through the SDK and RPC.
- [Advanced Local Development and Testing Guide](advanced-development-and-testing.md) —
  how to run and extend this contract's test suite generally.
- [Savings Vault Error Reference](error-codes.md) — current failure
  conditions and caller guidance.
- [Test Coverage Summary](test-coverage.md) — coverage matrix across all
  contract behavior.
