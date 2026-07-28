# Emergency Pause and Admin Misuse Threat Model

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
