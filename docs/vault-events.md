# Vault Event Schema

This document describes the structured events emitted by the Savings Vault contract.
Events are published via Soroban's `env.events().publish(topics, payload)` and are
designed for consumption by off-chain indexers, SDKs, and monitoring tools.

## Event Structure

Every event follows the Soroban event model:

```
topics: (Symbol, Address, ...)
payload: <event-specific data>
```

- **topics[0]**: Event type identifier (Symbol)
- **topics[1]**: Acting address — the user or admin involved

## Events Reference

### `initialize`

Emitted once when the contract is initialized.

| Field    | Type     | Description              |
|----------|----------|--------------------------|
| topic[0] | Symbol   | `"initialize"`           |
| topic[1] | Address  | Admin address            |
| payload  | Address  | Token contract address   |

---

### `deposit`

Emitted when a user deposits tokens into their vault.

| Field    | Type     | Description                          |
|----------|----------|--------------------------------------|
| topic[0] | Symbol   | `"deposit"`                          |
| topic[1] | Address  | User address                         |
| payload  | `(i128, i128)` | `(amount, new_balance)`    |

- `amount`: Tokens deposited in this transaction
- `new_balance`: User's total available balance after deposit

---

### `withdraw`

Emitted when a user withdraws available tokens from their vault.

| Field    | Type     | Description                                    |
|----------|----------|------------------------------------------------|
| topic[0] | Symbol   | `"withdraw"`                                   |
| topic[1] | Address  | User address                                   |
| payload  | `(i128, i128, i128)` | `(amount, new_balance, new_locked)` |

- `amount`: Tokens withdrawn in this transaction
- `new_balance`: User's available balance after withdrawal
- `new_locked`: User's total locked balance after withdrawal

---

### `lock`

Emitted when a user locks a portion of their available balance.

| Field    | Type     | Description                                              |
|----------|----------|----------------------------------------------------------|
| topic[0] | Symbol   | `"lock"`                                                 |
| topic[1] | Address  | User address                                             |
| payload  | `(i128, u64, i128, i128)` | `(amount, unlock_time, available, locked)` |

- `amount`: Tokens locked in this transaction
- `unlock_time`: Unix timestamp (seconds) when funds unlock
- `available`: User's available balance after locking
- `locked`: User's total locked balance after locking

---

### `withdraw_lock`

Emitted when a user withdraws a specific matured lock entry.

| Field    | Type     | Description                        |
|----------|----------|------------------------------------|
| topic[0] | Symbol   | `"withdraw_lock"`                  |
| topic[1] | Address  | User address                       |
| payload  | `(u64, i128)` | `(lock_id, amount)`       |

- `lock_id`: Identifier of the lock being withdrawn
- `amount`: Tokens released from the lock

---

### `transfer_admin`

Emitted when the admin transfers privileges to a new address.

| Field    | Type     | Description                       |
|----------|----------|-----------------------------------|
| topic[0] | Symbol   | `"transfer_admin"`                |
| topic[1] | Address  | Old admin address                 |
| payload  | Address  | New admin address                 |

---

## Design Notes

- **Topic consistency**: All events use `topic[0]` as the event type (Symbol) and
  `topic[1]` as the primary actor address. This enables efficient filtering by event
  type and by user/admin address.
- **Payload minimalism**: Payloads include only the data necessary to reconstruct
  the state change. Balance snapshots (`new_balance`, `new_locked`, `available`,
  `locked`) are included to avoid requiring indexers to maintain full state.
- **No sensitive data**: Events do not expose private keys, internal storage keys,
  or implementation details beyond what's needed for accounting.

## Testing

Event schema tests are in `contracts/savings_vault/src/test/event_schema.rs`.
These tests verify topic types, topic values, and payload types/values for every
event emitted by the contract.
