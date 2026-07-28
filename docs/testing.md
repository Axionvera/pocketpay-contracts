# Test Naming Conventions

This document defines the naming convention for unit tests in
`contracts/savings_vault/src/test/`, so tests added by different contributors
stay easy to scan and search.

This convention describes the pattern already used by most tests in this
repository; it does not require renaming existing tests. Follow it for new
tests, and prefer it when you touch a test that doesn't yet follow it.

## Pattern

```text
test_<function_under_test>_<scenario>[_panics]
```

- **`function_under_test`** — the contract method the test exercises, using
  its exact name from `lib.rs` (`deposit`, `withdraw`, `lock_funds`,
  `get_balance`, `get_locked_balance`, `can_withdraw`, `initialize`).
- **`scenario`** — the input or state condition under test, in a few words
  (`zero_amount`, `negative_amount`, `entire_balance`,
  `more_than_balance`, `multiple_times`).
- **`_panics`** — append this suffix when the test asserts the call panics
  (via `#[should_panic]` or an expected trap). Omit it for tests that assert
  a successful result or a returned value.

Every test name should read as a short sentence: which function, under what
condition, with what expected outcome.

## Examples from this repository

Good names already in `test/mod.rs`:

- `test_deposit_zero_panics` — `deposit` rejects a zero amount.
- `test_deposit_negative_panics` — `deposit` rejects a negative amount.
- `test_withdraw_more_than_balance_panics` — `withdraw` rejects an
  over-withdrawal.
- `test_withdraw_entire_balance` — `withdraw` succeeds when draining the full
  balance.
- `test_get_balance_default_zero_for_new_user_after_initialization` —
  `get_balance` returns `0` for a user who has never deposited.
- `test_lock_funds_multiple_times` — `lock_funds` succeeds across repeated
  calls.

Names to avoid:

- `test1`, `test_deposit_2` — no scenario or expected result.
- `deposit_test` — function name and `test` in the wrong order; harder to
  group alphabetically with the other `deposit` tests.
- `test_withdraw_fails` — vague; doesn't say which failure condition. Prefer
  `test_withdraw_more_than_balance_panics` or
  `test_withdraw_from_empty_balance_panics`.

## Coverage guidance

For each public contract function, name tests so that success and failure
paths are both easy to find by scanning names alone:

- At least one success-path test per function
  (`test_deposit`, `test_withdraw`).
- One test per distinct failure or edge case, each with its own name rather
  than combining multiple conditions into one test
  (`test_withdraw_zero_panics`, `test_withdraw_negative_panics`,
  `test_withdraw_from_empty_balance_panics`).
- Boundary conditions (zero, negative, exact balance, empty state) get their
  own scenario name instead of being folded into a generic `_edge_case` test.

## Scope

This document covers Rust unit test function names under
`contracts/savings_vault/src/test/`. See the
[Documentation Style Guide](docs-style-guide.md) for conventions covering
prose documentation instead.
