# Read Models — Balance Snapshot & Lock Summary

> Issue: #413 — Read models for user balance snapshots and lock summaries

## Overview

SDK and mobile clients need efficient ways to display a user's financial state
(unlocked, locked, total, and withdrawable balances) and a quick summary of
their lock portfolio (counts, amounts, maturity status, unlock-time window).

Two new read-only contract functions expose this information in a single call
each, removing the need for clients to issue multiple RPC queries and compute
derived values off-chain.

## New Contract Functions

### `get_balance_snapshot(user: Address) → BalanceSnapshot`

Returns a point-in-time snapshot of the user's balance state.

| Field          | Type   | Description                                             |
|----------------|--------|---------------------------------------------------------|
| `unlocked`     | `i128` | Available deposited balance, withdrawable via `withdraw` |
| `locked`       | `i128` | Sum of all non-withdrawn lock amounts                   |
| `total`        | `i128` | `unlocked + locked`                                     |
| `withdrawable` | `i128` | Sum of matured, non-withdrawn lock amounts              |

**Example SDK call:**

```typescript
const snap = await contract.call("get_balance_snapshot", userAddress);
// snap.unlocked  → display as "Available"
// snap.locked    → display as "Locked"
// snap.total     → display as "Total Balance"
// snap.withdrawable → display as "Ready to Withdraw"
```

### `get_lock_summary(user: Address) → LockSummary`

Returns an aggregated summary of the user's lock entries.

| Field                 | Type   | Description                                           |
|-----------------------|--------|-------------------------------------------------------|
| `active_count`        | `u32`  | Number of non-withdrawn locks                         |
| `total_locked_amount` | `i128` | Sum of amounts across all non-withdrawn locks         |
| `matured_count`       | `u32`  | Number of matured, non-withdrawn locks                |
| `withdrawable_amount` | `i128` | Sum of amounts across matured, non-withdrawn locks    |
| `earliest_unlock`     | `u64`  | Smallest unlock time among immature locks (0 if none) |
| `latest_unlock`       | `u64`  | Largest unlock time among immature locks (0 if none)  |

**Example SDK call:**

```typescript
const summary = await contract.call("get_lock_summary", userAddress);
// summary.active_count       → "2 active locks"
// summary.matured_count      → "1 ready to withdraw"
// summary.earliest_unlock    → "Next unlock: Jan 15"
// summary.latest_unlock      → "Last unlock: Mar 30"
```

## Authorization

Both functions are **read-only** — no authorization is required. Any caller can
query any user's balance snapshot or lock summary.

## Storage Iteration & Pagination Limitations

Both `get_balance_snapshot` and `get_lock_summary` perform a linear scan of all
lock IDs `1..next_lock_id` for the queried user. This means:

- **On-chain cost scales linearly** with the total number of locks ever created
  for that user (including withdrawn locks whose storage entries persist).
- For users with a **small to moderate number of locks** (< 100), this is
  efficient and practical for on-chain queries.
- For users with a **very large number of historical locks** (hundreds or
  thousands), the on-chain read cost may become expensive. In such cases:
  - Use the paginated `list_locks(user, offset, limit)` function to fetch locks
    in batches and compute summaries off-chain.
  - Consider building an off-chain indexer that subscribes to lock/withdraw
    events and maintains a pre-computed summary.

### What Must Be Computed Off-Chain

If on-chain iteration is too expensive for your use case:

1. **Subscribe to events:** `lock`, `withdraw_lock`, `extend_lock`, `deposit`,
   `withdraw` events contain all state changes needed to maintain balances.
2. **Maintain a local database** of lock entries per user, updated from events.
3. **Compute** `BalanceSnapshot` and `LockSummary` equivalents from the local
   copy — the formulas match exactly what the on-chain functions do.

### Page Size Limit

The existing `list_locks` function enforces a maximum page size of **50 entries**
per call (`MAX_LOCK_PAGE_SIZE`). To iterate all locks for a user with more than
50, issue multiple calls with increasing `offset`.

## Mobile Integration Notes

- Both read functions return Soroban custom types (`BalanceSnapshot`,
  `LockSummary`). The Soroban SDK for your platform (JS, Kotlin, Swift) will
  decode these as structured objects.
- Timestamps (`earliest_unlock`, `latest_unlock`) are Unix seconds. Convert to
  local time for display.
- `withdrawable` in `BalanceSnapshot` and `withdrawable_amount` in `LockSummary`
  represent the same aggregate — the sum of matured lock amounts. Use whichever
  response fits your screen's data needs.
- A user with `active_count == 0` and `unlocked == 0` has no funds in the vault.
- A user with `withdrawable > 0` has matured locks ready to be released via
  individual `withdraw_lock(user, lock_id)` calls.

## Test Coverage

| Scenario                      | `balance_snapshot` | `lock_summary` |
|-------------------------------|:------------------:|:--------------:|
| Empty / new user              | ✓                  | ✓              |
| Deposit only (no locks)       | ✓                  | ✓              |
| Single active lock            | ✓                  | ✓              |
| Matured lock                  | ✓                  | ✓              |
| Mixed matured + immature      | ✓                  | ✓              |
| After withdrawing a lock      | ✓                  | ✓              |
| Cross-user isolation          | ✓                  | ✓              |
| All locks matured             | —                  | ✓              |
| All locks withdrawn           | —                  | ✓              |
| Consistency with existing API | ✓                  | —              |
| Uninitialized contract        | ✓                  | ✓              |
