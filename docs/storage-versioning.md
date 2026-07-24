# Storage Versioning and Migration Behavior

## Overview

The Savings Vault contract uses storage versioning to ensure safe compatibility between different contract versions and support future migrations. This document defines how versioning works, expected migration behavior, and compatibility guarantees.

---

## Storage Version

The contract uses `DataKey::StorageVersion` stored in **instance storage** to track the version of the storage layout.

### Current Version

- `1`: Initial versioned storage layout (supports balances, locks, admin, token, initialized flag)

---

## Compatibility Guarantees

### Backward Compatibility

- **Missing StorageVersion**: Contracts deployed before versioning was added are treated as version `1` (backward compatible, no changes to existing storage).
- **Supported Versions**: All future contract versions must explicitly support all previous storage versions (or panic with `Unsupported storage version`).

### Forward Compatibility

- Newer versions of the contract can choose to support older storage versions (via migration or fallback handling).
- Unsupported versions cause the contract to panic safely (`Unsupported storage version: X`).

---

## Expected Migration Behavior

Migrations should follow these principles:

1. **Atomicity**: Migrations must either fully succeed or leave storage unchanged (utilize Soroban's transaction atomicity).
2. **Safety**: Migrations must never corrupt user balances or lock entries.
3. **Transparency**: Migrations must emit events so off-chain monitors can verify progress.
4. **Opt-in**: Users should have the choice to migrate (or not) whenever possible.

### Migration Path Example (Version 1 → 2)

If a future version (v2) changes the storage layout, it should:
1. Detect if storage is version 1 (via `DataKey::StorageVersion`).
2. Migrate the storage to the new layout.
3. Set `DataKey::StorageVersion` to 2.
4. Emit a `StorageMigrated` event.
5. Fail safely and revert if any step fails.

---

## Testing Coverage

The test suite covers these scenarios:
- `test_initialize_sets_storage_version_1`: Initialization sets StorageVersion to `1`.
- `test_legacy_missing_storage_version_works`: Legacy pre-versioning contracts function normally (treated as version `1`).
- `test_invalid_storage_version_fails_safely`: Unsupported storage versions cause a panic.

---

## Links

- [Contract Upgrade Strategy](upgrade-strategy.md)
- [Comprehensive Codebase Analysis](comprehensive-analysis.md)
- [Security Review](security-review.md)
