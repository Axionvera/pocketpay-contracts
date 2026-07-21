# Test Coverage Summary

This document maps the savings vault's public behaviours to the tests that
exercise them, so contributors can see what's already covered before adding
new tests. It is a coverage **map**, not a line-coverage report — see
[Additional Notes](#additional-notes).

Source: `contracts/savings_vault/src/test/` (`mod.rs`, `initialization.rs`,
`balance_conservation.rs`, `withdraw_lock.rs`, `maximum_amount_boundary.rs`,
`unauthorized_access.rs`, `lock_read_helpers.rs`).

## Initialization

| Behaviour | Status | Test(s) |
|---|---|---|
| Successful one-time initialization | Covered | `test_initialize`, `test_initialize_success` |
| Re-initialization is rejected | Covered | `test_initialize_twice_panics`, `test_initialize_fails_on_second_call` |
| Calling other functions before `initialize` panics | Covered | `test_deposit_before_initialization_panics`, `test_withdraw_before_initialization_panics`, `test_lock_funds_before_initialization_panics`, `test_read_functions_before_initialization`, `test_deposit_uninitialized_panics`, `test_withdraw_uninitialized_panics`, `test_lock_funds_uninitialized_panics`, `test_get_balance_uninitialized_panics`, `test_get_locked_balance_uninitialized_panics`, `test_can_withdraw_uninitialized_panics` |
| `initialize` emits an event | Covered | `test_initialize_emits_event` |
| `get_version` returns the contract version | Covered | `test_get_version` |

## Deposit

| Behaviour | Status | Test(s) |
|---|---|---|
| Basic deposit increases available balance | Covered | `test_deposit`, `test_multiple_deposits` |
| Deposit requires the depositing user's authorization | Covered | `test_deposit_requires_user_authorization`, `test_unauthorized_deposit_fails` |
| Zero / negative amounts are rejected | Covered | `test_deposit_zero_panics`, `test_deposit_negative_panics` |
| Failed token transfer does not mutate balance | Covered | `test_deposit_fails_when_token_transfer_fails` |
| Invalid deposits do not mutate balance (conservation) | Covered | `conservation_invalid_deposits_do_not_mutate` |
| `deposit` emits an event | Covered | `test_deposit_emits_event` |
| Deposits near `i128::MAX` don't overflow | Covered | `test_deposit_i128_max_succeeds`, `test_deposit_after_i128_max_preserves_balance_on_overflow`, `test_multiple_large_deposits_without_overflow`, `test_deposit_half_max_twice_equals_max` |

## Withdraw

| Behaviour | Status | Test(s) |
|---|---|---|
| Basic withdraw / full-balance withdraw | Covered | `test_withdraw`, `test_withdraw_returns_tokens_to_user`, `test_withdraw_entire_balance` |
| Withdraw requires the withdrawing user's authorization | Covered | `test_withdraw_requires_user_authorization`, `test_unauthorized_withdraw_fails` |
| Zero / negative / over-balance amounts are rejected | Covered | `test_withdraw_zero_panics`, `test_withdraw_negative_panics`, `test_withdraw_more_than_balance_panics`, `test_withdraw_from_empty_balance_panics`, `test_withdraw_exceeds_available_after_deposit_panics` |
| Failed withdraw does not mutate available/locked balance | Covered | `test_failed_withdraw_does_not_change_available_balance`, `test_failed_withdraw_does_not_change_available_balance_panics`, `test_failed_withdraw_does_not_change_locked_balance` |
| Withdraw exceeding available balance while funds are locked does not mutate state | Covered | `conservation_withdraw_exceeds_available_while_locked_does_not_mutate` |
| `withdraw` emits an event | Covered | `test_withdraw_emits_event` |
| Withdrawals near `i128::MAX` don't overflow | Covered | `test_withdraw_i128_max_after_deposit_succeeds`, `test_withdraw_over_large_balance_does_not_mutate`, `test_withdraw_partial_from_large_balance_preserves_remainder`, `test_large_withdraw_spans_available_and_matured_locks` |
| `withdraw_lock` (withdraw a single matured lock by ID) | Covered | `test_withdraw_matured_lock_success`, `test_withdraw_immature_lock_fails`, `test_withdraw_nonexistent_lock_fails`, `test_withdraw_repeated_lock_fails`, `test_withdraw_wrong_user_lock_fails`, `test_unauthorized_withdraw_lock_fails` |
| `withdraw_lock` emits an event | **Gap** | No dedicated `test_withdraw_lock_emits_event`-style test, unlike `deposit`/`withdraw`/`lock_funds`. The event is emitted in `lib.rs` but not asserted. |

## Locking

| Behaviour | Status | Test(s) |
|---|---|---|
| Basic lock / repeated locks accumulate | Covered | `test_lock_funds`, `test_lock_funds_multiple_times`, `test_repeated_lock_accumulates_balance_and_overwrites_unlock_time_later`, `test_repeated_lock_overwrites_unlock_time_with_earlier_value`, `test_repeated_lock_three_times_accumulates_and_keeps_last_unlock_time` |
| Lock requires the locking user's authorization | Covered | `test_lock_funds_requires_user_authorization`, `test_unauthorized_lock_fails` |
| Zero amount / past unlock time / over-balance locks are rejected | Covered | `test_lock_zero_panics`, `test_lock_past_time_panics`, `test_lock_from_empty_balance_panics`, `test_lock_more_than_balance_panics`, `test_lock_more_than_available_balance_panics` |
| Failed lock does not mutate available/locked balance | Covered | `test_failed_lock_does_not_change_available_balance`, `test_failed_lock_does_not_change_available_balance_panics`, `test_failed_lock_does_not_change_locked_balance` |
| Invalid locks do not mutate balance (conservation) | Covered | `conservation_invalid_locks_do_not_mutate` |
| `lock_funds` emits an event | Covered | `test_lock_funds_emits_event` |
| Locks near `i128::MAX` don't overflow | Covered | `test_lock_i128_max_succeeds`, `test_lock_over_large_balance_does_not_mutate`, `test_lock_partial_from_large_balance_preserves_state`, `test_large_lock_keeps_available_and_locked_consistent` |
| `can_withdraw` maturity boundary (before / at / after unlock, inclusive `>=` rule) | Covered | `test_can_withdraw_before_unlock`, `test_can_withdraw_one_second_before_unlock_returns_false`, `test_can_withdraw_exactly_at_unlock`, `test_can_withdraw_after_unlock`, `test_can_withdraw_one_second_after_unlock_returns_true`, `test_can_withdraw_no_locked_funds`, `test_can_withdraw_boundary_rule_is_inclusive_gte` |
| `get_locked_balance` correctness across the unlock boundary | Covered | `test_locked_balance_correct_before_at_and_after_unlock` |
| `get_lock(user, lock_id)` / `list_locks(user, offset, limit)` | Covered | `test_get_lock_empty_user_returns_none`, `test_get_lock_single_lock`, `test_get_lock_multi_lock_and_pagination`, `test_get_lock_uninitialized_panics`, `test_list_locks_uninitialized_panics` |
| `list_locks` pagination edge cases (`limit = 0`, offset beyond the last lock) | **Gap** | Only one pagination test (`test_get_lock_multi_lock_and_pagination`); zero-limit and out-of-range-offset cases aren't explicitly asserted. |

## Balances

| Behaviour | Status | Test(s) |
|---|---|---|
| `get_balance` default (no deposits / new user) | Covered | `test_get_balance_no_deposits`, `test_get_balance_default_zero_for_new_user_after_initialization` |
| Balances are isolated per user | Covered | `test_separate_user_balances`, `balance_isolation_between_users_deposit`, `balance_isolation_between_users_lock`, `balance_isolation_between_users_withdraw` |
| Balance is conserved across mixed deposit/withdraw/lock sequences | Covered | `conservation_deposit_withdraw_cycle`, `conservation_multiple_deposits_and_partial_withdrawals`, `conservation_lock_and_time_advance`, `conservation_withdraw_after_partial_lock_maturity`, `conservation_deposit_while_funds_locked`, `conservation_long_mixed_sequence`, `conservation_mixed_valid_and_invalid_sequence`, `conservation_table_driven_sequences` |

## Known Test Gaps

- **`withdraw_lock` event emission is not asserted.** `deposit`, `withdraw`,
  `lock_funds`, and `initialize` each have a `*_emits_event` test; `withdraw_lock`
  does not, even though `lib.rs` publishes an event for it.
- **`list_locks` pagination edge cases.** Zero-limit and out-of-range-offset
  behaviour for `list_locks` isn't explicitly covered.
- **No fuzz/property-based tests.** Coverage relies on hand-written cases
  (including a table-driven test in `balance_conservation.rs`) rather than
  randomized or property-based testing.

## Additional Notes

This document tracks *behavioural* coverage (which contract behaviours have
at least one test), not measured line/branch coverage. Run `cargo test` from
the repository root to execute the full suite (see [README](../README.md#test)).
