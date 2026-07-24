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
| `lock:{user}:{id}` | `LockEntry`| An individual active or matured lock entry for a user.
| `next_lock_id:{user}` | `u64`| Monotonically increasing next lock ID for a user.
| `admin`            | `Address` | Contract admin (set during `initialize`).
| `initialized`      | `bool`   | Guard to ensure `initialize` runs only once.

All operations validate inputs (non‑negative amounts, sufficient balances, future unlock times) and emit descriptive `require_auth` checks.

---

## Token-Backed Accounting and Asset Custody

The contract integrates with the **Stellar Asset Contract (SAC)** interface to manage real token custody:

- Calling `deposit` transfers the specified token amount from the user's wallet to the contract's address via `token_client.transfer` before updating internal persistent storage balances.
- Calling `withdraw` or `withdraw_lock` transfers the specified token amount from contract custody back to the user's wallet before updating internal balances or lock states.

Internal accounting (`Balance(user)` and `Lock(user, lock_id)`) reconciles 1:1 with real SAC token balances held at the contract address. If a token transfer reverts or fails (e.g., due to insufficient balance or allowance), the entire Soroban transaction rolls back with zero state changes.

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

The contract integrates with the **Stellar Asset Contract (SAC)** for real token custody on deposit, withdraw, and `withdraw_lock`. Internal persistent storage reconciles 1:1 with tokens held at the contract address.

---

## Future SDK Boundary

The current contract is a **stand‑alone savings vault**. To evolve into a full‑featured wallet SDK, consider the following extension points:

1. **Admin Recovery & Upgrade** – Implement admin‑controlled migration or upgrade mechanisms using Soroban `upgrade` primitives.
2. **Structured Errors** – Replace panic strings with a `#[contracterror]` enum for SDK/mobile callers.
3. **Off‑chain SDKs** – Provide JavaScript/TypeScript client libraries that abstract contract calls, handling address resolution, transaction building, and signing.

These boundaries maintain a clean separation between **on‑chain logic** (this repository) and **off‑chain SDKs** that developers will consume.

---

## Navigation (Documentation)

- The **README.md** provides quick‑start guides for building, testing, and deploying the contract.
- This **architecture.md** offers a deeper dive into internal design.
- [**sdk-contract-sequence.md**](sdk-contract-sequence.md) shows the end‑to‑end request flow (mobile → SDK → Soroban RPC → vault contract) for balance queries, deposits, withdrawals, and error paths.
- [**api-reference.md**](api-reference.md) documents the naming convention followed by `SavingsVault`'s public functions.
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
