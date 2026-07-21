# Savings Vault Error Reference

This reference describes current behavior in `contracts/savings_vault/src/lib.rs`.
The contract does **not** define a custom error enum or stable numeric error
codes. Validation failures use Rust panic messages; authorization and token
failures come from the Soroban host or configured token contract.

SDK and mobile callers should treat the text below as diagnostic information,
not a stable machine-readable API. A failed invocation does not commit that
invocation's state changes. Show users a friendly message, retain the complete
simulation or transaction diagnostic, and avoid branching on panic text.

## Initialisation errors

### `Contract is already initialized`

- **Current failure:** Panic message from `initialize`.
- **Meaning:** The one-time initialization flag already exists.
- **Likely cause:** A repeated initialization, a retry after success, or the
  wrong contract ID.
- **Caller/developer action:** Do not retry. Confirm the contract ID and use the
  existing deployment; this is not a transient network failure.

### Missing initialization data during withdrawal

- **Current failure:** Storage unwrap/trap with no custom message or code.
- **Meaning:** `withdraw` cannot load the token address stored by `initialize`.
- **Likely cause:** Initialization did not occur, or instance storage is
  unavailable or expired. Because `deposit` does not check initialization, it
  can create an internal balance before this failure is encountered.
- **Caller/developer action:** Ensure initialization succeeded before enabling
  vault operations and verify instance storage is live. Surface "vault is not
  initialized or unavailable" and retain the diagnostic.

Initialization also requires authorization from `admin`; see
[Unauthorised access errors](#unauthorised-access-errors).

## Invalid amount errors

### `Deposit amount must be greater than zero`

- **Current failure:** Panic message from `deposit`.
- **Meaning:** The deposit amount is zero or negative.
- **Likely cause:** Invalid input, unit conversion, sign handling, or an empty
  field converted to zero.
- **Caller/developer action:** Require a positive `i128` amount in the token's
  smallest unit before invoking the contract.

### `Withdrawal amount must be greater than zero`

- **Current failure:** Panic message from `withdraw`.
- **Meaning:** The withdrawal amount is zero or negative.
- **Likely cause:** Invalid input or an amount-conversion bug.
- **Caller/developer action:** Reject non-positive amounts before submission.

### `Lock amount must be greater than zero`

- **Current failure:** Panic message from `lock_funds`.
- **Meaning:** The lock amount is zero or negative.
- **Likely cause:** Invalid input or an amount-conversion bug.
- **Caller/developer action:** Require a positive amount before submission.

## Insufficient balance errors

These checks use the vault's **available internal balance**, not the wallet
balance or locked balance.

### `Insufficient balance`

- **Current failure:** Panic message from `withdraw`.
- **Meaning:** The withdrawal exceeds the available internal balance; a missing
  balance is treated as zero.
- **Likely cause:** The request is too large, no deposit is recorded, or some
  balance was moved to the locked bucket.
- **Caller/developer action:** Refresh `get_balance(user)`, cap the request to
  that value, and explain that locked funds are unavailable.

### `Insufficient balance to lock`

- **Current failure:** Panic message from `lock_funds`.
- **Meaning:** The lock amount exceeds the available internal balance.
- **Likely cause:** A stale displayed balance, an excessive request, or funds
  already moved to the locked bucket.
- **Caller/developer action:** Refresh `get_balance(user)` and allow no more than
  the returned available amount.

## Lock and unlock time errors

### `Unlock time must be in the future`

- **Current failure:** Panic message from `lock_funds`.
- **Meaning:** `unlock_time` is less than or equal to the current ledger
  timestamp; it must be strictly later when executed.
- **Likely cause:** A past timestamp, seconds/milliseconds confusion, clock skew,
  or submission too close to the selected time.
- **Caller/developer action:** Send Unix time in **seconds** and leave a safety
  margin beyond the latest ledger time.

### Locked funds are not yet withdrawable

- **Current condition:** `can_withdraw(user)` returns `false`; it does not fail.
- **Meaning:** No locked funds exist, or the ledger timestamp is earlier than
  the unlock time. At exactly the unlock timestamp it returns `true`.
- **Likely cause:** The lock has not matured or no lock exists.
- **Caller/developer action:** Treat `false` as normal state and disable the
  action. The current contract has no operation to release or withdraw locked
  funds; `can_withdraw` is only a query.

## Unauthorised access errors

### Missing required authorization

- **Current failure:** Soroban host authorization failure from `require_auth()`;
  no contract-defined message or numeric code exists.
- **Meaning:** Valid authorization for the required address is absent.
- **Likely cause:** `initialize` lacks `admin` authorization, or `deposit`,
  `withdraw`, or `lock_funds` lacks `user` authorization. The app may be trying
  to act for another address.
- **Caller/developer action:** Build and sign with the required address. Do not
  retry unchanged; request the correct wallet signature.

Read-only calls (`get_balance`, `get_locked_balance`, `get_lock`, `list_locks`,
and `can_withdraw`) do not call `require_auth()`.

## Other existing failure conditions

### Token transfer failure during withdrawal

- **Current failure:** Error or trap propagated by the configured token
  contract; the vault defines no wrapper error.
- **Meaning:** The token transfer from the vault contract to the user failed.
- **Likely cause:** Insufficient real token balance, an invalid or incompatible
  token address, token authorization failure, or token-contract rejection. An
  internal balance does not guarantee matching tokens are held.
- **Caller/developer action:** Inspect the nested token diagnostic. Verify the
  configured token and vault token balance; do not label this only as an
  internal-balance error.

## Existing custom errors

There are no custom savings-vault error variants or numeric codes. The quoted
panic messages above are all explicit application-level validation messages in
the current contract.

## Planned or recommended future errors

These recommendations are **not implemented**:

- Add a `#[contracterror]` enum with stable numeric variants for already
  initialized, invalid amount, insufficient balance, invalid unlock time, and
  not initialized.
- Add an explicit locked-funds-not-mature error if a locked-fund withdrawal
  operation is introduced.
- Distinguish token-transfer failures from vault accounting failures while
  preserving their underlying diagnostics.

Until then, callers should not invent numeric mappings for panic messages.
