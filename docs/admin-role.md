# Admin Role — Savings Vault

This document explains what the `admin` address recorded by `initialize(admin)` currently stores, what the admin can do today, and design considerations for future admin powers.

## What `initialize(admin)` stores

- The contract records the `admin` address in instance storage under the `Admin` key.
- It also sets an `Initialized` flag so `initialize()` can only be called once.
- The recorded admin address is required to have signed the `initialize()` transaction (the function calls `admin.require_auth()`).

## Current admin capabilities

### `get_admin()`
- **Access**: Public, no authorization required.
- **Description**: Returns the current admin address stored in instance storage.
- **State changes**: None (read-only).
- **Panics**: If the contract has not been initialized yet.

### `transfer_admin(new_admin)`
- **Access**: Admin-only.
- **Description**: Transfers admin privileges from the current admin to a new address.
- **Authorization**: Must be called by the current admin (requires `admin.require_auth()`).
- **State changes**: Updates the `Admin` key in instance storage to the new admin address, emits a `transfer_admin` event.
- **Panics**: If the contract has not been initialized, or if the caller is not the current admin.

## What the admin cannot do

- Cannot pause contract execution or halt deposits/withdrawals.
- Cannot migrate or sweep funds from user balances.
- Cannot recover or forcibly withdraw user funds.
- Cannot upgrade the contract (no `upgrade()` or proxy mechanism is present).
- Cannot change user balances or unlock times except via the existing user-authorized functions (which call `require_auth()` on the user address).

## Security & trust implications

- The admin's powers are currently limited to transferring admin rights; they cannot access or modify user funds.
- Users and auditors should review any future changes to the admin's capabilities carefully.
- Multi-signature (multisig) administration is recommended for the admin key to reduce the risk of a single point of failure.

## Future design considerations

When adding admin capabilities in the future, consider the following best practices:

- Principle of least privilege: give admin only the minimal necessary powers.
- Multi-signature or multisig guardianship: require multiple parties to authorize sensitive admin actions.
- Timelocks and delays: make critical changes subject to delays and on-chain announcements to allow user reaction time.
- Emergency pause vs. recovery: separate a limited emergency pause from powerful recovery/migration privileges.
- On-chain governance: consider decentralizing critical powers to a DAO or governance contract.
- Upgrade patterns: if supporting upgrades, prefer transparent proxy patterns, clearly documented migration steps, and on-chain governance or multisig protection.

## Where to find this in the code

- The admin value is stored under `DataKey::Admin` in [`contracts/savings_vault/src/lib.rs`](contracts/savings_vault/src/lib.rs).
- Admin helper functions: `assert_initialized()`, `assert_supported_storage_version()`, `assert_admin()`.
- Admin functions: `get_admin()`, `transfer_admin()`.

## Acceptance checklist

- [x] Admin role documentation exists.
- [x] Docs explain what `initialize(admin)` stores.
- [x] Docs explain current admin capabilities.
- [x] Docs explain what admin cannot do.
- [x] Docs mention future admin design considerations.

If you want, I can expand this file with recommended admin function implementations (pause, migrate, multisig examples) and accompanying tests.
