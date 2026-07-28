# Safe Admin Key Rotation Design — Savings Vault

> **Status:** Research & Design (Specification)
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This document specifies the design for safe admin key rotation in the Savings Vault smart contract. It establishes a secure two-step nomination and acceptance flow, authorization rules, storage layout, event schema, failure modes, and threat mitigations.

---

## Table of Contents

1. [Motivation](#motivation)
2. [Why Single-Step Transfer is Unsafe](#why-single-step-transfer-is-unsafe)
3. [Two-Step Nomination & Acceptance Protocol](#two-step-nomination--acceptance-protocol)
4. [Authorization Model](#authorization-model)
5. [Storage Layout & State Management](#storage-layout--state-management)
6. [Event Schema & Topics](#event-schema--topics)
7. [Failure Cases & Error Scenarios](#failure-cases--error-scenarios)
8. [Threat Model & Misuse Risks](#threat-model--misuse-risks)
9. [Future Governance Extensions](#future-governance-extensions)

---

## Motivation

Admin accounts in smart contract systems are subject to operational lifecycle events, including:

- **Key Rotation Policies**: Routine security practices requiring periodic key updates.
- **Key Loss or Degradation**: Migrating away from cold storage keys or hardware devices nearing retirement.
- **Compromise Mitigation**: Rapidly handing off administrative authority if an existing key is suspected to be compromised.
- **Ownership Handover**: Transferring control from an initial deployer key to a multi-signature account or DAO governance contract.

If admin capabilities are introduced or expanded in the Savings Vault (e.g., emergency pause, contract upgrades), maintaining a safe key rotation mechanism is essential to prevent permanent lockouts or loss of administrative control.

---

## Why Single-Step Transfer is Unsafe

A single-step transfer model (e.g., `set_admin(new_admin)`) updates the stored admin key immediately upon invocation by the current admin:

```
[ Current Admin ] ---> set_admin(new_admin) ---> [ New Admin Active Immediately ]
```

### Risks of Single-Step Transfer

1. **Fat-Finger / Typo Risk**: If the current admin passes an incorrect, unformatted, or un-owned address, administrative control is immediately transferred to a non-existent or inaccessible account. Because the old admin loses access instantaneously, **the contract becomes permanently orphaned**.
2. **Lack of Key Ownership Proof**: The single-step pattern does not verify whether the destination address has a valid secret key or can authorize transactions on-chain.
3. **No Opportunity to Cancel**: Once the transaction executes, the mistake cannot be reversed.

---

## Two-Step Nomination & Acceptance Protocol

To eliminate single-step transfer risks, the contract adopts a **Two-Step Nomination-Acceptance Protocol** (also known as Claimable Ownership).

```
   ┌────────────────┐
   │ Current Admin  │
   └───────┬────────┘
           │  1. propose_admin(new_candidate)
           ▼
   ┌────────────────┐                     ┌────────────────┐
   │ Pending Admin  │ ─── 2. accept_admin ──►│  Active Admin  │
   │  (Nominated)   │                     │   (Updated)    │
   └───────▲────────┘                     └────────────────┘
           │
           │ 3. revoke_nomination() (optional)
   ┌───────┴────────┐
   │ Current Admin  │
   └────────────────┘
```

### Protocol Steps

#### Step 1: Nomination (`propose_admin`)
The current admin nominates a candidate address by calling `propose_admin(env, new_candidate)`.
- **Precondition**: Contract must be initialized; caller must be the active admin; `new_candidate` must not be the zero/dead address or the current admin.
- **State Change**: `DataKey::PendingAdmin` is updated to store `new_candidate`. The active admin (`DataKey::Admin`) remains unchanged.
- **Event**: Emits `admin_proposed(current_admin, new_candidate)`.

#### Step 2: Acceptance (`accept_admin`)
The nominated candidate claims administrative control by calling `accept_admin(env)`.
- **Precondition**: Caller must be the address stored in `DataKey::PendingAdmin`.
- **State Change**:
  - `DataKey::Admin` is updated to the caller's address (`new_candidate`).
  - `DataKey::PendingAdmin` is removed from storage.
- **Event**: Emits `admin_accepted(old_admin, new_admin)`.

#### Step 3: Revocation / Cancellation (`revoke_nomination`)
At any time before the candidate accepts, the active admin can cancel the pending nomination by calling `revoke_nomination(env)`.
- **Precondition**: Caller must be the active admin; `DataKey::PendingAdmin` must exist.
- **State Change**: `DataKey::PendingAdmin` is cleared.
- **Event**: Emits `admin_proposal_revoked(current_admin, revoked_candidate)`.

---

## Authorization Model

| Function | Required Signer | Enforcement Mechanism |
| :--- | :--- | :--- |
| `propose_admin(new_candidate)` | Current Active Admin | `current_admin.require_auth()` + `stored_admin == current_admin` |
| `accept_admin()` | Pending Candidate | `pending_admin.require_auth()` + `stored_pending == pending_admin` |
| `revoke_nomination()` | Current Active Admin | `current_admin.require_auth()` + `stored_admin == current_admin` |

### Key Guarantees
- The current admin **cannot force** an un-owned address to become the admin.
- The new admin **must prove cryptographic control** of their secret key by successfully signing the `accept_admin` transaction.
- The active admin retains full control until the candidate explicitly accepts.

---

## Storage Layout & State Management

The rotation protocol introduces one additional storage key within **Instance Storage**:

```rust
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,         // Address (Active Admin)
    PendingAdmin,  // Address (Nominated Candidate)
    Initialized,   // bool
    Token,         // Address
}
```

### Storage Lifecycle

| State | `DataKey::Admin` | `DataKey::PendingAdmin` | Note |
| :--- | :--- | :--- | :--- |
| **Initial** | `AdminAddress` | *None* | Set during `initialize()`. |
| **Pending Rotation** | `AdminAddress` | `CandidateAddress` | Set during `propose_admin()`. |
| **Rotation Complete** | `CandidateAddress` | *None* | Updated and cleared during `accept_admin()`. |
| **Revoked** | `AdminAddress` | *None* | Cleared during `revoke_nomination()`. |

> [!NOTE]
> `PendingAdmin` is stored in **Instance Storage** alongside `Admin`. Extending the contract instance TTL keeps both keys alive simultaneously.

---

## Event Schema & Topics

All admin rotation events follow standard Soroban event guidelines (see [`docs/events.md`](file:///c:/Users/User/OneDrive/Desktop/GrantFox/contracts/pocketpay-contracts/docs/events.md)):

### 1. `AdminProposed`
Emitted when a new admin candidate is nominated.
- **Topic 0**: `Symbol::new(&env, "admin_proposed")`
- **Topic 1**: `current_admin` (`Address`)
- **Payload**: `pending_admin` (`Address`)

### 2. `AdminAccepted`
Emitted when the nominated candidate claims the admin role.
- **Topic 0**: `Symbol::new(&env, "admin_accepted")`
- **Topic 1**: `new_admin` (`Address`)
- **Payload**: `old_admin` (`Address`)

### 3. `AdminProposalRevoked`
Emitted when an active admin cancels a pending nomination.
- **Topic 0**: `Symbol::new(&env, "admin_proposal_revoked")`
- **Topic 1**: `current_admin` (`Address`)
- **Payload**: `revoked_pending_admin` (`Address`)

---

## Failure Cases & Error Scenarios

| Scenario | Trigger Condition | Outcome | Recovery / Action |
| :--- | :--- | :--- | :--- |
| **Non-Admin Proposes** | Caller of `propose_admin` is not `DataKey::Admin` | Transaction panics / Host Auth Error | Reject invocation. Only active admin can nominate. |
| **Non-Candidate Accepts** | Caller of `accept_admin` is not `DataKey::PendingAdmin` | Transaction panics ("No pending admin nomination") | Reject invocation. Only the nominated candidate can claim. |
| **Proposing Same Admin** | `new_candidate == current_admin` | Transaction panics ("Candidate is already admin") | Provide a new candidate address. |
| **Accepting Without Nomination** | `accept_admin` called when `DataKey::PendingAdmin` is empty | Transaction panics ("No pending admin nomination") | Active admin must first invoke `propose_admin`. |
| **Candidate Key Lost** | Candidate loses secret key before calling `accept_admin` | Rotation cannot complete; contract remains under current admin | Current admin calls `revoke_nomination` and proposes a new candidate. |
| **Stale Pending Entry** | Rotation abandoned midway | `PendingAdmin` remains in storage | Current admin calls `revoke_nomination` to clear state. |

---

## Threat Model & Misuse Risks

### 1. Key Theft / Compromised Active Admin
- **Threat**: An attacker steals the active admin key and immediately proposes their own malicious address.
- **Mitigation**: The two-step process gives off-chain monitoring indexers visibility into the `admin_proposed` event. Off-chain alerts can notify team members, allowing the active admin (or an emergency timelock/guardian) to call `revoke_nomination` or trigger an emergency pause before the attacker completes `accept_admin`.

### 2. Griefing / Unresponsive Candidate
- **Threat**: A nominated candidate refuses to call `accept_admin`, stalling protocol governance.
- **Mitigation**: Active admin authority is **never lost** during the pending phase. The current admin can revoke the nomination at any time via `revoke_nomination`.

### 3. Front-Running / Race Conditions
- **Threat**: An attacker attempts to front-run `accept_admin`.
- **Mitigation**: `accept_admin` relies on `pending_admin.require_auth()`, ensuring only a transaction signed by the exact nominated address can succeed.

---

## Future Governance Extensions

When transitioning from testnet to production mainnet, consider the following enhancements:

1. **Timelocked Rotation**: Require a mandatory delay (e.g., 48 hours) between `propose_admin` and when `accept_admin` becomes executable, allowing vault users time to review admin changes.
2. **Multisig Ownership**: Transition `DataKey::Admin` from a single key address to a Soroban multisig contract or Stellar smart wallet instance.
3. **DAO Governance Handoff**: Transfer `DataKey::Admin` to a decentralized governance voting contract once protocol parameters mature.

---

## References

- [Admin Role Overview](file:///c:/Users/User/OneDrive/Desktop/GrantFox/contracts/pocketpay-contracts/docs/admin-role.md)
- [Emergency Pause Design](file:///c:/Users/User/OneDrive/Desktop/GrantFox/contracts/pocketpay-contracts/docs/pause-design.md)
- [Vault Events Specification](file:///c:/Users/User/OneDrive/Desktop/GrantFox/contracts/pocketpay-contracts/docs/events.md)
