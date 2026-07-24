# Lock Read Helpers

The savings vault exposes read-only helpers for SDK and mobile clients to
display individual user lock records.

## Response shape

Each lock record is a `LockEntry` with three fields:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `u64` | Unique lock ID for the user (returned by `lock_funds`) |
| `amount` | `i128` | Locked amount in contract units |
| `unlock_time` | `u64` | Unix timestamp (seconds) when the lock matures |

Example JSON representation for SDK consumers:

```json
{
  "id": 1,
  "amount": 200,
  "unlock_time": 1735689600
}
```

Maturity is derived off-chain: a lock is matured when
`current_ledger_timestamp >= unlock_time`. The contract does not add a separate
`is_matured` field; compare `unlock_time` to the ledger timestamp instead.

## Functions

### `get_lock(user, lock_id) -> Option<LockEntry>`

Returns one stored lock when `lock_id` matches a record for `user`.
Returns `None` when no matching lock exists.

Read-only. No authorization required.

### `list_locks(user, offset, limit) -> Vec<LockEntry>`

Returns a page of lock records in creation order (oldest first). Includes
active and matured entries still stored for the user.

| Parameter | Behavior |
|-----------|----------|
| `offset` | Skip this many records from the start |
| `limit` | Maximum records to return; `0` returns an empty list |
| Page cap | Requests above 50 records per call are capped at 50 |

Read-only. No authorization required.

Pagination does not reduce storage read cost: the contract still loads the
user's full lock vector from persistent storage, then slices the result in
memory. Prefer reasonable page sizes for RPC responses.

## CLI examples

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  get_lock \
  --user <USER_ADDRESS> \
  --lock_id 1

soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  list_locks \
  --user <USER_ADDRESS> \
  --offset 0 \
  --limit 10
```

## Related docs

- [Architecture](architecture.md) — lock storage model
- [Authorization boundaries](authorization-boundaries.md) — read-only access rules
- [Contract invocation examples](invocation-examples.md) — all public functions
