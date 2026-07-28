# Sample Vault Interaction Walkthrough

This walkthrough demonstrates a complete `savings_vault` lifecycle — deploy,
initialize, deposit, lock, query, and withdraw — using the Soroban CLI. It is
intended as a reference for SDK and mobile integration work, so each step
states the expected on-chain state change alongside the command.

This project targets Stellar **testnet** only and is for educational and
testnet use; see the README's
[Security Considerations](../README.md#security-considerations). Placeholder
values below follow the conventions in
[docs/docs-style-guide.md](docs-style-guide.md#placeholders).

## Prerequisites

Complete the README's [Prerequisites](../README.md#prerequisites) and
[Deploy to Testnet](../README.md#deploy-to-testnet) setup (Rust, Soroban CLI,
WASM target, and a funded testnet identity) before following this walkthrough.
Network configuration (RPC URL, passphrase, Friendbot) is documented once in
[docs/deployment-environments.md](deployment-environments.md).

## 1. Deploy the contract

Build and deploy using the repository's deployment script, as described in the
README's [Deploy to Testnet](../README.md#deploy-to-testnet) section:

```bash
./scripts/deploy-testnet.sh deployer
```

The command prints a new contract ID. See
[docs/deployment-output-example.md](deployment-output-example.md) for what
that output looks like. Save it as `CONTRACT_ID_PLACEHOLDER` for the rest of
this walkthrough.

**State after this step:** a new contract instance exists on testnet with no
stored state yet.

## 2. Initialize the contract

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

`TOKEN_CONTRACT_ADDRESS_PLACEHOLDER` is the SAC (Stellar Asset Contract)
address the vault stores for future token-transfer use; see
[Current limitations](#current-limitations) below.

**State after this step:** the admin address and token address are recorded
in instance storage, and the one-time initialization flag is set (a second
`initialize` call will panic).

## 3. Deposit funds

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  deposit \
  --user USER_PUBLIC_KEY \
  --amount 1000
```

**State after this step:** the user's available balance is `1000`. This is an
internal accounting entry only — see the README's
[Deposit and custody limitation](../README.md#deposit-and-custody-limitation).

## 4. Query the available balance

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  get_balance \
  --user USER_PUBLIC_KEY
```

**Expected result:** `1000`.

## 5. Lock a portion of the balance

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  lock_funds \
  --user USER_PUBLIC_KEY \
  --amount 400 \
  --unlock_time UNLOCK_TIMESTAMP
```

`UNLOCK_TIMESTAMP` must be a Unix timestamp (seconds) strictly later than the
current ledger time.

**State after this step:** `400` moves from the available balance into a new
lock entry. The available balance is now `600`; the lock is not withdrawable
until the ledger timestamp reaches `UNLOCK_TIMESTAMP`. A user may hold several
independent locks with different unlock times at once; locking again does not
replace an existing lock.

## 6. Query balances before maturity

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  get_locked_balance \
  --user USER_PUBLIC_KEY
```

**Expected result:** `400` (the lock has not matured yet).

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  can_withdraw \
  --user USER_PUBLIC_KEY
```

**Expected result:** `false`. `get_balance` at this point still returns `600`
(the locked `400` is excluded until the lock matures).

## 7. Query balances after maturity

Once the ledger timestamp reaches or passes `UNLOCK_TIMESTAMP`, the same
`get_balance` call from step 4 returns `1000` (the matured lock is now
included), `get_locked_balance` returns `0`, and `can_withdraw` returns
`true`. No separate "unlock" call is needed or available — maturity is based
solely on the ledger timestamp; see
[docs/error-codes.md](error-codes.md#locked-funds-are-not-yet-withdrawable).

## 8. Withdraw funds

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --network testnet \
  -- \
  withdraw \
  --user USER_PUBLIC_KEY \
  --amount 1000
```

**State after this step:** the withdrawal is satisfied first from the
available (unlocked) balance, then from any matured locks, oldest first. In
this example the full `1000` — the `600` available balance plus the `400`
matured lock — is withdrawn, leaving an available balance of `0` and no
remaining lock entries. Unlike deposit, `withdraw` also invokes a real token
transfer from the contract to the user; see
[Current limitations](#current-limitations).

## Current limitations

- **Deposits are internal accounting only, but withdrawals attempt a real
  token transfer.** `deposit` never moves a real asset into the contract, yet
  `withdraw` calls the configured token contract to transfer the withdrawn
  amount to the user. An internal balance does not guarantee the contract
  actually holds matching tokens; see
  [docs/error-codes.md](error-codes.md#other-existing-failure-conditions) for
  the resulting failure mode.
- **No partial or early unlock.** A lock can only be spent, in full or in
  part, through `withdraw` once it has matured; there is no function to
  cancel a lock or release it before `UNLOCK_TIMESTAMP`.
- **`can_withdraw` is a query only.** It reports whether a matured lock
  exists; it does not itself move funds or release a lock.
- For the full list of current contract limitations (single-token support,
  no admin recovery, no upgrade or pause mechanism), see the README's
  [Known Limitations](../README.md#known-limitations) section.
