<<<<<<< HEAD
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
=======
﻿# Emergency Pause and Admin Misuse Threat Model

> **Status:** Documents the currently implemented Savings Vault behavior
>
> **Scope:** `contracts/savings_vault`
>
> **Network posture:** This contract is intended for development, educational,
> and Stellar testnet use. It is not production-ready or mainnet-ready.

## Purpose

This threat model documents the security and trust implications of the Savings
Vault administrator role and emergency pause mechanism.

The pause mechanism can reduce exposure during an incident by preventing new
deposits and locks. It also introduces centralized authority that can be
misused, compromised, lost, or exercised incorrectly.

This document focuses on:

- malicious administrator behavior;
- compromise of the administrator key;
- accidental or unnecessarily long pauses;
- withdrawal availability during a pause;
- administrator transfer risks;
- recovery assumptions;
- existing mitigations;
- limitations and residual risks.

This document describes current behavior. It does not propose or implement
contract logic changes.

## Related documentation

This threat model complements, rather than replaces:

- [Admin Role](admin-role.md)
- [Pause / Emergency Stop Design](pause-design.md)
- [Authorization Boundaries](authorization-boundaries.md)
- [Vault Custody Assumptions](vault-custody-assumptions.md)
- [Failure Mode Catalogue](failure-mode-catalogue.md)
- [Security Review Checklist](security-checklist.md)
- [Upgrade Strategy](upgrade-strategy.md)

## System summary

The Savings Vault records one administrator address in contract instance
storage.

The administrator can currently:

| Administrative action | Current behavior |
| --- | --- |
| `pause(admin, duration_secs)` | Blocks `deposit` and `lock_funds` until the pause expires or the administrator calls `unpause` |
| `unpause(admin)` | Clears an active pause before its expiry |
| `transfer_admin(admin, new_admin)` | Immediately assigns the administrator role to a new address |
| `get_admin()` | Publicly returns the stored administrator address |

Each privileged state-changing call requires authorization from the stored
administrator address.

The administrator cannot directly:

- withdraw tokens belonging to a user;
- modify user balances;
- change a lock amount or unlock timestamp;
- withdraw an immature lock;
- bypass a user's `require_auth()` requirement;
- upgrade the contract code;
- migrate or sweep user funds through an emergency recovery function.

## Pause behavior

The implemented pause is a global, withdraw-only emergency mode.

| Function or operation | During an active pause |
| --- | --- |
| `deposit` | Blocked |
| `lock_funds` | Blocked |
| `withdraw` | Not blocked by the pause mechanism |
| `withdraw_lock` | Not blocked by the pause mechanism |
| Lock maturation | Continues according to ledger time |
| Read-only functions | Remain available |
| Admin transfer | Remains available |
| `unpause` | Remains available to the administrator |

A pause has an expiry timestamp. Once ledger time reaches that timestamp,
`is_paused()` reports `false`. The stored pause flag and expiry are cleared
lazily the next time a function protected by `require_not_paused()` executes.

There is no hard-coded maximum value for `duration_secs`. The administrator can
also call `pause` again while a pause is active, replacing the current expiry
with a new expiry.

Therefore, each individual pause is time-bounded, but a malicious or
compromised administrator can repeatedly extend the effective pause period.

## Assets to protect

The relevant assets are:

1. **User tokens held by the vault**
   - Tokens transferred into the contract through `deposit`.

2. **Internal accounting state**
   - Available balances.
   - Lock entries.
   - Lock identifiers.
   - Unlock timestamps.

3. **Withdrawal availability**
   - The ability of users to withdraw available balances and matured locks.

4. **Administrative control**
   - The administrator address and its signing authority.

5. **Operational integrity**
   - Correct decisions about when to pause, extend a pause, or resume normal
     operation.

6. **Monitoring information**
   - Pause, unpause, and administrator-transfer events used by off-chain
     operators and users.

## Security objectives

The administrative and pause design should preserve the following objectives:

- A non-administrator must not invoke privileged actions.
- The administrator must not be able to seize or rewrite user balances.
- A pause must not block the withdrawal functions.
- An active incident should not accept new deposits or create new locks.
- Users and operators should be able to observe administrative changes.
- A temporary pause should not silently become permanent without continued
  administrator action.
- Loss or compromise of the administrator key should not be mistaken for a
  recoverable condition when no recovery mechanism exists.
- Documentation must not imply that withdrawal success is unconditional.

## Actors

### Honest administrator

An authorized operator who uses pause and admin-transfer capabilities according
to an incident-response process.

### Malicious administrator

An administrator who intentionally disrupts availability, transfers authority
to a hostile address, conceals an incident, or resumes operation while a known
risk remains.

### Compromised administrator

An attacker who obtains control of the administrator address or enough signing
authority to authorize administrative calls.

### Accidental operator

An authorized administrator who enters an incorrect duration, pauses the wrong
deployment, unpauses too early, or transfers authority to an incorrect address.

### User

A depositor whose tokens and lock records are managed by the vault. Users
authorize their own deposits, withdrawals, and lock operations.

### External token contract

The configured Stellar Asset Contract whose transfer behavior and availability
are required for deposits and withdrawals.

### Off-chain operator or indexer

A service that observes contract events, communicates incident status, and
helps users identify the current administrator and pause state.

## Trust boundaries and assumptions

### Administrator-key assumption

The contract assumes the stored administrator address is controlled securely.
The contract does not enforce multisignature approval, hardware-backed custody,
role separation, or an operational approval policy.

A Stellar multisignature account or a separate governance contract may be used
as the administrator address, but that is an external deployment decision and
is not enforced by the Savings Vault.

### Soroban authorization assumption

The contract relies on Soroban `Address::require_auth()` to validate user and
administrator authorization.

### Token-contract assumption

Withdrawals depend on the configured token contract accepting transfers from
the vault to the user. The pause mechanism cannot restore a token contract that
is unavailable, frozen, blacklisted, insolvent, incompatible, or otherwise
unable to complete a transfer.

### Ledger-time assumption

Pause expiry and lock maturity rely on the ledger timestamp.

### Monitoring assumption

Events are emitted on-chain, but the contract does not guarantee that an
off-chain indexer, wallet, status page, or alerting service will process them
correctly or promptly.

### Recovery assumption

The contract has no code-upgrade entrypoint and no emergency asset-migration or
admin-recovery function. Pausing creates investigation time; it does not patch
the deployed code or move users to a replacement contract.

## Threat summary

| ID | Threat | Primary impact | Existing control | Residual risk |
| --- | --- | --- | --- | --- |
| TM-ADMIN-01 | Malicious administrator repeatedly extends pauses | Denial of deposits and new locks | Withdrawals remain outside the pause guard; pauses have expiries | Effective pause can be prolonged indefinitely through repeated calls |
| TM-ADMIN-02 | Administrator key is compromised | Hostile pause, unpause, or admin transfer | Privileged calls require stored-admin authorization | An attacker controlling the key satisfies that authorization |
| TM-ADMIN-03 | Pause is triggered accidentally or with an excessive duration | Operational disruption | Pause expires according to ledger time; admin can unpause early | No maximum duration or secondary approval is enforced |
| TM-ADMIN-04 | Contract is unpaused before an incident is resolved | New deposits and locks are exposed to unresolved risk | Only the administrator can unpause | No timelock, review requirement, or on-chain reason is enforced |
| TM-ADMIN-05 | Admin role is transferred to a wrong or inaccessible address | Loss of administrative control | Current admin must authorize transfer; event is emitted | Transfer is immediate and has no acceptance step or rollback |
| TM-WITHDRAW-01 | Users interpret “withdrawals remain open” as guaranteed withdrawal success | Users may be unable to exit during an external or accounting failure | Pause does not call `require_not_paused()` in withdrawal functions | Token, solvency, authorization, maturity, storage, or network failures can still block withdrawal |
| TM-RECOVERY-01 | A vulnerability requires code replacement or asset migration | Incident cannot be repaired in place | Pause can stop new deposits and locks | No upgrade or emergency migration mechanism exists |
| TM-MONITOR-01 | Pause or admin-transfer events are not observed | Users and operators act on stale information | Events are emitted on-chain | Delivery and interpretation by off-chain consumers are best-effort |

## Detailed threat scenarios

### TM-ADMIN-01: Malicious administrator prolongs the pause

#### Scenario

A malicious administrator repeatedly calls `pause` before the current pause
expires. Each call replaces the expiry with a later value.

#### Impact

- New deposits remain blocked.
- Users cannot create new locks.
- Integrations expecting normal operation may fail.
- The protocol may suffer reputational or availability damage.
- The administrator can create an extended denial-of-service condition without
  directly taking user tokens.

#### What the administrator still cannot do

The pause does not grant authority to:

- withdraw user tokens;
- modify balances;
- rewrite lock entries;
- prevent existing locks from maturing;
- selectively pause one user while allowing another.

The pause is global, so it is not a per-user censorship mechanism.

#### Existing mitigations

- `withdraw` and `withdraw_lock` do not use the pause guard.
- Each pause has an expiry timestamp.
- Pause calls emit events.
- The pause state can be queried publicly.

#### Limitations

There is no maximum pause duration and no limit on how many times an
administrator may refresh the expiry. Time-bounded storage alone does not
prevent a continuously authorized administrator from maintaining the pause.

### TM-ADMIN-02: Administrator key compromise

#### Scenario

An attacker gains control of the administrator key or of the signing policy
behind the administrator address.

#### Attacker capabilities

The attacker can:

- pause deposits and new locks;
- unpause during an unresolved incident;
- repeatedly refresh the pause expiry;
- transfer the administrator role to another attacker-controlled address.

#### Attacker limitations

The attacker cannot use the administrator role alone to:

- withdraw another user's funds;
- authorize as another user;
- alter balances or lock timestamps;
- withdraw immature locks;
- invoke a non-existent upgrade or emergency sweep function.

#### Impact

The primary risks are administrative takeover, prolonged operational
disruption, unsafe resumption of activity, and permanent loss of the original
administrator's authority after `transfer_admin`.

#### Existing mitigations

- Privileged actions require authorization.
- Admin changes and pause transitions emit events.
- Admin transfer does not mutate accounting or user locks.
- User withdrawals remain outside the pause guard.

#### Residual risk

Authorization proves control of the configured administrator address; it does
not distinguish the legitimate operator from an attacker who has compromised
that address.

### TM-ADMIN-03: Accidental pause

#### Scenario

An honest administrator:

- pauses the wrong contract deployment;
- enters an unexpectedly long duration;
- pauses in response to a false alarm;
- refreshes the expiry unintentionally.

#### Impact

Deposits and new locks fail until the administrator calls `unpause` or the
effective pause expires.

#### Existing mitigations

- A zero-duration pause is rejected.
- The administrator may unpause early.
- Pause state and expiry are observable through contract state and events.
- Withdrawals remain outside the pause guard.

#### Limitations

The contract does not enforce:

- a maximum pause duration;
- a confirmation delay;
- a second administrator approval;
- a required reason code;
- a distinction between test, staging, and other deployed instances.

Operational procedures must provide those safeguards outside the contract.

### TM-ADMIN-04: Premature or malicious unpause

#### Scenario

The administrator calls `unpause` before the underlying incident has been
understood or resolved.

#### Impact

The contract begins accepting new deposits and lock operations while the
original vulnerability, token problem, or operational fault may still exist.

#### Existing mitigations

- Only the stored administrator may unpause.
- An unpause event is emitted.
- Users can independently query pause state.

#### Limitations

The contract does not require:

- evidence that a fix was deployed;
- independent review;
- a cooldown period;
- multisignature approval;
- a post-incident verification transaction.

Authorization alone does not establish that unpausing is safe.

### TM-ADMIN-05: Unsafe administrator transfer

#### Scenario

The current administrator calls `transfer_admin` with:

- an incorrectly entered address;
- an address whose key is unavailable;
- an attacker-controlled address;
- an address whose signing policy is misconfigured.

#### Impact

The old administrator immediately loses authority. If the new address cannot
sign, the contract may permanently lose the ability to:

- pause;
- unpause early;
- transfer administration again.

An existing pause can still become ineffective at its expiry, but no authorized
operator may remain to manage future incidents.

#### Existing mitigations

- The current administrator must authorize the transfer.
- The transfer emits an event.
- Accounting and user locks are not modified by the transfer.

#### Limitations

Admin transfer is a one-step operation. The new administrator does not need to
accept the role before it becomes active, and there is no rollback or recovery
authority.

### TM-WITHDRAW-01: Withdrawal availability is misunderstood

The implemented pause does not block `withdraw` or `withdraw_lock`. This is an
important protection, but it is not an unconditional guarantee that every user
can successfully withdraw at any time.

A withdrawal may still fail when:

- the user does not authorize the transaction;
- the requested amount exceeds the available balance;
- a selected lock has not matured;
- a lock identifier is invalid;
- the configured token contract rejects or cannot complete the transfer;
- the vault lacks sufficient token backing;
- relevant storage entries have expired or become unavailable;
- the network or RPC path is unavailable;
- a separate contract defect causes the call to fail.

Therefore, the accurate guarantee is:

> The emergency pause mechanism itself does not disable the withdrawal
> entrypoints.

It should not be described as a guarantee that all withdrawals will always
succeed under every failure condition.

## Withdrawal impact analysis

### Available, unlocked balances

Users may call `withdraw` during a pause. The call still requires user
authorization, sufficient available balance, compatible storage state, and a
successful token transfer.

### Immature locked balances

A pause does not accelerate lock maturity. Funds whose unlock timestamp has not
been reached remain unavailable according to the original lock rules.

### Matured locks

Matured locks remain withdrawable through `withdraw` or `withdraw_lock`, subject
to the normal authorization, accounting, token-transfer, and network
requirements.

### New locks

Users cannot create new locks while the pause is active.

### New deposits

Users cannot deposit additional tokens while the pause is active. This reduces
the number of users and assets exposed to a suspected incident.

### Administrator access to withdrawals

The administrator does not gain authority to withdraw on behalf of a user.
User authorization remains required.

## Recovery process assumptions

A pause is a containment tool, not a complete recovery mechanism.

A realistic incident response depends on the following off-chain steps:

1. Detect and verify the incident.
2. Identify the correct contract deployment.
3. Pause deposits and new locks when containment is appropriate.
4. Publish the affected contract address, pause expiry, and known impact.
5. Continue monitoring withdrawal behavior and token solvency.
6. Diagnose the root cause.
7. Determine whether the deployed contract can safely resume.
8. If code replacement is necessary, deploy a new contract and define a
   separately reviewed migration or user-exit process.
9. Obtain independent review before unpausing.
10. Publish a post-incident explanation and any remaining limitations.

The current contract does not enforce these steps.

## Recovery limitations

### No in-place code upgrade

There is no public contract-code upgrade entrypoint. A vulnerability in the
deployed logic cannot be patched merely by pausing and unpausing.

### No emergency fund migration

The administrator cannot sweep or migrate user assets to a replacement vault.
This limits admin abuse but also limits emergency recovery options.

### No lost-admin recovery

If the administrator key is lost or the role is transferred to an inaccessible
address, no secondary guardian or recovery address can restore control.

### No forced user recovery

The administrator cannot withdraw for users, bypass user authorization, or
override lock maturity.

### Token-level failures remain external

The vault cannot repair or override a configured token contract that is frozen,
blacklisted, incompatible, or otherwise unable to transfer.

## Existing mitigations

The current implementation includes the following safeguards:

- Privileged calls require authorization from the stored administrator.
- Pause applies globally rather than targeting individual users.
- Deposits and new locks are blocked during an active pause.
- Withdrawal functions are not protected by the pause guard.
- Locks continue to mature according to ledger time.
- Pauses include an expiry.
- Pause, unpause, and admin-transfer events are emitted.
- Admin transfer does not modify balances, token custody, lock records, or lock
  maturity.
- The administrator cannot directly seize user funds.
- User operations continue to require user authorization.

These controls reduce the impact of administrative abuse, but they do not
remove the single-administrator trust assumption.

## Recommended mitigations

The following are recommendations for future designs and deployment policy.
They are not implemented guarantees.

### Before any future mainnet consideration

- Use a multisignature or separately governed administrator address.
- Define a maximum pause duration in contract logic.
- Require a two-step administrator transfer:
  1. current administrator proposes the new address;
  2. new address accepts the role.
- Separate emergency pause authority from broader administrative authority.
- Require independent review before unpausing after a security incident.
- Operate event monitoring for pause, unpause, and admin-transfer activity.
- Publish an incident-response runbook with named responsibilities and
  escalation paths.
- Define a tested replacement-contract and user-migration strategy.
- Review storage TTL handling and token-solvency monitoring.
- Obtain an independent security audit.

### Operational controls for testnet

- Use a dedicated administrator identity rather than a personal everyday key.
- Verify the contract ID and network before signing a privileged call.
- Record the intended duration before calling `pause`.
- Confirm the computed expiry after the transaction.
- Monitor the pause and admin-transfer events.
- Announce the pause through a verifiable project channel.
- Require a second person to review the incident before unpausing.
- Test administrator transfer using non-sensitive testnet accounts before
  relying on the process.

## Limitations and residual risks

Even with the current controls:

- one administrator address remains a central point of control and failure;
- repeated pause calls can maintain operational disruption;
- an attacker with the administrator key can permanently transfer authority;
- an administrator can unpause while a vulnerability remains;
- an incorrect admin transfer may be irreversible;
- pause does not provide upgrade, migration, or token recovery;
- withdrawals depend on more than the pause state;
- event monitoring is not guaranteed;
- users must trust operational communication during an incident;
- the contract remains unsuitable for claims of production or mainnet
  readiness.

## Security checklist mapping

| Checklist area | Threat-model conclusion |
| --- | --- |
| Admin-only actions are gated | Implemented through administrator authorization and stored-address checks |
| Admin misuse is documented | Covered by malicious, compromised, accidental, transfer, and recovery scenarios |
| Withdrawal impact is documented | Pause does not block withdrawal entrypoints, but other failure conditions remain |
| Recovery limitations are documented | No upgrade, emergency migration, lost-admin recovery, or forced user recovery |
| Mitigations are documented | Existing controls and future recommendations are separated explicitly |
| Trust assumptions are documented | Single-admin, Soroban auth, token behavior, ledger time, monitoring, and recovery assumptions are explicit |

## Review triggers

Update this document whenever a change affects:

- administrator permissions;
- pause duration or coverage;
- administrator transfer;
- withdrawal behavior during pause;
- multisignature or governance integration;
- contract upgrades;
- emergency migration or token recovery;
- storage TTL behavior;
- event schemas;
- token custody assumptions.
>>>>>>> upstream/main
