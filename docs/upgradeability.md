# Contract Upgradeability — Savings Vault

> **Current Posture:** Non-Upgradeable (Immutable)
>
> **Status:** The Savings Vault contract does not implement any upgrade mechanism. Once deployed, the contract logic cannot be changed.

---

## Overview

The Savings Vault contract is currently **non-upgradeable**. This means:

- The contract WASM is deployed once and cannot be modified after deployment
- The `initialize(admin)` function records an admin address for reference only, but this admin has **no upgrade powers**
- Any bug fixes, feature additions, or protocol changes require deploying a new contract instance
- Users must manually migrate their funds from an old contract to a new contract if an upgrade is needed

This document explains the implications of this design choice, the trust model, storage migration considerations, and open questions for future decisions.

---

## Current Upgrade Posture

### Non-Upgradeable (Immutable)

The contract follows Strategy A (No Upgrade) as described in [upgrade-strategy.md](upgrade-strategy.md#a-no-upgrade-immutable).

**Characteristics:**
- Contract logic is fixed at deployment time
- No `upgrade()` function exists
- Admin address is stored for reference only and cannot modify contract behavior
- Changes require redeployment with a new contract ID

**Implications:**
- Users can trust that the contract code will never change
- Critical vulnerabilities cannot be patched without user migration
- New features require a new contract deployment
- The contract ID is permanent and unchanging

---

## Trust Model

### User Trust Assumptions

In a non-upgradeable contract, users trust:

1. **The initial deployment** - Users must trust that the deployed WASM is correct and has been audited
2. **The Soroban host** - Users trust the Stellar network and Soroban runtime to execute the contract as written
3. **The admin key (limited)** - Users trust the admin not to exploit any remaining powers (currently minimal)
4. **No future changes** - Users trust that the contract will remain immutable

### Advantages of Non-Upgradeable Design

- **Maximum code immutability** - The contract logic cannot be altered after deployment
- **No admin upgrade risk** - No single party can change contract behavior to steal funds
- **Simplified audit scope** - Auditors only need to review the initial deployment
- **Predictable behavior** - Users can verify the code once and trust it forever
- **Aligned with blockchain ethos** - "Code is law" principle

### Disadvantages of Non-Upgradeable Design

- **No emergency patching** - Critical bugs cannot be fixed without user migration
- **Feature stagnation** - New features require a new contract and user migration
- **Protocol incompatibility** - Soroban or Stellar protocol changes may break the contract
- **User migration burden** - Every upgrade requires all users to manually move funds
- **Stranded funds risk** - Users who don't migrate may lose access to funds

---

## Storage Migration Impact

### Current Storage Architecture

The contract uses two types of storage:

**Instance Storage:**
- Admin address
- Initialization flag
- Token address
- Contract-level configuration
- **TTL:** Tied to contract lifetime, must be extended periodically

**Persistent Storage:**
- User balances (`DataKey::Balance(user)`)
- Lock entries (`DataKey::Locks(user)`)
- Lock ID counters (`DataKey::NextLockId(user)`)
- **TTL:** Per-entry, must be extended periodically

### Migration Limitations

Since the contract is non-upgradeable, storage migration is **not automatic**. If a new contract is deployed:

1. **Balances do not transfer** - User balances in the old contract remain there
2. **Locks do not transfer** - Locked funds remain locked in the old contract
3. **Manual migration required** - Users must:
   - Call `withdraw()` on the old contract
   - Call `deposit()` on the new contract
   - Re-establish any locks with new unlock times

### Storage TTL Considerations

- **Instance storage TTL** must be extended on the old contract even after users migrate, or the contract becomes unusable
- **Persistent storage entries** for non-migrating users will eventually expire, potentially stranding funds
- **Migration window** must be communicated clearly to allow users to migrate before storage expires

### Locked Funds Migration Challenges

Locked funds present additional complexity:

- **Cannot withdraw** until the unlock time is reached
- **Migration requires waiting** for locks to mature or accepting new lock terms
- **Lock state preservation** is not automatic - users must re-establish locks in the new contract
- **Partial migration** is possible (migrate unlocked funds, leave locked funds until maturity)

---

## Known Open Questions

The following questions should be resolved before considering any change to the upgrade posture:

### Short-term Questions

1. **Should the contract remain non-upgradeable for mainnet?**
   - If yes: Document the migration process and communication strategy
   - If no: Choose an upgrade strategy (see [upgrade-strategy.md](upgrade-strategy.md))

2. **What is the migration communication plan?**
   - How will users be notified of a new contract deployment?
   - What channels will be used (website, social media, on-chain events)?
   - How much advance notice will be provided?

3. **What happens to locked funds during migration?**
   - Should users wait for locks to mature before migrating?
   - Should the new contract offer compensation for early lock termination?
   - Should there be a grace period for locked funds?

4. **How will storage TTL be managed during migration?**
   - Who is responsible for extending TTL on the old contract?
   - How long will the old contract remain operational?
   - What happens if storage expires before users migrate?

### Long-term Questions

5. **Should the admin role be expanded?**
   - Should the admin have upgrade powers in the future?
   - Should upgrade authority be delegated to a multisig or DAO?
   - Should there be a timelock on upgrades?

6. **What is the expected upgrade frequency?**
   - If upgrades are expected to be rare (yearly), non-upgradeable may be acceptable
   - If frequent (monthly), an upgrade mechanism may be necessary

7. **Should the contract eventually become immutable by design?**
   - Some protocols plan to "renounce" upgradeability after a stabilization period
   - Is this a goal for the Savings Vault?

8. **How will SDKs and frontends handle multiple contract versions?**
   - Will there be a registry contract to track the "current" version?
   - How will wallets detect which version a user is on?
   - Will there be a deprecation timeline for old versions?

---

## Current Limitations

Based on the non-upgradeable posture, the following limitations exist:

1. **No emergency response** - If a critical vulnerability is discovered, there is no way to patch it without user migration
2. **No feature iteration** - Adding new features (e.g., interest accrual, multi-asset support) requires a new contract
3. **No protocol adaptation** - Changes to Soroban or Stellar protocols may require migration
4. **No regulatory compliance** - Inability to adapt to new legal requirements (e.g., sanctions screening)
5. **No optimization** - Gas or storage optimizations cannot be applied to existing deployments

---

## Future Options

If the project decides to change the upgrade posture, the following options are available:

### Option 1: Remain Non-Upgradeable

- **Pros:** Maximum trustlessness, simple audit, aligned with blockchain ethos
- **Cons:** No emergency patching, user migration burden, feature stagnation
- **Best for:** Protocols that are simple, feature-complete, and unlikely to need changes

### Option 2: Implement Migration Contract

- **Pros:** Supported migration path, users opt-in, both versions auditable
- **Cons:** Users must actively migrate, locked funds complexity, two contract IDs
- **Best for:** Protocols that upgrade infrequently and want immutability guarantees
- **See:** [upgrade-strategy.md](upgrade-strategy.md#c-migration-contract)

### Option 3: Implement Proxy Pattern

- **Pros:** Seamless upgrades, preserves state, single contract ID
- **Cons:** Admin key risk, storage collision complexity, gas overhead
- **Best for:** Actively developed protocols with frequent upgrades
- **See:** [upgrade-strategy.md](upgrade-strategy.md#b-admin-controlled-upgrade-proxy-pattern)

### Option 4: Redeploy and Social Coordination

- **Pros:** Zero complexity, no new trust assumptions
- **Cons:** High user friction, fund stranding, no atomicity
- **Best for:** Early-stage projects with small user bases
- **See:** [upgrade-strategy.md](upgrade-strategy.md#d-redeploy-and-social-coordination)

---

## Recommendations

### For Current Stage (Testnet / Educational)

**Remain non-upgradeable** with the following considerations:

1. **Document the migration process** - Create clear guides for users on how to migrate between contract versions
2. **Establish communication channels** - Ensure users can be notified of new deployments
3. **Monitor storage TTL** - Plan for TTL extension on old contracts during migration windows
4. **Test migration flows** - Practice migration on testnet before any mainnet deployment

### Before Mainnet

**Re-evaluate the upgrade posture** based on:

1. **User feedback** - Are users comfortable with manual migration?
2. **TVL expectations** - Higher TVL may justify more sophisticated upgrade mechanisms
3. **Feature roadmap** - Are frequent feature additions expected?
4. **Regulatory environment** - Are there compliance requirements that may require changes?

**If remaining non-upgradeable:**
- Invest in excellent documentation and user education
- Establish a clear communication strategy for migrations
- Consider implementing a "registry" contract to track the current version
- Plan for long-term storage TTL management

**If implementing an upgrade mechanism:**
- Choose a strategy based on the comparison in [upgrade-strategy.md](upgrade-strategy.md#comparison-matrix)
- Implement security mitigations (multisig, timelock, events)
- Audit the upgrade mechanism thoroughly
- Communicate the change clearly to users

---

## Related Documentation

- [Upgrade Strategy Research](upgrade-strategy.md) - Detailed comparison of upgrade approaches
- [Admin Role Documentation](admin-role.md) - Details on the current admin role and powers
- [Pause / Emergency Stop Design](pause-design.md) - Research on emergency stop mechanisms
- [Storage TTL Guide](storage-ttl.md) - Storage lifetime and extension procedures
- [Audit Preparation Checklist](audit-preparation.md) - Documentation requirements for security reviews

---

## Acceptance Checklist

- [x] Current upgrade posture is clearly documented (non-upgradeable)
- [x] Trust implications are explained (advantages and disadvantages)
- [x] Storage migration impact is covered (architecture, limitations, TTL)
- [x] Known open questions are listed (short-term and long-term)
- [x] Future options are described with references to detailed research
- [x] Recommendations are provided for current stage and before mainnet
- [ ] README links to this document
