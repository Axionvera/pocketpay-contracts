# Matured-Lock Discovery Strategy

> Issue #414 -- Design a matured-lock discovery and pagination strategy for clients.

## Problem

Mobile clients need to display which locks are ready to withdraw ("matured"),
but the contract's existing `list_locks` endpoint returns **all** locks
(active, matured, and withdrawn) without distinguishing state. This forces
clients to:

1. Fetch full lock pages,
2. Compare each `unlock_time` against the current ledger timestamp,
3. Filter out withdrawn entries.

For users with many locks, this creates unnecessary RPC overhead and pushes
complexity onto every client implementation.

---

## Chosen Approach: On-Chain Filtered Helpers

The contract now exposes three dedicated read-only helpers that perform
matured-lock filtering on-chain:

| Function | Signature | Returns |
|----------|-----------|---------|
| `list_matured_locks` | `(user, offset, limit) -> Vec<LockEntry>` | Paginated list of matured, non-withdrawn locks |
| `get_matured_lock_count` | `(user) -> u32` | Count of matured, non-withdrawn locks |
| `get_matured_balance` | `(user) -> i128` | Total withdrawable lock amount |

### Why On-Chain Helpers (Not Pure Off-Chain)

| Factor | On-chain helper | Off-chain / client-filtered |
|--------|----------------|----------------------------|
| Client complexity | Low -- single RPC call | High -- fetch all, filter, sum |
| Correctness | Authoritative ledger timestamp | Clock drift risk on mobile |
| RPC payload size | Only matured locks returned | Full lock list transferred |
| State consistency | Single atomic read | Possible multi-call race |
| WASM size cost | ~2 KB additional | None |
| CPU instruction cost | Full scan per call | None (contract-side) |

The WASM size cost is minimal and the CPU cost is bounded by the per-user lock
count, which is already bounded by the user's deposit history.

---

## Storage Model & Scan Cost

Locks are stored as individual persistent entries keyed by
`DataKey::Lock(Address, u64)`. There is no secondary index by maturity status.
Discovery helpers perform a **linear scan** of all lock IDs from `1` to
`NextLockId(user) - 1`, applying the filter:

```
!lock.withdrawn && current_timestamp >= lock.unlock_time
```

### Cost Characteristics

| User lock count | Scan cost | Mitigation |
|----------------|-----------|------------|
| 1 -- 50 | Negligible | None needed |
| 50 -- 200 | Moderate | Use pagination; batch withdrawals |
| 200+ | High | Encourage lock consolidation; use off-chain indexer |

The `MAX_LOCK_PAGE_SIZE = 50` cap limits the **returned** page size but does
**not** limit the scan. For users with hundreds of locks, the full scan runs
regardless of the requested page size.

### Why No Secondary Index

Adding a `MaturedLocks(Address)` storage key that tracks matured lock IDs would
require **write-time maintenance**: every new lock would need to be added when
it matures (which is a future event, not a write-time event). Soroban has no
cron / deferred execution, so maintaining such an index is impossible without
an external trigger. The linear scan is the correct approach given the storage
model.

---

## Pagination Strategy

### Contract-Side Pagination

`list_matured_locks(user, offset, limit)` uses **logical offset** into the
filtered result set:

- `offset = 0, limit = 10` returns the first 10 matured locks.
- `offset = 10, limit = 10` returns the next 10.
- The offset counts **matured locks only**, not raw lock IDs.

This differs from `list_locks` where `offset` maps directly to lock IDs.

### SDK Pagination Pattern

```typescript
async function fetchAllMaturedLocks(
  contract: SavingsVaultClient,
  user: string,
  pageSize: number = 20,
): Promise<LockEntry[]> {
  const allMatured: LockEntry[] = [];
  let offset = 0;

  while (true) {
    const page = await contract.list_matured_locks(user, offset, pageSize);
    allMatured.push(...page);

    if (page.length < pageSize) break; // last page
    offset += page.length;
  }

  return allMatured;
}
```

### Mobile UI Recommendations

| Use Case | Recommended Call |
|----------|-----------------|
| Badge count ("3 ready") | `get_matured_lock_count(user)` |
| Dashboard total | `get_matured_balance(user)` |
| Lock list screen | `list_matured_locks(user, 0, 20)` + scroll pagination |
| "Withdraw All" pre-check | `get_matured_lock_count(user)` to confirm > 0 |
| Individual withdrawal | `get_lock(user, lock_id)` to verify before `withdraw_lock` |

---

## Event-Based Indexing (Off-Chain Alternative)

For applications that need real-time matured-lock notifications or historical
queries across all users, **event indexing** is the recommended off-chain
approach.

### Relevant Events

| Event | Topics | Data | When |
|-------|--------|------|------|
| `lock` | `(Symbol("lock"), user)` | `(amount, unlock_time, available, locked)` | Lock created |
| `withdraw_lock` | `(Symbol("withdraw_lock"), user)` | `(lock_id, amount)` | Lock withdrawn |
| `extend_lock` | `(Symbol("extend_lock"), user)` | `(lock_id, old_time, new_time, amount)` | Lock extended |

### Indexer Strategy

1. **Subscribe** to contract events via Horizon or RPC streaming.
2. **Track** lock creation events to build a local lock database with
   `(user, lock_id, amount, unlock_time)`.
3. **Update** on `withdraw_lock` events (mark withdrawn) and `extend_lock`
   events (update `unlock_time`).
4. **Query** locally for matured locks: `WHERE unlock_time <= NOW AND NOT withdrawn`.
5. **Push** notifications to mobile clients when locks mature.

### Indexer vs On-Chain Helpers

| | On-chain helpers | Event indexer |
|---|---|---|
| Setup cost | Zero (built in) | Moderate (infrastructure) |
| Latency | Per-query (RPC round-trip) | Near-real-time (event stream) |
| Cross-user queries | Not supported | Supported |
| Push notifications | Not possible | Possible |
| Historical analytics | Not available | Available |
| Trust model | Trustless (contract state) | Indexer must be trusted |

**Recommendation:** Use on-chain helpers for individual user queries in mobile
apps. Deploy an event indexer for admin dashboards, analytics, and push
notifications.

---

## Unsupported Discovery Cases

The following are explicitly **not supported** by the on-chain helpers and must
be handled off-chain:

| Case | Reason | Workaround |
|------|--------|------------|
| Cross-user matured lock queries | Storage is per-user; no global lock index | Event indexer |
| "Locks maturing in next N seconds" | Would require speculative time comparison | Client-side: fetch `list_locks`, filter by `unlock_time < now + N` |
| Sorted by maturity time | Locks are stored in creation order; no secondary sort | Client-side sort after fetch |
| Notification on maturity | No cron / push in Soroban | Event indexer + timer service |
| Bulk "withdraw all matured" | No batch withdrawal function | Client loops `withdraw_lock` per lock ID |
| Lock count exceeding ~500 per user | Linear scan cost becomes prohibitive | Encourage lock consolidation; use indexer |

---

## SDK Responsibilities

| Responsibility | Owner | Notes |
|---------------|-------|-------|
| Matured-lock badge count | SDK / Mobile | Call `get_matured_lock_count` |
| Withdrawable balance display | SDK / Mobile | Call `get_matured_balance` |
| Paginated matured lock list | SDK / Mobile | Call `list_matured_locks` with pagination |
| Maturity time comparison | Contract | On-chain helpers use authoritative ledger timestamp |
| Goal labels / titles | Mobile (local storage) | Map `lock_id` to user-defined labels client-side |
| Push notifications on maturity | Backend service | Event indexer + push infrastructure |
| "Locks maturing soon" view | SDK / Mobile | Fetch `list_locks`, filter by `unlock_time` range client-side |
| Withdrawal transaction signing | SDK / Mobile | Call `withdraw_lock` per lock ID with user auth |

---

## Tests

Comprehensive tests are in
`contracts/savings_vault/src/test/matured_lock_discovery.rs`:

- Empty user state
- All-immature filtering
- Mixed matured / immature filtering
- Withdrawn lock exclusion
- Pagination with offset and limit
- Pagination skipping immature locks
- Exact maturity boundary (T-1, T, T+1)
- Multi-user isolation
- Read-only state invariant
- Uninitialized contract panics
- Consistency with `can_withdraw`

---

## Related Docs

- [Lock read helpers](lock-read-helpers.md) -- `get_lock` and `list_locks`
- [Multi-lock storage](multi-lock-storage.md) -- storage model and lifecycle
- [Event schema](event-schema.md) -- on-chain event definitions
- [SDK contract sequence](sdk-contract-sequence.md) -- SDK integration patterns
- [Architecture](architecture.md) -- overall contract architecture
