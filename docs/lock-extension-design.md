# Lock Duration Extension Design & Specification

## 1. Overview & Rationale

The **Lock Duration Extension** feature allows users of the **PocketPay Savings Vault** to extend the unlock timestamp of an existing, active time-lock (`LockEntry`) to a further point in the future.

### Use Cases & Mobile UX Benefits
- **Disciplined Savings**: A user decides to increase their savings horizon without needing to withdraw and re-lock funds.
- **Yield & Bonus Tier Alignment**: Wallet applications offering promotional tiers or interest boosts based on lock duration can allow users to top-up lock time smoothly.
- **Gas Efficiency**: Extending an existing lock mutates a single storage entry in-place rather than requiring a withdrawal transaction followed by a new lock transaction.

---

## 2. Interface Definition & Parameter Rules

```rust
pub fn extend_lock(env: Env, user: Address, lock_id: u64, new_unlock_time: u64)
```

### Parameter Validation & Boundary Rules

| Validation Check | Condition | Contract Behavior |
|---|---|---|
| **Contract Status** | `!is_initialized()` or invalid storage version | Panics with `"Contract is not initialized"` or `"Unsupported storage version"` |
| **Emergency Pause** | Contract is paused and pause hasn't expired | Panics with `"Contract is paused"` |
| **Caller Authorization** | `user.require_auth()` | Reverts if caller signature does not match `user` |
| **Lock Existence** | Lock ID `lock_id` exists in persistent storage | Panics with `"Lock not found"` |
| **Withdrawal Status** | `lock.withdrawn == false` | Panics with `"Lock already withdrawn"` |
| **Future Timestamp** | `new_unlock_time > env.ledger().timestamp()` | Panics with `"Unlock time must be in the future"` |
| **Strict Duration Extension** | `new_unlock_time > lock.unlock_time` | Panics with `"New unlock time must be strictly greater than current unlock time"` |

> **Note on Lock Shortening**: Lock duration shortening (`new_unlock_time <= lock.unlock_time`) is **strictly forbidden**. Allowing lock shortening would defeat the purpose of time-bound savings locks and allow users to bypass lock commitments.

---

## 3. Authorization Architecture

- **Required Signer**: The `user` address (`Address::require_auth()`).
- **Owner Verification**: The contract verifies that `DataKey::Lock(user, lock_id)` belongs to the authenticating `user`.
- **No Third-Party Alteration**: Neither arbitrary 3rd party addresses nor the system `admin` can extend or alter a user's lock maturity without explicit `user` signature.

---

## 4. Accounting & Storage Impact

Lock duration extension is an **in-place metadata mutation**.

### Zero Net Accounting Impact
- **Available Balance (`DataKey::Balance(user)`)**: **Unchanged** ($0$ delta).
- **Total Locked Principal (`get_locked_balance`)**: **Unchanged** ($0$ delta).
- **SAC Token Balance at Vault Address**: **Unchanged** ($0$ token movement).
- **Storage Cost**: Updates `lock.unlock_time` inside existing `LockEntry` stored at `DataKey::Lock(user, lock_id)`.

---

## 5. Event Schema & Off-Chain Indexing

Emits an `extend_lock` structured event upon successful extension.

### Event Specification

- **Topic 0**: `Symbol::new(&env, "extend_lock")`
- **Topic 1**: `user` (`Address`) - Lock owner address.
- **Data Payload**: Tuple `(lock_id: u64, old_unlock_time: u64, new_unlock_time: u64, amount: i128)`

#### Example Event Representation
```json
{
  "contract": "C...SAVINGS_VAULT_ADDRESS",
  "topics": ["extend_lock", "G...USER_ADDRESS"],
  "value": [1, 1785000000, 1790000000, 1500000000]
}
```

---

## 6. Test Suite & Verification

The feature is locked down by unit test suite [`contracts/savings_vault/src/test/lock_extension.rs`](../contracts/savings_vault/src/test/lock_extension.rs):

1. `test_extend_lock_success`: Asserts storage update, accounting balance immutability, and event emission.
2. `test_extend_lock_defers_maturity`: Confirms `can_withdraw` returns `false` at old maturity and `true` only at new maturity.
3. `test_extend_lock_shortening_rejected`: Verifies rejection of lock duration reduction.
4. `test_extend_lock_same_duration_rejected`: Verifies rejection of same-timestamp extension calls.
5. `test_extend_lock_past_timestamp_rejected`: Verifies rejection of timestamps $\le$ current ledger time.
6. `test_extend_already_withdrawn_lock_rejected`: Verifies rejection on withdrawn locks.
7. `test_extend_lock_while_paused_rejected`: Verifies pause restriction.
