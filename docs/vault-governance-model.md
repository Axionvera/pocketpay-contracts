# Vault Governance Model: Administration, Parameters, and Misuse Threat Model

This document specifies the governance architecture, parameter mutability rules, emergency pause boundaries, and admin misuse threat model for the `SavingsVault` smart contract.

> **Safety Notice:** This contract is for educational and testnet use. It is not production- or mainnet-ready. For security guidelines and current scope, see the [Security Considerations](../README.md#security-considerations) section in the main README.

---

## 1. System Administration Architecture

The `SavingsVault` contract employs a single-admin operational model designed to maintain operational safety and emergency management while strictly preserving user asset custody.

### Admin Lifecycle

1. **Initialization (`initialize`):**
   The contract admin address is established once during contract initialization:
   ```rust
   pub fn initialize(env: Env, admin: Address, token: Address)
   ```
   The `admin` address must authorize the call via `admin.require_auth()`.

2. **Admin Transfer (`transfer_admin`):**
   The current admin can transfer administrative authority to a new address:
   ```rust
   pub fn transfer_admin(env: Env, new_admin: Address)
   ```
   This operation requires authorization from the stored admin (`current_admin.require_auth()`).

---

## 2. Parameter Classification: Mutable vs. Immutable

To protect users against centralization risks and unauthorized parameter manipulation, contract storage keys are strictly categorized into **immutable** (set-once at deployment/initialization) and **mutable** (admin-controlled) parameters.

| Parameter | Mutability | Storage Key | Description | Admin Control |
| --- | --- | --- | --- | --- |
| **Accepted Token Address** | **Immutable** | `DataKey::Token` | The Stellar Asset Contract (SAC) address custodying vault funds. | **None.** Set once during `initialize`; cannot be changed. |
| **Initialization State** | **Immutable** | `DataKey::Initialized` | Boolean flag preventing contract re-initialization. | **None.** Set to `true` on `initialize`; subsequent calls panic. |
| **Storage Version** | **Immutable** | `DataKey::StorageVersion` | Contract storage layout version tag (`v1`). | **None.** Controlled by contract binary logic. |
| **Admin Address** | **Mutable** | `DataKey::Admin` | The address with administrative privileges. | **Admin-only.** Can be updated via `transfer_admin`. |
| **Emergency Pause Flag** | **Mutable** | `DataKey::Paused` | Global boolean flag blocking deposits and locks. | **Admin-only.** Activated via `pause()`, deactivated via `unpause()`. |
| **Pause Expiry Timestamp** | **Mutable** | `DataKey::PauseExpiry` | Unix timestamp (seconds) when pause auto-expires. | **Admin-only.** Calculated as `ledger.timestamp() + duration_secs`. |
| **Minimum Deposit Amount** | **Mutable** | `DataKey::MinDepositAmount` | Optional floor rejecting deposits below `min_amount`. | **Admin-only.** Updated via `set_min_deposit_amount`. |
| **Maximum Lock Duration** | **Mutable** | `DataKey::MaxLockDurationSecs` | Optional ceiling rejecting locks exceeding `max_secs`. | **Admin-only.** Updated via `set_max_lock_duration`. |
| **Minimum Lock Duration** | **Mutable** | `DataKey::MinLockDurationSecs` | Optional floor rejecting locks shorter than `min_secs`. | **Admin-only.** Updated via `set_min_lock_duration`. |

---

## 3. Admin Configuration Boundaries

Administrative control is bounded by strict input validation and access controls:

1. **Strict Authorization Checks:**
   All configuration endpoints call `admin.require_auth()` and verify that the calling address matches `DataKey::Admin`. Calling any configuration function from a non-admin account panics with `"Not authorized"`.

2. **Input Validation Rules:**
   - **Pause Duration:** `pause(admin, duration_secs)` rejects `duration_secs == 0` with `"Pause duration must be greater than zero"`.
   - **Minimum Deposit Floor:** `set_min_deposit_amount(admin, min_amount)` rejects `min_amount < 0` with `"Min deposit amount cannot be negative"`.

3. **No Direct User Fund Control:**
   The admin interface **does not contain any function** to withdraw, transfer, seize, or lock user funds. User funds can only be moved with explicit signature authorization from the account owning the funds (`user.require_auth()`).

---

## 4. Emergency Pause Mechanics and User Protection

The emergency pause mechanism is designed to halt incoming capital during a suspected incident while guaranteeing user exit liquidity.

### Active Pause Scope

When `is_paused() == true`:

- **Blocked Functions:**
  - `deposit(user, amount)` — Panics with `"Contract is paused"`.
  - `lock_funds(user, amount, unlock_time)` — Panics with `"Contract is paused"`.

- **UNAFFECTED Functions (Always Available):**
  - `withdraw(user, amount)` — Users can always withdraw unlocked deposited balances.
  - `withdraw_lock(user, lock_id)` — Users can always withdraw matured locks.
  - All read-only query helpers (`get_balance`, `get_locked_balance`, `get_lock`, `list_locks`, `is_paused`, `get_version`).

### User Withdrawal Guarantee

> **Critical Safety Invariant:** Under NO circumstances can the admin disable user withdrawals. Neither `pause()`, parameter updates, nor admin rotation alter or block `withdraw` or `withdraw_lock`.

### Automatic Pause Expiry (Auto-Unpause)

To prevent an admin from indefinitely pausing the vault without intervention, pauses are time-bounded:
1. `pause(admin, duration_secs)` computes `expiry = ledger.timestamp() + duration_secs`.
2. Once `env.ledger().timestamp() >= expiry`, `is_paused()` automatically returns `false`.
3. The next mutating call lazily clears `Paused` and `PauseExpiry` storage entries without requiring an explicit `unpause` transaction.

---

## 5. Admin Misuse Threat Model

This section details potential misuse vectors by a malicious or compromised admin, system mitigations, and residual risks.

### Threat Scenarios and Mitigations

#### Scenario 1: Malicious Admin Attempts to Seize User Funds
- **Threat Vector:** Admin invokes contract functions seeking to transfer custodied SAC tokens out of the vault contract address to an external account.
- **Mitigation:** The contract contains no admin withdrawal or sweep capability. All token transfer operations (`token_client.transfer`) in `withdraw` and `withdraw_lock` require the target user's signature (`user.require_auth()`).

#### Scenario 2: Compromised Admin Attempts to Trap User Liquidity
- **Threat Vector:** Admin calls `pause()` with a maximum duration to freeze user assets inside the contract.
- **Mitigation:** `withdraw` and `withdraw_lock` do not evaluate `require_not_paused`. Users can immediately withdraw all available and matured funds regardless of pause status.

#### Scenario 3: Admin Sets Exorbitant Minimum Deposit Floor
- **Threat Vector:** Admin sets `MinDepositAmount` to `i128::MAX` to prevent new user deposits.
- **Mitigation:** Existing user balances remain withdrawable. The admin cannot alter existing user balance records.

#### Scenario 4: Admin Sets Short Maximum Lock Duration
- **Threat Vector:** Admin sets `MaxLockDurationSecs` to `1` to restrict new time-locks.
- **Mitigation:** Previously created time-lock entries maintain their recorded `unlock_time` and cannot be mutated or extended by the admin.

### Residual Risks and Governance Recommendations

1. **Single-Key Admin Risk:**
   - *Current State:* The admin is a single Stellar account address. If the private key is lost or compromised, governance parameters cannot be adjusted.
   - *Recommendation:* Before mainnet deployment, transition the admin address to a multi-signature account or DAO governance contract.

2. **Immutability of Accepted Token:**
   - *Current State:* If the underlying SAC token is upgraded, reissued, or deprecated, the vault cannot update `DataKey::Token`.
   - *Mitigation:* Users can withdraw their tokens from the existing vault contract and redeposit into a newly deployed vault instance.

---

## 6. Verification and Security Test Coverage

The governance rules and authorization boundaries are enforced by dedicated unit test suites under `contracts/savings_vault/src/test/`:

- **Unauthorized Admin Actions:** `governance_security.rs` verifies that non-admin callers cannot invoke `pause`, `unpause`, or config setters.
- **Admin Withdrawal Isolation:** `admin_invariant_guard.rs` proves the admin cannot withdraw or lock funds belonging to other users.
- **Pause Withdrawal Availability:** `pause.rs` confirms that `withdraw` and `withdraw_lock` succeed while `is_paused() == true`.
- **Re-initialization Rejection:** `initialization.rs` proves that `initialize` cannot be called twice.

---

## Related Documentation

- [Admin Role Specification](admin-role.md) — Detailed breakdown of admin functions and permissions
- [Emergency Pause and Threat Model](admin-pause-threat-model.md) — Comprehensive threat analysis for pause mechanics
- [Authorisation Rules](authorisation-rules.md) — System-wide authentication requirements per function
- [Formal Accounting Invariants](accounting-invariants.md) — Invariants for token custody and user isolation
