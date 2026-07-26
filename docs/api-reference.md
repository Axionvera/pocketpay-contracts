# API Reference: Function Naming Conventions

This document reviews the public function names on `SavingsVault`
(`contracts/savings_vault/src/lib.rs`) and records the naming convention they
follow, so future functions stay consistent. It covers naming only — for
behavior, parameters, and response shapes of individual functions, see the
linked docs below.

`SavingsVault` currently exposes only `pub fn` contract entry points; there
are no private/internal helper functions in `lib.rs` to name separately.

## Convention

| Category | Pattern | Purpose |
|---|---|---|
| Command | `<verb>` or `<verb>_<object>` | Mutates contract storage. |
| Single-value query | `get_<noun>` | Returns one stored or derived value. |
| Collection query | `list_<noun>` | Returns a paginated collection. |
| Capability query | `can_<verb>` | Returns `bool`; whether an action is currently allowed. |
| State query | `is_<state>` | Returns `bool`; whether the contract is currently in a state. |

## Current functions

| Function | Category | Notes |
|---|---|---|
| `initialize` | command | One-time setup call. |
| `deposit` | command | |
| `withdraw` | command | |
| `lock_funds` | command | `<verb>_<object>`. Kept as `lock_funds` rather than shortened to `lock` so it reads unambiguously as an action, distinct from the `is_paused`-style state queries. |
| `withdraw_lock` | command | `<verb>_<object>`; withdraws a specific lock by ID. |
| `pause` | command | |
| `unpause` | command | |
| `transfer_admin` | command | `<verb>_<object>`. |
| `get_balance` | single-value query | |
| `get_locked_balance` | single-value query | |
| `get_lock` | single-value query | Returns one lock by ID. See [Lock Read Helpers](lock-read-helpers.md). |
| `get_version` | single-value query | See [Version Metadata](version-metadata.md). |
| `get_token` | single-value query | |
| `get_admin` | single-value query | |
| `list_locks` | collection query | Paginated. See [Lock Read Helpers](lock-read-helpers.md). |
| `can_withdraw` | capability query | Answers "is this action currently allowed," not just current state. |
| `is_paused` | state query | Answers "is the contract currently in this state," not an action's permission. |

## `can_` vs. `is_`

Both prefixes name boolean-returning queries, but for different questions:

- `can_<verb>` asks whether an *action* is currently permitted
  (`can_withdraw`).
- `is_<state>` asks whether the contract is currently *in a state*
  (`is_paused`).

Keep this distinction when adding new boolean queries instead of using one
prefix for both.

## Outcome of this review

No renames were made. All seventeen public functions already follow one of
the patterns above, and per the acceptance criteria for this review,
unnecessary breaking renames of public functions that SDK or mobile code may
already depend on should be avoided (see
[SDK ↔ Contract Sequence Diagrams](sdk-contract-sequence.md) and
[Contract Invocation Examples](invocation-examples.md) for existing call
sites). New public functions should pick the category above that matches
their behavior and follow its pattern.

## Scope

This document covers naming conventions for `SavingsVault` contract
functions only. For test function naming, see [testing.md](testing.md).
