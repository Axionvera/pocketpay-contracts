# Vault State Machine

The savings vault contract operates as a state machine with two layers:
a **contract-level** lifecycle and a **per-user** account lifecycle.

## Contract Lifecycle

```
                  initialize(admin, token)
    ┌──────────┐ ────────────────────────────► ┌─────────────┐
    │          │                                │             │
    │ UNINIT   │                                │  INITIALIZED│
    │          │                                │             │
    └──────────┘                                └─────────────┘
         │                                            │
         │  any call except initialize                │  all functions
         │  panics: "Contract is not initialized"     │  available
         ▼                                            ▼
```

| State | Entered by | Exited by |
|-------|------------|-----------|
| `UNINIT` | Contract deployment | `initialize(admin, token)` |
| `INITIALIZED` | `initialize` succeeds | Never (contract is immutable after init) |

### Invalid transitions

| Attempt | Result |
|---------|--------|
| `initialize` on `INITIALIZED` | Panics: "Contract is already initialized" |
| Any function on `UNINIT` | Panics: "Contract is not initialized" |

---

## Per-User Account Lifecycle

Each user's vault state is derived from two storage values:
- **Available balance** (`Balance` key) — funds not locked
- **Lock entries** (`Locks` key) — vector of `LockEntry` records, each with an `unlock_time`

A lock is **active** when `ledger.timestamp() < unlock_time` and **matured** when
`ledger.timestamp() >= unlock_time`. Maturity is evaluated at read time; there is
no explicit unlock operation.

```
                    ┌──────────────────────────────────────┐
                    │                                      │
                    ▼                                      │
              ┌──────────┐   deposit(amount)    ┌──────────┐
     deposit  │          │ ───────────────────► │          │
   ─────────► │  EMPTY   │                      │  ACTIVE  │◄──────────┐
              │          │ ◄─────────────────── │          │           │
              └──────────┘   withdraw(all)      └──────────┘           │
                    ▲                               │    │              │
                    │                               │    │              │
                    │                    lock_funds │    │ time passes  │
                    │                               │    │ (maturity)   │
                    │                               ▼    ▼              │
                    │                          ┌──────────────┐         │
                    │                          │              │         │
                    │                          │   LOCKED     │─────────┘
                    │                          │  (active)    │
                    │                          │              │
                    │                          └──────┬───────┘
                    │                                 │
                    │                    time passes   │
                    │                    (maturity)    │
                    │                                 ▼
                    │                          ┌──────────────┐
                    │         withdraw         │              │
                    └──────────────────────────│   MATURED    │
                                               │              │
                                               └──────────────┘
```

### State definitions

| State | Available balance | Active locks | Matured locks | Meaning |
|-------|-------------------|--------------|---------------|---------|
| `EMPTY` | 0 | 0 | 0 | User has never deposited or has withdrawn everything |
| `ACTIVE` | > 0 | Any | Any | User has unlocked funds available |
| `LOCKED` | 0 | > 0 | Any | All funds are locked; nothing withdrawable yet |
| `MATURED` | 0 | 0 | > 0 | All locks have matured but haven't been withdrawn |

> **Note:** `ACTIVE` and `LOCKED` are not mutually exclusive — a user can have
> both available balance and active locks simultaneously. The table above shows
> the pure archetypes; in practice, users transition fluidly between them.

### Valid transitions

| Transition | Trigger | Result |
|------------|---------|--------|
| `EMPTY` → `ACTIVE` | `deposit(amount)` | Available balance += amount |
| `ACTIVE` → `ACTIVE` | `deposit(amount)` | Available balance += amount |
| `ACTIVE` → `ACTIVE` | `lock_funds(amount, t)` | Available balance -= amount; new lock created |
| `ACTIVE` → `LOCKED` | `lock_funds(all_balance, t)` | Available balance = 0; new lock created |
| `ACTIVE` → `EMPTY` | `withdraw(all_available)` | Available balance = 0; matured locks consumed |
| `LOCKED` → `MATURED` | Time passes (ledger >= unlock_time) | Locks mature automatically |
| `LOCKED` → `ACTIVE` | `deposit(amount)` | Available balance += amount |
| `MATURED` → `EMPTY` | `withdraw(matured_amount)` or `withdraw_lock(id)` | Matured locks removed |
| `MATURED` → `ACTIVE` | `deposit(amount)` | Available balance += amount |

### Invalid transitions

| Attempt | Condition | Error |
|---------|-----------|-------|
| `deposit(0)` | amount <= 0 | "Deposit amount must be greater than zero" |
| `deposit(-n)` | amount <= 0 | "Deposit amount must be greater than zero" |
| `withdraw(0)` | amount <= 0 | "Withdrawal amount must be greater than zero" |
| `withdraw(-n)` | amount <= 0 | "Withdrawal amount must be greater than zero" |
| `withdraw(amount)` | amount > available + matured | "Insufficient balance" |
| `lock_funds(0, t)` | amount <= 0 | "Lock amount must be greater than zero" |
| `lock_funds(amount, t)` | amount > available balance | "Insufficient balance to lock" |
| `lock_funds(amount, t)` | t <= ledger.timestamp() | "Unlock time must be in the future" |
| `withdraw_lock(id)` | lock not found | "Lock not found" |
| `withdraw_lock(id)` | lock.active (not matured) | "Lock has not matured yet" |

---

## Token Transfer Flow

```
   deposit:  User ──(token transfer)──► Vault contract ──(credit)──► User balance
  withdraw:  User balance ──(debit)──► Vault contract ──(token transfer)──► User
```

The token transfer happens **before** the internal balance update on deposit,
and **before** the balance deduction on withdraw. If the token transfer fails,
the transaction reverts and no state changes are persisted.

---

## Authorization Model

| Operation | Who must authorize |
|-----------|-------------------|
| `initialize` | Admin address |
| `deposit` | User address |
| `withdraw` | User address |
| `withdraw_lock` | User address |
| `lock_funds` | User address |
| `transfer_admin` | Current admin address |
| `get_balance`, `get_locked_balance`, `can_withdraw`, `get_lock`, `list_locks`, `get_admin`, `get_version` | No authorization required |

---

## Error States Summary

| Error | Panic message | Triggered in |
|-------|--------------|-------------|
| Not initialized | "Contract is not initialized" | All functions except `initialize` |
| Already initialized | "Contract is already initialized" | `initialize` (2nd call) |
| Bad storage version | "Unsupported storage version" | `deposit`, `withdraw`, `lock_funds` |
| Zero/negative amount | "… amount must be greater than zero" | `deposit`, `withdraw`, `lock_funds` |
| Insufficient balance | "Insufficient balance" | `withdraw` |
| Insufficient balance to lock | "Insufficient balance to lock" | `lock_funds` |
| Past unlock time | "Unlock time must be in the future" | `lock_funds` |
| Lock not found | "Lock not found" | `withdraw_lock` |
| Lock not matured | "Lock has not matured yet" | `withdraw_lock` |
| Not admin | "Not authorized" | `transfer_admin` |