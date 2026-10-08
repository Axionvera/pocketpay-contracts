# Savings Vault deposit limit policy (issue #452)

The vault enforces a positive deposit amount and optional **admin-configured
minimum and maximum amounts per individual deposit call**. Amounts are signed
`i128` **atomic units** of the configured token, not display-denominated
amounts. This policy does not enforce a cumulative per-user quota.

## Current on-chain behavior

| Setting | Method | Default | Effective rule |
| --- | --- | --- | --- |
| Minimum | `set_min_deposit_amount(admin, min_amount)` / `get_min_deposit_amount()` | `0` | `deposit` amount must be at least the positive minimum |
| Maximum | `set_max_deposit_amount(admin, max_amount)` / `get_max_deposit_amount()` | `0` | `deposit` amount must be at most the positive maximum |
| Both | `get_config()` | `0`, `0` | Returns `min_deposit_amount` and `max_deposit_amount` together |

A configured zero disables that one bound, **not** the base requirement that
`deposit` amount be greater than zero. Positive bounds are **inclusive**:
`min <= amount <= max` when both are enabled. Setting a negative bound fails.
When both are nonzero, `min <= max` must remain true; a setter attempting
to invert the interval fails without changing the existing configuration.

Only the stored admin may change limits; the caller's `admin` argument must
authorize and match the configured admin. Deposit callers separately authorize
as the actual `user`. The maximum is **global configuration applied to every
individual deposit**, not an amount users can choose. A configuration change
takes effect on the next deposit; previously accepted deposits are not changed.

## Validation, funds safety and client errors

The contract checks initialization, storage version and pause state; requires
user authorization; then checks positivity, minimum and maximum. It performs
all limit checks **before** the Stellar Asset Contract token transfer and
before increasing internal available balance. A rejected deposit cannot move
funds or credit the vault as a side effect of that invocation.

| Error | Code | Meaning |
| --- | ---: | --- |
| `AmountNotPositive` | 1001 | Zero or negative deposit |
| `AmountBelowMinimumDeposit` | 1005 | Amount below a positive minimum |
| `MinDepositAmountNegative` | 1007 | Invalid negative minimum setting |
| `AmountAboveMaximumDeposit` | 1008 | Amount above a positive maximum |
| `MaxDepositAmountNegative` | 1009 | Invalid negative maximum setting |
| `DepositLimitRangeInvalid` | 1010 | Nonzero max below configured min, or min above configured max |
| `NotAuthorizedAdmin` | 2001 | Non-admin attempts to change a limit |

Other existing errors (initialization, storage version, paused state, missing
token and token-transfer failures) remain applicable. Frontends should read
`get_config()` before constructing a transaction, convert user-entered
token amounts to integer atomic units, and still handle on-chain rejection
because limits or pause state may change between read and submission.

## Per-user cap strategy: deliberately **not implemented**

A maximum **per call** does not limit total deposits: with a maximum of
`200`, a user may make two successful deposits of `200`, subject to their
token balance. That is by design for this patch.

Before introducing a cumulative cap, maintainers must agree on what it means:

- **Lifetime funded volume**: maintain `CumulativeDeposited(user)` that only
  increases, even if the user withdraws; this prevents resetting the cap by
  withdrawing and redepositing.
- **Current net exposure**: define precisely whether unlocked and locked
  balances count, how withdrawals and multiple matured locks reduce exposure,
  and how negative or overflow cases are rejected.

A safe future implementation would add a dedicated per-user counter (and a
configurable ceiling), use checked addition before the SAC transfer, and update
the counter atomically with token custody and internal accounting. Define the
reset/window policy and migration/backfill behavior **before** deploying it.
Neither current available balance alone nor an unbounded scan over lock
records is a reliable lifetime-cap counter. SDKs must **not** describe the
present `max_deposit_amount` as a wallet-level or lifetime deposit cap.

## Upgrade, verification and rollout

Adding configuration state and a field to `ContractConfig` changes the
serialized response shape. Coordinate contract/SDK releases and confirm
compatibility with existing deployed storage before upgrading; do not assume
the old contract WASM or indexers expose the new getter. For already-initialized
contracts the unset maximum reads as `0` (no maximum), preserving their
per-call deposit behavior until the admin explicitly sets one.

Four focused regression cases live in
`contracts/savings_vault/src/test/maximum_deposit_amount.rs` and cover
inclusive boundaries, unchanged SAC custody on rejected deposits, repeat
deposits, disabling the limit, inconsistent configuration, and unauthorized
or negative updates. They are registered through
`contracts/savings_vault/src/test/mod.rs`. **They are not claimed as run or
passing here**. Contract deployment, hosted CI and any reward acceptance
remain separate gates.
