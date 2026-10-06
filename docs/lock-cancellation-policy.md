# Lock Cancellation Policy

## Decision

Savings-vault locks are **not cancellable**. Once `lock_funds` succeeds, the
principal remains committed to that lock until its recorded `unlock_time`.
Users may extend a lock to a later time, but neither users nor administrators
can shorten or cancel it.

The public `cancel_lock(user, lock_id)` entrypoint exists to make this policy
machine-readable for SDKs and UIs. It does not release funds. After validating
the caller and lock state it fails with
`ContractError::LockCancellationUnsupported` (`5005`).

## Authorization and state checks

`cancel_lock` applies the same identity boundary as other owner-controlled lock
operations:

1. the vault must be initialized and on a supported storage version;
2. `user.require_auth()` must succeed;
3. `DataKey::Lock(user, lock_id)` must exist;
4. the lock must not already be withdrawn; and
5. a valid active or matured lock is rejected with error `5005`.

These checks are intentionally ordered so an unauthorised caller cannot use the
method as an ownership oracle, and stale clients still receive the existing
`LockNotFound` / `LockAlreadyWithdrawn` errors where appropriate.

There is no administrator override. An admin cannot cancel another user's lock,
and the emergency-pause mechanism does not create a cancellation escape hatch.

## State and custody invariants

A failed cancellation changes nothing:

- the lock amount, owner, maturity time, and `withdrawn` flag are unchanged;
- the user's available and locked balances are unchanged;
- the vault and user SAC token balances are unchanged; and
- no cancellation event is emitted.

After maturity, the supported release path remains
`withdraw_lock(user, lock_id)`. Maturity does not convert cancellation into an
alternate withdrawal operation.

## Savings guarantee

Allowing cancellation before maturity would weaken the central savings
guarantee: a user (or compromised user key) could undo a commitment immediately
after creating it. An administrator override would be worse because compromise
of the admin key could release third-party savings.

Fail-closed cancellation keeps the guarantee simple: the earliest release time
is the lock's recorded `unlock_time`. The only operation that can change that
time is `extend_lock`, which moves it later.

## Client guidance

SDKs and applications should expose cancellation as unsupported rather than as a
retryable failure. Map error `5005` to stable copy such as:

> This savings lock cannot be cancelled. It can be withdrawn after maturity.

For matured locks, refresh the lock and use `withdraw_lock`; do not retry
`cancel_lock`.

See also [Threat Model](threat-model.md) and [Error Codes](error-codes.md).
