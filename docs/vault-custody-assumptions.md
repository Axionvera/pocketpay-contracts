# Vault Custody Assumptions

## Savings Vault Contract — Token-Backed Custody Model

---

## Overview

The Savings Vault contract holds user tokens on-chain. This document makes explicit what the contract guarantees, what it does not guarantee, and the assumptions it makes about the configured token, the Stellar network, and its own correctness.

**Intended audience**: integrators, auditors, wallet developers, SDK maintainers.

---

## 1. What the Vault Guarantees

### 1.1 Deposit Atomicity

**Guarantee**: A successful `deposit(user, amount)` call does exactly two things atomically:
1. Transfers `amount` tokens from `user` to the contract via SAC
2. Credits `amount` to the user's internal balance

**Proof**: The SAC transfer (line 427) occurs before the balance update (line 438). Both happen within a single Soroban transaction — either both succeed or neither does.

**What this means for integrators**:
- A wallet showing a "deposit pending" state after a confirmed transaction is incorrect — the deposit is either fully credited or fully rejected.
- A failed deposit (panic) leaves the user's token balance, internal balance, and lock state completely unchanged.

### 1.2 Withdrawal Atomicity

**Guarantee**: A successful `withdraw(user, amount)` correctly:
1. Deducts `amount` from the user's available balance (deposited balance + matured locks)
2. Transfers `amount` tokens from the contract to `user` via SAC

**Proof**: The SAC transfer (line 499) happens before the storage update (line 539-544). If the transfer fails, the transaction reverts — balance, locks, and events remain as they were.

### 1.3 Balance Conservation

**Guarantee**: `get_balance(user) + get_locked_balance(user)` always equals the net amount deposited by that user (minus any withdrawals). The vault never creates or destroys tokens.

**Proof**: Property-based tests (`property_vault_accounting.rs`, `balance_conservation.rs`) verify this invariant through thousands of random operation sequences.

### 1.4 Lock Immutability

**Guarantee**: Once created, a lock entry's `amount` and `unlock_time` cannot be modified by anyone — including the admin. Locks are only removed by `withdraw()` (which deducts from matured locks) or `withdraw_lock(id)` (which removes a specific matured lock). Neither changes the remaining lock entries.

### 1.5 Admin Boundaries

**Guarantee**: The admin CANNOT:
- Withdraw user funds
- Modify user balances
- Change lock parameters
- Bypass authorization for any user operation

The admin CAN:
- Transfer admin role (to a new address)
- Pause the contract (blocks deposits and locks only — withdrawals remain available)

### 1.6 Withdrawal Availability During Pause

**Guarantee**: During an emergency pause, `withdraw()` and `withdraw_lock()` remain fully functional. Users can always exit. Only deposits and new lock creation are blocked.

---

## 2. What the Vault Does NOT Guarantee

### 2.1 Token Solvency

**Does not guarantee**: The contract always has enough SAC tokens to cover all user balances.

**Why**: The vault tracks balances internally via `DataKey::Balance(user)`. It assumes the SAC token balance equals the sum of all user balances. If tokens are transferred to the contract address outside of `deposit()`, the internal accounting will not reflect them. Conversely, if the SAC token is deflationary (burns on transfer), the contract's actual balance may fall below the sum of user balances.

**Mitigation**: The `deposit()` function requires the user to authorize the transfer. External (non-deposit) transfers to the contract are possible at the SAC level but have no effect on internal accounting.

### 2.2 Token Behavior Standards

**Does not guarantee**: The configured SAC token behaves like a standard Stellar Asset Contract.

**Why**: The vault calls `token_client.transfer()`. If the token has custom logic (fees, blacklists, pausability), the vault's 1:1 accounting assumption may break. A fee-on-transfer token would mean the vault receives fewer tokens than it credits — creating a solvency gap.

**Recommendation**: Only use standard SAC tokens (no custom transfer logic) with the vault. The `initialize()` function does not validate the token's behavior beyond accepting any `Address`.

### 2.3 Front-Running Protection

**Does not guarantee**: Protection against MEV or front-running.

**Why**: Soroban does not have a public mempool in the same sense as Ethereum. However, validators can theoretically reorder transactions. Lock maturity-based attacks (claiming tokens microseconds before a lock matures) are not economically viable in practice due to Soroban's ledger close model, but explicit protection is not implemented.

### 2.4 Upgrade Path

**Does not guarantee**: The contract can be upgraded or its logic changed.

**Why**: The contract has no proxy pattern, no upgrade entrypoint, and no migration mechanism beyond `try_migrate()` for storage layout changes only. Logic changes require deploying a new contract and migrating user funds.

### 2.5 Multi-Sig Admin

**Does not guarantee**: Admin actions require multi-signature approval.

**Why**: The admin is a single address. If the admin key is compromised, the attacker can transfer admin, pause the contract, and potentially front-run withdrawals during an emergency. Multi-sig admin via a Stellar multi-sig account or a separate governance contract is recommended before mainnet.

---

## 3. Known Limitations

### 3.1 Lock Page Size

`list_locks()` is capped at `MAX_LOCK_PAGE_SIZE = 50` entries per call. Users with more than 50 active locks must paginate. The contract does not return a `has_more` flag — callers must track this themselves.

### 3.2 No Partial Lock Withdrawal

`withdraw_lock(id)` withdraws the entire lock amount. There is no way to partially claim a single lock. Partial access to locked funds goes through `withdraw(amount)` which uses available balance first, then matured locks.

### 3.3 No Interest or Yield

The vault is a pass-through custody contract. Deposited tokens do not earn interest, staking rewards, or yield. The vault's purpose is time-based access control, not capital appreciation.

### 3.4 Storage TTL

Soroban persistent storage entries have a Time-To-Live (TTL). If an entry's TTL expires, the data is lost. The vault does not extend TTL automatically. In production, a TTL extension mechanism (cron, keeper, user-triggered) is needed.

### 3.5 No Emergency Token Recovery

There is no admin function to recover tokens from the contract outside the normal withdrawal flow. If the configured token becomes permanently paused or blacklisted, funds may be irretrievable. An emergency recovery path is a planned future feature.

### 3.6 Event Delivery

Events are emitted on-chain but are NOT guaranteed delivery to off-chain consumers. Indexers, relays, and SDKs consume events on a best-effort basis. Event format changes between contract versions may break consumers — see `event_compatibility.rs` for regression tests.

---

## 4. Token Custody Flow

### Deposit Flow

```
User Wallet                Vault Contract              SAC Token
    │                           │                          │
    ├─ deposit(user, 100) ─────→│                          │
    │                           ├─ token.transfer(         │
    │                           │    user→vault, 100) ────→│
    │                           │                          ├─ balance(user) -= 100
    │                           │                          ├─ balance(vault) += 100
    │                           │←─────────────────────────┤
    │                           ├─ balance[user] = 0 + 100 │
    │                           ├─ emit deposit(user, 100) │
    │←── success ───────────────┤                          │
```

### Withdrawal Flow

```
User Wallet                Vault Contract              SAC Token
    │                           │                          │
    ├─ withdraw(user, 50) ─────→│                          │
    │                           ├─ check balance >= 50     │
    │                           ├─ check matured locks     │
    │                           ├─ token.transfer(         │
    │                           │    vault→user, 50) ─────→│
    │←─── tokens arrive ────────┤                          │
    │                           ├─ balance[user] -= 50     │
    │                           ├─ emit withdraw(user, 50) │
    │←── success ───────────────┤                          │
```

---

## 5. Production Readiness Assessment

| Area                | Status              | Notes                                    |
|---------------------|---------------------|------------------------------------------|
| Token custody       | ✅ Complete         | SAC integration tested                   |
| Authorization       | ✅ Complete         | require_auth on all mutating calls       |
| Event emission      | ✅ Complete         | All state changes emit events            |
| Fuzz testing        | ✅ Complete         | Proptest fuzz on accounting invariants   |
| Rollback safety     | ✅ Complete         | Failed transfers tested, state unchanged |
| Emergency pause     | ✅ Complete         | Pause with auto-expiry                   |
| Custom errors       | ❌ Missing          | Panic strings only                       |
| Multi-sig admin     | ❌ Missing          | Single key trust model                   |
| Upgrade path        | ❌ Missing          | No proxy or migration pattern            |
| Storage TTL         | ❌ Missing          | No automatic TTL extension               |
| Token recovery      | ❌ Missing          | No admin emergency withdrawal            |

**Overall**: Safe for testnet. Mainnet deployment requires custom errors, multi-sig admin, and a TTL strategy.

---

## 6. References

- [SECURITY_REVIEW.md](SECURITY_REVIEW.md) — Full security review
- [audit-readiness.md](audit-readiness.md) — Pre-audit assessment
- [accounting-invariants.md](accounting-invariants.md) — Accounting guarantees
- [authorization-boundaries.md](authorization-boundaries.md) — Auth model
- [failure-mode-catalogue.md](failure-mode-catalogue.md) — Known failure modes
- [pause-design.md](pause-design.md) — Emergency pause specification
