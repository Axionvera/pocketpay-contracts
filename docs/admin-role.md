# Admin Role — Savings Vault

This document explains what the `admin` address recorded by `initialize(admin)` currently stores, what (if anything) the admin can do today, and design considerations for future admin powers.

## What `initialize(admin)` stores

- The contract records the `admin` address in instance storage under the `Admin` key.
- It also sets an `Initialized` flag so `initialize()` can only be called once.
- The recorded admin address is required to have signed the `initialize()` transaction (the function calls `admin.require_auth()`).

## Current admin capabilities (today)

- None beyond being recorded in contract storage. The contract currently does not implement any admin-only functions such as pausing, migrating, fund recovery, or changing settings.
- The `admin` value may be used by off-chain tooling or by future on-chain upgradeable/admin-only functions if added later.

## What the admin cannot do (today)

- Cannot pause contract execution or halt deposits/withdrawals.
- Cannot migrate or sweep funds from user balances.
- Cannot recover or forcibly withdraw user funds.
- Cannot upgrade the contract (no `upgrade()` or proxy mechanism is present).
- Cannot change user balances or unlock times except via the existing user-authorized functions (which call `require_auth()` on the user address).

## Security & trust implications

- Recording an admin address by itself does not grant powers; the contract's code determines those powers. At present, storing the admin is informational and preparatory only.
- Users and auditors should treat the admin as inert unless/until admin-only functions are explicitly added and documented.

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

## Acceptance checklist

- [x] Admin role documentation exists.
- [x] Docs explain what `initialize(admin)` stores.
- [x] Docs explain current admin capabilities (none beyond storage).
- [x] Docs explain what admin cannot do.
- [x] Docs mention future admin design considerations.

If you want, I can expand this file with recommended admin function implementations (pause, migrate, multisig examples) and accompanying tests.
