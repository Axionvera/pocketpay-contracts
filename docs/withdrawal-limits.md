# Withdrawal limits

Savings Vault users may configure an optional **per-transaction withdrawal
ceiling** with `set_withdrawal_limit(user, max_amount)`.

- `0` disables the ceiling.
- A negative value is rejected with `WithdrawalLimitNegative (1008)`.
- An enabled ceiling applies to both `withdraw` and matured
  `withdraw_lock` calls.
- An amount above the ceiling is rejected with
  `WithdrawalLimitExceeded (4003)` before token transfer or accounting
  mutation.

## Why the limit is user-controlled

The vault documents withdrawals as an exit path that remains available even
during an emergency pause. An administrator-controlled ceiling could strand
existing principal, especially because a matured lock is withdrawn
all-or-nothing by lock ID.

The ceiling is therefore stored per user and can only be changed by that user.
If a matured lock is larger than the current ceiling, its owner can raise the
ceiling or set it to `0`, then withdraw the lock normally. The vault admin
cannot use this setting to reduce another user's ability to exit.

## Security boundary

This feature is a **self-imposed transaction guard**. It can help applications
and users catch an unexpectedly large withdrawal before submission, but it is
not a spending allowance, rate limit, or protection against compromise of the
user's signing key: the same authenticated user can change or disable the
ceiling.

Existing withdrawal rules still apply:

- amounts must be positive;
- plain withdrawals cannot exceed the unlocked balance;
- lock withdrawals require the authenticated owner, an existing matured lock,
  and a lock that has not already been withdrawn;
- replay protection remains the lock's `withdrawn` state; and
- token transfer succeeds before accounting state is committed.

The ceiling does not split locks or create partial lock withdrawals.
