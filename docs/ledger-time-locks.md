# Ledger Time and Lock Maturity Documentation

This document explains how the **Savings Vault Contract** utilizes Stellar Soroban **ledger time** for time-based fund locking, lock maturity validation, emergency pause auto-expiration, boundary conditions, and the testing approach.

---

## 1. Ledger Time Assumptions

Soroban smart contracts operate on a deterministic on-chain clock exposed via the environment API (`env.ledger().timestamp()`).

### Core Assumptions
- **Time Source**: `env.ledger().timestamp()` returns the Unix timestamp (in seconds) recorded in the header of the current Stellar ledger. The timestamp is established by validator consensus.
- **Monotonicity**: Ledger timestamps are strictly non-decreasing across consecutive ledgers (`T_{ledger N+1} >= T_{ledger N}`).
- **Granularity & Units**: Time is measured in 64-bit unsigned integer seconds (`u64`). Sub-second granularity is not supported by the ledger header timestamp.
- **Ledger Closing Resolution**: On the Stellar network, ledgers close approximately every 5–6 seconds. Time advances in discrete steps rather than continuously.
- **No External Clock Dependence**: The contract does not rely on off-chain web servers or local client clocks. All time validations are evaluated exclusively against the validator-consensus ledger timestamp.

---

## 2. Lock Mechanics & Maturity Checks

When funds are locked using `lock_funds(user, amount, unlock_time)`, a [`LockEntry`](../contracts/savings_vault/src/lib.rs) record is created in persistent storage.

### Lock Properties
- `created_time` (`u64`): Recorded as the current ledger timestamp `env.ledger().timestamp()` at transaction execution time.
- `unlock_time` (`u64`): Target Unix timestamp in seconds after which the locked funds become withdrawable.

### State Transition & Validation Rules

| Operation / Check | Condition | Contract Action |
| :--- | :--- | :--- |
| **Lock Creation** | `unlock_time > env.ledger().timestamp()` | **Allowed**: Deducts `amount` from available balance and stores `LockEntry`. |
| **Lock Creation** | `unlock_time <= env.ledger().timestamp()` | **Rejected**: Panics with `"Unlock time must be in the future"`. Zero-duration and past timestamps are strictly forbidden. |
| **Maturity Check** | `env.ledger().timestamp() >= lock.unlock_time` | **Matured**: Funds are eligible for withdrawal. `can_withdraw(user)` returns `true`. |
| **Withdrawal Attempt** | `withdraw_lock(user, lock_id)` where `current_time < lock.unlock_time` | **Rejected**: Panics with `"Lock has not matured yet"`. |
| **Emergency Pause Expiry**| `current_time >= pause_expiry` | **Auto-Unpause**: Active emergency pause automatically expires and clears on the next mutating call. |

### Available vs. Locked Balance Rules
1. Calling `lock_funds` immediately reduces the user's available balance (`DataKey::Balance(user)`) by `amount` and stores the lock in `DataKey::Lock(user, lock_id)`.
2. Matured locks (`current_time >= unlock_time`) are **not** automatically transferred back to `Balance(user)` on read queries (`get_balance` returns unlocked deposits only).
3. To claim matured funds, the user invokes `withdraw_lock(user, lock_id)`, which verifies maturity, transfers SAC tokens from contract custody to the user's wallet, and sets `lock.withdrawn = true`.

---

## 3. Boundary Conditions & Edge Cases

### 1. Inclusive Creation Boundary (`unlock_time <= current_time`)
- Setting `unlock_time == current_time` (a zero-duration lock) is **rejected**.
- Because the condition checked is `unlock_time <= current_time`, a lock cannot be created in a state where it is already matured at the exact second of creation.

### 2. Minimum Valid Lock Duration (`unlock_time == current_time + 1`)
- The shortest lock duration accepted by `lock_funds` is **1 second in the future** (`unlock_time = current_time + 1`).
- At $T = \text{creation\_time}$, the 1-second lock remains immature (`can_withdraw` returns `false`).
- At $T = \text{creation\_time} + 1$, the ledger timestamp reaches `unlock_time`. The maturity operator `current_time >= unlock_time` evaluates to `true`, making the lock immediately withdrawable via `withdraw_lock`.

### 3. Exact Maturity Second (`current_time == unlock_time`)
- Maturity validation uses inclusive comparison (`>=`).
- As soon as `env.ledger().timestamp()` equals `unlock_time`, `withdraw_lock` succeeds. No additional offset or delay beyond `unlock_time` is required.

### 4. One Second Prior (`current_time == unlock_time - 1`)
- At `unlock_time - 1`, the lock is still immature. Calling `withdraw_lock` panics with `"Lock has not matured yet"`.

### 5. Emergency Pause Expiration Boundary
- When an admin invokes `pause(admin, duration_secs)`, the pause expiry is calculated as `expiry = current_time + duration_secs`.
- At `current_time < expiry`, `is_paused()` returns `true`, blocking `deposit` and `lock_funds`.
- At `current_time >= expiry`, `is_paused()` returns `false` and mutating calls clear the pause flag lazily without requiring explicit `unpause(admin)` calls.

---

## 4. Testing Approach for Time-Based Logic

Testing time-dependent smart contract logic requires precise, reproducible control over the ledger timestamp.

### Simulation in Soroban Unit Tests
The test suite utilizes the `soroban-sdk::testutils` framework and a standardized helper function defined in [`contracts/savings_vault/src/test/test_helpers.rs`](../contracts/savings_vault/src/test/test_helpers.rs):

```rust
/// Sets the ledger's current timestamp (in unix seconds) for tests that
/// simulate time-based behaviour, such as lock/unlock schedules.
pub fn set_ledger_timestamp(env: &Env, timestamp: u64) {
    env.ledger().set_timestamp(timestamp);
}
```

### Dedicated Test Modules

| Test Module | Coverage & Test Focus |
| :--- | :--- |
| [`zero_duration_lock.rs`](../contracts/savings_vault/src/test/zero_duration_lock.rs) | Tests rejection of `unlock_time == current_time` across zero and non-zero base timestamps, confirms failed locks do not mutate balances, and verifies exact 1-second duration lock maturity. |
| [`withdraw_lock.rs`](../contracts/savings_vault/src/test/withdraw_lock.rs) | Verifies that `withdraw_lock` panics when `current_time < unlock_time`, succeeds when `current_time >= unlock_time`, and rejects duplicate withdrawal attempts on already-withdrawn locks. |
| [`pause.rs`](../contracts/savings_vault/src/test/pause.rs) | Tests admin pause activation, zero-duration pause rejection, auto-expiration when timestamp advances past `pause_expiry`, and unpause overrides. |
| [`replay_protection.rs`](../contracts/savings_vault/src/test/replay_protection.rs) | Confirms that advancing ledger timestamps does not alter authorization requirements or compromise isolation across user accounts. |

---

## Summary for Developers & Integrators

- **Always pass future timestamps**: Frontends and SDKs must specify `unlock_time` strictly greater than the estimated current ledger timestamp.
- **Account for ledger interval**: Because Stellar ledgers close every ~5 seconds, setting an `unlock_time` only 1–2 seconds in the future may cause the transaction to execute in a ledger where `current_time >= unlock_time` has already passed if submission is delayed, or fail if processed immediately in the same ledger.
- **Use `can_withdraw` for UI state**: Mobile clients should query `can_withdraw(user)` or inspect `unlock_time` against `env.ledger().timestamp()` before enabling withdrawal UI elements.
