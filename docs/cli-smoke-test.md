# Soroban CLI Smoke Test Guide

A quick, step-by-step verification flow for confirming that a deployed
`savings_vault` contract responds correctly to every public function. Run
this after deployment to catch configuration or initialization problems
before deeper integration testing.

This guide targets **testnet** or a **local Soroban sandbox**. It is not a
full walkthrough — see [docs/walkthrough.md](walkthrough.md) for a
detailed lifecycle example with state explanations.

> **This contract is for educational and testnet use.** See the README's
> [Security Considerations](../README.md#security-considerations).

## Setup assumptions

Before starting, ensure you have:

1. **Rust** (latest stable), **Soroban CLI**, and the **wasm32-unknown-unknown**
   target installed — see the README's [Prerequisites](../README.md#prerequisites).
2. **A deployed contract** — either via the automated script or manual deploy:
   ```bash
   ./scripts/deploy-testnet.sh deployer
   ```
   Save the printed contract ID as `CONTRACT_ID_PLACEHOLDER`.
3. **A funded identity** — for testnet, fund via
   [Friendbot](https://friendbot.stellar.org); for local sandbox, the
   `default` identity is used automatically.
4. **Network configured** — for testnet:
   ```bash
   soroban network add \
     --global testnet \
     --rpc-url https://soroban-testnet.stellar.org:443 \
     --network-passphrase "Test SDF Network ; September 2015"
   ```

The examples below use the canonical placeholders from
[docs/placeholders.md](placeholders.md). Replace them with your actual
values before running.

---

## Smoke test steps

Run each step in order. Every command should succeed; if any step fails,
check the [troubleshooting guide](troubleshooting.md) before continuing.

### 1. Initialize the contract

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin ADMIN_PUBLIC_KEY \
  --token TOKEN_CONTRACT_ADDRESS_PLACEHOLDER
```

A successful call returns no output. A second call to `initialize` will
panic — this is expected and confirms re-initialization protection works.

### 2. Deposit funds (state-changing)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  deposit \
  --user USER_PUBLIC_KEY \
  --amount 1000
```

No output on success. This creates an internal accounting entry for the
user's balance.

### 3. Query available balance (read-only)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  get_balance \
  --user USER_PUBLIC_KEY
```

**Expected output:** `1000`

### 4. Lock a portion of the balance (state-changing)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  lock_funds \
  --user USER_PUBLIC_KEY \
  --amount 400 \
  --unlock_time UNLOCK_TIMESTAMP
```

`UNLOCK_TIMESTAMP` must be a Unix timestamp (seconds) in the future. No
output on success. The locked amount is moved from the available balance.

### 5. Query locked balance (read-only)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  get_locked_balance \
  --user USER_PUBLIC_KEY
```

**Expected output:** `400`

### 6. Check withdrawal eligibility (read-only)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  can_withdraw \
  --user USER_PUBLIC_KEY
```

**Expected output:** `false` (the lock has not matured yet)

### 7. Withdraw funds (state-changing)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  withdraw \
  --user USER_PUBLIC_KEY \
  --amount 1000
```

No output on success. Once the lock matures, the full balance (available +
matured lock) is withdrawable.

### 8. Verify final balance (read-only)

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  get_balance \
  --user USER_PUBLIC_KEY
```

**Expected output:** `0`

---

## Summary

| Step | Function | Type | Expected result |
|------|----------|------|-----------------|
| 1 | `initialize` | state-changing | no output |
| 2 | `deposit` | state-changing | no output |
| 3 | `get_balance` | read-only | `1000` |
| 4 | `lock_funds` | state-changing | no output |
| 5 | `get_locked_balance` | read-only | `400` |
| 6 | `can_withdraw` | read-only | `false` |
| 7 | `withdraw` | state-changing | no output |
| 8 | `get_balance` | read-only | `0` |

All eight steps pass? The contract is deployed and responding correctly.

## Local sandbox variant

To run the same smoke test against a local sandbox, replace
`--network testnet` with `--network standalone` and use the `default`
identity for `--source` and `--user`. No Friendbot funding is needed.

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source default \
  --network standalone \
  -- \
  get_balance \
  --user default
```

See [docs/local-development.md](local-development.md) for full local
sandbox setup and limitations.

## Next steps

- Run the full Rust test suite: `cargo test`
- Review the [invocation examples](invocation-examples.md) for all
  function signatures and arguments
- Check the [walkthrough](walkthrough.md) for a detailed lifecycle
  example with state explanations
