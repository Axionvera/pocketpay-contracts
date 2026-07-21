# Balance Reconciliation Design Note

This document describes how the savings vault's internal accounting should
reconcile with real token balances once SAC-backed token transfers are
implemented. It covers the current accounting model, the target reconciliation
model, failure modes, and the invariants that tests must enforce.

This is a design note, not a description of implemented behavior. See
[docs/architecture.md](architecture.md) for the current state and
[README.md](../README.md#known-limitations) for the known custody limitation.

---

## Background

The savings vault maintains two conceptually distinct balance representations:

- **Internal balance** — a number stored in the contract's persistent storage
  under `DataKey::Balance(user)`. It is updated by `deposit`, `withdraw`, and
  `lock_funds` calls. It exists entirely within the contract and is not
  connected to any on-chain asset transfer.

- **Token-backed balance** — the amount of a real Stellar Asset Contract (SAC)
  token actually held at the vault's contract address. This is the on-chain
  asset custody layer. It can be queried by calling `balance` on the token
  contract with the vault's address.

Today the two representations are decoupled. `deposit` writes to internal
storage but does not call `token_client.transfer`. `withdraw` does call
`token_client.transfer`, but only because a real SAC token is configured in
tests to simulate the future behavior. In production today there is no
guarantee that a user's internal balance is backed by tokens actually held at
the vault address.

Once real SAC integration is complete, every internal balance increment must
correspond to an inbound token transfer, and every internal balance decrement
must correspond to an outbound transfer. The sections below define what that
alignment must look like and what can go wrong.

---

## Internal Versus Token-Backed Balances

### Internal accounting model

The contract tracks each user's funds across two storage keys:

| Storage key              | Meaning                                              |
|--------------------------|------------------------------------------------------|
| `Balance(user)`          | Available (unlocked) funds credited to the user      |
| `Locks(user)`            | Vector of `LockEntry` values; each holds `amount` and `unlock_time` |

The sum of a user's available balance plus all their lock amounts equals the
total funds the contract believes it owes that user:

```
internal_total(user) = Balance(user) + sum(lock.amount for lock in Locks(user))
```

`get_balance(user)` returns `Balance(user)` plus any lock amounts where
`current_time >= lock.unlock_time` (matured locks). `get_locked_balance(user)`
returns the sum of lock amounts where `current_time < lock.unlock_time`
(unmatured locks). Together they always equal `internal_total(user)`.

### Token-backed model (target state)

When SAC integration is complete, the vault contract address must hold real
tokens equal to the sum of all users' internal totals:

```
token_contract.balance(vault_address) == sum(internal_total(user) for all users)
```

This is the **aggregate reconciliation invariant**. It must hold after every
successful state-changing operation.

Per-user reconciliation is not directly verifiable on-chain (the token contract
tracks the vault's aggregate balance, not individual user shares), but it holds
by construction if every deposit increases the vault's token balance by exactly
the deposited amount and every withdrawal decreases it by exactly the withdrawn
amount.

### Current gap

Until `deposit` calls `token_client.transfer` to pull tokens from the user into
the vault, the internal balance will exceed the vault's real token balance. The
current tests for `withdraw` paper over this gap by having test setup code
transfer tokens into the vault separately before invoking `withdraw`. That
pattern must be replaced by a `deposit` implementation that performs the
transfer atomically.

---

## Reconciliation Assumptions

The following assumptions must hold for the reconciliation invariant to be
maintained once SAC integration is implemented.

1. **Atomic deposit**: `deposit` must call `token_client.transfer(user, vault,
   amount)` and update `Balance(user)` in the same invocation. If the transfer
   fails, the internal balance must not be updated. If the internal balance
   update fails after a successful transfer, the tokens are stranded; see
   [Failure modes](#failure-modes).

2. **Atomic withdrawal**: `withdraw` already calls `token_client.transfer(vault,
   user, amount)` before updating internal storage. This ordering means a token
   transfer failure prevents any accounting change, which is safe. The reverse
   (accounting change before transfer) would be dangerous because a subsequent
   transfer failure would leave the internal balance decremented while the user
   received nothing.

3. **No out-of-band transfers**: No path other than `deposit` should increase
   the vault's token balance, and no path other than `withdraw` should decrease
   it. Out-of-band transfers (e.g. sending tokens directly to the vault address)
   create surplus that cannot be attributed to any user and are irrecoverable
   without an admin sweep mechanism.

4. **Single token per vault**: The contract stores one token address set at
   initialization. Reconciliation assumes all deposits and withdrawals use that
   single token. Mixed-token operations would break the aggregate invariant.

5. **No token contract re-entrancy**: The vault does not expect the token
   contract to call back into the vault during a transfer. If the configured
   token has hooks that do so, accounting may be corrupted. The token address
   should be a standard SAC with no custom hooks.

---

## Failure Modes

The following scenarios can cause internal accounting to diverge from the real
token balance.

### FM-1: Deposit without token transfer (current state)

**Scenario**: `deposit` increments `Balance(user)` but does not call
`token_client.transfer`.

**Effect**: `internal_total` exceeds the vault's real token balance by the
deposited amount. A subsequent `withdraw` will attempt a token transfer that
may fail because the vault does not hold enough tokens.

**Detection**: Compare `sum(internal_total(user))` against
`token_contract.balance(vault_address)`. Any positive difference indicates
unmatched deposits.

**Resolution**: Add the `token_client.transfer(user, vault, amount)` call to
`deposit` before the storage write.

### FM-2: Token transfer succeeds but storage write fails (partial deposit)

**Scenario**: During `deposit`, the SAC transfer completes but the subsequent
`env.storage().persistent().set(...)` panics (e.g. due to a storage limit or
host error).

**Effect**: Tokens are in the vault but the user's `Balance` was not
incremented. The vault's real balance exceeds `internal_total`. The user has
lost access to their funds even though the vault holds them.

**Detection**: `token_contract.balance(vault_address)` exceeds
`sum(internal_total(user))`.

**Resolution**: Because Soroban contract invocations are atomic, a panic rolls
back all state changes in that invocation, but the token transfer is itself a
sub-invocation. Whether it can be rolled back depends on the host's cross-call
atomicity guarantees. This must be verified against Soroban host semantics
before implementing SAC integration. If sub-invocation atomicity is not
guaranteed, a two-phase approach or an idempotency key pattern may be needed.

### FM-3: Withdrawal token transfer fails after balance check

**Scenario**: `withdraw` confirms the user has sufficient internal balance, then
calls `token_client.transfer(vault, user, amount)`, which fails (e.g. vault
token balance is lower than expected due to FM-1 or FM-3 on a prior call).

**Effect**: The invocation panics (propagated from the token contract). Because
the panic occurs before any storage write in `withdraw`, neither `Balance(user)`
nor `Locks(user)` is modified. The user's internal balance is unchanged and
their funds remain credited. No double-spend occurs.

**Visible symptom**: The transaction fails with a token-contract error. The
error-codes reference documents this under "Token transfer failure during
withdrawal".

**Resolution**: Ensure the vault always holds tokens at least equal to
`sum(internal_total(user))` by fixing FM-1.

### FM-4: Out-of-band token deposit creates irrecoverable surplus

**Scenario**: Someone sends tokens directly to the vault address via the SAC
`transfer` function without calling `vault.deposit`.

**Effect**: `token_contract.balance(vault_address)` exceeds
`sum(internal_total(user))`. The surplus tokens are held by the vault but not
credited to any user. They cannot be withdrawn through normal vault operations.

**Detection**: Same comparison as FM-1: a positive difference between real
balance and internal total, but in this case internal accounting was never
incremented at all.

**Resolution**: An admin sweep function or a surplus attribution mechanism
would be needed. Neither exists today. Document and reject out-of-band deposits
in user-facing SDKs.

### FM-5: Storage TTL expiry silently zeroes a balance

**Scenario**: The persistent storage entry for `Balance(user)` or `Locks(user)`
expires (TTL lapses). The `unwrap_or(0)` default means the contract sees a
zero balance.

**Effect**: The user's internal balance appears to be zero even though tokens
may be held at the vault address. `get_balance` returns `0`; `withdraw` fails
with "Insufficient balance". No tokens are transferred out; the tokens remain
stranded in the vault.

**Detection**: The real token balance exceeds internal totals. An expired
storage entry can be confirmed via RPC. See [docs/storage-ttl.md](storage-ttl.md).

**Resolution**: Restore the expired persistent entry (Soroban supports
restoration of archived entries) and extend TTL. This underscores that TTL
management is a custody-safety requirement, not just an operational concern.

---

## Invariants That Tests Must Enforce

The following invariants should be covered by automated tests. Some are already
tested; the rest are targets for the SAC-integration test suite.

### I-1: Balance conservation (currently tested)

```
get_balance(user) + get_locked_balance(user) == net_deposited(user)
```

where `net_deposited = sum(successful deposits) - sum(successful withdrawals)`.

This invariant must hold after every operation in any sequence of deposits,
withdrawals, locks, and time advances. It is tested extensively in
`src/test/balance_conservation.rs`.

### I-2: Non-negativity (currently tested)

```
get_balance(user) >= 0
get_locked_balance(user) >= 0
```

Both values must never be negative. Tested within the `assert_conserved` helper
in `balance_conservation.rs`.

### I-3: Failed operations do not mutate state (currently tested)

Any operation that panics (invalid amount, insufficient balance, past unlock
time) must leave `get_balance(user)` and `get_locked_balance(user)` unchanged.
Tested across `conservation_invalid_deposits_do_not_mutate`,
`conservation_invalid_withdrawals_do_not_mutate`,
`conservation_invalid_locks_do_not_mutate`, and their companion panic tests.

### I-4: Aggregate token backing (target, not yet tested)

Once SAC integration is implemented:

```
token_contract.balance(vault_address) == sum(get_balance(u) + get_locked_balance(u)
                                             for all users u)
```

This must be asserted after every deposit and withdrawal in integration tests.
Tests that currently transfer tokens into the vault manually outside of
`deposit` must be updated to exercise the integrated deposit path.

### I-5: Per-deposit token correspondence (target, not yet tested)

For a single deposit of amount `A` by user `u`:

```
delta(token_contract.balance(vault_address)) == A
delta(get_balance(u) + get_locked_balance(u)) == A
```

Both deltas must equal the deposit amount. The token balance and the internal
balance must increase by the same amount in the same invocation.

### I-6: Per-withdrawal token correspondence (target, not yet tested)

For a single withdrawal of amount `A` by user `u`:

```
delta(token_contract.balance(vault_address)) == -A
delta(get_balance(u) + get_locked_balance(u)) == -A
delta(token_contract.balance(user_address)) == +A
```

Tokens must leave the vault, the internal balance must decrease, and the user's
wallet balance must increase, all by the same amount.

### I-7: Lock operations do not change token balance (target, not yet tested)

`lock_funds` moves funds between the available and locked buckets but does not
transfer any tokens. The vault's real token balance must be unchanged after a
lock operation:

```
delta(token_contract.balance(vault_address)) == 0  after lock_funds(user, amount, unlock_time)
```

### I-8: User balance isolation (currently partially tested)

An operation by user A must not change the balance of user B. Covered by
`balance_isolation_between_users_*` snapshot tests. This must also be verified
at the token level once integration is complete: a deposit by user A must not
increase the apparent entitlement of user B.

---

## Navigation

- [Architecture Documentation](architecture.md) — state model, storage keys, and the internal-vs-custody distinction
- [Contract Error Reference](error-codes.md) — all current failure modes including token transfer failures
- [Storage TTL Guide](storage-ttl.md) — TTL risk, expiry consequences, and how to extend storage
- [Sample Vault Interaction Walkthrough](walkthrough.md) — end-to-end example with current limitations noted
