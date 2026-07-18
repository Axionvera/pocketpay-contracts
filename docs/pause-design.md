# Pause / Emergency Stop Design

> **Status:** Research & Design (not implemented)
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This document explores whether a pause (emergency stop) mechanism should be added to the Savings Vault contract in a future release. It is a research artifact — **no pause logic is being implemented as part of this document**.

---

## Table of Contents

1. [Motivation](#motivation)
2. [What Could Be Paused](#what-could-be-paused)
3. [Who Could Trigger a Pause](#who-could-trigger-a-pause)
4. [Abuse Risks](#abuse-risks)
5. [Recovery Process](#recovery-process)
6. [Alternatives to a Pause](#alternatives-to-a-pause)
7. [Design Sketch (Reference)](#design-sketch-reference)
8. [Open Questions](#open-questions)
9. [Recommendation](#recommendation)

---

## Motivation

The Savings Vault currently has **no emergency stop capability**. If a critical bug is discovered, there is no way for the admin to halt contract operations while the issue is investigated and resolved. This is common in early-stage DeFi contracts but becomes a liability as real value accumulates.

### Scenarios where a pause could be useful

| Scenario | Impact without Pause | How Pause Helps |
|---|---|---|
| **Critical bug discovered** | Funds at risk while fix is developed | Stop all state-changing operations immediately |
| **Oracle / price feed anomaly** | Incorrect lock/unlock logic triggered | Pause until feed is healthy again |
| **Protocol-level exploit (e.g. Soroban host function)** | Cascading attack across contracts | Contain damage; protect user balances |
| **Governance attack on admin key** | Malicious admin could drain funds | A time-delayed pause gives users a window to exit |
| **Unexpected ledger behavior** | Ledger close times or timestamps become unreliable | Pause time-sensitive lock operations |

### When a pause is NOT useful

- User-level errors (wrong amount, wrong address) — these are self-correcting.
- Minor UI bugs that do not affect on-chain state.
- Temporary network congestion (the Stellar network itself handles this).

---

## What Could Be Paused

Not every function needs to be pauseable. Granularity matters: the more functions paused, the safer the contract but the more disruptive to users. The table below evaluates each function:

| Function | Pause? | Rationale |
|---|---|---|
| `deposit` | **Yes** | Prevent users from depositing into a potentially compromised vault. Depositors would otherwise be unaware of the risk. |
| `withdraw` | **Debatable** | Pausing withdrawals traps user funds, which is a severe trust violation. An alternative is to allow *only withdrawals* during a pause (a "withdraw-only" emergency mode). |
| `lock_funds` | **Yes** | Lock operations are complex and touch unlock-time logic; they should be stopped during an incident. |
| `get_balance` | **No** | Read-only; no risk. Should always remain available. |
| `get_locked_balance` | **No** | Read-only; no risk. |
| `can_withdraw` | **No** | Read-only; no risk. |

### Recommended granularity

Three levels of pause, from least to most restrictive:

1. **Deposits paused** — No new funds can enter the vault. Withdrawals and locks continue normally. Useful for gradual wind-down.
2. **State changes paused** — Deposits and locks paused; withdrawals still allowed. The "withdraw-only" safety net.
3. **Full pause** — All mutating functions paused. Used only in extreme emergencies with a known short resolution path.

---

## Who Could Trigger a Pause

Centralization of the pause authority is the single biggest risk in this design. The following options are ordered from most centralized (simplest) to most decentralized (complex).

### Option A: Single Admin Key (current admin)

- **Pros:** Simple to implement; fast response in emergencies.
- **Cons:** If the admin key is compromised, the attacker can pause the contract indefinitely, effectively freezing all user funds (a **griefing vector**). This replaces one risk (contract bug) with another (key compromise).
- **Verdict:** Acceptable for testnet; unacceptable for mainnet without additional safeguards.

### Option B: Multi-signature Admin

- **Pros:** Requires M-of-N signatures to pause/unpause, raising the bar for an attacker.
- **Cons:** Slower to react; coordination overhead; still a centralized set of signers.
- **Verdict:** A reasonable step up from Option A for a beta mainnet launch.

### Option C: Time-Limited + Guardian Set

- **Pros:** A pause automatically expires after a configurable duration (e.g., 7 days). A set of "guardians" (could be protocol team, community members, or oracles) can independently trigger a pause. No single entity can pause indefinitely.
- **Cons:** More complex to implement and test. Guardians must be compensated or incentivized.
- **Verdict:** Best-practice for production-grade DeFi. Used by Aave, Compound, and MakerDAO.

### Option D: DAO / Governance Vote

- **Pros:** Fully decentralized; aligned with web3 ethos.
- **Cons:** Extremely slow (hours to days); useless for time-critical exploits. Governance attacks are a real threat.
- **Verdict:** Not suitable as the *only* pause mechanism, but can complement a guardian set.

---

## Abuse Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Admin freezes funds permanently** | Critical | Time-bounded pauses; multi-sig; guardian rotation |
| **Attacker pauses to manipulate market** | High | Event emission on pause/unpause for transparency |
| **Pause used to censor specific users** | Medium | Pause applies globally, not per-user (no targeted censorship) |
| **Pause masking an ongoing exploit** | High | Pause event must include a reason string; off-chain monitoring alerts |
| **Race condition: pause during a cross-contract call** | Medium | Soroban's atomic transaction model mitigates this; a pause check at the top of each function is sufficient |

### The "Pause as a Weapon" Problem

The most dangerous abuse is an admin who pauses the contract and *never unpauses*. This is indistinguishable from a rug pull from the user's perspective. Mitigations:

- **Maximum pause duration:** A hard-coded limit (e.g., 30 days) after which the contract auto-unpauses.
- **Withdraw-only fallback:** Even during a full pause, if the pause exceeds N days, switch to withdraw-only mode so users can exit.
- **Pause events:** Emit a Soroban event on every pause/unpause so indexers and watchdogs can alert the community.

---

## Recovery Process

A well-defined recovery process is as important as the pause mechanism itself.

### Proposed workflow

```
      +-------------+
      |   NORMAL    |
      +------+------+
             |
      Pause triggered
      (guardian / admin)
             |
             v
      +------+------+
      |   PAUSED    |<--------+
      +------+------+         |
             |                |
      Incident investigated   |
      Fix deployed            |
             |                |
             v                |
      +------+------+         |
      |  UNPAUSING  |---------+ (re-pause if fix is incomplete)
      +------+------+
             |
      Unpause confirmed
             |
             v
      +------+------+
      |   NORMAL    |
      +-------------+
```

### Steps

1. **Trigger:** Guardian(s) call `pause(reason)`. An event is emitted with the reason string and block timestamp.
2. **Communicate:** Off-chain channels (Discord, Twitter, status page) inform users of the pause and expected resolution time.
3. **Diagnose:** Developers investigate the root cause. The contract remains paused.
4. **Fix:** A patched contract is developed, tested, and (if the fix requires a new WASM) deployed via `upgrade()`.
5. **Verify:** The fix is reviewed by at least one other developer.
6. **Unpause:** Guardian(s) call `unpause()`. An event is emitted.
7. **Post-mortem:** A public post-mortem is published within 72 hours.

### What if the admin key is lost?

If the only pause authority is lost and the contract is paused, recovery becomes impossible. This is another argument for:

- Multi-sig guardians (no single point of failure).
- Auto-expiring pauses.

---

## Alternatives to a Pause

Before implementing a full pause mechanism, consider these lighter-weight alternatives:

### 1. Circuit Breakers (per-function limits)

Instead of a binary pause, enforce per-transaction or per-block limits:

```rust
// Example: max deposit per transaction
const MAX_DEPOSIT: i128 = 1_000_000_000_000; // 1M XLM in stroops
```

- **Pros:** No admin authority needed; self-enforcing.
- **Cons:** Cannot respond to novel attack vectors. An attacker can split large exploits across many transactions.

### 2. Timelock on Admin Actions

Require a delay (e.g., 48 hours) before any admin action takes effect. During the delay, users can withdraw.

- **Pros:** Gives users an exit window; no pause needed.
- **Cons:** Useless against instant exploits (e.g., flash-loan attacks).

### 3. Upgrade-Only Safety

If the contract implements an `upgrade()` function, a bug can be fixed by deploying a new WASM. The old contract remains functional during the fix window.

- **Pros:** No pause logic needed; contract stays simple.
- **Cons:** Upgrade may take hours/days; no way to stop activity during that window.

### 4. Social Layer

Rely on off-chain communication: if a bug is found, ask users to stop interacting via social channels.

- **Pros:** Zero implementation cost.
- **Cons:** Not enforceable; unlikely to work at scale or with malicious actors.

---

## Design Sketch (Reference)

If a pause mechanism were implemented, it might look like this in the Soroban SDK:

```rust
// Storage keys
enum DataKey {
    // ... existing keys ...
    Paused,            // bool — global pause flag
    PauseExpiry,       // u64  — timestamp when pause auto-expires
}

// Events
fn pause_event(env: &Env, reason: Symbol, paused_by: Address);
fn unpause_event(env: &Env, unpaused_by: Address);

// Modifier-style check (Soroban has no macros, so inline it)
fn require_not_paused(env: &Env) {
    let paused: bool = env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);

    if paused {
        // Check auto-expiry
        let expiry: u64 = env.storage()
            .instance()
            .get(&DataKey::PauseExpiry)
            .unwrap_or(0);

        if env.ledger().timestamp() >= expiry && expiry != 0 {
            // Auto-unpause
            env.storage().instance().set(&DataKey::Paused, &false);
            return;
        }
        panic!("Contract is paused");
    }
}

pub fn pause(env: Env, admin: Address, reason: Symbol, duration_secs: u64) {
    admin.require_auth();
    require_admin(&env, &admin); // helper to check admin == stored admin

    let expiry = env.ledger().timestamp() + duration_secs;
    env.storage().instance().set(&DataKey::Paused, &true);
    env.storage().instance().set(&DataKey::PauseExpiry, &expiry);

    pause_event(&env, reason, admin);
}

pub fn unpause(env: Env, admin: Address) {
    admin.require_auth();
    require_admin(&env, &admin);

    env.storage().instance().set(&DataKey::Paused, &false);
    env.storage().instance().set(&DataKey::PauseExpiry, &0);

    unpause_event(&env, admin);
}
```

> **Note:** This is a sketch only. A real implementation would need thorough testing, event definitions, and integration with the Soroban event system.

---

## Open Questions

1. **Should withdrawals be pauseable?** The strongest argument against pausing withdrawals is that it traps user funds and breaks trust. A "withdraw-only" emergency mode is preferred by many DeFi protocols. Should we implement this from day one?

2. **Who are the guardians?** For a mobile wallet targeting end-users, is a multi-sig of PocketPay team members sufficient? Should there be an external security council?

3. **What is the maximum pause duration?** 7 days? 30 days? Should this be configurable at initialization or hard-coded?

4. **Should pause be per-function or global?** Granular pauses (deposit-only, lock-only) give more flexibility but add complexity and testing surface.

5. **How does pause interact with locked funds?** If funds are locked with an unlock time, and a pause extends beyond that unlock time, should the funds auto-unlock or remain frozen? Current thinking: locked funds should still become available at their unlock time regardless of pause, since the lock is a user-initiated commitment.

6. **Should there be a pause fee or bond?** To prevent frivolous pauses, should guardians stake tokens that are slashed if a pause is deemed unnecessary? This adds significant complexity.

7. **Event indexing:** Soroban events are not as mature as Ethereum events. Will pause/unpause events be reliably indexed by the PocketPay backend and third-party explorers?

8. **Testnet vs. mainnet posture:** Should the pause mechanism be present on testnet (for integration testing) even if it is not yet activated on mainnet?

---

## Recommendation

**For the current testnet phase:** No pause mechanism is needed. The contract is simple, the value at risk is zero, and the complexity cost is not justified.

**Before mainnet:** Implement a **time-bounded, withdraw-only pause** with the following characteristics:

- Admin (multi-sig) can trigger a pause with a mandatory reason string.
- Pause automatically expires after a configurable duration (suggested default: 7 days).
- During a pause, deposits and locks are blocked, but **withdrawals remain open**.
- If a pause exceeds the maximum duration without renewal, the contract auto-unpauses.
- Pause/unpause events are emitted for off-chain monitoring.
- A separate `emergency_shutdown` (irreversible, full pause) can be considered as a nuclear option for an extreme scenario, requiring a higher threshold (e.g., all guardians).

This balances safety with user trust: users can always exit, but new funds cannot enter a potentially compromised contract.

---

## References

- [Aave V3 — Pause & Emergency Mechanisms](https://docs.aave.com/developers/core/emergency)
- [Compound — Pause Guardian](https://docs.compound.finance/v2/governance/#pause-guardian)
- [OpenZeppelin — Pausable.sol](https://docs.openzeppelin.com/contracts/4.x/api/security#Pausable)
- [Soroban Events Documentation](https://soroban.stellar.org/docs/learn/events)
- [Soroban Auth & Multi-sig](https://soroban.stellar.org/docs/learn/auth)
