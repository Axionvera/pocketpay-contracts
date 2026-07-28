# Post-Deployment Configuration Freezing Design

> **Status:** Research & Design
>
> **Scope:** Savings Vault contract (`contracts/savings_vault`)
>
> This document specifies the design for freezing selected configuration parameters after deployment. It defines freezable parameters, the one-way latch freeze mechanism, authorization requirements, user trust implications, and reference implementation details.

---

## Table of Contents

1. [Motivation](#motivation)
2. [Identified Freezable Parameters](#identified-freezable-parameters)
3. [Freeze Mechanism Architecture](#freeze-mechanism-architecture)
4. [Authorization & Security Control](#authorization--security-control)
5. [User Impact & Trust Implications](#user-impact--trust-implications)
6. [Reference Implementation Sketch](#reference-implementation-sketch)
7. [Testing & Verification Strategy](#testing--verification-strategy)
8. [Acceptance Checklist](#acceptance-checklist)

---

## Motivation

In early stages of smart contract deployment, administrators may need flexibility to configure operational parameters (such as token addresses or fee parameters). However, once user funds are deposited into the vault, administrative mutability becomes a primary security concern:

- **Parameter Misconfiguration / Key Compromise:** If an administrative key is compromised, an attacker could point the contract's `Token` address to a malicious asset contract or alter protocol parameters to drain user funds.
- **Trust Minimization:** Users and auditors expect that key protocol parameters cannot be unilaterally altered after deposits are active.

A **post-deployment freeze mechanism** resolves this by allowing administrators to irreversibly lock (freeze) specific configuration parameters once initial setup and testing are verified.

---

## Identified Freezable Parameters

The table below outlines contract parameters, their mutability status, and the rationale for freezing:

| Parameter | Storage Key | Initial Setting | Freezable? | Impact of Freezing |
|---|---|---|---|---|
| **Stellar Asset Contract Token** | `DataKey::Token` | `initialize(admin, token)` | **Yes** | Prevents changing the underlying asset contract. Ensures funds are always backed by the original token. |
| **Contract Administration** | `DataKey::Admin` | `initialize(admin, token)` | **Yes** | Renounces or locks administrative privileges, rendering configuration parameters permanently static. |
| **Deposit / Lock Limits** *(Proposed)* | `DataKey::DepositLimit` | Governance / Admin | **Yes** | Locks operational limits (e.g. max deposit size, min lock duration) so policy rules cannot change arbitrarily. |

---

## Freeze Mechanism Architecture

### 1. One-Way Latch Pattern
The freeze mechanism uses an **irreversible boolean flag** (a "one-way latch"). Once a parameter's freeze flag is set to `true`, no function (including admin-authorized calls) can reset it to `false` or modify the underlying parameter value.

### 2. Storage Model
Storage keys for freeze flags use persistent or instance storage:

```rust
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // ... existing keys ...
    Token,             // Token Address
    TokenFrozen,       // bool flag: true if Token address is permanently frozen
    AdminFrozen,       // bool flag: true if Admin privileges are permanently frozen
}
```

### 3. Event Schema
Every freeze action MUST emit an on-chain Soroban event so off-chain indexers and wallets can update contract immutability badges:

- **Topic 0**: `Symbol::new(&env, "freeze")`
- **Topic 1**: `param_name` (`Symbol`, e.g., `"token"`, `"admin"`)
- **Payload**: `executor` (`Address`) - Address of the administrator executing the freeze action.

```json
{
  "topics": ["freeze", "token"],
  "value": "GB...ADMIN_ADDRESS"
}
```

---

## Authorization & Security Control

1. **Strict Admin Authorization**:
   - Only the designated administrator address stored in `DataKey::Admin` can execute freeze functions.
   - Requires explicit signature verification via `admin.require_auth()`.

2. **No Recovery / Unfreeze Function**:
   - The contract deliberately contains **no `unfreeze` function**.
   - If `env.storage().instance().get(&DataKey::TokenFrozen).unwrap_or(false)` evaluates to `true`, any attempt to modify `DataKey::Token` panics immediately with `"Parameter is frozen"`.

3. **Protection Against Pre-Initialization Freezing**:
   - Freeze actions can only be executed after `initialize()` has been successfully called.

---

## User Impact & Trust Implications

| Trust Metric | Before Freezing | After Freezing |
|---|---|---|
| **Admin Control** | Admin retains potential capability to update settings | Admin power is permanently revoked for frozen parameters |
| **Asset Security** | Vulnerable to admin key leakage / compromised wallet | Immutable guarantee that deposited tokens map to original SAC asset |
| **User Transparency** | Users must trust admin governance posture | Users verify immutability via on-chain `TokenFrozen` flag & `freeze` events |

---

## Reference Implementation Sketch

```rust
// In lib.rs

pub fn freeze_token(env: Env, admin: Address) {
    admin.require_auth();

    // Verify caller is stored admin
    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    if admin != stored_admin {
        panic!("Unauthorized caller");
    }

    // Verify not already frozen
    let is_frozen: bool = env
        .storage()
        .instance()
        .get(&DataKey::TokenFrozen)
        .unwrap_or(false);

    if is_frozen {
        panic!("Token configuration is already frozen");
    }

    // Persist permanent freeze flag
    env.storage().instance().set(&DataKey::TokenFrozen, &true);

    // Emit event
    env.events().publish(
        (Symbol::new(&env, "freeze"), Symbol::new(&env, "token")),
        admin,
    );
}
```

---

## Testing & Verification Strategy

Unit tests must validate the following behavioral invariants:

1. **Valid Freeze Execution**: Admin can invoke `freeze_token()`, setting `TokenFrozen` to `true`.
2. **Unauthorized Caller Rejection**: Calling `freeze_token()` from a non-admin user panics.
3. **Double Freeze Protection**: Calling `freeze_token()` a second time panics with `"Token configuration is already frozen"`.
4. **Parameter Mutability Guard**: Any future function attempting to update `Token` panics if `TokenFrozen` is `true`.

---

## Acceptance Checklist

- [x] Freeze mechanism is designed (one-way latch pattern, event emission).
- [x] Freezable parameters are identified (`Token`, `Admin`, operational parameters).
- [x] Authorisation is documented (`admin.require_auth()`, no unfreeze vector).
- [x] User impact is explained (trust minimization, transparency).
- [x] Test specifications and reference implementation sketches are provided.
