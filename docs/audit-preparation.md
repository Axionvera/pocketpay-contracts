# Audit Preparation Checklist — Savings Vault

> **Status:** In Progress (not audit-ready)
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This checklist identifies what documentation, test coverage, threat model items, deployment details, and known limitations must be in place before the project is submitted for an external security review or formal audit. Completing this checklist does **not** mean the contract is audit-ready; it means the groundwork required to enter an audit productively has been laid.

---

## Table of Contents

1. [Code Freeze & Scope](#1-code-freeze--scope)
2. [Contract API Documentation](#2-contract-api-documentation)
3. [Storage Model Documentation](#3-storage-model-documentation)
4. [Threat Model](#4-threat-model)
5. [Test Coverage](#5-test-coverage)
6. [Known Limitations](#6-known-limitations)
7. [Deployment & Network Assumptions](#7-deployment--network-assumptions)
8. [Unresolved Design Questions](#8-unresolved-design-questions)
9. [Supporting Documentation](#9-supporting-documentation)

---

## 1. Code Freeze & Scope

Before engaging an auditor, the codebase must be stable and the audit boundary clearly defined.

- [ ] A specific git commit or tag has been chosen as the audit target.
- [ ] No new features or refactors are merged while the audit is in progress.
- [ ] The audit scope is documented: which contracts and files are in-scope vs. out-of-scope.
- [ ] All dependencies (Soroban SDK version, Rust toolchain version) are pinned in `Cargo.toml` / `Cargo.lock`.
- [ ] The compiled WASM artifact at the audit commit is reproducible (same hash across clean builds).
- [ ] A CHANGELOG or commit log is available so the auditor can understand recent changes.

---

## 2. Contract API Documentation

Auditors need a complete, accurate description of every public function.

- [ ] Every public function is listed with its name, parameters, parameter types, return type, and a brief description of its effect.
- [ ] Authorization requirements are documented for each function (who must sign, which `require_auth()` calls are made).
- [ ] Side effects — storage writes, event emissions — are listed per function.
- [ ] Error conditions and panic messages for each function are documented (see [error-codes.md](error-codes.md)).
- [ ] Read-only vs. state-changing functions are clearly distinguished.
- [ ] The current public API surface is:

| Function | Auth Required | State-Changing | Description |
|---|---|---|---|
| `initialize(admin)` | `admin` | Yes | One-time setup; records the admin address and initialization flag. |
| `deposit(user, amount)` | `user` | Yes | Adds `amount` to the user's internal available balance. |
| `withdraw(user, amount)` | `user` | Yes | Subtracts `amount` from the user's internal available balance. |
| `get_balance(user)` | None | No | Returns the user's available (unlocked) internal balance. |
| `lock_funds(user, amount, unlock_time)` | `user` | Yes | Moves `amount` from available balance to the locked bucket until `unlock_time`. |
| `get_locked_balance(user)` | None | No | Returns the user's total locked internal balance. |
| `can_withdraw(user)` | None | No | Returns `true` if any locked funds have passed their unlock timestamp. |

---

## 3. Storage Model Documentation

Auditors must understand every storage key, its type, its lifetime, and who can write it.

- [ ] Every storage key is listed with its type, storage tier (persistent vs. instance), and read/write access rules.
- [ ] The difference between persistent and instance storage, and the TTL implications of each, are explained (see [storage-ttl.md](storage-ttl.md)).
- [ ] Initialization guard logic is documented (the `Initialized` flag preventing re-initialization).
- [ ] The current storage model is:

| Key | Type | Storage Tier | Writer | Description |
|---|---|---|---|---|
| `balance:{user}` | `i128` | Persistent | `deposit`, `withdraw`, `lock_funds` | User's available unlocked balance. |
| `locks:{user}` | `Vec<LockEntry>` | Persistent | `lock_funds` | List of active and matured lock entries for the user. |
| `next_lock_id:{user}` | `u64` | Persistent | `lock_funds` | Monotonically increasing ID assigned to each lock. |
| `Admin` | `Address` | Instance | `initialize` | Contract admin address recorded at initialization. |
| `Initialized` | `bool` | Instance | `initialize` | Guard flag preventing re-initialization. |

- [ ] Storage TTL behavior under ledger archival is documented and tested.
- [ ] Consequences of instance storage expiry (contract removal) on user balances are explained.

---

## 4. Threat Model

A threat model documents what the contract is designed to protect, who the trusted parties are, and what attack surfaces exist.

- [ ] Assets at risk are identified (user internal balances; future: real token custody).
- [ ] Trusted parties and their current capabilities are listed:
  - **Admin**: Recorded in storage; currently has no privileged on-chain capabilities beyond initialization.
  - **Users**: Can only affect their own balances via `require_auth()`-gated functions.
  - **Contract deployer**: Identical to admin at initialization time; no post-deploy powers.
- [ ] Untrusted inputs and their validation are documented:
  - `amount` parameters: validated to be strictly positive.
  - `unlock_time`: validated to be strictly in the future.
  - Balances: validated against available balance before deduction.
- [ ] Re-entrancy risk is assessed (Soroban's execution model mitigates classic EVM re-entrancy; confirm applicability for future cross-contract calls).
- [ ] Integer overflow / underflow risk is assessed (`i128` arithmetic; document safe-math guarantees from the SDK).
- [ ] Front-running and transaction ordering risks are assessed (especially for lock/unlock timing).
- [ ] Risks specific to Soroban / Stellar are identified:
  - Ledger timestamp manipulation window.
  - Storage TTL and archival behavior.
  - Footprint and resource limits.
- [ ] Denial-of-service vectors (e.g., exhausting storage, unbounded loops) are documented.
- [ ] Future cross-contract call risks (SAC integration) are flagged as out of scope until implemented.

---

## 5. Test Coverage

Auditors expect a test suite that exercises both the happy path and all documented failure conditions.

- [ ] All public functions have at least one passing happy-path test.
- [ ] All documented error / panic conditions have corresponding negative tests.
- [ ] Authorization failures are tested: attempts to call state-changing functions without the required signer are rejected.
- [ ] Boundary conditions are tested:
  - [ ] Deposit of exactly `1` unit (minimum valid amount).
  - [ ] Withdrawal of the full available balance.
  - [ ] Locking the full available balance.
  - [ ] `unlock_time` set to exactly `ledger_time + 1` second.
  - [ ] Attempting to lock with `unlock_time == ledger_time` (must be rejected).
- [ ] Re-initialization is tested and confirmed to panic.
- [ ] The test suite passes cleanly with no warnings via `cargo test`.
- [ ] Tests cover the `can_withdraw` return value before and after the unlock timestamp is reached.
- [ ] Code coverage tooling (e.g., `cargo-llvm-cov`) has been run and a coverage report is available.
- [ ] Any coverage gaps are documented and justified.

---

## 6. Known Limitations

Auditors must know what is intentionally out of scope so they do not file findings against unimplemented features.

- [ ] All known limitations are documented in a single, easily found location (see README [Known Limitations](../README.md#known-limitations) and below).
- [ ] Each limitation has a note on whether it is by design (acceptable for current scope) or a planned future improvement.

| Limitation | Status | Reference |
|---|---|---|
| **No real token custody** — deposits update internal accounting only; no XLM or SAC token is transferred into contract custody. | By design for current scope; SAC integration planned. | [architecture.md](architecture.md) |
| **Single unlock time per user** — calling `lock_funds` multiple times overwrites the previous unlock timestamp. | Known design gap; per-lock entries planned. | README |
| **No upgrade mechanism** — `upgrade()` is not implemented; the contract WASM cannot be changed after deployment. | Research complete; no upgrade chosen yet. | [upgrade-strategy.md](upgrade-strategy.md) |
| **No pause / emergency stop** — there is no mechanism to halt operations if a critical bug is found. | Research complete; no pause chosen yet. | [pause-design.md](pause-design.md) |
| **No admin recovery** — the admin cannot recover or migrate user funds. | By design for current scope. | [admin-role.md](admin-role.md) |
| **No stable error codes** — validation failures use Rust panic messages, not a `#[contracterror]` enum. | Known gap; planned improvement. | [error-codes.md](error-codes.md) |
| **Events not yet implemented** — the event schema is defined but no events are emitted by the contract. | Known gap; schema proposed. | [events.md](events.md) |
| **Testnet only** — not intended for mainnet deployment in current form. | By design for current scope. | README |

---

## 7. Deployment & Network Assumptions

Auditors need to know the exact environment the contract is designed for and how it is deployed.

- [ ] The target network (testnet vs. mainnet) is documented.
- [ ] The Soroban RPC endpoint and network passphrase are documented.
- [ ] The deployment process (script or manual steps) is documented and reproducible (see [deployment-environments.md](deployment-environments.md)).
- [ ] The `initialize(admin)` admin address for any audited deployment is documented, including who controls it.
- [ ] Contract ID handoff to SDK and mobile clients is documented (see [contract-id-handoff.md](contract-id-handoff.md)).
- [ ] Storage TTL extension procedures are documented (see [storage-ttl.md](storage-ttl.md)).
- [ ] Any environment variables, secrets, or off-chain components that interact with the contract are listed.
- [ ] The WASM artifact size and the build command used to produce it are recorded.
- [ ] The Rust toolchain version and `soroban-cli` version used for the audited build are pinned and recorded.

---

## 8. Unresolved Design Questions

Open questions that could affect the audit scope or findings should be surfaced up front.

- [ ] **SAC integration**: When real token custody is added, will a single SAC token address be fixed at initialization, or will multiple tokens be supported? How will the custody model change the attack surface?
- [ ] **Upgrade strategy**: Will the contract remain immutable, use an admin-controlled `upgrade()`, or use a migration contract? The choice affects admin trust assumptions. See [upgrade-strategy.md](upgrade-strategy.md).
- [ ] **Pause mechanism**: Will an emergency stop be added? If so, who can trigger it, and what is the abuse-prevention model? See [pause-design.md](pause-design.md).
- [ ] **Per-lock entries**: Will `lock_funds` be changed to support multiple independent locks per user? This would change storage layout and `get_locked_balance` semantics.
- [ ] **Admin recovery**: Should the admin ever be allowed to recover funds from inactive or lost-key accounts? If so, what governance safeguards are required?
- [ ] **Error codes**: Will a stable `#[contracterror]` enum be added before audit? Auditors will note the absence of machine-readable error codes as a finding.
- [ ] **Events**: Will contract events be implemented before audit? Auditors will note their absence.
- [ ] **Mainnet readiness**: What additional review gates (legal, operational, insurance) are required before a mainnet deployment is permitted?

---

## 9. Supporting Documentation

Confirm that the following reference documents are complete, accurate, and up to date at the audit commit.

- [ ] [README.md](../README.md) — project overview, build, test, and deploy instructions.
- [ ] [architecture.md](architecture.md) — project structure, state management, storage model, SDK integration.
- [ ] [admin-role.md](admin-role.md) — admin address, current capabilities, and future design considerations.
- [ ] [error-codes.md](error-codes.md) — all current panic messages and their meanings.
- [ ] [events.md](events.md) — proposed event schema (note: events not yet implemented).
- [ ] [pause-design.md](pause-design.md) — research on pause / emergency stop (not implemented).
- [ ] [upgrade-strategy.md](upgrade-strategy.md) — research on upgrade paths (not implemented).
- [ ] [storage-ttl.md](storage-ttl.md) — persistent vs. instance storage TTL behavior and extension commands.
- [ ] [deployment-environments.md](deployment-environments.md) — RPC endpoints, network passphrases, and deployment commands.
- [ ] [contract-id-handoff.md](contract-id-handoff.md) — how to pass a deployed contract ID to SDK and mobile clients.

---

## Acceptance Checklist

- [ ] Audit preparation checklist exists (`docs/audit-preparation.md`).
- [ ] Checklist covers contract API documentation.
- [ ] Checklist covers storage model documentation.
- [ ] Checklist covers test coverage expectations.
- [ ] Checklist covers known limitations.
- [ ] Checklist covers deployment and network assumptions.
- [ ] README links to this checklist.

---

*Last updated: 2026-07-19*
