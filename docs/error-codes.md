# Savings Vault Error Reference

This reference describes error behavior in `contracts/savings_vault/src/lib.rs`.
The contract defines a custom error enum with stable numeric error codes via
the `#[contracterror]` attribute. See [error-code-standard.md](./error-code-standard.md)
for the complete error code standard and SDK mapping guidance.

SDK and mobile callers should use the numeric error codes for reliable error
handling and display user-friendly messages based on the error category.

## Configuration errors (1000-1999)

### `AlreadyInitialized` (Code: 1001)

- **Current failure:** Returns `ContractError::AlreadyInitialized` from `initialize`.
- **Meaning:** The one-time initialization flag already exists.
- **Likely cause:** A repeated initialization, a retry after success, or the
  wrong contract ID.
- **Caller/developer action:** Do not retry. Confirm the contract ID and use the
  existing deployment; this is not a transient network failure.

### `NotInitialized` (Code: 1002)

- **Current failure:** Returns `ContractError::NotInitialized` when attempting operations
  that require initialization.
- **Meaning:** The contract has not been initialized.
- **Likely cause:** Operations called before initialization or instance storage unavailable.
- **Caller/developer action:** Ensure initialization succeeded before enabling
  vault operations.

## Validation errors (2000-2999)

### `InvalidDepositAmount` (Code: 2001)

- **Current failure:** Returns `ContractError::InvalidDepositAmount` from `deposit`.
- **Meaning:** The deposit amount is zero or negative.
- **Likely cause:** Invalid input, unit conversion, sign handling, or an empty
  field converted to zero.
- **Caller/developer action:** Require a positive `i128` amount in the token's
  smallest unit before invoking the contract.

### `InvalidWithdrawAmount` (Code: 2002)

- **Current failure:** Returns `ContractError::InvalidWithdrawAmount` from `withdraw`.
- **Meaning:** The withdrawal amount is zero or negative.
- **Likely cause:** Invalid input or an amount-conversion bug.
- **Caller/developer action:** Reject non-positive amounts before submission.

### `InvalidLockAmount` (Code: 2003)

- **Current failure:** Returns `ContractError::InvalidLockAmount` from `lock_funds`.
- **Meaning:** The lock amount is zero or negative.
- **Likely cause:** Invalid input or an amount-conversion bug.
- **Caller/developer action:** Require a positive amount before submission.

### `InvalidUnlockTime` (Code: 2004)

- **Current failure:** Returns `ContractError::InvalidUnlockTime` from `lock_funds`.
- **Meaning:** `unlock_time` is less than or equal to the current ledger
  timestamp; it must be strictly later when executed.
- **Likely cause:** A past timestamp, seconds/milliseconds confusion, clock skew,
  or submission too close to the selected time.
- **Caller/developer action:** Send Unix time in **seconds** and leave a safety
  margin beyond the latest ledger time.

## Balance errors (4000-4999)

These checks use the vault's **available internal balance**, not the wallet
balance or locked balance.

### `InsufficientBalance` (Code: 4001)

- **Current failure:** Returns `ContractError::InsufficientBalance` from `withdraw`.
- **Meaning:** The withdrawal exceeds the available internal balance; a missing
  balance is treated as zero.
- **Likely cause:** The request is too large, no deposit is recorded, or some
  balance was moved to the locked bucket.
- **Caller/developer action:** Refresh `get_balance(user)`, cap the request to
  that value, and explain that locked funds are unavailable.

### `InsufficientBalanceToLock` (Code: 4002)

- **Current failure:** Returns `ContractError::InsufficientBalanceToLock` from `lock_funds`.
- **Meaning:** The lock amount exceeds the available internal balance.
- **Likely cause:** A stale displayed balance, an excessive request, or funds
  already moved to the locked bucket.
- **Caller/developer action:** Refresh `get_balance(user)` and allow no more than
  the returned available amount.

### `FundsLockedUntilMaturity` (Code: 4003)

- **Current failure:** Returns `ContractError::FundsLockedUntilMaturity` from `withdraw`.
- **Meaning:** The withdrawal amount exceeds the available balance and would
  require withdrawing from immature (unmatured) locked funds. This is a specific
  error that occurs when the user has locked funds that have not yet reached
  their unlock time.
- **Likely cause:** The user attempted to withdraw more than their available
  (unlocked) balance, and the shortfall would need to come from locked funds
  that are still immature (current_time < unlock_time).
- **Caller/developer action:** Check `get_balance(user)` to see available funds
  and `get_locked_balance(user)` to see locked funds. Only withdraw up to the
  available balance. Wait for locks to mature (check with `can_withdraw(user)`)
  before attempting to withdraw locked funds.

## Authorization errors (3000-3999)

### `Unauthorized` (Code: 3001)

- **Current failure:** Soroban host authorization failure from `require_auth()`;
  the contract defines this error for documentation purposes, but the actual
  failure comes from the Soroban host.
- **Meaning:** Valid authorization for the required address is absent.
- **Likely cause:** `initialize` lacks `admin` authorization, or `deposit`,
  `withdraw`, or `lock_funds` lacks `user` authorization. The app may be trying
  to act for another address.
- **Caller/developer action:** Build and sign with the required address. Do not
  retry unchanged; request the correct wallet signature.

Read-only calls (`get_balance`, `get_locked_balance`, and `can_withdraw`) do not
call `require_auth()`.

## Lock and unlock time behavior

### Locked funds are not yet withdrawable

- **Current condition:** `can_withdraw(user)` returns `false`; it does not fail.
- **Meaning:** No locked funds exist, or the ledger timestamp is earlier than
  the unlock time. At exactly the unlock timestamp it returns `true`.
- **Likely cause:** The lock has not matured or no lock exists.
- **Caller/developer action:** Treat `false` as normal state and disable the
  action. The current contract has no operation to release or withdraw locked
  funds; `can_withdraw` is only a query.

## Lock errors (5000-5999)

### `LockNotFound` (Code: 5001)

- **Current failure:** Reserved for future use.
- **Meaning:** No lock found for the specified lock ID.
- **Likely cause:** Invalid lock ID or lock has been consumed.
- **Caller/developer action:** Verify lock ID and check lock status.

## Other failure conditions

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

## Error Code Stability

All error codes defined in the `ContractError` enum are stable and backward compatible:

- Existing error codes will never change
- New error codes will be added within their category ranges
- Deprecated error codes will be marked in documentation but remain functional
- See [error-code-standard.md](./error-code-standard.md) for the complete standard

## SDK Integration

For SDK mapping guidance and mobile UX recommendations, see
[error-code-standard.md](./error-code-standard.md).
