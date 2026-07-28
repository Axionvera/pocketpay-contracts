# Comprehensive Codebase Analysis Package

This package is the current, implementation-aligned entry point for understanding
the PocketPay Savings Vault repository. It supersedes earlier analysis notes
that were written before the current error model, pause logic, and token-backed
custody flow were fully implemented.

## Scope

- **Repository type:** Rust workspace for a single Soroban smart contract crate
- **Primary artifact:** `contracts/savings_vault` compiled to WASM
- **Current runtime model:** On-chain only; there is no application server, no
  REST/GraphQL API surface, and no operational database schema in this repo
- **Source of truth:** `contracts/savings_vault/src/lib.rs`, crate manifests,
  and the registered Rust test suite under `contracts/savings_vault/src/test/`

## Package Contents

- [Repo Map And Workflows](codebase-analysis/repo-map-and-workflows.md)
  - Directory structure
  - Core modules and responsibilities
  - Storage schema and event surface
  - End-to-end workflows and interaction mechanisms
- [Quality And Debt Assessment](codebase-analysis/quality-and-debt.md)
  - Code quality and test posture
  - Performance, security, and maintainability findings
  - Technical debt inventory
  - Prioritized recommendations

## Executive Summary

The repository is best understood as a **single-contract custody system** built
for the Stellar Soroban platform:

- **One contract, one crate, one runtime unit**
  - The `SavingsVault` contract is implemented in a single `lib.rs` file.
  - Logical separation exists inside that file through sections for state,
    storage keys, errors, helpers, admin/configuration, fund movement, read
    models, and lock management.
- **Token-backed accounting**
  - Internal balances are bookkeeping only.
  - Real custody is enforced through the configured Stellar Asset Contract
    (SAC), with token transfers executed in `deposit`, `withdraw`, and
    `withdraw_lock`.
- **Per-user sharded storage**
  - Global configuration lives in instance storage.
  - User balances and lock records live in persistent storage keyed by address.
  - Each lock is stored independently as `DataKey::Lock(user, lock_id)`.
- **Safety-oriented business rules**
  - Mutating user operations require `require_auth()`.
  - Emergency pause blocks new deposits and locks, but always leaves user exits
    (`withdraw`, `withdraw_lock`) available.
  - Errors are exposed through a stable `#[contracterror]` enum rather than
    brittle panic strings.
- **Strong local test coverage**
  - The repo contains a broad Rust test suite with focused behavior tests,
    event schema regression tests, and property-based accounting checks.

## Core Architectural Patterns

### 1. Monolithic Contract, Segmented Internals

The implementation is monolithic at the file/module level, but internally
organized around consistent responsibility zones:

- Contract state types and storage keys
- Stable error taxonomy
- Initialization and migration helpers
- Admin and pause controls
- Token custody flows
- Lock lifecycle management
- Read models for SDK/mobile consumption

### 2. Transfer-First Mutation Ordering

The contract intentionally performs token transfers before mutating internal
storage in custody-sensitive flows:

- `deposit`: transfer user -> contract, then credit balance
- `withdraw`: transfer contract -> user, then debit balance
- `withdraw_lock`: transfer contract -> user, then mark lock withdrawn

On Soroban this ordering is acceptable because failed host calls roll back the
entire transaction, and the accompanying tests explicitly validate zero state
drift on failed transfers.

### 3. Read Models Over Raw Storage

The contract exposes raw reads (`get_balance`, `get_lock`, `list_locks`) and
higher-level aggregated reads (`get_balance_snapshot`, `get_lock_summary`).
This reduces off-chain SDK work for common screens, but creates a tradeoff:
some read helpers linearly scan a user's historical lock IDs.

### 4. Operationally Enforced Storage Lifecycle

The contract models storage versioning in code, but storage TTL renewal is an
operational concern rather than a self-healing in-contract mechanism. That is a
key part of the repository's technical debt and production-readiness gap.

## Interface Inventory

### On-chain Public Interface

The contract exposes 23 public entry points grouped into:

- Lifecycle and metadata
- Admin and configuration
- Pause controls
- User custody flows
- Lock lifecycle flows
- Read helpers and aggregated read models

See the detailed inventory in
[Repo Map And Workflows](codebase-analysis/repo-map-and-workflows.md).

### Events

The contract emits events for all major state transitions, including:

- `initialize`
- `deposit`
- `withdraw`
- `lock`
- `extend_lock`
- `withdraw_lock`
- `pause`
- `unpause`
- `xferadmin`
- Configuration updates: `cfg_min`, `cfg_maxlk`, `cfg_minlk`

### API Endpoints

There are **no HTTP API endpoints** implemented in this repository.

- No REST handlers
- No GraphQL schema
- No web server
- No RPC service owned by this repo

The effective API surface is the Soroban contract method set exposed by
`SavingsVault`.

### Database Schemas

There is **no application database schema** in this repository.

- No SQL migrations
- No ORM models
- No Supabase/Postgres/MySQL schema
- No persistence layer outside Soroban storage

The closest equivalent to a "schema" is the on-chain `DataKey` storage model
documented in `lib.rs` and summarized in the repo-map document.

## Dependencies And Integrations

### Direct Build-Time Dependencies

- `soroban-sdk = 22.0.0`
- `proptest = 1` for dev/test only

### Third-Party Runtime / Operational Integrations

- **Stellar Soroban runtime**
  - Contract execution environment
  - Ledger timestamp source
  - Authentication host for `require_auth()`
- **Stellar Asset Contract (SAC)**
  - Token transfer and custody backend
- **Soroban CLI**
  - Deployment and invocation tooling
- **Stellar testnet ecosystem**
  - Friendbot
  - Testnet RPC
  - Explorer workflows
- **GitHub workflow dispatch**
  - PR automation via `.github/workflows/trigger-auto-merge.yml`

## Important Current Findings

- The repository has evolved faster than some of its docs.
- Several existing documents still describe removed or superseded behavior:
  - panic-string-only error handling
  - vector-based `Locks(user)` storage as the primary model
  - missing pause/events despite those features now existing
  - outdated task-runner targets and old file paths
- A standalone TypeScript test exists under `tests/atomicity/`, but it does not
  match the current Rust workspace structure and appears disconnected from the
  actual build/test toolchain.

## Validation Performed For This Package

- Reviewed the full workspace layout, manifests, scripts, docs, and contract
  implementation
- Mapped the full public contract interface and event surface
- Reviewed the registered Rust test suite and supporting fixtures
- Ran `cargo test` successfully from the workspace root during this analysis

For the detailed technical debt and risk review, continue to
[Quality And Debt Assessment](codebase-analysis/quality-and-debt.md).
