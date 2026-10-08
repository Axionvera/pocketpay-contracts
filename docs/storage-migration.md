# Savings Vault State Migration and Upgrade Review

This guide records the **current implemented storage contract** in
[`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs)
and the review requirements for any change that persists new keys or
changes existing encoded values. It is not an automatic or reversible
migration engine.

The Savings Vault is intended for testnet/educational use; an implementation
or documentation change alone does not prove an existing deployed contract is
upgradeable or safe to migrate.

## Actual storage model at schema v1

`STORAGE_VERSION: u64 = 1` is a **storage schema version**, recorded under
`DataKey::StorageVersion` in **instance storage**. It is not the same as
`get_version()`, which currently returns the independent WASM release string
`0.1.0`.

| Storage area | `DataKey` | Encoded value / contract interpretation |
| --- | --- | --- |
| Instance | `Initialized` | `bool` flag after initialization |
| Instance | `Admin` | authorized admin `Address` |
| Instance | `Token` | accepted token contract `Address` |
| Instance | `StorageVersion` | `u64` schema marker, currently 1 |
| Instance | `Paused` / `PauseExpiry` | pause flag / expiration timestamp `u64` |
| Instance | `MinDepositAmount` | optional `i128` floor; 0 disables it |
| Instance | `MinLockDurationSecs` / `MaxLockDurationSecs` | `u64` duration bounds; 0 disables each |
| Persistent | `Balance(Address)` | available, non-locked `i128` amount |
| Persistent | `Lock(Address, u64)` | `LockEntry` for one owner and monotonically assigned lock ID |
| Persistent | `NextLockId(Address)` | next `u64` lock ID; defaults to 1 |
| Declared legacy helper | `Locks(Address)` | a legacy enum variant/helper; do not assume it replaces the per-ID `Lock` storage |

A `LockEntry` includes `id: u64`, `owner: Address`, `amount: i128`,
`created_time: u64`, `unlock_time: u64`, and `withdrawn: bool`.
After a successful `withdraw_lock`, the individual lock entry remains
stored with `amount = 0` and `withdrawn = true`; it is **not deleted**.
The available balance and locked balance represent different principal
buckets; matured locks remain in locked balance until separately withdrawn.

Do not change a `DataKey` variant, key tuple, serialized `LockEntry`
shape, storage tier, or ledger TTL expectation without an explicit existing
state compatibility plan. Instance and persistent storage have different
lifecycle/expiration behavior; copying keys across them is not a transparent
upgrade.

## What the current migration code really does

The existing `try_migrate(&env)` reads `StorageVersion` with
`unwrap_or(0)` and supports exactly the following cases:

| Stored marker | Current behavior | Result |
| --- | --- | --- |
| `1` | Current schema | Returns successfully without changes |
| Missing (`0`) | Legacy v0 to v1 compatibility | Stores marker `1`; existing user balances/locks are left unchanged |
| Greater than `1` or otherwise unsupported | `StorageVersionUnsupported` (`6001`) | Contract invocation fails; no downgrade |
| `NotInitialized` | Mutating/read helper requiring initialization calls its initialization guard first | Fails with `NotInitialized` (`3002`) |

`initialize` invokes migration before writing the initialized admin/token
configuration, then explicitly stores schema version 1. For an already
initialized contract, `get_version` also checks/migrates the marker before
returning the release version string. Other stateful entry points guard
initialization and check the storage schema before accessing state.

**There is no implemented v1 → v2 state transformation** today. The
`try_migrate` function writes a `log!` message for legacy marker adoption;
it does **not emit a dedicated Soroban migration contract event**. SDKs must
not subscribe to a nonexistent `StorageMigrated` event or assume old or
future schemas can be loaded by this WASM.

## Planned storage changes: required review and implementation

1. Identify precisely which keys and encoded structs change. Record
   old/new values, storage tier, existing-account coverage, TTL and
   authorization assumptions.
2. Decide whether the change is compatible with existing on-chain state.
   Preserve old encoded reads where practical. If a migration is required,
   specify the ordered transformation **before** raising `STORAGE_VERSION`.
3. Add explicit version branches for every supported prior version.
   **Do not merely bump the constant**: today's v0 migration writes the
   current constant without applying a future v1 → v2 transformation. A
   version bump without a matching migration can strand or misdecode funds.
4. Reject unknown future markers (`6001`) rather than silently assuming
   compatibility; keep required-entry failures explicit (`6002`).
5. Preserve balance conservation, lock IDs, locked/withdrawn flags, token
   custody, admin authorization, pause behavior and transaction rollback.
6. Update the migration version table, `CHANGELOG.md`, contract interface
   documentation, SDK/mobile compatibility fixtures, and the
   [Storage Change Checklist](storage-change-checklist.md) in the same PR.
7. Propose a deployment order, upgraded-contract proof, and a **forward-fix**
   recovery plan. Soroban contract state changes are not assumed reversible:
   the current guard intentionally rejects downgrades.

Any new event must be explicitly implemented and tested before advertising
it to SDK/mobile indexers. Logs alone are not a stable contract event API.

## Tests expected in a storage-changing PR

The current focused tests are in
[`contracts/savings_vault/src/test/storage_version.rs`](../contracts/savings_vault/src/test/storage_version.rs):

- `test_initialize_sets_storage_version_1` — initialization writes schema 1.
- `test_legacy_missing_storage_version_works` — markerless initialized state
  retains its data while adopting v1.
- `test_invalid_storage_version_fails_safely` — unsupported schema does not
  silently continue.

A real new schema or key-shape change must **add focused migration regressions**
for every supported old layout, unknown newer versions, missing required
entries, token balances, available/locked principal, live/matured/withdrawn
locks, and rollback on failed transitions. Use the existing Soroban test
harness with representative pre-upgrade state rather than fabricating
pass results or running the full suite by default.

Also review [Storage Versioning](storage-versioning.md),
[Storage Audit](storage-audit.md),
[Accounting Invariants](accounting-invariants.md), and
[Contract Error Codes](error-codes.md) for effects on dependent consumers.

## Compatibility and rollback boundaries

- A WASM `get_version()` string is informational; it is not the on-chain
  storage migration marker.
- Do not assume keys remain readable after changing an enum's serialized
  representation, replacing `LockEntry`, or altering storage tier.
- Do not assume an authorized source wallet has approved operations on
  another user's stored lock/balance; existing `require_auth` boundaries
  must remain intact.
- Reject unsupported versions and avoid partially applied state changes.
  Soroban's transaction atomicity protects a reverted invocation, but does
  not prove a multi-deployment upgrade is safe.
- Never promise restoration by simply deploying an older WASM. Prefer
  a reviewed forward migration/fix and documented operational recovery,
  preserving withdrawal paths where the current contract permits them.

**Release gate:** a storage-changing pull request is incomplete until its
current-state schema table, migration path or explicit no-migration rationale,
targeted compatibility evidence, and the completed checklist are reviewable.
