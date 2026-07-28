# Savings Vault Error Reference (Canonical)

This reference documents the **real, contract-defined `ContractError` enum**
in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs).
The contract uses `#[contracterror]` with a `#[repr(u32)]` discriminant, so
errors are exposed to SDKs and mobile clients as **stable `u32` codes** — not
as arbitrary panic strings.

> **Breaking-change contract.** Numeric codes in this file are part of the
> cross-repo SDK interface. Any code renumber here MUST be version-bumped and
> co-ordinated with SDK + mobile releases. See the mapping guidance in
> [`sdk-error-mapping-guide.md`](./sdk-error-mapping-guide.md).

## Category Ranges

| Range | Category | Examples |
| --- | --- | --- |
| **1001–1099** | **Validation** | Bad amounts, bad timestamps, bad durations |
| **2001–2099** | **Authorisation** | Wrong caller role (non-admin calling admin-only) |
| **3001–3099** | **Lifecycle** | Initialization, pause state |
| **4001–4099** | **Accounting** | Insufficient available balance |
| **5001–5099** | **Locks** | Missing lock, already withdrawn, immature, unchanged extend |
| **6001–6099** | **Storage** | Schema version, missing required entry |
| **7001–7099** | **Token** | Token configuration issues |
| **8001–8099** | **Admin rotation** | Invalid new-admin address |

## 1000s — Validation

### `AmountNotPositive` (1001)

- **Raised by:** `deposit`, `withdraw`, `lock_funds`
- **Meaning:** The submitted `amount` is `0` or negative.
- **Likely cause:** Empty field coerced to `0`, a sign bug, or a unit-conversion
  bug between whole-token and stroops / the asset's decimal exponent.
- **Caller action:** Block submission client-side until `amount > 0`. Format the
  error to show the asset's decimal representation, not the raw `i128` stroop
  value.

### `UnlockTimeNotInFuture` (1002)

- **Raised by:** `lock_funds`, `extend_lock`
- **Meaning:** `unlock_time <= current ledger timestamp`.
- **Likely cause:** Past time, seconds / ms confusion, clock skew, or choosing a
  time too close to submission.
- **Caller action:** Send Unix time in **seconds** and leave a safety margin
  (≥ 30 s) above the last-seen ledger time.

### `LockDurationExceedsMaximum` (1003)

- **Raised by:** `lock_funds`
- **Meaning:** `unlock_time - now > max_lock_duration`.
- **Likely cause:** Wrong asset config read, or a UI picker allowing years
  beyond the configured max.
- **Caller action:** Read `MaxLockDuration` from config (or its storage-backed
  setter) and clamp the picker before submission.

### `LockDurationBelowMinimum` (1004)

- **Raised by:** `lock_funds`
- **Meaning:** `unlock_time - now < min_lock_duration`.
- **Likely cause:** UX allowing 1-second locks when min is e.g. 1 day.
- **Caller action:** Same as above — pre-clamp.

### `AmountBelowMinimumDeposit` (1005)

- **Raised by:** `deposit`
- **Meaning:** `amount < min_deposit_amount`.
- **Likely cause:** Asset decimal mismatch, or a UX not honouring the configured
  floor.
- **Caller action:** Read the configured min and reject below it client-side.

### `PauseDurationMustBePositive` (1006)

- **Raised by:** `pause` (admin)
- **Meaning:** `duration_secs == 0` on the admin pause call.
- **Likely cause:** Accidental zero input.
- **Caller action:** Admin UI only; enforce a minimum (e.g. 1 hour) when
  submitting.

### `MinDepositAmountNegative` (1007)

- **Raised by:** `set_min_deposit_amount` (admin)
- **Meaning:** Attempt to set the global min deposit to a negative `i128`.
- **Likely cause:** Admin-console sign bug.
- **Caller action:** Admin console validation.

## 2000s — Authorisation

### `NotAuthorizedAdmin` (2001)

- **Raised by:** all admin-only entrypoints (`pause`, `unpause`,
  `set_min_deposit_amount`, `set_max_lock_duration`, `set_min_lock_duration`,
  `transfer_admin`)
- **Meaning:** The `require_auth`-verified caller does not match the stored
  `Admin`.
- **Likely cause:** Wrong wallet connected, or trying to use a governance role
  that was never transferred to.
- **Caller action:** Confirm the connected address is the current admin (use
  `get_admin()`), or — for app backends — route through a signer that holds
  the admin role.
- **Distinction from host auth failures:** Soroban's host-level auth failures
  (e.g. signature missing) raise a host `Status(Auth, …)`; this code is a
  contract-level **role check** that runs AFTER the host has confirmed the
  caller signed.

## 3000s — Lifecycle

### `AlreadyInitialized` (3001)

- **Raised by:** `initialize`
- **Meaning:** The `StorageVersion` flag already exists.
- **Likely cause:** A double-call to `initialize` (e.g. an infra retry after a
  success that the caller didn't observe) or pointing at the wrong deployed
  contract.
- **Caller action:** Do not retry. Use `get_version()` / `get_token()` to
  confirm the contract is already live.

### `NotInitialized` (3002)

- **Raised by:** every guarded public entrypoint (`get_version`, `get_token`,
  `pause`, `deposit`, `withdraw`, `lock_funds`, `list_locks`, `get_admin`, …)
- **Meaning:** The contract was deployed but `initialize` hasn't run.
- **Likely cause:** A deploy script that forgot the init call, or a race where
  the UI renders operations before init lands on-chain.
- **Caller action:** Gate all vault UI behind a contract-ready check
  (`get_version()` succeeds).

### `ContractPaused` (3003)

- **Raised by:** state-mutating non-withdrawal operations (`deposit`,
  `lock_funds`, `extend_lock`, plus admin-setters if the team later chooses
  to tighten them). `withdraw`, `withdraw_lock`, and reads remain **allowed**.
- **Meaning:** Emergency pause active and not yet expired.
- **Likely cause:** Admin-incident response.
- **Caller action:** Show an incident banner. Allow / encourage withdrawal;
  block new deposits and lock creation. Use `is_paused()` to poll expiry.

## 4000s — Accounting

### `InsufficientBalance` (4001)

- **Raised by:** `withdraw`
- **Meaning:** `amount > available_balance(user)`. Locked funds are NOT in
  `available_balance`.
- **Likely cause:** Stale displayed balance, or the user is trying to withdraw
  more than their unlocked funds.
- **Caller action:** Call `get_balance(user)` first, cap the submit, and show
  "Locked balance is unavailable. Wait for it to mature with `can_withdraw`."

### `InsufficientBalanceToLock` (4002)

- **Raised by:** `lock_funds`
- **Meaning:** `amount > available_balance(user)`.
- **Semantic disambiguation from 4001:** Same underlying test, different
  operation. SDKs SHOULD show a distinct copy:
  - 4001 → "You don't have enough available to withdraw **X** tokens."
  - 4002 → "You don't have enough unlocked balance to lock **X** tokens."

## 5000s — Locks

### `LockNotFound` (5001)

- **Raised by:** `withdraw_lock`, `extend_lock`
- **Meaning:** No `Lock { amount, unlock_time, withdrawn }` stored under the
  `DataKey::Lock(owner, id)` key.
- **Likely cause:** Stale lock ID, lock storage TTL expired and was not
  bumped, or wrong owner (the lookup is scoped to the authenticated owner).
- **Caller action:** Re-fetch via `get_lock` / `list_locks`. If TTL expiry is
  plausible, check storage TTL tooling.

### `LockAlreadyWithdrawn` (5002)

- **Raised by:** `withdraw_lock`, `extend_lock`
- **Meaning:** The lock's `withdrawn` boolean is `true`.
- **Likely cause:** UI double-submit after a success the user didn't see, or a
  retry of a completed transaction.
- **Caller action:** Idempotent on the client side: if `get_lock(owner, id)`
  says `withdrawn == true`, treat as success and don't re-submit.

### `LockNotMatured` (5003)

- **Raised by:** `withdraw_lock`
- **Meaning:** `now < lock.unlock_time`.
- **Likely cause:** UI enabled the "withdraw lock" button too early due to
  local-clock skew, or the user manually forced the call.
- **Caller action:** Gate the button behind `can_withdraw(user)` AND a
  per-lock `now >= unlock_time` check using the last-seen ledger timestamp,
  not local device time.

### `ExtendLockTimeNotIncreased` (5004)

- **Raised by:** `extend_lock`
- **Meaning:** `new_unlock_time <= lock.unlock_time`.
- **Likely cause:** UX that lets the user pick an earlier time when "extending".
- **Caller action:** Pre-clamp to `max(lock.unlock_time + 1, selection)`.

## 6000s — Storage

### `StorageVersionUnsupported` (6001)

- **Raised by:** `try_migrate`, `assert_supported_storage_version`
- **Meaning:** On-chain storage has a version strictly greater than the code's
  `CURRENT_STORAGE_VERSION` (downgrade / rollback attempt), OR migration code
  cannot interpret the stored layout.
- **Likely cause:** Wrong WASM deployed (older version vs. newer storage), or
  a failed upgrade path.
- **Caller action:** Escalate to contract deployment owners; do NOT auto-retry.

### `RequiredStorageEntryMissing` (6002)

- **Raised by:** paths that `.unwrap_or_else(|| RequiredStorageEntryMissing)`
  a required instance-storage cell (e.g. `Admin`, `Token`).
- **Meaning:** Contract storage is internally inconsistent (a mandatory
  singleton was never written or was dropped by an accidental TTL expiry).
- **Likely cause:** Deployment bug: `initialize` failed half-way, or an
  admin-rotation code path omitted the write. In principle unreachable on a
  correctly-initialized contract; presence of 6002 is ALERTS-level.
- **Caller action:** Pause the UI. Page on-call.

## 7000s — Token

### `TokenNotConfigured` (7001)

- **Raised by:** SAC-custody paths if the `Token` storage cell is missing
  (`deposit`, `withdraw`, `withdraw_lock`).
- **Meaning:** The configured asset address is not set; custody transfers can't
  run.
- **Likely cause:** Corrupt initialization or migration.
- **Caller action:** Escalate.

## 8000s — Admin Rotation

### `CannotTransferAdminToSelf` (8001)

- **Raised by:** `transfer_admin`
- **Meaning:** `new_admin == current_admin`.
- **Likely cause:** Form submitted with the same address.
- **Caller action:** Admin console validation; treat as no-op success if the
  user intention was to "keep admin".

### `CannotTransferAdminToContractAddress` (8002)

- **Raised by:** `transfer_admin`
- **Meaning:** `new_admin == contract_address` (i.e. setting the vault itself
  as its own admin, which would permanently orphan admin-only operations).
- **Likely cause:** Paste error selecting the contract instead of the signer.
- **Caller action:** Admin console guard that blacklists the contract's own
  address.

## Error Code Stability

- Existing codes **will never be renumbered** within a major release line.
- New codes **will be added inside their category range** (1001–1099, …) to
  keep SDK route-by-thousand-category logic correct.
- Deprecated codes **will keep their number**; deprecation is signalled via
  variant docs only.
- See [`error-code-standard.md`](./error-code-standard.md) for design rationale.

## SDK Integration

For how to map these `u32` codes into TypeScript / Kotlin / Swift user-facing
messages + analytics, see [`sdk-error-mapping-guide.md`](./sdk-error-mapping-guide.md).
