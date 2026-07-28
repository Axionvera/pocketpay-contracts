# Vault Failure Mode Catalogue

This catalogue documents the expected safe-failure behavior for the Savings Vault contract. It is intended for contributors, reviewers, and auditors who need to understand how vault operations behave when inputs are invalid, authorization fails, or external state changes do not succeed.

## Purpose

The contract is designed to fail safely. In practice, that means:

- invalid operations should revert with a clear panic message,
- state should not be partially mutated when a precondition fails,
- balances should remain consistent across rejected operations,
- and the contract should remain understandable and auditable.

## Failure Categories

| Category | Affected functions | Typical trigger | Expected behaviour | Related tests |
|---|---|---|---|---|
| Initialization / lifecycle | `initialize` | Re-initializing an already initialized contract | Panics with `Contract is already initialized` and does not create a second instance state | `test_initialize`, `test_initialize_twice_panics` |
| Invalid amount input | `deposit`, `withdraw`, `lock_funds` | Zero or negative amounts | Panics immediately with a descriptive error; no balance mutation occurs | `test_deposit_zero_panics`, `test_deposit_negative_panics`, `test_withdraw_zero_panics`, `test_withdraw_negative_panics`, `test_lock_zero_panics` |
| Insufficient available balance | `withdraw`, `lock_funds` | Attempting to withdraw or lock more than the unlocked balance | Panics with an insufficient-balance error; no state mutation occurs | `test_withdraw_more_than_balance_panics`, `test_withdraw_from_empty_balance_panics`, `test_withdraw_exceeds_available_after_deposit_panics`, `test_lock_more_than_balance_panics` |
| Invalid unlock timing | `lock_funds` | Unlock time is in the past or equal to the current ledger time | Panics with `Unlock time must be in the future`; the lock is not created | `test_lock_past_time_panics` |
| Authorization failure | `initialize`, `deposit`, `withdraw`, `lock_funds` | Caller does not provide the required signature (`require_auth` fails) | The transaction aborts before any state-changing side effects | All tests exercise auth via `env.mock_all_auths()`; Soroban SDK enforces auth at the host level |
| Token transfer failure | `withdraw` | The underlying `token::Client::transfer` call fails (e.g., contract has insufficient token balance) | The transfer is called **before** the internal balance is decremented, so a failed transfer panics and leaves the vault's internal accounting intact | `test_withdraw`, `test_withdraw_entire_balance`, `test_failed_withdraw_does_not_change_available_balance` |
| Missing token configuration | `withdraw` | The contract was not initialized with a valid token address (e.g., `initialize` never called) | The `DataKey::Token` storage lookup returns `None`; the `.unwrap()` panics before any transfer or balance mutation | No dedicated test; guarded by `initialize` being a prerequisite |
| Read-only queries on unset state | `get_balance`, `get_locked_balance`, `can_withdraw` | User has never deposited or locked funds | Returns `0` or `false` via `unwrap_or(0)` / `unwrap_or(false)` without panicking | `test_get_balance_no_deposits`, `test_can_withdraw_no_locked_funds` |

## Expected Error Behaviour by Function

### `initialize(admin, token)`

- The function is one-shot; guarded by the `Initialized` flag in instance storage.
- A second call panics with `Contract is already initialized`.
- The admin address and token address are stored in instance storage exactly once.
- Both `admin` and `token` are arbitrary `Address` values — the contract does not validate them beyond requiring `admin.require_auth()`.
- If the token address points to a non-existent or non-token contract, `withdraw` will fail at transfer time (see Token transfer failure category above).

### `deposit(user, amount)`

- Zero or negative deposits panic with `Deposit amount must be greater than zero` before any storage mutation.
- The function requires `user.require_auth()` — the caller must be the depositing user.
- On success, the user's available balance is incremented by `amount`.
- **Note:** The current implementation only updates the internal vault balance; it does **not** call `token::Client::transfer` to pull tokens from the user. Deposits rely on an external token transfer step (visible in test helpers).

### `withdraw(user, amount)`

- Zero or negative withdrawals panic with `Withdrawal amount must be greater than zero` before any storage mutation.
- Withdrawals larger than the available balance panic with `Insufficient balance` before any storage mutation.
- **Token transfer ordering:** The `token::Client::transfer` call happens **before** the internal balance is decremented. This ordering ensures:
  - If the token transfer fails (e.g., contract holds insufficient tokens), the entire call panics and the vault's internal balance is never decremented.
  - If the token transfer succeeds, the balance update follows and is guaranteed to succeed (no further fallible operations after the transfer).
- The token address is read from instance storage (`DataKey::Token`). If missing (contract not initialized), the `.unwrap()` panics before any transfer or balance mutation.

### `lock_funds(user, amount, unlock_time)`

- Zero or negative lock amounts panic with `Lock amount must be greater than zero`.
- A past or current unlock time (`unlock_time <= ledger.timestamp()`) panics with `Unlock time must be in the future`.
- Lock amounts above the available balance panic with `Insufficient balance to lock`.
- On success, the amount is moved atomically from available balance to locked balance, and the unlock time is stored.
- **Note:** Overlapping locks overwrite the previous `UnlockTime` — the latest lock's timestamp governs when `can_withdraw` returns `true`, regardless of prior locks.

### `get_balance(user)`, `get_locked_balance(user)`, `can_withdraw(user)`

- These are read-only helpers and do not mutate state.
- They return `0` or `false` for empty/unset state via `unwrap_or(0)` / `unwrap_or(false)` rather than panicking.
- `can_withdraw` returns `true` when both: (a) locked balance > 0, AND (b) `ledger.timestamp() >= unlock_time` (inclusive check).

## Accounting Invariants

The contract should preserve the following invariants across rejected operations:

- **Available balance** must not decrease when a withdrawal is rejected.
- **Locked balance** must not increase or decrease when a failed operation is rejected.
- **Internal vs. token balance consistency:** The vault's internal accounting (`DataKey::Balance`) must not drift from the contract's actual token holdings when an external transfer fails.

These invariants are enforced by the ordering of operations in `withdraw`: the token transfer (which may fail) happens before the internal balance is decremented. In Soroban, all cross-contract calls are atomic within a transaction — a failed token transfer causes the entire `withdraw` invocation to revert, leaving the vault's storage untouched.

## Related Test Coverage

The unit test suite lives in [`contracts/savings_vault/src/test.rs`](../contracts/savings_vault/src/test.rs), with helpers in [`test_helpers.rs`](../contracts/savings_vault/src/test_helpers.rs). Snapshot files are stored under [`test_snapshots/test/`](../contracts/savings_vault/test_snapshots/test/).

### Initialization
- `test_initialize` — Successful initialization records admin and token.
- `test_initialize_twice_panics` — Second call panics with `Contract is already initialized`.

### Deposit
- `test_deposit_zero_panics` — Zero deposit panics with `Deposit amount must be greater than zero`.
- `test_deposit_negative_panics` — Negative deposit panics with same error.

### Withdrawal
- `test_withdraw_zero_panics` — Zero withdrawal panics with `Withdrawal amount must be greater than zero`.
- `test_withdraw_negative_panics` — Negative withdrawal panics with same error.
- `test_withdraw_more_than_balance_panics` — Over-withdrawal panics with `Insufficient balance`.
- `test_withdraw_from_empty_balance_panics` — Withdrawing from never-deposited user panics.
- `test_withdraw_exceeds_available_after_deposit_panics` — Withdrawing deposit+1 panics.
- `test_withdraw` — Happy-path withdrawal via token transfer.
- `test_withdraw_entire_balance` — Full balance withdrawal succeeds.

### Balance invariants under failure
- `test_failed_withdraw_does_not_change_available_balance` — Valid partial and full withdrawals preserve correct remainder.
- `test_failed_withdraw_does_not_change_available_balance_panics` — Over-withdraw of 1 unit panics; balance stays at deposit amount.
- `test_failed_withdraw_does_not_change_locked_balance` — Over-withdraw when funds are locked panics; both available and locked balances unchanged.

### Locking
- `test_lock_zero_panics` — Zero lock panics with `Lock amount must be greater than zero`.
- `test_lock_more_than_balance_panics` — Lock exceeding available balance panics with `Insufficient balance to lock`.
- `test_lock_past_time_panics` — Lock with past timestamp panics with `Unlock time must be in the future`.

### Queries
- `test_get_balance_no_deposits` — Balance returns 0 for users who never deposited.
- `test_can_withdraw_no_locked_funds` — Returns false when no funds are locked.
- `test_can_withdraw_before_unlock`, `test_can_withdraw_after_unlock`, `test_can_withdraw_exactly_at_unlock` — Lock/unlock timing edge cases.

### Isolation
- `test_separate_user_balances` — Alice and Bob balances are independent; withdrawing from one does not affect the other.

## Audit Readiness Notes

This catalogue should be updated whenever a new failure mode is introduced, a panic message changes, or a new safety invariant is added. Keeping it aligned with the implementation and tests improves maintainability and makes the contract easier to review.
