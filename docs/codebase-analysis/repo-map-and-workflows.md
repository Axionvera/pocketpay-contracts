# Repo Map And Workflows

This document maps the repository's real implementation structure, the contract
surface area, and the principal runtime workflows.

## 1. Repository Topology

```text
pocketpay-contracts/
|- Cargo.toml                      # Workspace manifest
|- Cargo.lock
|- README.md
|- Makefile                        # Minimal build/size targets
|- .env.example                    # Deployment/runtime variable examples
|- .github/workflows/
|  \- trigger-auto-merge.yml       # PR automation only, not CI validation
|- contracts/
|  \- savings_vault/
|     |- Cargo.toml                # Contract crate manifest
|     |- README.md
|     |- src/
|     |  |- lib.rs                 # Entire contract implementation
|     |  \- test/                  # Registered Rust unit/property tests
|     |- proptest-regressions/     # Saved failing seeds for property tests
|     \- test_snapshots/           # Snapshot artifacts for event/schema tests
|- docs/                           # Architecture, security, testing, ops docs
|- scripts/
|  |- deploy-testnet.sh
|  \- report-wasm-size.sh
\- tests/
   \- atomicity/
      \- transfer-atomicity.test.ts
```

## 2. Workspace And Dependency Layout

### Workspace

- Root `Cargo.toml` defines a workspace over `contracts/*`.
- The workspace uses release settings optimized for small WASM output:
  - `opt-level = "z"`
  - `lto = true`
  - `panic = "abort"`
  - `codegen-units = 1`

### Contract Crate

- Crate name: `savings-vault`
- Edition: Rust 2021
- Output type: `cdylib`
- Runtime dependency: `soroban-sdk`
- Dev dependencies:
  - `soroban-sdk` with `testutils`
  - `proptest`

## 3. Core Modules And Responsibilities

Although the runtime code is in a single file, it is conceptually split into
distinct subsystems.

### `contracts/savings_vault/src/lib.rs`

#### State types

- `LockEntry`
  - Canonical per-lock record
  - Tracks owner, amount, creation time, unlock time, and withdrawn state
- `BalanceSnapshot`
  - Read model for unlocked/locked/total/withdrawable balances
- `LockSummary`
  - Read model for counts, totals, and unlock-time ranges

#### Storage schema

- `DataKey`
  - Enumerates every storage key used by the contract
  - Separates global instance storage from user-specific persistent storage

#### Error model

- `ContractError`
  - Stable `u32` error taxonomy grouped by category ranges:
    - 1000s validation
    - 2000s authorization
    - 3000s lifecycle
    - 4000s accounting
    - 5000s lock handling
    - 6000s storage/migration
    - 7000s token
    - 8000s admin rotation

#### Internal helpers

- Initialization guard
- Storage migration and version checks
- Admin identity validation
- Pause-state enforcement with lazy auto-unpause
- Historical lock reconstruction helper

#### Public entrypoints

- Lifecycle and metadata
- Pause and config administration
- User deposit/withdrawal flows
- Lock creation, extension, and withdrawal
- Read-only helpers for balances and locks
- Admin rotation

## 4. Test Suite Structure

The registered Rust test suite lives under `contracts/savings_vault/src/test/`
and is orchestrated by `mod.rs`.

### Main categories

- Initialization and storage versioning
- Auth and unauthorized access
- Balance conservation and isolation
- Lock lifecycle and replay protection
- Pause behavior and pause-state reads
- Event schema and event ordering
- Token-backed custody and rollback
- Property-based invariant testing

### Important support assets

- `test_helpers.rs`
  - Shared fixture creation
  - Token setup helpers
  - Ledger timestamp control
  - Mocked-auth and strict-auth environments
- `test_snapshots/`
  - Event and behavior snapshots used as regression fixtures
- `proptest-regressions/`
  - Persisted failing seeds for reproducing property-test issues

## 5. On-Chain Storage Schema

This repository has no SQL or document database. The storage schema is the
Soroban `DataKey` enum.

### Instance storage keys

| Key | Purpose |
| --- | --- |
| `Admin` | Current admin address |
| `Initialized` | One-time initialization guard |
| `Token` | Configured SAC token address |
| `StorageVersion` | Current storage version marker |
| `Paused` | Emergency pause flag |
| `PauseExpiry` | Timestamp when pause auto-expires |
| `MinDepositAmount` | Global deposit floor |
| `MaxLockDurationSecs` | Global max lock duration |
| `MinLockDurationSecs` | Global min lock duration |

### Persistent storage keys

| Key | Purpose |
| --- | --- |
| `Balance(Address)` | User's available balance |
| `Lock(Address, u64)` | User lock record keyed by owner and lock id |
| `NextLockId(Address)` | Next lock id counter for a user |

### Transitional / confusing key

| Key | Status |
| --- | --- |
| `Locks(Address)` | Declared in `DataKey` but not used as the primary live storage model |

That unused variant is part of the repo's technical debt because many older docs
still describe it as the active schema.

## 6. Contract Public Interface

This is the effective API surface of the repository.

### Lifecycle and metadata

- `initialize(env, admin, token)`
- `get_version(env)`
- `get_token(env)`
- `get_admin(env)`

### Pause and configuration

- `pause(env, admin, duration_secs)`
- `unpause(env, admin)`
- `is_paused(env)`
- `set_min_deposit_amount(env, admin, min_amount)`
- `get_min_deposit_amount(env)`
- `set_max_lock_duration(env, admin, max_duration_secs)`
- `get_max_lock_duration(env)`
- `set_min_lock_duration(env, admin, min_duration_secs)`
- `get_min_lock_duration(env)`

### User fund flows

- `deposit(env, user, amount)`
- `withdraw(env, user, amount)`

### Lock lifecycle

- `lock_funds(env, user, amount, unlock_time)`
- `extend_lock(env, user, lock_id, new_unlock_time)`
- `withdraw_lock(env, user, lock_id)`

### Read helpers

- `get_balance(env, user)`
- `get_balance_snapshot(env, user)`
- `get_lock_summary(env, user)`
- `get_locked_balance(env, user)`
- `can_withdraw(env, user)`
- `get_lock(env, user, lock_id)`
- `list_locks(env, user, offset, limit)`

### Admin rotation

- `transfer_admin(env, admin, new_admin)`

## 7. Event Surface

The contract publishes events for major state transitions and admin changes.

| Event name/topic 0 | Trigger |
| --- | --- |
| `initialize` | Initial contract setup |
| `deposit` | Successful deposit |
| `withdraw` | Successful available-balance withdrawal |
| `lock` | Successful lock creation |
| `extend_lock` | Successful lock extension |
| `withdraw_lock` | Successful matured lock withdrawal |
| `pause` | Pause activated |
| `unpause` | Pause cleared |
| `xferadmin` | Admin transferred |
| `cfg_min` | Min deposit rule updated |
| `cfg_maxlk` | Max lock duration updated |
| `cfg_minlk` | Min lock duration updated |

Events use Soroban topics for indexing and tuples/scalars as payloads. Topic 0
is always the action name, while topic 1 is usually the user or admin address.

## 8. Interaction Mechanisms Between Components

At runtime, the system interaction model is:

1. A wallet, SDK, CLI user, or test invokes a `SavingsVault` method.
2. Soroban enforces host auth when the method calls `require_auth()`.
3. The contract reads/writes Soroban instance or persistent storage.
4. Custody-sensitive flows call the configured SAC token contract.
5. The contract emits events describing the state transition.

### Component relationship map

- **Wallet / SDK / Soroban CLI**
  - Builds the transaction
  - Provides auth for the user or admin address
- **SavingsVault contract**
  - Validates lifecycle, auth, and business rules
  - Owns the accounting model
  - Coordinates storage and event emission
- **SAC token contract**
  - Performs the actual token transfer
  - Represents the source of real custody
- **Indexers / mobile apps**
  - Read contract state directly
  - Or reconstruct state from events off-chain

## 9. Core Workflows

### Initialize

1. Deployer/admin calls `initialize(admin, token)`.
2. Contract rejects repeat initialization.
3. Admin signature is required.
4. Global config is written to instance storage.
5. Storage version is set.
6. `initialize` event is emitted.

### Deposit

1. Contract verifies initialized state, migration state, storage version, and
   pause status.
2. User signature is required.
3. Amount and configured minimum are validated.
4. SAC transfer moves tokens from user to contract.
5. Internal `Balance(user)` is incremented.
6. `deposit` event is emitted.

### Withdraw

1. Contract verifies initialized state and storage version.
2. User signature is required.
3. Available balance is checked against `Balance(user)`.
4. SAC transfer moves tokens from contract to user.
5. Internal `Balance(user)` is decremented.
6. `withdraw` event is emitted.

### Lock funds

1. Contract verifies initialized state, version, and pause status.
2. User signature is required.
3. Amount and unlock-time rules are validated.
4. Global min/max lock-duration rules are enforced on creation.
5. Available balance is checked.
6. `NextLockId(user)` is incremented.
7. `Lock(user, next_id)` is written.
8. `Balance(user)` is decremented.
9. Locked total is recomputed for the emitted payload.
10. `lock` event is emitted.

### Extend lock

1. Contract verifies initialized state, version, and pause status.
2. User signature is required.
3. Lock existence and non-withdrawn status are validated.
4. `new_unlock_time` must be in the future and strictly greater than the
   current unlock time.
5. Lock entry is updated in place.
6. `extend_lock` event is emitted.

### Withdraw matured lock

1. Contract verifies initialized state and storage version.
2. User signature is required.
3. Lock is loaded by `(user, lock_id)`.
4. Contract verifies the lock exists, is not already withdrawn, and is mature.
5. SAC transfer moves the lock amount from contract to user.
6. Lock is marked withdrawn and its amount is zeroed.
7. `withdraw_lock` event is emitted.

### Pause / unpause

1. Admin-only methods require both `require_auth()` and an admin-role match.
2. `pause` writes `Paused = true` and sets `PauseExpiry`.
3. `unpause` clears both fields immediately.
4. Mutating non-withdrawal methods call `require_not_paused()`.
5. Expired pauses are cleared lazily on the next mutating call.

## 10. Data Flow Summary By Responsibility

### User funds

- Real token movement is handled by SAC.
- Logical fund partitioning is handled by:
  - `Balance(user)` for unlocked funds
  - `Lock(user, id)` for time-locked funds

### Read models

- `get_balance` returns only available balance.
- Matured locks remain in lock storage until individually withdrawn.
- `get_balance_snapshot` and `get_lock_summary` aggregate across lock history.

### Error propagation

- Business-rule failures become typed `ContractError` values.
- Host-level auth failures remain Soroban auth failures rather than contract
  error codes.

## 11. What This Repo Does Not Contain

To avoid over-generalizing this project into a traditional web system:

- No HTTP controllers or API routes
- No database migrations
- No background workers
- No message queues
- No frontend application
- No off-chain service implementation for SDK orchestration

The closest off-chain integration guidance lives in docs such as
`sdk-contract-sequence.md`, `invocation-examples.md`, and `read-models.md`, but
those are consumer-facing documents, not implemented services.
