# Savings Vault Contract Workspace

This directory is the contract workspace for PocketPay's savings vault logic. It contains the Soroban contract implementation and its tests, and is included as a member of the Cargo workspace defined at the repository root.

For project-wide setup, architecture, deployment guidance, and contribution instructions, see the [root README](../../README.md).

## Public functions

The contract exposes these public functions in `src/lib.rs`:

- `initialize(env, admin, token)` initializes the contract with its admin and token addresses. It can only be called once.
- `deposit(env, user, amount)` records a deposit in the user's available vault balance.
- `withdraw(env, user, amount)` withdraws an amount from the user's available balance.
- `get_balance(env, user)` returns the user's available balance.
- `lock_funds(env, user, amount, unlock_time)` moves available funds into the locked balance until a Unix timestamp.
- `get_locked_balance(env, user)` returns the user's locked balance.
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
