# Known Limitations (Savings Vault)

> **Note:** This repository is for educational and **testnet-focused** development. The items below reflect current constraints in the smart contract as implemented today and are intended for planning and transparency, not for “mainnet readiness” claims.

This document centralizes limitations that are currently described in `README.md`, and captures likely future work so they are visible as the project evolves.

---

## Current limitations / constraints

### 1) Internal balance tracking only (no real token transfers)
- The contract maintains user balances internally (available/unlocked balance and locked balance).
- It does **not** integrate with the Stellar Asset Contract (SAC) or any token contract to perform actual `transfer()` calls.
- As a result, the “wallet vault” logic is self-contained and not yet connected to external assets.

### 2) Single unlock timestamp per user (lock overwrite behavior)
- The contract stores a single `unlock_time` value per user.
- Calling `lock_funds(user, amount, unlock_time)` again for the same user **overwrites** the stored unlock timestamp.
- A future design may support multiple concurrent locks (e.g., per-lock entries) rather than one shared unlock time.

### 3) No upgrade mechanism
- The contract does not implement `upgrade()` and does not include a proxy/upgrade pattern.
- Once deployed, there is no on-chain upgrade path encoded in the current contract.

### 4) No admin recovery / migration mechanism
- The contract does not provide any admin-only function to recover, migrate, or sweep balances.
- There is no emergency recovery or migration flow for funds in the current design.

---

## What’s being worked on (future work)

Because this is still early-stage / educational, future work may include:
- Integrating with an external token contract (e.g., SAC) so deposits/withdrawals correspond to real asset movement.
- Supporting multiple locks per user rather than a single unlock time.
- Evaluating safe upgrade patterns (only if/when required) with clear governance and user protections.
- Designing an emergency pause and/or recovery/migration approach—but only after explicitly defining trust assumptions and safety properties.

---

## Status

These limitations reflect what is implemented **today**. If you are planning to build on top of this repository, verify the current contract behavior in code and tests before relying on any assumption.

