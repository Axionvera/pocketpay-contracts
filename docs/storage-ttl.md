# Storage TTL — Savings Vault

This document explains how Soroban storage TTL (time-to-live) works, how the
vault contract uses persistent and instance storage, why TTL matters for
on-chain data survival, and how to extend storage when needed.

---

## What is storage TTL?

Every entry written to the Stellar ledger has a **TTL (time-to-live)**: a
ledger sequence number after which the entry is considered expired. Soroban
tracks TTL at the level of individual storage entries.

- An entry whose TTL has **not** expired is **live**: it can be read and
  written normally.
- An entry whose TTL **has** expired is no longer accessible. Attempting to
  read it returns nothing (as if it never existed).

TTL is counted in **ledger sequences**, not wall-clock time. On testnet, a new
ledger closes roughly every 5–6 seconds.

> **Important:** Expired storage does not mean the ledger entry is immediately
> deleted. It means contract code can no longer access it. Restoration is
> possible (see [Restoring expired entries](#restoring-expired-entries)), but
> prevention through regular TTL extension is simpler.

---

## The two storage types used by this contract

The vault contract uses two Soroban storage types with different TTL behaviour.

### Persistent storage

Used for all **user-owned data**:

| DataKey               | What it holds                        |
|-----------------------|--------------------------------------|
| `Balance(user)`       | Available (unlocked) balance         |
| `LockedBalance(user)` | Locked balance                       |
| `UnlockTime(user)`    | Unix timestamp when locked funds unlock |

Persistent entries have the **longest default TTL** on the Stellar network. On
mainnet the default is set by the network as a protocol constant; on testnet it
is shorter to make expiry easier to observe. Persistent entries also benefit
from **automatic TTL bumping** by the network fee system when a transaction
reads or writes them — but that bump is incremental and may not be enough for
long-lived data.

Because user balances represent real value, they must remain readable for as
long as users need them. If a `Balance` entry expires, the contract will see a
balance of `0` (the `unwrap_or(0)` default) and a user attempting to withdraw
will be told they have no funds. **This is the primary TTL risk for this
contract.**

### Instance storage

Used for **contract-level data** shared across all users:

| DataKey       | What it holds                             |
|---------------|-------------------------------------------|
| `Admin`       | Admin address recorded during `initialize`|
| `Initialized` | Guard flag preventing re-initialization   |
| `Token`       | Token contract address                    |

Instance storage TTL is tied to the **contract instance itself**. The instance
entry has its own TTL, and all instance storage keys expire together when the
instance TTL lapses. Extending the instance TTL keeps all instance keys alive.

Instance storage has a **shorter default TTL** than persistent storage.
Because `Initialized`, `Admin`, and `Token` must be readable for every
transaction, the instance TTL should be monitored and extended regularly —
especially on testnet where defaults are lower.

---

## Why TTL matters for this contract

| Scenario | Consequence if TTL expires |
|---|---|
| `Balance(user)` expires | `get_balance` returns `0`; withdrawals fail with "Insufficient balance" |
| `LockedBalance(user)` expires | `get_locked_balance` returns `0`; `can_withdraw` returns `false` |
| `UnlockTime(user)` expires | `can_withdraw` may return `false`; locked funds appear inaccessible |
| Instance storage expires | `deposit`, `withdraw`, `lock_funds` all panic — contract is non-functional until restored |

On testnet, storage expires faster than on mainnet. Always extend TTL after
deploying to testnet if you plan to keep the contract active for more than a
short session.

---

## Checking current TTL

Use the Soroban CLI to inspect the TTL of the contract instance:

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  get_balance \
  --user deployer
```

That command will fail with a read error if instance storage has expired. A
cleaner way to inspect expiry information is via the RPC directly, but the
`extend` command described below is the practical remedy regardless.

---

## Extending storage TTL

### Extend the contract instance

The contract instance holds the `Admin`, `Initialized`, and `Token` keys.
Extend it with:

```bash
soroban contract extend \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  --ledgers-to-extend 500000
```

`--ledgers-to-extend` sets how many additional ledgers the TTL should cover
from the current ledger. At ~5 seconds per ledger, `500000` ledgers is roughly
29 days. Adjust to suit your usage window.

### Extend persistent storage entries

Persistent storage entries (user balances) can be extended per key:

```bash
soroban contract extend \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  --ledgers-to-extend 500000 \
  --key-xdr <BASE64_XDR_OF_KEY>
```

Encoding a `DataKey` as XDR requires tooling outside the scope of this guide.
In practice, the simplest approach is:

1. Extend the **instance** TTL (see above) to keep contract-level data alive.
2. Interact with the contract regularly — each read/write transaction
   **bumps the TTL** of the touched persistent entries automatically.
3. For long-running testnet sessions where no transactions are expected,
   schedule periodic invocations or extend keys manually using the XDR method.

---

## Restoring expired entries

If a persistent entry has already expired, it can be restored before being
written again. Use the `soroban contract restore` command:

```bash
soroban contract restore \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  --key-xdr <BASE64_XDR_OF_KEY>
```

Restoration makes the entry live again and resets its TTL to the minimum
persistent TTL. After restoring, extend the TTL immediately.

> **Testnet note:** Testnet state is periodically reset by the Stellar
> Development Foundation. After a testnet reset, all contract data is wiped
> regardless of TTL — redeploy and reinitialize from scratch.

---

## Practical checklist for testnet deployments

After deploying and initializing the contract:

- [ ] Extend the instance TTL with `soroban contract extend` for your intended
  session length.
- [ ] Make at least one deposit call so user balance entries are created with
  an initial TTL.
- [ ] If leaving the contract idle for days, schedule a periodic extension or
  a lightweight read transaction to bump persistent entry TTLs.
- [ ] Before reporting a "missing balance" bug, check whether instance or
  persistent storage has expired first.

---

## Summary

| Storage type | Keys | Default TTL | Primary risk |
|---|---|---|---|
| Persistent | `Balance`, `LockedBalance`, `UnlockTime` | Longest | Balance data becomes invisible if not bumped |
| Instance | `Admin`, `Initialized`, `Token` | Shorter | Contract becomes non-functional until restored |

Extend both types proactively. The instance must be extended explicitly; user
balance entries are bumped automatically on each read/write but benefit from
manual extension during idle periods.

---

## Further reading

- [Soroban State Archival — Stellar Docs](https://developers.stellar.org/docs/learn/encyclopedia/storage/state-archival)
- [soroban contract extend — CLI reference](https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli)
- [docs/architecture.md](architecture.md) — storage design overview
- [docs/troubleshooting.md](troubleshooting.md) — common deployment issues
