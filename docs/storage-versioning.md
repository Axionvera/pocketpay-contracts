# Savings Vault Storage Versioning

The contract's **storage schema marker** and **WASM release version** are separate.
For the actual key/value layout, compatibility considerations, upgrade review,
and required focused migration evidence, see the
[Storage Migration and Upgrade Review](storage-migration.md).

## Current behavior

The code in `contracts/savings_vault/src/lib.rs` defines
`STORAGE_VERSION: u64 = 1`, stored at instance
`DataKey::StorageVersion`. `get_version()` currently returns
`"0.1.0"` as a separate release identifier.

| StorageVersion | Behavior |
| --- | --- |
| `1` | Current schema; proceed |
| Absent | `try_migrate()` reads `0` and writes a `1` marker, preserving legacy v0 data |
| `2` or any unsupported value | Reject with `StorageVersionUnsupported` (6001); no downgrade |

The only implemented migration is marker adoption from v0 to v1. There is
**no v1→v2 data migration yet**. Before changing any storage value layout,
the contract must gain the exact transformation for older supported state.
Bumping `STORAGE_VERSION` on its own is unsafe.

The v0→v1 path writes a log record but **does not publish a migration event**.
Future migration events should only be documented as supported after they
exist in the contract and their topics/payloads have focused tests.

## Required guarantees for future upgrades

- Atomic invocations: a failed on-chain call must not leave partial writes.
- Preservation of deposited, available, locked, matured and withdrawn
  principal, owner/lock identities, contract-held tokens, and authorization.
- Explicit handling of unsupported future schema markers.
- Documented instance/persistent storage TTL implications.
- Targeted before/after storage tests with representative old-format values.
- SDK/mobile compatibility plan for changed public state reads, events and
  error codes.
- Forward-fix/rollback limitations. **Older WASM is not automatically a safe
  rollback**, and the current version guard rejects schema downgrades.

## Existing focused evidence

In `contracts/savings_vault/src/test/storage_version.rs`:

- `test_initialize_sets_storage_version_1`
- `test_legacy_missing_storage_version_works`
- `test_invalid_storage_version_fails_safely`

These cover current marker logic, **not** a future v2 migration. A new schema
requires new focused tests as described in
[Storage Migration and Upgrade Review](storage-migration.md).

See also the [Storage Change Checklist](storage-change-checklist.md),
[Contract Upgrade Strategy](upgrade-strategy.md), and
[Storage Audit](storage-audit.md).
