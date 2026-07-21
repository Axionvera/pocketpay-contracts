# Storage Migration Guide

## Overview
This document describes the storage versioning and migration strategy for the Stellar PocketPay Savings Vault contract. The goal is to ensure safe, reversible, and well-documented upgrades to the contract's storage layout as features evolve.

## Storage Versioning
The contract uses a `StorageVersion` key stored in instance storage to track the current schema version. The current version is defined as a constant `STORAGE_VERSION` in `lib.rs`.

### Version History
| Version | Changes | Migration Required |
|---------|---------|--------------------|
| 1       | Initial version with token transfers and multi-locks | No |

## Migration Pattern

### How Migrations Work
1. Every public contract function calls `try_migrate()` first, which checks the current storage version
2. If the stored version is older than `STORAGE_VERSION`, the appropriate migrations are executed
3. Migrations are incremental: version N → N+1, N+1 → N+2, etc.
4. Migrations are logged using Soroban's logging system for auditability
5. If a migration fails or an invalid version is encountered, the contract panics (no state changes applied)

### Implementing a New Migration
When you need to make a breaking change to the storage layout:
1. **Increment `STORAGE_VERSION`**: Update the constant in `lib.rs` to the next integer
2. **Add a Migration Case**: In `try_migrate()`, add a new match arm for the old version
3. **Test Extensively**: Write unit and integration tests for the migration scenario
4. **Update This Document**: Add the new version to the Version History table
5. **Update CHANGELOG.md**: Document the storage change and migration

#### Example: Migration from v1 to v2
```rust
// In lib.rs:
pub const STORAGE_VERSION: u64 = 2;

// In try_migrate():
match current_version {
    0 => { /* existing v0→v1 migration */ },
    1 => {
        // Example: Add a new metadata field to user balances
        // ... migration logic here ...
        env.storage().instance().set(&DataKey::StorageVersion, &2u64);
        log!(env, "Migrated storage from version 1 to version 2");
    }
    _ => panic!("Unsupported storage version: {}", current_version),
}
```

## Safety Checks
- **No Partial Migrations**: Migrations are designed to be atomic - either all steps succeed or the contract panics with no changes applied
- **Version Guards**: The contract panics if it encounters a storage version newer than `STORAGE_VERSION` to prevent accidental downgrades
- **Test Migrations**: Always test migrations thoroughly in a local environment first using the Soroban test framework
- **Immutable Migrations**: Once a migration is deployed to mainnet, it must never be changed; instead, add a new migration

## Testing Migrations
To test migration scenarios:
1. Use the test environment to set up a contract at an older storage version
2. Call a contract function (which triggers `try_migrate`)
3. Verify that the storage layout is correctly upgraded
4. Verify that all existing functionality still works

See the test file `test/mod.rs` for examples (TODO: add migration-specific tests).

## Rollback Strategy
If a deployed migration causes issues:
1. **DO NOT deploy a downgraded contract version** (the version guard will panic)
2. Instead, deploy a new version with a fix and an incremental migration
3. Consider emergency pausing if available (see `pause-design.md`)

## Example Migration Scenarios (Hypothetical)
- v0 → v1: Initial version, added token address and version storage
- v1 → v2: Add per-user metadata or new lock features
- v2 → v3: Change serialization format or move storage to different buckets

## Links
- [Contract Implementation](../contracts/savings_vault/src/lib.rs)
- [Architecture Documentation](architecture.md)
- [CHANGELOG.md](../CHANGELOG.md)
