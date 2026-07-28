# Local Development Guide

This guide explains how to build, test, and interact with the PocketPay savings vault contract using a local Soroban sandbox environment for faster iteration.

## Prerequisites

Ensure you have the following installed before proceeding:

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Soroban CLI** (v22.0.0 or newer, compatible with the SDK version used)
   ```bash
   cargo install --locked soroban-cli
   ```

3. **WASM target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

## Build the Contract

Compile the contract to a WASM binary:

```bash
# Debug build
cargo build --target wasm32-unknown-unknown

# Optimized release build (recommended for local testing)
cargo build --target wasm32-unknown-unknown --release

# Optimized release build with size report
make build-release
```

The compiled `.wasm` file will be at:
```
target/wasm32-unknown-unknown/release/savings_vault.wasm
```

## Local verification

Before opening a pull request, run the single local verification command from the repository root:

```bash
make verify
```

This checks formatting, Clippy, the workspace test suite, and the release WASM build. See [CONTRIBUTING.md](../CONTRIBUTING.md#build-format-and-test) and the README [Local verification](../README.md#local-verification) section.

## Run Unit Tests

The project includes a comprehensive unit test suite that runs natively without needing a network:

```bash
cargo test
```

All tests use the Soroban SDK test utilities and don't require deployment to a network.

## Local Sandbox Workflow

Soroban CLI provides a local sandbox for testing contracts without needing to connect to a network. Here's how to use it:

### 1. Deploy to Local Sandbox

```bash
# Deploy the contract to the local sandbox
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source default \
  --network standalone
```

This will output a Contract ID that you'll use for subsequent invocations.

### 2. Initialize the Contract

```bash
# Replace YOUR_CONTRACT_ID with the ID from the deploy step
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  initialize \
  --admin default
```

### 3. Invoke Contract Functions

Now you can test all contract functions locally:

```bash
# Deposit 1000 units
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  deposit \
  --user default \
  --amount 1000

# Check balance
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  get_balance \
  --user default

# Lock some funds (replace UNLOCK_TIMESTAMP with a future Unix timestamp)
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  lock_funds \
  --user default \
  --amount 500 \
  --unlock_time 1800000000

# Check locked balance
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  get_locked_balance \
  --user default

# Withdraw available funds
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source default \
  --network standalone \
  -- \
  withdraw \
  --user default \
  --amount 300
```

## Resetting the Local Sandbox

To clear all state and start fresh:

```bash
# Clear the local sandbox state
soroban network reset standalone
```

## Local Development vs Testnet

| Aspect | Local Sandbox | Testnet |
|--------|---------------|---------|
| Speed | Fast (no network) | Slower (network calls) |
| Cost | Free | Requires testnet XLM |
| Persistence | Reset with `network reset` | Persists on ledger |
| Friendbot | Not needed | Required for funding |

## Limitations

- The local sandbox does not simulate ledger time progression. When testing time-based features like `lock_funds`, you'll need to manually adjust timestamps.
- The sandbox uses a local identity (`default`) that doesn't require funding.
- Events are not persisted in the same way as on a real network.

## Common Issues

### Build Failures

**Problem:** `cargo build` fails with "linker not found" or similar errors.
**Solution:** Ensure the WASM target is installed:
```bash
rustup target add wasm32-unknown-unknown
```

**Problem:** Compilation errors about missing Soroban SDK types.
**Solution:** Update Soroban CLI to the latest version:
```bash
cargo install --locked soroban-cli
```

### Test Failures

**Problem:** Tests fail with "contract not initialized" errors.
**Solution:** Some tests require contract initialization. Check if the test has a setup function that needs to be called first, or run the full test suite:
```bash
cargo test --all
```

**Problem:** Tests pass individually but fail when run together.
**Solution:** This usually indicates shared state. The savings vault contract uses static storage, so tests may interfere with each other. Run tests in a single thread:
```bash
cargo test -- --test-threads=1
```

### Sandbox Issues

**Problem:** `soroban network reset standalone` fails or doesn't clear state.
**Solution:** Manually delete the sandbox data directory:
```bash
rm -rf .soroban/
soroban network start standalone
```

**Problem:** Contract invocation returns "Contract not found" after redeployment.
**Solution:** Each deployment generates a new contract ID. Make sure you're using the latest contract ID from the deploy output.

### Time-Based Features

**Problem:** `lock_funds` with a future timestamp doesn't work as expected.
**Solution:** The local sandbox doesn't advance time automatically. You can't test time-based withdrawals on the sandbox without manually manipulating ledger time. Use testnet for time-sensitive testing or mock time in unit tests.

## Next Steps

Once you've tested locally and everything works as expected, you can proceed to deploy to testnet using the instructions in the [README](../README.md#deploy-to-testnet).
