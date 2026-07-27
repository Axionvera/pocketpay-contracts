# Savings Vault — SDK / Mobile Error Mapping Guide

This guide is for SDK + mobile developers integrating against the Savings
Vault contract. The contract now exposes a **stable `u32` error code surface**
via `#[contracterror] #[repr(u32)]` defined in
[`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs).

> **Cross-repo compatibility contract.** These `u32` codes are the **ONLY**
> supported interface for error branching on the client side. Panic message
> text may change between soroban-sdk upgrades — never regex-match panic
> strings in production code. Match on numeric codes instead.

## 1. How errors surface

When the contract calls `env.error_contract(ContractError::Variant)`:

1. The Soroban host traps with a `HostError` of kind `ContractError`.
2. The `u32` discriminant from the enum (`#[repr(u32)]`) is the **stable
   identifier** of the error.
3. Off-chain SDKs receive this `u32` via the transaction result.

A failed invocation commits **no state changes** from that invocation (Soroban
rollback is atomic); SDKs do not need to reconcile partial writes.

### How to extract the code

Pseudo-code for a `try_*` contract client call:

```ts
// TypeScript / stellar-sdk v12+
import { Contract } from '@stellar/stellar-sdk';

const contract = new Contract(vaultAddress);
const sim = await txBuilder.call(contract.call('deposit', user, amount)).simulate();
if (sim.error) {
  const code = extractContractErrorCode(sim.error); // see below
  await handleVaultError(code, context);
}
```

Use a `code`-extract helper that walks the diagnostic until it finds the
`ContractError` status. For the `soroban-sdk`/`@stellar/stellar-sdk` variants,
the pattern is: "Status with ContractError type whose payload is the u32".
Keep this helper versioned per host SDK, but keep `handleVaultError(code)`
stable — that is the cross-repo contract.

## 2. Category routing (by 1000-group)

Branch SDK logic in two layers: (1) **category route** by `Math.floor(code / 1000)`,
then (2) **specific copy / analytics** by exact `code`. This way future codes
added inside a category (e.g. 1008, 1009, ...) will still route to the right
top-level UX bucket.

| `code / 1000` | Category | UX bucket |
| --- | --- | --- |
| `1` | Validation | Show inline field error next to the bad input |
| `2` | Authorisation (contract-level role check) | Show "You're not the vault admin."; route admin-only views away |
| `3` | Lifecycle | Show a banner (pause / not-initialized / already-setup) |
| `4` | Accounting | Show `get_balance(user)` and cap the submit |
| `5` | Locks | Show lock-specific copy; re-fetch lock list |
| `6` | Storage | Page on-call; show a rare "Vault configuration issue" |
| `7` | Token | Same as 6xxx — asset config issue |
| `8` | Admin rotation | Admin console form validation |

**Host auth failures** (missing signature, wrong signer) raise a host-level
`Status(Auth, …)`, not 2001. Distinguish these:

```ts
switch (categoryOf(err)) {
  case 'HostAuth':   return promptWalletSignature();   // host Auth
  case 2:            return showNotVaultAdmin();        // 2001 role check
  …
}
```

## 3. Per-code SDK map (canonical)

Columns:

- **`code`** — the stable `u32`.
- **Trigger method** — which SDK call raised it; use to scope the copy.
- **User copy** — what mobile shows the end user. Interpolate `{amount}` /
  `{asset}` using the token's stored decimals (never raw `i128` stroops).
- **Analytics event** — cross-repo deterministic event name for amplitude /
  mixpanel / firebase.
- **SDK retry?** — `NEVER` = do not auto-retry (user-action or logic bug),
  `PAGE-ONCALL` = alert the deployment-owners, `UI-GATED` = the SDK should
  have blocked it before submission.

| code | Trigger | User copy | Analytics | Retry? |
| --- | --- | --- | --- | --- |
| 1001 | `deposit` | "Enter an amount greater than 0 {asset}." | `vault.err.1001.deposit` | UI-GATED |
| 1001 | `withdraw` | "Enter an amount greater than 0 {asset}." | `vault.err.1001.withdraw` | UI-GATED |
| 1001 | `lock_funds` | "Enter an amount greater than 0 {asset}." | `vault.err.1001.lock` | UI-GATED |
| 1002 | `lock_funds` | "Unlock time must be later than right now." | `vault.err.1002.lock` | UI-GATED |
| 1002 | `extend_lock` | "New unlock time must be later than the current one." | `vault.err.1002.extend` | UI-GATED |
| 1003 | `lock_funds` | "Lock duration exceeds the maximum allowed." | `vault.err.1003` | UI-GATED |
| 1004 | `lock_funds` | "Lock duration is below the minimum allowed." | `vault.err.1004` | UI-GATED |
| 1005 | `deposit` | "Minimum deposit is {min} {asset}." | `vault.err.1005` | UI-GATED |
| 1006 | `pause` (admin) | "Pause duration must be greater than 0 seconds." | `vault.err.1006` | UI-GATED |
| 1007 | `set_min_deposit_amount` (admin) | "Minimum deposit can't be negative." | `vault.err.1007` | UI-GATED |
| 2001 | any admin entrypoint | "This action requires the vault admin role." | `vault.err.2001.{method}` | NEVER |
| 3001 | `initialize` | "The vault is already set up." | `vault.err.3001` | NEVER |
| 3002 | any guarded method | "The vault is still being set up. Try again in a moment." | `vault.err.3002.{method}` | NEVER (poll `get_version`) |
| 3003 | `deposit`,`lock_funds`,`extend_lock` | "Vault is paused for an incident. Deposits and locks are blocked. You can still withdraw." | `vault.err.3003.{method}` | NEVER (poll `is_paused`) |
| 4001 | `withdraw` | "Insufficient available balance. You can withdraw up to {available} {asset}. Locked funds are unavailable until they mature." | `vault.err.4001` | NEVER |
| 4002 | `lock_funds` | "Insufficient unlocked balance. You can lock up to {available} {asset}." | `vault.err.4002` | NEVER |
| 5001 | `withdraw_lock` / `extend_lock` | "We couldn't find that lock. Refresh and try again." | `vault.err.5001.{method}` | NEVER (refetch list) |
| 5002 | `withdraw_lock` / `extend_lock` | "This lock has already been withdrawn." | `vault.err.5002.{method}` | NEVER (treat as success) |
| 5003 | `withdraw_lock` | "This lock hasn't matured yet. Check back at {unlockTimeISO}." | `vault.err.5003` | NEVER |
| 5004 | `extend_lock` | "New unlock time must be later than the current unlock time." | `vault.err.5004` | UI-GATED |
| 6001 | any guarded method | "A vault storage version mismatch was detected. Please contact support." | `vault.err.6001.{method}` | PAGE-ONCALL |
| 6002 | any guarded method | "A vault storage entry is missing. Please contact support." | `vault.err.6002.{method}` | PAGE-ONCALL |
| 7001 | `deposit` / `withdraw` / `withdraw_lock` | "The vault's token is misconfigured. Please contact support." | `vault.err.7001.{method}` | PAGE-ONCALL |
| 8001 | `transfer_admin` (admin) | "New admin address must be different from the current admin." | `vault.err.8001` | NEVER |
| 8002 | `transfer_admin` (admin) | "The vault cannot be its own admin." | `vault.err.8002` | NEVER |

## 4. Reference SDK implementation (TypeScript)

```ts
// sdk/src/vault/errors.ts
export const VaultError = {
  AmountNotPositive: 1001,
  UnlockTimeNotInFuture: 1002,
  LockDurationExceedsMaximum: 1003,
  LockDurationBelowMinimum: 1004,
  AmountBelowMinimumDeposit: 1005,
  PauseDurationMustBePositive: 1006,
  MinDepositAmountNegative: 1007,

  NotAuthorizedAdmin: 2001,

  AlreadyInitialized: 3001,
  NotInitialized: 3002,
  ContractPaused: 3003,

  InsufficientBalance: 4001,
  InsufficientBalanceToLock: 4002,

  LockNotFound: 5001,
  LockAlreadyWithdrawn: 5002,
  LockNotMatured: 5003,
  ExtendLockTimeNotIncreased: 5004,

  StorageVersionUnsupported: 6001,
  RequiredStorageEntryMissing: 6002,

  TokenNotConfigured: 7001,

  CannotTransferAdminToSelf: 8001,
  CannotTransferAdminToContractAddress: 8002,
} as const;

export type VaultErrorCode = (typeof VaultError)[keyof typeof VaultError];

export function isContractError(code: number): code is VaultErrorCode {
  return Object.values(VaultError).includes(code as VaultErrorCode);
}

export function categoryOf(code: number): number {
  return Math.floor(code / 1000);
}
```

Handler skeleton:

```ts
// sdk/src/vault/onError.ts
import { VaultError, categoryOf } from './errors';

export async function handleVaultError(
  code: number,
  ctx: { method: string; asset: { decimals: number; symbol: string } },
): Promise<void> {
  switch (categoryOf(code)) {
    case 1: return renderInlineValidation(code, ctx);
    case 2: return renderRoleBanner();
    case 3: return renderLifecycleBanner(code, ctx);
    case 4: return renderBalanceCallout(code, ctx);
    case 5: return refetchLockList(code, ctx);
    case 6:
    case 7:
      pageOncall(`vault.err.${code}`, ctx.method);
      return renderSupportBanner();
    case 8: return renderAdminFormError(code);
    default:
      if (isContractError(code)) {
        log.warn(`unknown canonical code ${code}; forward to fallback`);
      }
      return renderGenericFallback();
  }
}
```

## 5. Reference SDK implementation (Rust client)

```rust
// crates/vault-sdk/src/err.rs
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    AmountNotPositive = 1001,
    UnlockTimeNotInFuture = 1002,
    // … keep in 1:1 lockstep with the on-chain enum
}

impl VaultError {
    pub fn from_u32(code: u32) -> Option<Self> {
        use VaultError::*;
        Some(match code {
            1001 => AmountNotPositive,
            1002 => UnlockTimeNotInFuture,
            1003 => LockDurationExceedsMaximum,
            1004 => LockDurationBelowMinimum,
            1005 => AmountBelowMinimumDeposit,
            1006 => PauseDurationMustBePositive,
            1007 => MinDepositAmountNegative,
            2001 => NotAuthorizedAdmin,
            3001 => AlreadyInitialized,
            3002 => NotInitialized,
            3003 => ContractPaused,
            4001 => InsufficientBalance,
            4002 => InsufficientBalanceToLock,
            5001 => LockNotFound,
            5002 => LockAlreadyWithdrawn,
            5003 => LockNotMatured,
            5004 => ExtendLockTimeNotIncreased,
            6001 => StorageVersionUnsupported,
            6002 => RequiredStorageEntryMissing,
            7001 => TokenNotConfigured,
            8001 => CannotTransferAdminToSelf,
            8002 => CannotTransferAdminToContractAddress,
            _ => return None,
        })
    }
}
```

Usage against a `try_*` client:

```rust
match client.try_withdraw(&user, &amount) {
    Ok(res) => Ok(res?),
    Err(soroban_client::Error::Contract(code)) => match VaultError::from_u32(code) {
        Some(VaultError::InsufficientBalance) => {
            let available = client.get_balance(&user)?;
            Err(AppError::WithdrawUpTo(available))
        }
        Some(other) => Err(AppError::Vault(other, method)),
        None => Err(AppError::UnknownContractCode(code, method)),
    },
    Err(other) => Err(AppError::Transport(other)),
}
```

## 6. Pre-flight validation (recommended)

Doing these client-side saves a round trip AND prevents 1001–1007 / 5004 /
8001–8002 from ever hitting the chain. They are still listed on-chain as a
belt-and-suspenders check, but SDKs should treat them as UI bugs when they
reach the contract.

```ts
// sdk/src/vault/preflight.ts
export function validateDeposit(amountBI: bigint, cfg: { minDepositBI: bigint }) {
  if (amountBI <= 0n) return VaultError.AmountNotPositive;
  if (amountBI < cfg.minDepositBI) return VaultError.AmountBelowMinimumDeposit;
  return null;
}

export function validateLock(
  amountBI: bigint,
  unlockTimeS: number,
  ledgerTimeS: number,
  cfg: { minLockDurationS: number; maxLockDurationS: number; availableBI: bigint },
) {
  if (amountBI <= 0n) return VaultError.AmountNotPositive;
  if (amountBI > cfg.availableBI) return VaultError.InsufficientBalanceToLock;
  if (unlockTimeS <= ledgerTimeS) return VaultError.UnlockTimeNotInFuture;
  const d = unlockTimeS - ledgerTimeS;
  if (d < cfg.minLockDurationS) return VaultError.LockDurationBelowMinimum;
  if (d > cfg.maxLockDurationS) return VaultError.LockDurationExceedsMaximum;
  return null;
}
```

## 7. Further reading

- [`error-codes.md`](./error-codes.md) — per-code canonical meaning and
  contract caller action.
- [`error-code-standard.md`](./error-code-standard.md) — design rationale,
  stability guarantees, versioning rules for adding new codes.
- [`sdk-contract-sequence.md`](./sdk-contract-sequence.md) — flows with error
  paths drawn in.
