# Admin & Emergency Mechanism Threat Model

This document analyses the security risks associated with admin-controlled and emergency mechanisms in the Savings Vault contract. It covers malicious admin, compromised admin, accidental misuse, and blocked-withdrawal scenarios, along with mitigations and trust assumptions.

**Honesty note:** The contract currently centralises administrative control in a single `admin` address. This document does not attempt to downplay that centralisation risk. It exists to help contributors, reviewers, and users understand exactly what the admin can do, what they cannot do, and what residual risks remain.

---

## Table of Contents

1. [Current Admin Surface](#1-current-admin-surface)
2. [Threat Scenarios](#2-threat-scenarios)
   - [2.1 Malicious Admin](#21-malicious-admin)
   - [2.2 Compromised Admin Key](#22-compromised-admin-key)
   - [2.3 Accidental Pause or Misconfiguration](#23-accidental-pause-or-misconfiguration)
   - [2.4 Blocked Withdrawals](#24-blocked-withdrawals)
   - [2.5 Admin Abandonment / Lost Key](#25-admin-abandonment--lost-key)
3. [Withdrawal Impact Analysis](#3-withdrawal-impact-analysis)
4. [Mitigations](#4-mitigations)
5. [Trust Assumptions](#5-trust-assumptions)
6. [Acceptance Checklist](#6-acceptance-checklist)

---

## 1. Current Admin Surface

### What the admin address stores

The `initialize(admin, token)` function records the admin `Address` in instance storage under `DataKey::Admin`. The admin must sign the initialization transaction (`admin.require_auth()`). Once set, the admin address is immutable — there is no `transfer_admin`, `set_admin`, or admin rotation function in the current contract.

### What the admin CAN do today

| Capability | Status | Notes |
|---|---|---|
| Recorded in storage | ✅ | Admin address is persisted during `initialize` |
| Sign `initialize` | ✅ | Required to authorise contract setup |
| Pause deposits/locks | ❌ | No `pause()` function exists |
| Unpause the contract | ❌ | No `unpause()` function exists |
| Freeze configuration | ❌ | No `admin_frozen` or `token_frozen` flags exist |
| Transfer admin role | ❌ | No `transfer_admin()` function exists |
| Sweep or recover user funds | ❌ | Admin has no access to user balances |
| Change user balances or locks | ❌ | All state-changing functions require `user.require_auth()` |
| Upgrade the contract | ❌ | No proxy or `upgrade()` mechanism exists |

### What the admin CANNOT do today

- **Cannot pause contract execution** or halt deposits/withdrawals.
- **Cannot migrate or sweep funds** from any user's vault.
- **Cannot recover or forcibly withdraw** user funds.
- **Cannot change user balances** or unlock times except via the existing user-authorised functions.
- **Cannot upgrade the contract** — no upgrade entry point or proxy pattern is present.
- **Cannot freeze the token address** — no `token_frozen` latch exists.

> **Key takeaway:** In the current contract, the admin is **inert**. Storing the admin address grants zero operational powers. The admin value is informational and preparatory only. This is by design — least privilege until admin functions are explicitly added and audited.

### Future admin capabilities (documented but not yet implemented)

The upstream feature set (visible in the project README and architecture docs) defines several admin-controlled mechanisms planned for future versions:

| Planned capability | Risk level | Description |
|---|---|---|
| `pause(admin, duration_secs)` | Medium | Admin can block new deposits and locks for a specified duration; withdrawals remain open |
| `unpause(admin)` | Low | Admin can deactivate an active pause before its duration expires |
| `transfer_admin(new_admin)` | High | Transfer the admin role to a new address |
| `token_frozen` / `admin_frozen` latches | Medium | One-way freeze flags that permanently lock the token address or admin configuration |

The threat scenarios below cover both the **current inert state** and the **future state** where these capabilities are active.

---

## 2. Threat Scenarios

### 2.1 Malicious Admin

**Scenario:** The admin address is controlled by an actor who intentionally abuses their powers.

**Current impact (inert admin):** None. The admin has no operational powers — they cannot pause, freeze, sweep funds, or block withdrawals. A malicious admin today can do nothing beyond what any other address can do.

**Future impact (with pause):** If `pause()` is implemented:

| Action | Impact on users | Severity |
|---|---|---|
| Call `pause(admin, very_long_duration)` | New deposits and locks are blocked indefinitely; existing locked funds remain locked beyond their maturity | **High** — users cannot deposit or create new locks, but can still withdraw available balances |
| Call `pause()` repeatedly | Extends the denial-of-service window | **High** — prolonged unavailability |
| Call `pause()` just before a large unlock event | Prevents users from creating new locks at favourable times | **Medium** — timing attack on user lock strategies |

**What the malicious admin still cannot do (even with pause):**

- Cannot withdraw user funds (`withdraw` requires `user.require_auth()`)
- Cannot change user balances or lock records
- Cannot steal tokens held by the contract (SAC transfer requires contract invocation)
- Cannot `unpause` if the pause duration has not elapsed (depends on implementation)

**Residual risk:** An admin who can pause indefinitely creates a **liveness failure** — the contract remains safe (funds are not stolen) but becomes unusable. Users must wait for the pause duration to expire or for the admin to unpause.

---

### 2.2 Compromised Admin Key

**Scenario:** The admin's private key is stolen through phishing, malware, insider threat, or operational security failure.

**Current impact (inert admin):** Low. The attacker gains no operational control over the vault. The only risk is reputational — users may lose confidence if the admin key is known to be compromised, even though the key currently grants no powers.

**Future impact (with pause and transfer_admin):**

| Action | Impact | Severity |
|---|---|---|
| Call `transfer_admin(attacker_address)` | Attacker permanently takes over the admin role | **Critical** — irreversible unless a multisig or governance mechanism exists |
| Call `pause(admin, max_duration)` | DOS on deposits and locks | **High** |
| Call `pause()` then refuse to unpause | Permanent liveness failure | **Critical** — contract becomes a "zombie" where withdrawals still work but no new activity is possible |

**Key risk amplifier:** Without `admin_frozen`, the compromised admin can also change the token address (if `set_token` exists) to a malicious token contract, enabling token-draining attacks during deposit.

**Residual risk:** A single-key admin model means **one compromised key = full admin takeover**. There is no secondary approval, no multisig, and no timelock on admin actions.

---

### 2.3 Accidental Pause or Misconfiguration

**Scenario:** A legitimate admin accidentally pauses the contract, sets an excessively long pause duration, or misconfigures a freeze latch.

**Current impact (inert admin):** None. No pause mechanism exists to accidentally trigger.

**Future impact (with pause and freeze latches):**

| Action | Impact | Severity |
|---|---|---|
| Accidental `pause(admin, 31536000)` (1 year) | Deposits and locks blocked for a year | **High** — requires admin to unpause; if admin is unavailable, users wait |
| Accidental `token_frozen = true` | Token address permanently locked; cannot fix a wrong token address | **Critical** — irreversible one-way latch |
| Accidental `admin_frozen = true` | Admin configuration permanently locked; cannot rotate admin if key is later compromised | **High** — irreversible |

**Mitigation:** The `unpause()` function provides a recovery path for accidental pauses. Freeze latches are one-way by design (security feature), so they should require explicit confirmation (e.g., a separate `freeze_token()` function with clear naming, not a generic `set_config`).

---

### 2.4 Blocked Withdrawals

**Scenario:** Users cannot withdraw their funds due to admin action or inaction.

**Critical design property: `withdraw` does NOT require admin authorisation.**

The `withdraw(user, amount)` function calls `user.require_auth()` — only the fund owner can withdraw. The admin has **no gatekeeping role** over withdrawals. This is the single most important safety property of the contract.

**What CAN block withdrawals:**

| Cause | Admin-related? | Mitigation |
|---|---|---|
| Token transfer failure (contract holds insufficient SAC tokens) | Indirectly — if admin misconfigured the token address | Verify token address at initialization; freeze latch after verification |
| Contract not initialized | Yes — admin must call `initialize` | One-time setup; document as deployment prerequisite |
| Soroban network outage | No | Off-chain; outside contract scope |
| Ledger TTL expiry on balance entries | No (Soroban lifecycle) | Monitor and extend storage TTL |

**What CANNOT block withdrawals:**

- Admin calling `pause()` — `withdraw` is explicitly excluded from pause restrictions (by design in the upstream spec)
- Admin refusing to sign — admin signature is not required for withdrawals
- Admin abandoning the contract — withdrawals are user-authenticated

---

### 2.5 Admin Abandonment / Lost Key

**Scenario:** The admin loses their private key, the key holder becomes unavailable, or the administering organisation ceases operations.

**Current impact (inert admin):** None. The contract continues to function normally. Users can deposit, withdraw, and lock funds without any admin involvement.

**Future impact (with pause):**

| Concern | Impact | Severity |
|---|---|---|
| Contract is paused and admin cannot unpause | Permanent liveness failure for deposits/locks | **High** if pause has no expiry |
| Admin frozen latch is set, admin key lost | Cannot change admin; stuck with inert admin | **Low** — is actually a security benefit |
| Token address needs changing (e.g., token migration) | Cannot update because admin is lost and `admin_frozen` is false | **Medium** — contract locked to old token |

**Mitigation:** Pause should always have a maximum duration (enforced in the contract, not just as a parameter). After the duration expires, the contract auto-unpauses, ensuring liveness even if the admin disappears.

---

## 3. Withdrawal Impact Analysis

This section answers the question: *"If the admin goes rogue or is compromised, can users still get their money out?"*

### Current contract (inert admin)

| Scenario | Can users withdraw? | Notes |
|---|---|---|
| Admin is malicious | ✅ Yes | Admin has no powers; `withdraw` is user-authenticated |
| Admin key is compromised | ✅ Yes | Same as above |
| Admin disappears | ✅ Yes | Contract is fully self-service |
| Admin never calls `initialize` | ❌ No | Contract is unusable; no user can deposit or query balances |

### Future contract (with pause)

| Scenario | Can users withdraw? | Notes |
|---|---|---|
| Admin pauses the contract | ✅ Yes | `withdraw` is excluded from pause by design |
| Admin pauses + refuses to unpause | ✅ Yes | Withdrawals remain open; only deposits/locks are blocked |
| Admin changes token to malicious address | ❌ **No** | `withdraw` calls `token_client.transfer` using the stored token — if changed to a malicious token, transfers may fail or drain to attacker |
| Admin freezes token address (latch) | ✅ Yes | If frozen to a legitimate token, withdrawals work normally |
| Admin is compromised AND `transfer_admin` exists | ⚠️ Conditional | If attacker transfers admin to themselves and then changes token address, withdrawals are at risk |

> **Critical insight:** The `token` address stored in `DataKey::Token` is the single most powerful configuration value. If an admin can change it to a malicious token contract, they can intercept or redirect withdrawals. The `token_frozen` one-way latch exists specifically to mitigate this risk — once frozen, not even the admin can change the token address.

---

## 4. Mitigations

### Implemented mitigations (current contract)

| Mitigation | How it helps |
|---|---|
| Admin is inert by default | Zero attack surface until admin functions are explicitly added |
| `user.require_auth()` on all state-changing functions | Admin cannot act on behalf of users |
| `initialize()` is one-shot | Admin cannot re-initialize to override state |
| `withdraw` does not check admin | Withdrawals cannot be blocked by admin action or inaction |

### Recommended mitigations (before adding admin powers)

| Mitigation | Counters | Priority |
|---|---|---|
| **Multisig admin** | Compromised key (2.2), malicious admin (2.1) | **Critical** — before any admin function goes live |
| **Timelocked admin actions** | Malicious admin (2.1), accidental pause (2.3) | **High** — users get a warning period before changes take effect |
| **Maximum pause duration (contract-enforced)** | Indefinite pause (2.1, 2.5) | **High** — prevents permanent liveness failure |
| **`token_frozen` one-way latch** | Token address manipulation (2.2, 3) | **High** — freeze the token address after verifying it is correct |
| **`admin_frozen` one-way latch** | Admin rotation by attacker (2.2) | **Medium** — permanently locks admin configuration |
| **Pause event emission** | Auditability, detection of malicious pause | **Medium** — enables off-chain monitoring |
| **Admin action events** | Forensic analysis after compromise | **Medium** — log all admin state changes |
| **On-chain governance (DAO)** | Single-point-of-failure admin (2.1, 2.2, 2.5) | **Future** — decentralize admin powers to token-holders |
| **Emergency withdrawal escape hatch** | Contract becomes permanently unusable | **Future** — allow users to exit with their funds after a long timeout |

---

## 5. Trust Assumptions

This contract makes the following trust assumptions about the admin. These are explicit — users and integrators should understand them before depositing funds.

### Current trust assumptions

| Assumption | Honest assessment |
|---|---|
| The admin will call `initialize` correctly | ✅ Required once at deployment; verifiable on-chain |
| The admin will set a legitimate token address | ✅ Verifiable; the token address is visible in instance storage |
| The admin will not abuse future powers | ⚠️ Not yet relevant — admin has no powers |
| The admin's key is secure | ⚠️ Low risk today — compromising the key grants no powers |

### Future trust assumptions (when admin functions are added)

| Assumption | Risk if violated |
|---|---|
| The admin will not pause the contract maliciously | Deposits and locks blocked; withdrawals remain open |
| The admin will not pause indefinitely | Permanent liveness failure for deposits/locks |
| The admin will unpause after legitimate emergencies | Users regain full functionality |
| The admin's key remains secure | Attacker can pause, transfer admin, or change token (if not frozen) |
| The admin will freeze the token address after verifying it | Without freezing, a compromised admin can redirect withdrawals |

### What users do NOT need to trust the admin for

- **Withdrawing their own funds** — always user-authenticated, never admin-gated
- **Checking their balance** — read-only, no auth required
- **Locking their own funds** — user-authenticated
- **Contract continuing to operate** — admin is not required for day-to-day operations

### Centralisation acknowledgment

> **This contract uses a single-admin model.** The admin address recorded at initialization is a single point of trust for future administrative actions. In the current code, this risk is theoretical (the admin is inert). Before any admin powers are activated, the project should implement multisig, timelocks, or on-chain governance to distribute trust. Relying on a single key for administrative control is **not recommended for production deployments** holding significant value.

---

## 6. Acceptance Checklist

- [x] Admin threat model document exists
- [x] Malicious admin scenario is covered (Section 2.1)
- [x] Compromised admin scenario is covered (Section 2.2)
- [x] Accidental pause/misconfiguration is covered (Section 2.3)
- [x] Blocked withdrawal scenario is covered (Section 2.4)
- [x] Admin abandonment is covered (Section 2.5)
- [x] Withdrawal impact is explained for each scenario (Section 3)
- [x] Current vs. future admin capabilities are clearly distinguished (Section 1)
- [x] Mitigations are listed with priority levels (Section 4)
- [x] Trust assumptions are explicitly stated (Section 5)
- [x] Centralisation risk is honestly acknowledged (Section 5)
- [x] README links to this document (to be done)

---

## Related Documents

- [Admin Role](admin-role.md) — Current admin capabilities and design considerations
- [Failure Mode Catalogue](failure-mode-catalogue.md) — Expected errors and safe-failure behaviour
- [Architecture Documentation](architecture.md) — State model and storage design
- [Security Considerations](../README.md#security-considerations) — README security section

---

*Last updated: 2026-07-28*
