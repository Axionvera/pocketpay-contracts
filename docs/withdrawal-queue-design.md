# Withdrawal Queue Design Note

> **Status:** Design / Out of Current Scope
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This document explores the design of a withdrawal queue model for possible
> future delayed-withdrawal support. It is a design artifact — **no withdrawal
> queue is implemented as part of this document and current withdrawal behaviour
> is unchanged.**

---

## Table of Contents

1. [Motivation](#motivation)
2. [Current Withdrawal Behaviour](#current-withdrawal-behaviour)
3. [What Is a Withdrawal Queue?](#what-is-a-withdrawal-queue)
4. [Pending Withdrawal State](#pending-withdrawal-state)
5. [Queue Identifiers](#queue-identifiers)
6. [Cancellation](#cancellation)
7. [Maturity and Execution](#maturity-and-execution)
8. [Storage Implications](#storage-implications)
9. [Accounting Implications](#accounting-implications)
10. [Interaction with Existing Features](#interaction-with-existing-features)
11. [Scope Decision](#scope-decision)
12. [Open Questions](#open-questions)

---

## Motivation

Future vault versions may need delayed withdrawals for scenarios such as:

- **Mandatory review windows** — regulators or app policies may require a
  cooling-off period between a withdrawal request and final execution.
- **Anti-rug protection** — a brief delay gives on-chain observers or a
  governance process a window to flag suspicious mass withdrawals.
- **Scheduled/planned withdrawals** — users may wish to schedule a withdrawal
  at a future time (e.g. salary payout, recurring DeFi harvest).
- **Layered security** — a time-delay between request and execution limits the
  damage an attacker can cause with a compromised signing key; the legitimate
  user has time to cancel before funds move.

The current vault already models time-locked funds (see `lock_funds` and
`LockEntry`). A withdrawal queue is a complementary concept: rather than
locking funds *in* the vault for a period, it represents a *pending exit* that
has been requested but not yet executed.

---

## Current Withdrawal Behaviour

Today `withdraw(user, amount)` and `withdraw_lock(user, lock_id)` are
synchronous and immediate:

1. Authorization is verified (`user.require_auth()`).
2. The available balance is checked.
3. `token_client.transfer` moves tokens from the vault contract to the user in
   the same transaction.
4. Internal storage (`Balance`, `Lock`) is updated atomically.

There is no intermediate "pending" state: a withdrawal either succeeds fully or
reverts entirely. This document does **not** propose changing this behaviour.
Any future queue mechanism must be introduced as an additive opt-in feature,
not a replacement for the existing direct-withdrawal path.

---

## What Is a Withdrawal Queue?

A withdrawal queue introduces a two-phase withdrawal lifecycle:

```
Phase 1 — Request:      user calls queue_withdrawal(amount)
                        → a WithdrawalRequest entry is created
                        → funds are reserved (deducted from available balance)
                        → a maturity timestamp is set
                        → a queue ID is returned

Phase 2 — Execution:    after maturity, user calls execute_withdrawal(queue_id)
                        → token transfer occurs
                        → the request record is removed or marked complete
```

Between the two phases the funds exist in a **pending** state: they are no
longer in the user's available balance, but they have not yet left the vault.

---

## Pending Withdrawal State

A pending withdrawal entry holds the information needed to track and eventually
execute (or cancel) the withdrawal.

### Proposed `WithdrawalRequest` struct

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawalRequest {
    /// Unique identifier for this request, scoped to the requesting user.
    pub id: u64,
    /// The address that created this request and will receive the funds.
    pub owner: Address,
    /// The amount reserved for this withdrawal (in token base units).
    pub amount: i128,
    /// Ledger timestamp when this request was submitted.
    pub created_time: u64,
    /// Earliest ledger timestamp at which `execute_withdrawal` may be called.
    pub maturity_time: u64,
    /// Whether this request has been executed or cancelled.
    /// A completed/cancelled request is kept for auditability until pruned.
    pub status: WithdrawalStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WithdrawalStatus {
    /// Request submitted, awaiting maturity.
    Pending,
    /// Maturity reached and token transfer completed.
    Executed,
    /// Cancelled by the user before execution; funds returned to balance.
    Cancelled,
}
```

### State transition diagram

```
                queue_withdrawal(amount, delay)
                         │
                         ▼
                   ┌──────────┐
                   │ PENDING  │
                   └──────────┘
                    /         \
  cancel_withdrawal /           \ execute_withdrawal
  (before maturity)/             \ (after maturity)
                  /               \
                 ▼                 ▼
          ┌────────────┐    ┌──────────────┐
          │ CANCELLED  │    │   EXECUTED   │
          └────────────┘    └──────────────┘
```

| Status | Meaning | Funds location |
|--------|---------|----------------|
| `Pending` | Request created; delay window active | Reserved in vault (not in `Balance(user)`) |
| `Executed` | Token transfer completed | Sent to user's wallet |
| `Cancelled` | User cancelled before maturity | Returned to `Balance(user)` |

---

## Queue Identifiers

Each `WithdrawalRequest` must be uniquely addressable so callers can poll its
status, cancel it, or execute it.

### Proposed approach: per-user monotonic counter

This mirrors the existing `NextLockId(Address)` pattern:

| Storage key | Type | Purpose |
|-------------|------|---------|
| `NextQueueId(Address)` | `u64` | Monotonically increasing counter; generates unique IDs per user |
| `WithdrawalReq(Address, u64)` | `WithdrawalRequest` | Individual request keyed by `(owner, id)` |

**Advantages:**
- Consistent with `NextLockId` / `Lock(Address, u64)` — minimal new patterns.
- IDs are stable and predictable (1, 2, 3 …).
- Easy to paginate (iterate from 1 to `NextQueueId - 1`).

**Alternative: global sequential ID**

A single contract-wide `NextQueueId: u64` counter and `WithdrawalReq(u64)`
would give globally unique IDs at the cost of a single additional instance
storage key. This is useful if the queue is to be indexed globally (e.g. for
admin monitoring), but adds contention risk in high-throughput scenarios.

The per-user approach is recommended for alignment with the existing lock model.

---

## Cancellation

Cancellation must:

1. Require `user.require_auth()` — only the owner may cancel their own request.
2. Verify the request exists and has `status == Pending`.
3. Verify maturity has **not** been reached (cancellation after maturity is
   ambiguous; it should be treated as execution instead).
4. Return the reserved `amount` to `Balance(user)`.
5. Set `status = Cancelled` (or remove the entry outright).

### Keeping vs pruning completed entries

Keeping cancelled and executed entries provides an on-chain audit trail at the
cost of persistent storage. Given Soroban's per-byte storage fees and the risk
of indefinite ledger growth, the recommended behaviour is to **mark entries as
`Cancelled` or `Executed` and allow them to be pruned by a separate admin or
user operation** (analogous to deleting fully-withdrawn locks). The prune
operation must be authorised by the owner.

---

## Maturity and Execution

`execute_withdrawal(user, queue_id)` is callable only when
`ledger.timestamp() >= maturity_time`. The execution path mirrors
`withdraw_lock`:

1. Load `WithdrawalRequest` from `WithdrawalReq(user, queue_id)`.
2. Assert `status == Pending`.
3. Assert `ledger.timestamp() >= maturity_time`.
4. Call `token_client.transfer(vault, user, amount)`.
5. Set `status = Executed` (and zero the amount to prevent replay).

The token transfer occurs before the storage update (same safe ordering as
`withdraw` and `withdraw_lock`) so that a transfer failure reverts the
transaction without any partial state change.

### Minimum and maximum delay bounds

To prevent abuse:

- A **minimum delay** (e.g. one ledger close, or a configurable `min_delay_secs`
  set at initialization) prevents queue entries from being used as a free
  "reserve then immediately withdraw" mechanism that bypasses normal withdrawal.
- A **maximum delay** (configurable, or set by governance) prevents funds from
  being locked in a pending state indefinitely if a user never executes.

These bounds should be stored in instance storage at initialization time:

| Storage key | Type | Suggested default |
|-------------|------|-------------------|
| `MinWithdrawalDelay` | `u64` (seconds) | `300` (5 minutes) |
| `MaxWithdrawalDelay` | `u64` (seconds) | `2_592_000` (30 days) |

---

## Storage Implications

Introducing a withdrawal queue requires the following new storage keys:

### Persistent storage (per-user)

| New key | Type | Description |
|---------|------|-------------|
| `NextQueueId(Address)` | `u64` | Monotonic counter for queue IDs per user |
| `WithdrawalReq(Address, u64)` | `WithdrawalRequest` | Individual pending/completed request |

### Instance storage (global config)

| New key | Type | Description |
|---------|------|-------------|
| `MinWithdrawalDelay` | `u64` | Minimum seconds between request and execution |
| `MaxWithdrawalDelay` | `u64` | Maximum seconds a request may remain pending |

### Storage version impact

Adding new storage keys is additive and does not break existing reads, but
requires a **storage version bump** from `v1` to `v2` and a corresponding
migration entry in `try_migrate`. The migration for v1 → v2 would be a no-op
data migration (no existing data needs to change; new keys simply start absent
and default to zero/absent). See [docs/storage-migration.md](storage-migration.md)
for the migration pattern to follow.

### TTL considerations

`WithdrawalRequest` entries stored in persistent storage are subject to the
same TTL risks as `LockEntry` values. Completed and cancelled requests should
be pruned promptly to limit ledger growth. See
[docs/storage-ttl.md](storage-ttl.md) for TTL management guidance.

### Balance invariant during the pending window

While a `WithdrawalRequest` is `Pending`, the reserved amount is removed from
`Balance(user)` but has not yet left the vault's token custody. The extended
token custody invariant becomes:

```
token_contract.balance(vault_address)
  == sum(Balance(u) for all users)
   + sum(LockEntry.amount for all non-withdrawn locks)
   + sum(WithdrawalRequest.amount for all Pending requests)
```

This ensures the vault continues to hold tokens corresponding to every user's
complete internal claim. See [docs/balance-reconciliation.md](balance-reconciliation.md)
for the base reconciliation invariants this extends.

---

## Accounting Implications

### Available balance

When `queue_withdrawal(amount)` is called:

```
Balance(user) -= amount          // funds reserved for the pending request
```

When `execute_withdrawal(queue_id)` completes:

```
// No change to Balance(user) — already deducted at request time
token_client.transfer(vault, user, amount)   // tokens leave vault custody
```

When `cancel_withdrawal(queue_id)` completes:

```
Balance(user) += amount          // reserved funds returned
```

### Accounting invariants (extended)

| Invariant | Expression |
|-----------|-----------|
| Balance non-negativity | `Balance(user) >= 0` at all times |
| Pending reservation | `sum(pending_requests.amount) == tokens reserved but not transferred` |
| Token custody | `vault_token_balance == sum(Balance) + sum(active_locks) + sum(pending_requests)` |
| No double-spend | A pending request amount cannot be locked, withdrawn, or queued again |

The no-double-spend invariant is enforced by deducting `amount` from
`Balance(user)` atomically when the request is created. The pending amount is
isolated in the `WithdrawalRequest` record and cannot be touched until
execution or cancellation.

### Impact on `get_balance`

`get_balance(user)` currently returns `Balance(user)` plus matured lock
amounts. With a queue, pending amounts are already excluded from `Balance(user)`
at creation time, so `get_balance` would not need to change. However, a new
helper such as `get_pending_withdrawals(user)` (or inclusion in `get_balance`
with a flag) should be added to make the reserved amount visible to callers.

### Impact on `get_locked_balance`

`get_locked_balance(user)` returns the sum of active (not-yet-matured) lock
amounts. Pending withdrawal requests are conceptually distinct from locks —
they represent funds *leaving* the vault rather than funds being held inside
it — so they should **not** be included in `get_locked_balance`. A separate
`get_pending_withdrawal_total(user)` helper is more appropriate.

### Fee consideration

If fees are introduced in a future version (see
[docs/vault-fee-model.md](vault-fee-model.md)), the fee must be deducted from
the pending amount at request time (not at execution time) to avoid
underpayment if token prices or fee rates change during the delay window. The
reserved amount should therefore be `amount + fee` at request time, with the
fee portion held separately in a fee accumulator key.

---

## Interaction with Existing Features

### Emergency pause

Under the current pause model, `deposit` and `lock_funds` are blocked; `withdraw`
and `withdraw_lock` remain open. If a withdrawal queue is added:

- `queue_withdrawal` (Phase 1 request) should be **blocked during pause** — no
  new reservations should be created while the contract is under an active pause
  incident.
- `execute_withdrawal` (Phase 2 execution) should remain **open during pause**,
  consistent with the existing user-exit policy. Pending requests that were
  created before the pause should be executable during the pause window.

This mirrors the pause/lock interaction: new lock operations are blocked on
pause, but matured locks remain withdrawable.

### Storage versioning and migration

Any implementation of the withdrawal queue must bump `STORAGE_VERSION` to `2`
and add a v1 → v2 migration branch in `try_migrate`. The migration is a no-op
data transformation (new keys start absent), but the version bump ensures all
deployed contracts go through the migration path before processing queue
operations. See [docs/storage-migration.md](storage-migration.md).

### Admin transfer

`transfer_admin` does not interact with the withdrawal queue. Pending requests
belong to individual users and are unaffected by admin key rotation.

---

## Scope Decision

**The withdrawal queue is out of scope for the current vault implementation.**

Reasons:

1. **Complexity**: A two-phase withdrawal lifecycle introduces new state,
   new error paths, new accounting invariants, and new test surface. Adding
   it prematurely increases the attack surface before the contract has been
   audited.
2. **No immediate requirement**: The existing synchronous `withdraw` and
   `withdraw_lock` paths satisfy current PocketPay testnet use cases.
3. **Additive design**: This document confirms the queue can be added in a
   future vault version without breaking existing withdrawal behaviour.
   Current users and integrations are not affected.

When the queue is eventually implemented, it must:

- Not change the behaviour of `withdraw` or `withdraw_lock`.
- Introduce a storage version bump (v1 → v2).
- Add `queue_withdrawal`, `execute_withdrawal`, `cancel_withdrawal`, and
  `get_pending_withdrawal_total` as new public functions.
- Extend the token custody invariant as described in
  [Accounting Implications](#accounting-implications).
- Include full test coverage for the new state transitions, cancellation,
  double-spend prevention, pause interaction, and TTL edge cases.

---

## Open Questions

| Question | Notes |
|----------|-------|
| Should the minimum delay be configurable at initialization or hard-coded? | Configurable is more flexible; hard-coded is simpler to audit |
| Should cancelled and executed requests be pruned automatically or by a separate user call? | Automatic pruning reduces storage growth but complicates gas estimation |
| Should `execute_withdrawal` be callable by anyone (e.g. a relayer) or only the request owner? | Owner-only is simpler and safer; relayer support enables automation |
| Should there be a global cap on pending requests per user? | A cap prevents storage exhaustion attacks but requires extra validation |
| How does the queue interact with a future fee model? | Fee must be reserved at request time — see [Accounting Implications](#accounting-implications) |
| Should `queue_withdrawal` accept a specific maturity timestamp or a delay in seconds? | A delay in seconds is simpler and avoids timestamp manipulation |

---

## Navigation

- [State Machine Documentation](state-machine.md) — Current withdrawal state transitions
- [Balance Reconciliation Design Note](balance-reconciliation.md) — Reconciliation invariants this document extends
- [Storage Audit](storage-audit.md) — Current storage keys; new keys would be added here
- [Storage Migration Guide](storage-migration.md) — How to implement the v1 → v2 storage version bump
- [Storage TTL Guide](storage-ttl.md) — TTL management for the new `WithdrawalRequest` entries
- [Vault Fee Model](vault-fee-model.md) — Fee accounting implications for queued withdrawals
- [Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md) — Pause behaviour with the queue
- [Failure Mode Catalogue](failure-mode-catalogue.md) — Failure modes that a queue implementation must cover
