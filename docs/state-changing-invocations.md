# State-Changing Function Invocation Examples

> **Network posture:** This contract is intended for development, educational, and Stellar testnet use. It is **not production-ready or mainnet-ready**.
>
> All commands below target **Stellar testnet**. Replace placeholders with your own values.

This document provides copy-paste ready Soroban CLI examples for every state-changing (write) function in the Savings Vault contract.

For read-only function examples (`get_balance`, `get_locked_balance`, `can_withdraw`, `get_lock`, `list_locks`, `get_version`, `is_paused`), see [Read-Only Function Invocations](read-only-invocations.md).

---

## Prerequisites

Before running these commands, ensure you have:

1. A [deployed contract](../README.md#deploy-to-testnet) and its contract ID.
2. A funded testnet identity (e.g. `deployer`).
3. Network configured as `testnet`:
   ```bash
   soroban network add \
     --global testnet \
     --rpc-url https://soroban-testnet.stellar.org:443 \
     --network-passphrase "Test SDF Network ; September 2015"
   ```

All examples assume:

- `YOUR_CONTRACT_ID` — your deployed contract's ID.
- `deployer` — your testnet identity (used as both `--source` and address arguments).
- `YOUR_TOKEN_ADDRESS` — the SAC token address to use with the vault.

---

## 1. Initialize

Sets the contract administrator and the Stellar Asset Contract (SAC) token that the vault will custody. **Can only be called once.**

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin deployer \
  --token YOUR_TOKEN_ADDRESS
```

> **Expected output:** No output on success. Call any other function before initializing to see a panic.

---

## 2. Deposit

Transfers tokens from the user to the contract and credits the user's internal balance. The transfer uses the SAC token configured during `initialize`.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  deposit \
  --user deployer \
  --amount 10000000
```

> **Expected output:** No output on success. Requires the user to have approved the vault contract to spend their tokens.

---

## 3. Withdraw

Transfers unlocked tokens from the contract back to the user. The available balance is the total balance minus any locked amounts.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  withdraw \
  --user deployer \
  --amount 5000000
```

> **Expected output:** No output on success. Fails if the requested amount exceeds the available (unlocked) balance.

---

## 4. Lock Funds

Reserves a portion of the user's available balance until a specified Unix timestamp. Returns the assigned `lock_id`.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  lock_funds \
  --user deployer \
  --amount 20000000 \
  --unlock_time 1800000000
```

> **Tips:**
> - Generate a future Unix timestamp with `date +%s --date="+7 days"`.
> - The returned `lock_id` is needed for `get_lock` and `withdraw_lock`.
> - The unlock time must be in the future relative to ledger time.

---

## 5. Withdraw Lock (Single Lock)

Withdraws the full amount of a specific matured lock entry by its `lock_id`. Created for locks that represent a single timed commitment.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  withdraw_lock \
  --user deployer \
  --lock_id 1
```

> **Expected output:** No output on success. Fails if the lock is not yet matured or the lock ID does not exist.

---

## 6. Pause

Activates the emergency pause mechanism, blocking `deposit` and `lock_funds` while leaving `withdraw` and `withdraw_lock` available. The pause auto-expires after `duration_secs` seconds.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  pause \
  --admin deployer \
  --duration_secs 3600
```

> **Expected output:** No output on success. Only the contract admin can pause. Duration of zero is rejected.

---

## 7. Unpause

Deactivates an active pause before its auto-expiry. Resumes normal `deposit` and `lock_funds` operations.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  unpause \
  --admin deployer
```

> **Expected output:** No output on success. Only the contract admin can unpause. Calling unpause when not paused is a no-op.

---

## 8. Transfer Admin

Transfers the admin role to a new address. Only the current admin can call this.

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  transfer_admin \
  --admin deployer \
  --new_admin YOUR_NEW_ADMIN_ADDRESS
```

> **Expected output:** No output on success. After transfer, only the new admin can call admin-only functions.

---

## Common Flags Reference

| Flag | Description | Example |
| --- | --- | --- |
| `--id` | Deployed contract ID | `YOUR_CONTRACT_ID` |
| `--source` | Signing identity | `deployer` |
| `--network` | Network configuration name | `testnet` |

For network-specific overrides (different RPC URL, custom passphrase), see the [Deployment Environments](deployment-environments.md) guide.

---

## Error Handling

All state-changing functions panic with a string message on failure. Common failures include:

- **Unauthorized** — The signing key does not match the user or admin address.
- **Insufficient balance** — Withdraw or lock amount exceeds the available balance.
- **Already initialized** — `initialize` was already called.
- **Contract is paused** — `deposit` or `lock_funds` called while paused.
- **Lock not matured** — `withdraw_lock` called before the unlock time.

For a full list of failure modes, see the [Failure Mode Catalogue](failure-mode-catalogue.md) and [Error Code Reference](error-codes.md).

---

## See Also

- [Read-Only Function Invocations](read-only-invocations.md) — Balance queries, lock queries, and capability checks.
- [CLI Smoke Test Guide](cli-smoke-test.md) — Quick post-deployment verification flow for all contract functions.
- [Sample Vault Interaction Walkthrough](walkthrough.md) — End-to-end deploy, deposit, lock, query, and withdraw walkthrough.
