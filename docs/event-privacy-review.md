# Smart Contract Event Privacy & Sensitive Data Exposure Review

## Overview

Smart contract events emitted on public blockchains like Stellar (Soroban) are permanently recorded on the public ledger. Anyone—including indexers, block explorers, transaction monitoring platforms, and third-party observers—can read, aggregate, and analyze event topics and payloads.

This document provides a comprehensive **Event Privacy & Data Exposure Review** for the PocketPay Savings Vault smart contract. It establishes guidelines on minimum required payloads, identifies sensitive data risks, and documents event data boundaries.

---

## Sensitive Data Risks & Privacy Invariants

### 1. Account Linkage & Financial Footprinting
- **Risk**: Publishing user wallet addresses alongside exact transaction amounts allows off-chain observers to map user financial activities, observe deposit frequency, estimate total wealth, and correlate wallet addresses across dApps.
- **Mitigation**: Events must only emit data essential for off-chain indexing and balance synchronization. User addresses (`Address`) in `topic[1]` are necessary for user-scoped indexing, but no extra personal identifiers or off-chain metadata (such as IP hashes, device IDs, or user names) must ever be included.

### 2. Off-Chain PII & Metadata Exposure
- **Risk**: Including off-chain Personally Identifiable Information (PII), email hashes, mobile phone numbers, or user notes in event payloads creates immutable privacy violations on-chain.
- **Mitigation Invariant**: **Zero PII on-chain**. Smart contract events must NEVER contain PII, user identity references, off-chain transaction reference IDs, or unhashed memos.

### 3. Over-Emitted & Duplicate State Data
- **Risk**: Emitting redundant balance metrics or duplicate event topics inflates transaction WASM execution gas, increases event log bloat, and creates indexer parsing ambiguities.
- **Mitigation Invariant**: **Single emission, minimal payload**. Each contract state transition must emit exactly one canonical event with a minimal data payload.

---

## Minimum Required Payload Guidance

Events should follow a strict **Data Minimization Principle**: include only the parameters required by off-chain indexers and user-facing clients to reconstruct state changes accurately.

### Canonical Event Payload Specification

| Event Symbol | Topic 0 | Topic 1 | Payload Format | Minimum Necessary Payload Data | Justification |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `initialize` | `Symbol("initialize")` | `admin: Address` | `token: Address` | `token` | Required for SDKs to identify the custody token contract. |
| `deposit` | `symbol_short!("deposit")` | `user: Address` | `(amount: i128, new_balance: i128)` | Deposit delta + post-state balance | Enables real-time balance indexers to sync available balance. |
| `withdraw` | `symbol_short!("withdraw")` | `user: Address` | `(amount: i128, new_balance: i128)` | Withdrawal delta + post-state balance | Enables real-time balance indexers to sync available balance. |
| `lock` | `symbol_short!("lock")` | `user: Address` | `(amount: i128, unlock_time: u64, available: i128, locked: i128)` | Lock delta, unlock time, & balance snapshots | Required for mobile apps to present lock countdowns and totals. |
| `withdraw_lock` | `Symbol("withdraw_lock")` | `user: Address` | `(lock_id: u64, amount: i128)` | Lock ID + amount released | Identifies specific lock entry resolved and tokens returned. |
| `pause` | `symbol_short!("pause")` | `admin: Address` | `expiry: u64` | Expiry timestamp | Notifies indexers and users of active pause and auto-expiry time. |
| `unpause` | `symbol_short!("unpause")` | `admin: Address` | `()` | Unit payload (`()`) | Simple state transition notification with zero payload overhead. |
| `xferadmin` | `symbol_short!("xferadmin")` | `old_admin: Address` | `new_admin: Address` | New admin address | Required for SDK/indexer permission tracking. |

---

## Data Boundary Rules

### What MUST Be Emitted
- **Action Type (`topic[0]`)**: Standard symbol identifying the contract function.
- **Primary Actor (`topic[1]`)**: The user or admin address executing the transaction (required for indexer filtering).
- **State Deltas & Snapshots**: Numerical amounts (`i128`), timestamps (`u64`), or identifiers (`u64`) needed for client reconciliation.

### What MUST NEVER Be Emitted
1. **Private Keys, Mnemonic Phrases, or Signatures**: Cryptographic secrets must never enter contract storage or events.
2. **Off-Chain Identity Data**: Names, email addresses, phone numbers, location data, or IP addresses.
3. **Raw Internal Keys or Storage Pointers**: Internal storage layout implementation details.
4. **Duplicate Events**: Emitting multiple events for a single action with different symbol variants.

---

## Compliance & Testing Verification

All event emissions are validated in unit and schema regression tests located at:
- `contracts/savings_vault/src/test/event_schema.rs`
- `contracts/savings_vault/src/test/event_compatibility.rs`

These tests verify that every action emits exactly the canonical topics and minimal payload documented above.
