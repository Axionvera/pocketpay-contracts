# Vault ownership model

This document makes the Savings Vault ownership rules explicit for auditors,
SDK authors, and wallet integrations. It describes the contract as implemented
on current `main`; it does not introduce a new role or storage scheme.

The core rule is simple: **the user address is both the authorization identity
for user mutations and the namespace for that user's vault state**. Ownership
is an authority boundary, not a confidentiality boundary: read helpers are
public, while writes require the affected user's Soroban authorization.

## Ownership at a glance

| State or capability | Owner / authority | Representation | Enforcement |
| --- | --- | --- | --- |
| Available balance | User address | `DataKey::Balance(user)` | User-authenticated mutations |
| Individual lock | User address | `DataKey::Lock(user, lock_id)` plus `LockEntry.owner` | User-authenticated lock mutations and address-keyed lookup |
| Lock ID sequence | User address | `DataKey::NextLockId(user)` | IDs allocated within the user's namespace |
| Accepted token | Contract configuration | `DataKey::Token` | Set by initialization/configuration rules |
| Admin role | Configured admin address | `DataKey::Admin` | Admin authentication for admin-only operations |
| Read helpers | Public | Address supplied by caller | No authorization; read-only |

The admin role does **not** own user balances or locks and does not provide an
override for user withdrawal authorization.

## User identity

The contract uses Soroban `Address` values as user identifiers. A user-facing
mutation takes the affected `user: Address` as an explicit argument and calls
`user.require_auth()`.

Examples include:

- `deposit(user, amount)`
- `withdraw(user, amount)`
- `lock_funds(user, amount, unlock_time)`
- `withdraw_lock(user, lock_id)`
- `extend_lock(user, lock_id, new_unlock_time)`

This binds authorization to the same address used to select storage. A caller
cannot choose Alice's storage key while authorizing only as Bob: the operation
requires Alice's authorization when `user == Alice`.

The contract does not use a separate application-level user ID. Wallets and SDKs
should therefore treat the Stellar/Soroban address passed to the contract as the
canonical on-chain ownership identifier.

## Balance ownership

Available, unlocked principal is stored under:

```rust
DataKey::Balance(user)
```

A successful deposit:

1. requires `user.require_auth()`;
2. transfers tokens from `user` to the vault contract;
3. reads `Balance(user)`;
4. credits only that same `Balance(user)` entry.

A standard withdrawal follows the same identity boundary in reverse:

1. requires `user.require_auth()`;
2. reads only `Balance(user)`;
3. rejects an amount greater than that available balance;
4. transfers withdrawn tokens from the contract back to `user`.

There is no global pooled accounting key used to decide an individual user's
available balance. The contract may physically custody fungible tokens at one
contract address, but the accounting claim is partitioned by user address.

### Balance invariant

For ownership purposes, an address's vault principal is the combination of:

- its available balance at `Balance(user)`; and
- the non-withdrawn lock records in that same user's lock namespace.

Moving funds into a lock reduces the same user's available balance and creates a
lock under the same address namespace. It does not transfer ownership to the
admin, the contract operator, or another user.

## Lock ownership

Each independent lock is stored under the composite persistent-storage key:

```rust
DataKey::Lock(user, lock_id)
```

and the stored value repeats the owner explicitly:

```rust
LockEntry {
    id: lock_id,
    owner: user,
    amount,
    created_time,
    unlock_time,
    withdrawn,
}
```

This gives the model two useful audit signals:

1. the storage namespace identifies the owner; and
2. `LockEntry.owner` records the same owner in the value returned to clients.

New lock IDs are allocated from `DataKey::NextLockId(user)`. IDs are therefore
stable within a user's namespace, not globally unique by themselves. Integrators
must identify a lock by **(owner address, lock ID)** rather than by lock ID
alone.

### Lock creation

`lock_funds(user, ...)` requires the user's authorization, debits
`Balance(user)`, writes `Lock(user, next_id)`, and stores
`LockEntry.owner = user`.

### Lock withdrawal

`withdraw_lock(user, lock_id)`:

- requires `user.require_auth()`;
- loads exactly `Lock(user, lock_id)`;
- enforces maturity and replay protection;
- transfers the lock's tokens back to that same `user`;
- writes the withdrawn state back to the same composite key.

Supplying Bob's address with Alice's lock ID looks in Bob's namespace, not
Alice's. Supplying Alice's address requires Alice's authorization.

## Cross-user restrictions

The ownership model prevents cross-user state mutation through the combination
of address-keyed storage and Soroban host authorization.

### A user cannot spend another user's available balance

A call targeting `Balance(alice)` must use `user = alice`, and user mutations
require Alice's authorization. Bob's authorization alone is insufficient.

### A user cannot mutate another user's lock

Lock mutation looks up `Lock(user, lock_id)` under the authenticated user's
address namespace. Identical numeric lock IDs owned by different users are
different storage keys.

### The admin cannot withdraw user funds

Administrative authority is separate from vault ownership. Admin
authentication governs admin-specific controls such as admin rotation and
pause management. User withdrawals continue to require the user's
authorization; there is no admin sweep or proxy-withdraw path.

### Reads are intentionally public

Helpers such as `get_balance(user)`, `get_lock(user, lock_id)`,
`list_locks(user, ...)`, and balance/lock summaries do not establish authority
and do not require user authentication. This is appropriate for on-chain state:
ownership controls mutation, not visibility.

Applications must not interpret successful reads as proof that the caller owns
the returned state. If an SDK needs to make an ownership-sensitive decision, it
must retain the owner address alongside the data and rely on contract
authorization for the eventual mutation.

## SDK and wallet integration rules

Integrations should preserve these ownership semantics:

- Key cached balance data by the full owner address.
- Key lock data by the pair `(ownerAddress, lockId)`.
- Never treat a numeric lock ID as globally unique.
- Keep the owner address returned in a `LockEntry` when presenting or caching
  the lock off-chain.
- Build mutating calls with the same address that is expected to authorize the
  operation.
- Do not model the admin as a custodian who can move individual user funds.
- Treat read helpers as public query APIs rather than authenticated account
  sessions.

For multi-account wallets, changing the selected account must also change the
address used for balance queries, lock queries, and subsequent mutations.

## Audit checklist

When changing vault storage or user-facing mutations, verify all of the
following:

- [ ] User-owned state remains namespaced by the affected `Address`.
- [ ] A mutation touching user-owned state requires the affected user's auth.
- [ ] A lock's storage-key owner and `LockEntry.owner` remain consistent.
- [ ] Lock IDs are interpreted together with the owner address.
- [ ] Cross-user tests still prove that one account cannot mutate another's
      balance or locks.
- [ ] Admin changes do not gain an implicit user-fund override.
- [ ] Read-only helpers remain clearly separated from mutation authority.

## Related documentation

- [Authorisation Rules & Security Matrix](./authorisation-rules.md)
- [Multi-Lock Storage Model & Architecture](./multi-lock-storage.md)
- [Vault Custody Assumptions](./vault-custody-assumptions.md)
- [Contributor Security Checklist](./security-checklist.md)
- [Storage Audit](./storage-audit.md)
