# Storage Audit: Savings Vault Contract
This document provides a comprehensive audit of all storage keys used in the Savings Vault contract, including:
- Storage key definitions and types
- Mutation points (which functions modify which keys)
- Invariants that must always hold
- TTL management guidelines

---

## 1. Storage Layers
The contract uses two Soroban storage layers:

| Layer | Purpose |
|-------|---------|
| **Instance Storage** | Stores configuration and initialization state (admin, token, initialized flag, storage version) |
| **Persistent Storage** | Stores per-user state (balances, locks, lock ID counter) |

---

## 2. Storage Key Audit

### Instance Storage Keys
| Key | Type | Default | Initialization | Mutation Points | Invariants |
|-----|------|---------|----------------|-----------------|------------|
| `DataKey::Admin` | `Address` | None | Set once in `initialize` | `transfer_admin` | - Immutable after `transfer_admin`; only admin can change |
| `DataKey::Initialized` | `bool` | None | Set to `true` in `initialize` | None (never modified after initialization) | - Always `true` after initialization; never unset |
| `DataKey::Token` | `Address` | None | Set once in `initialize` | None (never modified after initialization) | - Immutable after initialization |
| `DataKey::StorageVersion` | `u64` | None | Set to `STORAGE_VERSION` (1) in `initialize` | None (never modified after initialization) | - Always equals `STORAGE_VERSION` |

### Persistent Storage Keys
| Key | Type | Default | Initialization | Mutation Points | Invariants |
|-----|------|---------|----------------|-----------------|------------|
| `DataKey::Balance(Address)` | `i128` | `0` (implicit) | Not set at initialization | `deposit`, `withdraw`, `lock_funds` | - ≥ 0 at all times; represents available balance |
| `DataKey::Lock(Address, u64)` | `LockEntry` | None | Not set at initialization | `lock_funds`, `withdraw`, `withdraw_lock` | - `LockEntry.amount` ≥ 0; `LockEntry.id` unique for user; `LockEntry.unlock_time` ≥ 0 |
| `DataKey::NextLockId(Address)` | `u64` | `1` (implicit) | Not set at initialization | `lock_funds` | - Strictly increasing (monotonic); never decreases |

---

## 3. Mutation Point Mapping
This section maps each storage key to the functions that modify it:

### Instance Storage
| Function | Modifies |
|----------|----------|
| `initialize` | `Admin`, `Initialized`, `Token`, `StorageVersion` |
| `transfer_admin` | `Admin` |

### Persistent Storage
| Function | Modifies |
|----------|----------|
| `deposit` | `Balance(user)` |
| `withdraw` | `Balance(user)`, `Lock(user, id)` |
| `lock_funds` | `Balance(user)`, `Lock(user, id)`, `NextLockId(user)` |
| `withdraw_lock` | `Lock(user, id)` |

---

## 4. Critical Storage Invariants
These invariants must hold at all times, across all function calls:

### Invariant 1: User Available Balance ≥ 0
- **Description**: `Balance(user)` must never be negative
- **Enforced By**: `deposit`, `withdraw`, `lock_funds`
- **Tested By**: `balance_conservation.rs` tests and `property_vault_accounting.rs` proptests

### Invariant 2: User Lock Entry Amounts ≥ 0
- **Description**: Every `LockEntry.amount` in `Locks(user)` must be ≥ 0
- **Enforced By**: `lock_funds`
- **Tested By**: `balance_conservation.rs` tests

### Invariant 3: NextLockId Is Monotonic
- **Description**: `NextLockId(user)` must never decrease; only increments by 1 per `lock_funds` call
- **Enforced By**: `lock_funds`
- **Tested By**: `lock_read_helpers.rs` tests

### Invariant 4: Lock Entry IDs Are Unique Per User
- **Description**: No two `LockEntry` in `Locks(user)` share the same `id`
- **Enforced By**: `lock_funds` (uses `NextLockId` to generate unique IDs)
- **Tested By**: Implicit via monotonic `NextLockId`

### Invariant 5: Token Custody Invariant
- **Description**: The sum of all user balances + sum of all user locked balances ≤ SAC balance of the contract
- **Enforced By**: All functions that perform token transfers (`deposit`, `withdraw`, `withdraw_lock`)
- **Tested By**: `property_vault_accounting::prop_global_token_custody` (proptest)

### Invariant 6: Initialized Flag Remains True After Initialization
- **Description**: Once `initialize` has been called, `Initialized` remains `true` forever
- **Enforced By**: `initialize` (sets flag; no other function modifies it)
- **Tested By**: `initialization.rs` tests

---

## 5. TTL Management Guidelines
See [storage-ttl.md](storage-ttl.md) for full TTL management guidelines!

---

## 6. Storage Upgrade / Migration Plan
See [upgrade-strategy.md](upgrade-strategy.md) for future upgrade/migration planning!
