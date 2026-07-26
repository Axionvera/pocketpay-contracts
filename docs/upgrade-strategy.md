# Contract Upgrade Strategy — Savings Vault

> **Status:** Research & Design (not implemented)
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This document explores possible upgrade strategies for future versions of the Savings Vault contract. It is a research artifact — **no upgrade mechanism is being implemented as part of this document**.

---

## Table of Contents

1. [Current State](#current-state)
2. [Why Upgradability Matters](#why-upgradability-matters)
3. [Strategy Comparison](#strategy-comparison)
   - [A. No Upgrade (Immutable)](#a-no-upgrade-immutable)
   - [B. Admin-Controlled Upgrade (Proxy Pattern)](#b-admin-controlled-upgrade-proxy-pattern)
   - [C. Migration Contract](#c-migration-contract)
   - [D. Redeploy and Social Coordination](#d-redeploy-and-social-coordination)
4. [Comparison Matrix](#comparison-matrix)
5. [Soroban-Specific Considerations](#soroban-specific-considerations)
6. [Security Risks](#security-risks)
7. [Open Questions](#open-questions)
8. [Recommendation](#recommendation)

---

## Current State

As of the current codebase, the Savings Vault contract has **no upgrade mechanism** (logic cannot be changed after deployment). However, it does have **storage versioning** to support future safe migrations:
- `DataKey::StorageVersion` is stored in instance storage (set to `1` during initialization).
- The `assert_supported_storage_version` helper checks for version compatibility on all function calls.
- Missing version (legacy pre-versioning contracts) are treated as version `1` for backward compatibility.

The `initialize(admin, token)` function records an admin address (for future reference) and token address.

From the README:

> **No upgrade mechanism**: The contract does not implement `upgrade()`. Consider adding this for mainnet.

This document exists to help maintainers evaluate their options before committing to an upgrade strategy.

---

## Why Upgradability Matters

| Concern | Risk Without Upgradability |
|---|---|
| **Bug fixes** | A critical vulnerability cannot be patched; funds may be permanently at risk |
| **Feature additions** | New use cases (e.g., interest accrual, multi-asset support) require a fresh deployment |
| **Protocol changes** | Soroban host function upgrades or Stellar protocol changes may break assumptions |
| **Regulatory compliance** | Inability to adapt to legal requirements (e.g., sanctions screening) |
| **Gas / storage optimizations** | Efficiency improvements cannot be applied to existing user balances |

However, upgradability is a double-edged sword: it introduces trust assumptions and new attack surfaces.

---

## Strategy Comparison

### A. No Upgrade (Immutable)

The contract is deployed once and never changed. If a new version is needed, a completely new contract is deployed and users must migrate manually.

**How it works:**
1. Deploy the contract and never change the wasm.
2. If a bug is found or features are needed, deploy a new contract instance with a new contract ID.
3. Users withdraw from the old contract and deposit into the new one.

**Pros:**
- Maximum trustlessness — users know the code can never change.
- No admin key risk — no single party can alter contract behavior.
- Simplest to audit and reason about.
- Aligned with blockchain ethos of "code is law."

**Cons:**
- No ability to patch critical bugs.
- User experience is painful on every upgrade — everyone must manually move funds.
- Stranded funds if users don't migrate in time.
- Frontend and integrations must track multiple contract IDs over time.
- No emergency response capability.

**Best for:** Protocols that are simple, feature-complete, and unlikely to need changes. High-value vaults where immutability is a selling point.

---

### B. Admin-Controlled Upgrade (Proxy Pattern)

A proxy contract holds all state (balances, locks, admin address) and delegates all logic calls to an implementation contract. The admin can point the proxy to a new implementation, effectively upgrading the contract logic without moving user funds.

```
┌──────────────────────────────────────────────┐
│                  Proxy Contract               │
│  ┌──────────────────────────────────────┐    │
│  │  Storage (balances, locks, admin)     │    │
│  └──────────────────────────────────────┘    │
│                      │                       │
│              delegates calls to               │
│                      ▼                       │
│  ┌──────────────────────────────────────┐    │
│  │  Implementation Contract v1          │    │
│  │  (logic only, no storage)            │    │
│  └──────────────────────────────────────┘    │
│                                               │
│  Admin can replace ▼ with new implementation │
│                                               │
│  ┌──────────────────────────────────────┐    │
│  │  Implementation Contract v2          │    │
│  │  (new/patched logic)                 │    │
│  └──────────────────────────────────────┘    │
└──────────────────────────────────────────────┘
```

**How it works:**
1. Deploy a proxy contract that holds all storage.
2. Deploy an implementation contract with the logic.
3. The proxy delegates all invocations to the implementation via Soroban's `try_call` / cross-contract call.
4. Admin calls `upgrade(new_wasm_hash)` on the proxy to switch to a new implementation.
5. Storage layout must remain compatible between versions (or a migration function must be included).

**Pros:**
- Seamless upgrades — users keep using the same contract ID.
- All balances and lock state are preserved across upgrades.
- Can patch bugs quickly without user action.
- Single contract ID for frontends and integrations.

**Cons:**
- Admin key becomes a high-value target — compromise means full control.
- Increased complexity — proxy is harder to audit.
- Storage layout compatibility is fragile; a mistake can corrupt data.
- Gas overhead from delegation on every call.
- Users must trust the admin not to deploy malicious upgrades.
- Soroban's cross-contract call model adds nuances (see [Soroban-Specific Considerations](#soroban-specific-considerations)).

**Security mitigations:**
- **Timelock**: Require a delay (e.g., 48 hours) between proposing and executing an upgrade, giving users time to exit.
- **Multisig admin**: Require multiple signers (e.g., 3-of-5) to approve an upgrade.
- **Governance**: Delegate upgrade authority to a DAO or governance contract.
- **Upgrade events**: Emit on-chain events on every upgrade so off-chain monitors can alert users.
- **Implementation allowlist**: Restrict which implementation hashes are accepted.

**Best for:** Actively developed protocols where feature iteration and bug fixes are expected.

---

### C. Migration Contract

A dedicated migration contract is deployed alongside each version. Users opt-in to migrate their balances from the old contract to the new one via the migration contract, which atomically withdraws from the old and deposits into the new.

```
┌──────────────────┐     ┌──────────────────────┐     ┌──────────────────┐
│  Vault v1        │     │  Migration Contract   │     │  Vault v2        │
│  (old contract)  │◄────│                      │────►│  (new contract)  │
│                  │     │  1. withdraw(v1)      │     │                  │
│  Balances:       │     │  2. deposit(v2)        │     │  Balances:       │
│    Alice: 100    │     │  3. emit event         │     │    Alice: 100    │
│    Bob:   50     │     │                        │     │    Bob:   50     │
└──────────────────┘     └──────────────────────┘     └──────────────────┘
         ▲                        │                          ▲
         │                        │                          │
         └──────── User calls migrate() ─────────────────────┘
```

**How it works:**
1. Deploy v2 of the vault contract.
2. Deploy a migration contract that knows both v1 and v2 contract IDs.
3. Users call `migrate(user)` on the migration contract.
4. The migration contract:
   - Calls `withdraw(user, balance)` on v1.
   - Calls `deposit(user, balance)` on v2.
   - Emits a `Migrated` event.
5. After all users migrate, v1 can be archived.

**Pros:**
- No proxy complexity — both v1 and v2 are standard, auditable contracts.
- Users opt-in — no forced upgrades.
- Migration contract can be audited independently.
- Clear audit trail via migration events.
- v1 remains immutable and trustworthy.

**Cons:**
- Users must actively migrate — some may never do so, leaving funds stranded.
- Locked funds need special handling (lock times must be preserved or compensated).
- Two contract IDs to track during the migration period.
- Migration contract itself must be trusted or audited.
- Frontends need to detect which version a user is on and guide migration.

**Locked-funds considerations:**
- Locked balances can be migrated but the unlock time must be preserved in v2.
- Alternatively, locked funds can remain in v1 until they unlock, then be migrated.
- A "partial migration" design lets users migrate unlocked funds immediately and locked funds later.

**Best for:** Protocols that upgrade infrequently and want to preserve immutability guarantees while providing a supported migration path.

---

### D. Redeploy and Social Coordination

The simplest "upgrade": deploy a new contract, announce it, and expect users to move their funds manually. This is the baseline strategy and is effectively what happens today since no upgrade mechanism exists.

**How it works:**
1. Deploy a new contract instance with improved logic.
2. Announce the new contract ID through official channels (website, Twitter, Discord, on-chain memo).
3. Users call `withdraw` on the old contract and `deposit` on the new contract.
4. Old contract remains accessible indefinitely (or until storage expires).

**Pros:**
- Zero additional code complexity.
- No new trust assumptions.
- Fully transparent — users can compare old and new wasm.
- Works with any contract design.

**Cons:**
- High user friction — every user must take action.
- Inevitable fund stranding — some users will miss the announcement.
- Locked funds cannot be moved until unlock time.
- Frontend fragmentation — which contract ID is "current"?
- No atomicity — users may withdraw but forget to deposit.
- Repeated redeploys erode user confidence.

**Best for:** Early-stage projects with small user bases, testnet-only deployments, or protocols where immutability is paramount and upgrades are expected to be extremely rare.

---

## Comparison Matrix

| Criterion | A. Immutable | B. Proxy Upgrade | C. Migration Contract | D. Redeploy |
|---|---|---|---|---|
| **Upgradability** | None | Instant | Opt-in | Manual |
| **User action required** | Deploy new + migrate | None | Call migrate() | Withdraw + deposit |
| **Preserves balances** | N/A (new contract) | Yes (automatic) | Yes (atomic migration) | No (manual) |
| **Preserves lock state** | N/A | Yes (if storage compat) | Yes (with design) | No |
| **Admin trust required** | None | High | Medium (migration contract) | None |
| **Code complexity** | Minimal | High | Medium | Minimal |
| **Audit surface** | Smallest | Largest | Medium | Smallest |
| **Gas overhead (per call)** | None | Moderate (delegation) | None | None |
| **Stranded fund risk** | High (on each upgrade) | None | Medium (non-migrators) | High |
| **Attack surface** | None new | Admin key, storage collision | Migration contract bugs | None new |
| **Suitable for mainnet** | Yes (if feature-complete) | Yes (with multisig + timelock) | Yes (audited migration) | Only very early stage |
| **Soroban compatibility** | Full | Requires careful design | Full | Full |

---

## Soroban-Specific Considerations

Soroban has several characteristics that affect upgrade strategy design:

### Contract Hashes and Deployments
- Soroban contracts are identified by a hash of the wasm. Deploying updated wasm produces a new hash.
- Contract instances are created from a wasm hash and receive a unique contract ID.
- There is no native `delegatecall` like Ethereum — cross-contract calls are explicit.

### Proxy Implementation on Soroban
- A proxy must use cross-contract calls (`env.invoke_contract()`) to delegate to the implementation.
- The proxy stores the implementation's contract ID or wasm hash in its own storage.
- All function arguments must be passed through, which may require serialization/deserialization.

### Storage Model
- Soroban uses a combination of instance, persistent, and temporary storage with TTL (time-to-live).
- Instance storage is tied to the contract and has its own TTL.
- Persistent storage entries also have TTL and must be periodically extended.
- A proxy's storage architecture must account for TTL management across upgrades.

### Sac (Stellar Asset Contract) Integration
- If the vault later integrates with SAC for real token transfers, the proxy or migration contract must handle token approvals and transfers correctly.
- Token allowances to the old contract must be migrated or re-granted.

### Ledger Entry Limits
- Soroban enforces limits on ledger entry sizes and counts.
- A proxy that accumulates storage over many upgrades could hit these limits.

---

## Security Risks

### General Upgrade Risks

| Risk | Severity | Description |
|---|---|---|
| **Admin key compromise** | Critical | If the admin key is stolen, an attacker can deploy a malicious upgrade that drains all funds. |
| **Storage collision** | Critical | A new implementation that misinterprets the storage layout can corrupt balances or lock state. |
| **Upgrade rug-pull** | Critical | A malicious or coerced admin deploys an upgrade that steals funds. |
| **Incomplete migration** | High | Users who don't migrate within the window may lose access to funds if the old contract is sunset. |
| **Lock state corruption** | High | Locked funds with timestamps may become permanently locked or prematurely unlocked after an upgrade. |
| **Frontend desync** | Medium | Wallets and dapps may continue pointing to the old contract after an upgrade. |
| **Event mismatch** | Medium | Off-chain indexers may miss or misinterpret events emitted by new implementations. |
| **TTL expiry during migration** | Medium | Storage entries may expire if migration takes too long. |
| **Governance attack** | Medium | If upgrade control is delegated to governance, a governance attack can force malicious upgrades. |

### Risk Mitigations Summary

1. **Multisig admin**: Require M-of-N signatures for any upgrade.
2. **Timelock**: Enforce a delay between upgrade proposal and execution (e.g., 48–72 hours).
3. **On-chain events**: Emit `UpgradeProposed` and `UpgradeExecuted` events.
4. **Implementation verification**: Publish and verify the wasm hash of every implementation.
5. **Storage versioning**: Include a `version: u32` field in storage so implementations can detect and handle version mismatches.
6. **Emergency freeze**: Allow a separate "guardian" role to freeze the contract without upgrading it (see [pause-design.md](pause-design.md)).
7. **Gradual rollout**: Deploy upgrades to testnet first; run a bug bounty before mainnet.
8. **User opt-out window**: Allow users to withdraw all funds during the timelock period before an upgrade takes effect.

---

## Open Questions

The following questions should be resolved before selecting and implementing an upgrade strategy:

1. **What is the expected upgrade frequency?** If upgrades are rare (once a year or less), a migration contract may be sufficient. If frequent (monthly), a proxy may be justified.

2. **Who holds the admin key(s)?** Is it a single developer, a multisig of core team members, or a DAO? The answer determines the trust model.

3. **What happens to locked funds during an upgrade?** Should locked balances and unlock timestamps be preserved exactly? Is there a grace period?

4. **How will users be notified of upgrades?** On-chain events? Off-chain announcements? In-wallet prompts?

5. **Should the upgrade mechanism itself be upgradeable?** Can the upgrade process be changed after deployment, or is it immutable?

6. **Will the contract eventually become immutable?** Some protocols plan to "renounce" upgradeability after a stabilization period.

7. **How does the upgrade strategy interact with the pause mechanism?** Should the contract be paused during an upgrade?

8. **What is the fallback if an upgrade goes wrong?** Can the proxy roll back to a previous implementation? Can a migration be reversed?

9. **How are frontends and SDKs kept in sync?** Is there a registry contract that always points to the latest version?

10. **What are the gas and storage cost implications?** Proxy delegation adds gas overhead. How much?

---

## Recommendation

> **For the current stage of the project (testnet / educational use):**
>
> Stick with **Strategy D (Redeploy)** — the current approach. The user base is small, the contract is simple, and the overhead of a proxy or migration contract outweighs the benefits.
>
> **Before mainnet:**
>
> Re-evaluate based on the answers to the open questions above. The most balanced approach for a savings vault with non-trivial TVL is likely **Strategy C (Migration Contract)** combined with a **timelocked multisig admin**:
>
> - Each version of the vault remains immutable and independently auditable.
> - A migration contract provides a supported, atomic migration path.
> - A timelock + multisig on the migration contract prevents unilateral forced upgrades.
> - Locked funds are handled with explicit migration logic.
>
> **Strategy B (Proxy)** should only be chosen if the team expects frequent iterative upgrades and is willing to invest in the significant audit, monitoring, and operational overhead required to secure a proxy.
>
> **Strategy A (Immutable)** is the gold standard for trust minimization but is only viable if the contract is provably bug-free and feature-complete — a high bar for any DeFi protocol.

---

## References

- [Admin Role Documentation](admin-role.md) — details on the current admin address and its (lack of) powers.
- [Pause / Emergency Stop Design](pause-design.md) — complementary research on emergency stop mechanisms.
- [Soroban Docs: Contract Lifecycle](https://soroban.stellar.org/docs/getting-started/deploy-to-testnet)
- [Soroban Docs: Cross-Contract Calls](https://soroban.stellar.org/docs/learn/authorization#cross-contract-calls)
- [Soroban Docs: Storage](https://soroban.stellar.org/docs/learn/persisting-data)

---

## Acceptance Checklist

- [x] Document exists at `docs/upgrade-strategy.md`.
- [x] Current lack of upgrade support is clearly stated.
- [x] At least three approaches are compared (four covered: immutable, proxy, migration contract, redeploy).
- [x] Security risks are listed with mitigations.
- [x] Open questions are documented for future decision-making.
