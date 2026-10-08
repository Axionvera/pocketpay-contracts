# Savings Vault Public API & Compatibility Policy

This document is the source-facing API reference for the `SavingsVault`
contract in `contracts/savings_vault/src/lib.rs`. It records the callable
surface that SDKs, mobile clients, indexers, and operators can rely on, and it
defines how that surface may change.

The reference below is reconciled against sponsor `main` at
`7988c6efec9a73162ed7d2fffb3b8b6ebd5a7b67`.

## Version model

The contract currently exposes semantic version **`0.1.0`** from
`get_version()`. Its storage schema is independently versioned as
**`STORAGE_VERSION = 1`**.

These versions serve different purposes:

- the semantic contract version identifies the callable/observable contract
  release;
- the storage version guards the on-chain layout and migration path.

Do not infer that a storage version bump is required for every API change, or
that a semantic version bump makes an incompatible storage change safe. A
storage change must follow [Storage Versioning](storage-versioning.md) and
[Storage Migration](storage-migration.md).

## Compatibility surface

Unless an item is explicitly marked **Experimental**, the following are part
of the stable consumer contract:

- public function names, parameter order/types, and return types;
- authorization requirements and the pause/withdrawal safety guarantees;
- public return structs and their field meanings;
- numeric `ContractError` codes and their meanings;
- structured event topic symbols, topic order, and payload shapes;
- token-custody and balance-accounting semantics;
- storage-version/migration behavior that affects deployed state.

The canonical numeric error reference is
[docs/error-codes.md](error-codes.md). The canonical on-chain event schema is
[docs/event-schema.md](event-schema.md). Function naming conventions are
documented separately in [docs/api-reference.md](api-reference.md).

**Experimental surface.** There are currently no public entry points marked
Experimental. A future experimental item must be explicitly labelled
`Experimental` in source-facing documentation before merge. Experimental
items may change without the full stable-surface deprecation cycle, but their
error/event/storage effects must still be documented.

## Common rules

- `Env` is the Soroban execution environment and is shown in source
  signatures below; generated clients do not supply it as an ordinary
  application argument.
- User mutations call `user.require_auth()`. Admin mutations call
  `admin.require_auth()` and verify that the signer is the stored admin.
- Most initialized-state operations guard the current storage version. See the
  source and the storage-versioning docs for the exact migration path.
- `deposit`, `lock_funds`, and `extend_lock` are blocked while the
  emergency pause is active. `withdraw` and `withdraw_lock` remain
  available so users can exit during an incident. Read-only queries remain
  available.
- Amounts are native `i128` token units. Timestamps and durations are
  `u64` seconds.
- `list_locks` and `list_matured_locks` cap a requested page at the
  contract's `MAX_LOCK_PAGE_SIZE`.

## Public entry points

### Lifecycle and identity

| Function | Authorization | Return | Behavior / storage impact | Notable errors / event |
| --- | --- | --- | --- | --- |
| `initialize(env, admin, token)` | `admin` | `()` | One-time initialization. Stores admin, initialization flag, accepted token, and storage version. | `AlreadyInitialized`. Emits `initialize` with topics `("initialize", admin)` and token address as data. |
| `get_version(env)` | None | `String` | Returns the hard-coded semantic contract version (`0.1.0`). May be called before initialization. | If already initialized, the current storage version is checked. |
| `get_token(env)` | None | `Address` | Returns the configured Stellar Asset Contract address. | `NotInitialized`, `TokenNotConfigured`, storage-version errors. |
| `get_admin(env)` | None | `Address` | Returns the stored admin. | `NotInitialized`, `RequiredStorageEntryMissing`. |
| `transfer_admin(env, admin, new_admin)` | Current `admin` | `()` | Replaces the stored admin. The new admin is not required to sign this transfer. | `NotAuthorizedAdmin`, `CannotTransferAdminToSelf`, `CannotTransferAdminToContractAddress`, `RequiredStorageEntryMissing`. Emits `xferadmin` with old admin as topic subject and new admin as data. |

### Emergency pause

| Function | Authorization | Return | Behavior / storage impact | Notable errors / event |
| --- | --- | --- | --- | --- |
| `pause(env, admin, duration_secs)` | Current `admin` | `()` | Sets the pause flag and an expiry timestamp. A zero duration is rejected. | `PauseDurationMustBePositive`, admin/lifecycle errors. Emits `pause` with expiry as data. |
| `unpause(env, admin)` | Current `admin` | `()` | Clears the pause flag and expiry immediately. | Admin/lifecycle errors. Emits `unpause`. |
| `is_paused(env)` | None | `bool` | Reports the effective pause state. An expired pause reads as `false`; the read does not mutate storage. | `NotInitialized`. |

An expired pause is cleared lazily by the mutating pause guard. Consumers
should use `is_paused` or `get_config` rather than reading a cached pause
flag as authoritative.

### Configuration

| Function | Authorization | Return | Behavior / storage impact | Notable errors / event |
| --- | --- | --- | --- | --- |
| `set_min_deposit_amount(env, admin, min_amount)` | Current `admin` | `()` | Stores the deposit floor. `0` disables the configurable floor; deposits must still be positive. | `MinDepositAmountNegative`, admin/lifecycle errors. Emits `cfg_min` with the new amount. |
| `get_min_deposit_amount(env)` | None | `i128` | Returns the configured floor, or `0` when unset. | None from the getter itself. |
| `set_max_lock_duration(env, admin, max_duration_secs)` | Current `admin` | `()` | Stores the maximum allowed lock duration. `0` disables the upper bound. | Admin/lifecycle errors. Emits `cfg_maxlk`. |
| `get_max_lock_duration(env)` | None | `u64` | Returns the configured upper bound, or `0` when unset. | None from the getter itself. |
| `set_min_lock_duration(env, admin, min_duration_secs)` | Current `admin` | `()` | Stores the minimum allowed lock duration. `0` disables the lower bound. | Admin/lifecycle errors. Emits `cfg_minlk`. |
| `get_min_lock_duration(env)` | None | `u64` | Returns the configured lower bound, or `0` when unset. | None from the getter itself. |
| `get_config(env)` | None | `ContractConfig` | Returns token, admin, semantic version, effective pause state/expiry, and the three configurable limits in one read. | `NotInitialized`, `TokenNotConfigured`, `RequiredStorageEntryMissing`, storage-version errors. |

### Deposits and available balance

| Function | Authorization | Return | Behavior / storage impact | Notable errors / event |
| --- | --- | --- | --- | --- |
| `deposit(env, user, amount)` | `user` | `()` | Transfers `amount` from the user to the contract's configured token balance, then credits `Balance(user)`. Blocked while paused. | `AmountNotPositive`, `AmountBelowMinimumDeposit`, `TokenNotConfigured`, `ContractPaused`, lifecycle/storage errors. Emits `deposit` with data `(amount, new_balance)`. |
| `withdraw(env, user, amount)` | `user` | `()` | Debits the user's unlocked balance and transfers the token amount from the contract to the user. It does not withdraw lock entries. Available while paused. | `AmountNotPositive`, `InsufficientBalance`, `TokenNotConfigured`, lifecycle/storage errors. Emits `withdraw` with data `(amount, remaining_balance)`. |
| `get_balance(env, user)` | None | `i128` | Returns only the user's unlocked/deposited balance. | Lifecycle/storage-version errors. |
| `get_balance_snapshot(env, user)` | None | `BalanceSnapshot` | Returns unlocked, all non-withdrawn locked, total, and currently matured/withdrawable locked amounts in one read. | Lifecycle/storage-version errors. |

### Locks

| Function | Authorization | Return | Behavior / storage impact | Notable errors / event |
| --- | --- | --- | --- | --- |
| `lock_funds(env, user, amount, unlock_time)` | `user` | `u64` lock ID | Moves `amount` from unlocked balance into a new independent `LockEntry`, increments the per-user lock ID, and returns the new ID. Blocked while paused. | `AmountNotPositive`, `UnlockTimeNotInFuture`, `LockDurationExceedsMaximum`, `LockDurationBelowMinimum`, `InsufficientBalanceToLock`, `ContractPaused`, lifecycle/storage errors. Emits `lock` with data `(amount, unlock_time, new_available_balance, total_active_locked)`. |
| `extend_lock(env, user, lock_id, new_unlock_time)` | `user` | `()` | Extends an existing non-withdrawn lock to a strictly later future timestamp. Principal and token custody do not change. Blocked while paused. | `LockNotFound`, `LockAlreadyWithdrawn`, `UnlockTimeNotInFuture`, `ExtendLockTimeNotIncreased`, `ContractPaused`, lifecycle/storage errors. Emits `extend_lock` with data `(lock_id, old_unlock_time, new_unlock_time, amount)`. |
| `withdraw_lock(env, user, lock_id)` | `user` | `()` | Marks one matured lock withdrawn and transfers its principal from the contract to the user. Other locks are untouched. Available while paused. | `LockNotFound`, `LockAlreadyWithdrawn`, `LockNotMatured`, `TokenNotConfigured`, lifecycle/storage errors. Emits `withdraw_lock` with data `(lock_id, withdrawn_amount)`. |
| `get_locked_balance(env, user)` | None | `i128` | Sums all non-withdrawn lock principal, whether matured or immature. | Lifecycle/storage-version errors. |
| `can_withdraw(env, user)` | None | `bool` | Returns `true` when at least one non-withdrawn lock is mature. | Lifecycle/storage-version errors. |
| `get_lock(env, user, lock_id)` | None | `Option<LockEntry>` | Reads one user-scoped lock entry. Missing IDs return `None`. | Lifecycle/storage-version errors. |
| `list_locks(env, user, offset, limit)` | None | `Vec<LockEntry>` | Returns locks in creation order, including withdrawn entries, with offset/limit pagination. `limit == 0` returns an empty page. | Lifecycle/storage-version errors. |
| `list_matured_locks(env, user, offset, limit)` | None | `Vec<LockEntry>` | Returns only matured, non-withdrawn locks in creation order. | Lifecycle/storage-version errors. |
| `get_matured_lock_count(env, user)` | None | `u32` | Counts matured, non-withdrawn locks. | Lifecycle/storage-version errors. |
| `get_matured_balance(env, user)` | None | `i128` | Sums the principal of matured, non-withdrawn locks. | Lifecycle/storage-version errors. |
| `get_lock_summary(env, user)` | None | `LockSummary` | Returns aggregate non-withdrawn lock counts/amounts, matured subset totals, and earliest/latest unlock timestamps. | Lifecycle/storage-version errors. |

## Public return types

### `LockEntry`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | `u64` | Per-user monotonically increasing lock ID. |
| `owner` | `Address` | Lock owner. |
| `amount` | `i128` | Locked token principal. |
| `created_time` | `u64` | Ledger timestamp at lock creation. |
| `unlock_time` | `u64` | Earliest maturity timestamp. |
| `withdrawn` | `bool` | Whether this lock has already been paid out. |

### `BalanceSnapshot`

| Field | Type | Meaning |
| --- | --- | --- |
| `unlocked` | `i128` | Available deposited balance. |
| `locked` | `i128` | Principal in all non-withdrawn locks. |
| `total` | `i128` | `unlocked + locked`. |
| `withdrawable` | `i128` | Principal in matured, non-withdrawn locks. |

### `LockSummary`

| Field | Type | Meaning |
| --- | --- | --- |
| `active_count` | `u32` | Number of non-withdrawn locks. |
| `total_locked_amount` | `i128` | Principal across non-withdrawn locks. |
| `matured_count` | `u32` | Matured, non-withdrawn lock count. |
| `withdrawable_amount` | `i128` | Matured, non-withdrawn principal. |
| `earliest_unlock` | `u64` | Earliest unlock timestamp among non-withdrawn locks; `0` when none exist. |
| `latest_unlock` | `u64` | Latest unlock timestamp among non-withdrawn locks; `0` when none exist. |

### `ContractConfig`

| Field | Type | Meaning |
| --- | --- | --- |
| `token` | `Address` | Accepted token contract. |
| `admin` | `Address` | Current admin. |
| `version` | `String` | Semantic contract version (`0.1.0` at this reference). |
| `paused` | `bool` | Effective pause state after expiry handling. |
| `pause_expiry` | `u64` | Pause-expiry timestamp, or `0`. |
| `min_deposit_amount` | `i128` | Configured deposit floor; `0` disables it. |
| `max_lock_duration` | `u64` | Configured upper duration bound; `0` disables it. |
| `min_lock_duration` | `u64` | Configured lower duration bound; `0` disables it. |

## Errors

`ContractError` is a `#[contracterror]` enum with stable numeric
discriminants. Consumers should branch on numeric code, not panic/debug text.

Current categories are:

| Range | Category |
| --- | --- |
| 1001–1099 | Validation/configuration values |
| 2001–2099 | Authorization |
| 3001–3099 | Lifecycle/pause |
| 4001–4099 | Accounting |
| 5001–5099 | Lock state |
| 6001–6099 | Storage |
| 7001–7099 | Token configuration |
| 8001–8099 | Admin rotation |

See the canonical [Savings Vault Error Reference](error-codes.md) for every
code, raising function, meaning, and caller guidance. Renumbering or
repurposing an existing code is a breaking API change.

## Events

Mutating functions publish structured Soroban events. The compact summary
below is for navigation; [Savings Vault Event Schema](event-schema.md) is the
canonical decoding contract.

| Function | Event symbol | Topic subject | Data |
| --- | --- | --- | --- |
| `initialize` | `initialize` | admin | token address |
| `pause` | `pause` | admin | expiry timestamp |
| `unpause` | `unpause` | admin | unit |
| `set_min_deposit_amount` | `cfg_min` | admin | minimum amount |
| `set_max_lock_duration` | `cfg_maxlk` | admin | maximum duration |
| `set_min_lock_duration` | `cfg_minlk` | admin | minimum duration |
| `deposit` | `deposit` | user | `(amount, new_balance)` |
| `withdraw` | `withdraw` | user | `(amount, remaining_balance)` |
| `lock_funds` | `lock` | user | `(amount, unlock_time, available_balance, active_locked)` |
| `extend_lock` | `extend_lock` | user | `(lock_id, old_unlock_time, new_unlock_time, amount)` |
| `withdraw_lock` | `withdraw_lock` | user | `(lock_id, withdrawn_amount)` |
| `transfer_admin` | `xferadmin` | old admin | new admin |

Diagnostic `log!` strings are **not** part of the public event contract.

## Storage implications

Consumers must use contract entry points rather than read raw contract
storage. The storage model still matters for compatibility and upgrades:

- instance storage carries admin, initialization state, token address, storage
  version, pause state/expiry, and configurable limits;
- persistent storage carries per-user available balances, individual
  `LockEntry` records keyed by `(user, lock_id)`, and the next per-user lock
  ID;
- locking moves accounting value from available balance into a lock entry but
  does not move tokens out of contract custody;
- `deposit` transfers configured-token custody from user to contract;
  `withdraw` and `withdraw_lock` transfer custody from contract to user.

Changing the meaning or keying of deployed persistent data is a storage
compatibility change even if every public function signature stays the same.
Follow the migration documents and bump `STORAGE_VERSION` when the deployed
layout requires migration.

## Change classification

### Compatible / additive

Examples that can remain compatible when correctly documented and tested:

- adding a new read-only function without changing existing behavior;
- adding a new event for a brand-new function without changing an existing
  event's schema;
- adding a new error code in the appropriate unused category for a new path;
- implementation refactors that preserve observable inputs, outputs, auth,
  errors, events, accounting, and storage semantics;
- documentation corrections that make this reference match source.

Additive changes still require this reference to be updated in the same PR.

### Experimental

An item is experimental only when it is explicitly labelled that way in this
reference (and, where practical, in its source documentation). Experimental
surface must still define auth, errors, events, and storage effects. Promotion
to stable requires a normal API review.

### Breaking

Treat a change as breaking when it does any of the following to an existing
stable surface:

- removes/renames a function or changes parameter order/type or return shape;
- adds a stricter authorization requirement or changes who may withdraw;
- renumbers, removes, or changes the meaning of an existing error code;
- changes an existing event symbol, topic order/subject, payload order/type, or
  removes an event clients may consume;
- changes token custody, balance/lock accounting, maturity boundaries, or
  pause safety behavior;
- changes public struct fields in a way that existing generated clients cannot
  decode compatibly;
- changes deployed storage semantics without a compatible migration;
- turns an input that was previously valid into an ordinary failure without a
  documented compatibility decision.

The current semantic version is pre-1.0 (`0.1.0`), so callers must not infer
SemVer 1.x guarantees from the major component. Even so, a breaking change
must be explicitly labelled **BREAKING**, update the hard-coded contract
version and changelog, and be coordinated with SDK/mobile consumers. Once the
contract reaches 1.x, stable-surface breaking changes require a major semantic
version bump.

## Review requirements for API-affecting changes

A PR that touches the public API must include an API compatibility review in
addition to its ordinary issue acceptance evidence:

1. **Surface diff:** list every changed function signature, public struct,
   authorization rule, error code, event, or accounting/storage behavior.
2. **Classification:** mark each change compatible, experimental, or breaking,
   with rationale.
3. **Errors/events:** update
   [error-codes.md](error-codes.md) and/or
   [event-schema.md](event-schema.md) in the same PR when affected.
4. **Storage:** if deployed layout/semantics change, update storage-version and
   migration docs and provide focused migration/state-preservation coverage.
5. **Consumers:** identify SDK/mobile/indexer impact and any coordinated
   release or regeneration required.
6. **Focused validation:** run the tests that exercise the changed contract
   behavior and compatibility boundary. Documentation-only source-sync changes
   do not require unrelated contract suites.
7. **Reference:** update this file and any README/API navigation links before
   requesting review.

A maintainer should not approve an API-affecting PR while this reference says
something different from the source.

## Related documentation

- [Canonical error codes](error-codes.md)
- [Structured event schema](event-schema.md)
- [Storage versioning](storage-versioning.md)
- [Storage migration](storage-migration.md)
- [Function naming conventions](api-reference.md)
- [Lock read helpers](lock-read-helpers.md)
- [Matured-lock discovery](matured-lock-discovery.md)
- [SDK error mapping](sdk-error-mapping-guide.md)
