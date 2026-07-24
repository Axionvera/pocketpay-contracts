# Vault Event Schema

This document outlines the expected event topics, payloads, and naming conventions for actions in the **Savings Vault Contract**. 

SDK maintainers can use this stable schema to consume contract events safely.

---

## Breaking Changes

Event schema changes are considered breaking and require updates to consumer SDKs. The following changes are breaking:
- Changing the order of topics or payload fields
- Changing the type of any topic or payload field
- Removing a topic or payload field
- Changing the event name (topic 0)

Non-breaking changes include adding new optional fields to the payload (if the schema allows) or adding new event types.

When making any event schema change, the event compatibility tests in
`contracts/savings_vault/src/test/event_compatibility.rs` and
`contracts/savings_vault/src/test/event_schema.rs` must be updated to match the
new schema.

---

## Event Naming & Structure Conventions

All events emitted by the Savings Vault contract follow standard Soroban event guidelines:
- **Topics**: A list of topics used for filtering/routing.
  - Topic 0: The event name (e.g., Symbol representing the action).
  - Topic 1: The primary entity involved in the action (typically the `Address` of the user/admin).
- **Payload**: The data associated with the event (represented as a Soroban type or tuple).

---

## Event Definitions

### 1. Initialize Event
Emitted once when the contract is initialized by the administrator.

- **Topic 0**: `Symbol::new(&env, "initialize")`
- **Topic 1**: `admin` (`Address`) - The admin address recorded for the contract.
- **Payload**: `token` (`Address`) - The token address associated with the vault.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["initialize", "GB...ADMIN_ADDRESS"],
  "value": "GB...TOKEN_ADDRESS"
}
```

---

### 2. Deposit Event
Emitted when a user deposits funds into their vault.

- **Topic 0**: `Symbol::new(&env, "deposit")`
- **Topic 1**: `user` (`Address`) - The address of the depositor.
- **Payload**: A tuple containing:
  1. `amount` (`i128`) - The amount deposited.
  2. `new_balance` (`i128`) - The user's new available balance.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["deposit", "GD...USER_ADDRESS"],
  "value": [1000, 5000]
}
```

---

### 3. Withdraw Event
Emitted when a user withdraws funds from their vault.

- **Topic 0**: `Symbol::new(&env, "withdraw")`
- **Topic 1**: `user` (`Address`) - The address of the withdrawer.
- **Payload**: A tuple containing:
  1. `amount` (`i128`) - The amount withdrawn.
  2. `new_balance` (`i128`) - The user's new available balance.
  3. `new_locked` (`i128`) - The user's new locked balance.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["withdraw", "GD...USER_ADDRESS"],
  "value": [500, 4500, 0]
}
```

---

### 4. Lock Event
Emitted when a portion of the user's balance is locked.

- **Topic 0**: `Symbol::new(&env, "lock")`
- **Topic 1**: `user` (`Address`) - The address of the user.
- **Payload**: A tuple containing:
  1. `amount` (`i128`) - The amount locked.
  2. `unlock_time` (`u64`) - The UNIX timestamp (seconds) when the funds unlock.
  3. `new_balance` (`i128`) - The user's new available (unlocked) balance.
  4. `new_locked` (`i128`) - The user's new locked balance.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["lock", "GD...USER_ADDRESS"],
  "value": [2000, 1785000000, 2500, 2000]
}
```

---

### 5. Pause Event
Emitted when the admin activates an emergency pause.

- **Topic 0**: `Symbol::new(&env, "pause")`
- **Topic 1**: `admin` (`Address`) - The admin address that triggered the pause.
- **Payload**: `expiry` (`u64`) - The Unix timestamp (seconds) when the pause auto-expires.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["pause", "GB...ADMIN_ADDRESS"],
  "value": 1785000600
}
```

---

### 6. Unpause Event
Emitted when the admin deactivates an active pause.

- **Topic 0**: `Symbol::new(&env, "unpause")`
- **Topic 1**: `admin` (`Address`) - The admin address that triggered the unpause.
- **Payload**: `()` - Empty payload (unit type).

#### Example Payload (JSON Representation)
```json
{
  "topics": ["unpause", "GB...ADMIN_ADDRESS"],
  "value": null
}
```

---

### 7. Withdraw Lock Event
Emitted when a user withdraws a specific matured lock by ID.

- **Topic 0**: `Symbol::new(&env, "withdraw_lock")`
- **Topic 1**: `user` (`Address`) - The address of the withdrawer.
- **Payload**: A tuple containing:
  1. `lock_id` (`u64`) - The lock entry ID withdrawn.
  2. `amount` (`i128`) - The amount transferred out of the vault.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["withdraw_lock", "GD...USER_ADDRESS"],
  "value": [1, 500]
}
```

---

### 8. Transfer Admin Event
Emitted when the current admin transfers admin privileges to a new address.

- **Topic 0**: `symbol_short!("xferadmin")` — short symbol `xferadmin` (on-chain topic name)
- **Topic 1**: `old_admin` (`Address`) - The previous admin address.
- **Payload**: `new_admin` (`Address`) - The new admin address.

> **Note:** The on-chain topic is the short symbol `xferadmin`, not `transfer_admin`.
> Indexers and SDKs must filter on `xferadmin`.

#### Example Payload (JSON Representation)
```json
{
  "topics": ["xferadmin", "GB...OLD_ADMIN_ADDRESS"],
  "value": "GB...NEW_ADMIN_ADDRESS"
}
```
