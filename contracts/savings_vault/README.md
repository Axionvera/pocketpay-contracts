# Savings Vault Contract Workspace

This directory is the contract workspace for PocketPay's savings vault logic. It contains the Soroban contract implementation and its tests, and is included as a member of the Cargo workspace defined at the repository root.

For project-wide setup, architecture, deployment guidance, and contribution instructions, see the [root README](../../README.md).

## Documentation
- **Audit Readiness Review: [docs/audit-readiness.md](../../docs/audit-readiness.md) — Pre-audit assessment of storage, authorization, token custody, accounting, events, and documentation gaps.
- **Custody Assumptions: [docs/vault-custody-assumptions.md](../../docs/vault-custody-assumptions.md) — Token custody guarantees, limitations, token flow diagrams, and production readiness assessment.
- **State Machine: [docs/state-machine.md](../../docs/state-machine.md) — Contract lifecycle, user account states, valid/invalid transitions, and error states.
- **Failure Mode Catalogue: [docs/failure-mode-catalogue.md](../../docs/failure-mode-catalogue.md) - Complete list of all known failure modes with test links
- **All other docs are in the repo root `docs/` directory.

## Public functions

The contract exposes these public functions in `src/lib.rs`:

- `initialize(env, admin, token)` initializes the contract with its admin and token addresses. It can only be called once.
- `deposit(env, user, amount)` records a deposit in the user's available vault balance.
- `withdraw(env, user, amount)` withdraws an amount from the user's available balance.
- `get_balance(env, user)` returns the user's available balance.
- `lock_funds(env, user, amount, unlock_time)` moves available funds into the locked balance until a Unix timestamp. `unlock_time` must be strictly later than the current ledger timestamp — a zero-duration lock (`unlock_time == current_time`) is rejected with `"Unlock time must be in the future"`; the shortest valid duration is one second.
- `get_locked_balance(env, user)` returns the user's locked balance.
- `get_lock(env, user, lock_id)` returns one lock record when it exists.
- `list_locks(env, user, offset, limit)` returns a paginated list of lock records.
- `can_withdraw(env, user)` reports whether the user's locked balance has reached its unlock time.

## Test

From the repository root, run the complete workspace test suite:

```bash
cargo test --workspace
```

## Build

Build the optimized WASM artifact from the repository root:

```bash
cargo build --release --target wasm32-unknown-unknown
```

The artifact is written to `target/wasm32-unknown-unknown/release/savings_vault.wasm`.

## Contributing

Keep contract logic changes focused and include or update tests for every behavior change. Run the workspace test suite before opening a pull request.