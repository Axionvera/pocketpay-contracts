# Architecture Documentation

## Overview

This repository contains the **Stellar PocketPay – Savings Vault Contract**. The contract is written in **Rust** and compiled to WebAssembly (WASM) to run on the **Soroban** blockchain platform. The architecture focuses on clear separation of concerns, deterministic on‑chain state management, and future extensibility for SDK integration.

---

## Project Structure

```text
stellar-pocketpay-contracts/
├── Cargo.toml                 # Workspace root
├── .gitignore
├── README.md
├── docs/
│   └── architecture.md        # ← This document
└── contracts/
    └── savings_vault/
        ├── Cargo.toml          # Contract crate
        └── src/
            ├── lib.rs          # Contract implementation
            └── test.rs         # Unit tests
```

The `contracts/savings_vault` directory houses the on‑chain logic. All other files are tooling, documentation, and repository metadata.

---

## State Management & Storage

The contract uses **Soroban SDK storage primitives**:

- **Persistent storage** (`storage::set`, `storage::get`) for user balances. Data stored here survives ledger expiry and is the source of truth for the vault.
- **Instance storage** for the admin address and initialization flag. This is scoped to the contract instance and cleared when the contract is removed.

The state model is deliberately simple:

| Key                | Type   | Description |
|--------------------|--------|-------------|
| `balance:{user}`   | `i128` | Unlocked funds available to a user.
| `locks:{user}`     | `Vec<LockEntry>` | List of active and matured lock entries for a user.
| `next_lock_id:{user}` | `u64`| Monotonically increasing next lock ID for a user.
| `admin`            | `Address` | Contract admin (set during `initialize`).
| `initialized`      | `bool`   | Guard to ensure `initialize` runs only once.

All operations validate inputs (non‑negative amounts, sufficient balances, future unlock times) and emit descriptive `require_auth` checks.

---

## Secure Storage

On‑chain storage is inherently **secure**: data is stored in the ledger and can only be modified by authorized contract calls. The contract enforces authentication using `require_auth(env, caller)` for any state‑changing function, ensuring that only the address owning the funds can deposit, withdraw, or lock them.

---

## Stellar SDK Integration

The contract depends on the **Soroban SDK** (part of the Stellar ecosystem) for:

- **Environment handling** (`Env`) – provides access to ledger data and transaction context.
- **Address and authentication** – `Address` type and `require_auth` enforce permissions.
- **Storage APIs** – `storage::set`, `storage::get`, and `storage::has` for deterministic on‑chain state.
- **Testing utilities** – `testutils` to simulate ledger operations in unit tests.

Future enhancements may integrate the **Stellar Asset Contract (SAC)** to enable real token transfers, moving beyond internal balance bookkeeping.

---

## Future SDK Boundary

The current contract is a **stand‑alone savings vault**. To evolve into a full‑featured wallet SDK, consider the following extension points:

1. **Token Transfer Layer** – Call the SAC `transfer` function to move XLM or custom assets on‑chain.
2. **Admin Recovery & Upgrade** – Implement admin‑controlled migration or upgrade mechanisms using Soroban `upgrade` primitives.
3. **Off‑chain SDKs** – Provide JavaScript/TypeScript client libraries that abstract contract calls, handling address resolution, transaction building, and signing.

These boundaries maintain a clean separation between **on‑chain logic** (this repository) and **off‑chain SDKs** that developers will consume.

---

## Navigation (Documentation)

- The **README.md** provides quick‑start guides for building, testing, and deploying the contract.
- This **architecture.md** offers a deeper dive into internal design.
- Additional module‑level docs (e.g., `admin-role.md`) cover specific responsibilities.

Refer to the **Documentation** section of the README for links to all docs.

---

## Contributing

When contributing, keep the following in mind:

- Follow the existing storage conventions.
- Write unit tests for any new state transitions.
- Update this architecture document if you add new modules or change the state model.

---

*Last updated: 2026‑07‑17*
