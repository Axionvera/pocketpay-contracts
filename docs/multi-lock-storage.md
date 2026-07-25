# Multi-Lock Storage Model & Architecture

## Overview

The **PocketPay Savings Vault Contract** supports **multiple independent time locks per user**. Unlike single-balance lock designs, this architecture enables mobile users to manage multiple concurrent savings goals (e.g., "Emergency Fund", "Vacation", "Rent") with distinct lock amounts, creation timestamps, unlock dates, and withdrawal states.

---

## Data Structures & Storage Schema

### 1. `LockEntry` Struct

Each lock entry stored on-chain contains complete metadata describing the locked funds:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEntry {
    pub id: u64,
    pub owner: Address,
    pub amount: i128,
    pub created_time: u64,
    pub unlock_time: u64,
    pub withdrawn: bool,
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `id` | `u64` | Stable, monotonically increasing 64-bit unsigned integer ID unique to the user |
| `owner` | `Address` | Stellar address of the user who owns the lock |
| `amount` | `i128` | Token amount locked in contract atomic units (0 once withdrawn) |
| `created_time` | `u64` | Ledger Unix timestamp (seconds) when `lock_funds` was invoked |
| `unlock_time` | `u64` | Ledger Unix timestamp (seconds) after which funds can be redeemed via `withdraw_lock` |
| `withdrawn` | `bool` | Boolean flag (`false` = active/matured lock, `true` = redeemed lock) |

---

### 2. Storage Key Design (`DataKey`)

Storage keys are defined in the `DataKey` enum to prevent collisions across users and across distinct locks:

```rust
pub enum DataKey {
    ...
    /// Individual lock entry stored by (owner, lock_id)
    Lock(Address, u64),
    /// Monotonically increasing lock ID counter per user
    NextLockId(Address),
    ...
}
```

#### Collision Prevention & Isolation Guarantee

- **Cross-User Isolation**: Locks are stored under `DataKey::Lock(owner, lock_id)`. The composite key includes the user's `Address`. User A's `Lock(UserA, 1)` and User B's `Lock(UserB, 1)` map to completely separate Soroban persistent storage keys, eliminating cross-user data interference or key collisions.
- **Stable Sequential Identifiers**: `NextLockId(user)` initializes at `1` for each user and increments sequentially (`1, 2, 3...`) upon successful lock creation. Unsuccessful operations (e.g., insufficient balance or past unlock time) do not advance the counter, keeping IDs strictly sequential without gaps.

---

## Lock Lifecycle & State Machine

```
               ┌──────────────────────────────────────────┐
               │              Deposit Funds               │
               └────────────────────┬─────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Available Balance (Liquid)                      │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │ lock_funds(user, amount, unlock_time)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Active Time Lock (Unmatured)                    │
│   DataKey::Lock(user, id) -> LockEntry { withdrawn: false, amount }    │
│   current_timestamp < unlock_time                                      │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │ Ledger Timestamp Advances
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        Matured Time Lock (Redeemable)                  │
│   current_timestamp >= unlock_time                                     │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   │ withdraw_lock(user, lock_id)
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│                       Withdrawn / Redeemed Lock                        │
│   DataKey::Lock(user, id) -> LockEntry { withdrawn: true, amount: 0 }  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Core Contract Interface Methods

### 1. `lock_funds(env, user, amount, unlock_time) -> u64`
- **Action**: Moves `amount` from the user's available balance into a newly allocated `LockEntry`.
- **Validation**:
  - `amount > 0`
  - `unlock_time > env.ledger().timestamp()`
  - `amount <= get_balance(user)`
  - Contract initialized and not paused
- **Return**: `u64` representing the allocated `lock_id`.

### 2. `withdraw_lock(env, user, lock_id)`
- **Action**: Redeems a specific matured lock by ID, transferring tokens directly back to `user` via SAC.
- **Validation**:
  - Lock exists for `user` with `lock_id`
  - `lock.withdrawn == false`
  - `env.ledger().timestamp() >= lock.unlock_time`
- **State Effect**: Sets `lock.withdrawn = true` and `lock.amount = 0`.

### 3. `get_lock(env, user, lock_id) -> Option<LockEntry>`
- **Action**: Read-only lookup for a single lock entry by user and ID.

### 4. `list_locks(env, user, offset, limit) -> Vec<LockEntry>`
- **Action**: Returns a paginated list of lock records for `user` in creation order (oldest first).
- **Pagination Safety**: Capped at `MAX_LOCK_PAGE_SIZE = 50` per query.

---

## SDK & Mobile Integration Guidance

### 1. Goal Labeling & Client Storage
Because smart contract WASM bytecode is optimized for minimal binary size, string goal titles (e.g. "Vacation") are stored off-chain or client-side, mapped to the stable numeric `lock_id`:

```typescript
interface MobileSavingsGoal {
  lockId: number;         // On-chain stable identifier (LockEntry.id)
  title: string;          // Client-side goal name ("Emergency Fund")
  amount: bigint;         // On-chain locked amount
  unlockTime: number;     // Unix timestamp (seconds)
  withdrawn: boolean;     // Withdrawal status
}
```

### 2. Maturity Verification Off-Chain
Mobile apps can determine maturity without extra contract RPC calls by comparing `LockEntry.unlock_time` against the current ledger timestamp or network clock:

```typescript
const isMatured = (lock: LockEntry, currentLedgerTimeSec: number): boolean => {
  return !lock.withdrawn && currentLedgerTimeSec >= lock.unlock_time;
};
```

### 3. Pagination Best Practices
When displaying user savings vaults in mobile dashboards:
- Query `list_locks(user, offset = 0, limit = 20)` for initial page render.
- Fetch subsequent pages as the user scrolls.
- Capped at 50 records per invocation to prevent RPC timeouts or memory overruns.
