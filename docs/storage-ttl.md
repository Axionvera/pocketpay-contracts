# Storage TTL Review

## Overview

This document summarizes the storage TTL assumptions for the savings vault contract based on the implementation in [contracts/savings_vault/src/lib.rs](../contracts/savings_vault/src/lib.rs). The goal is to document what the repository currently uses and what maintainers should expect, without changing contract behavior.

## Soroban storage TTL and this repository

Soroban storage entries have a TTL that governs how long they remain accessible. The current contract uses two storage categories:

- Instance storage for contract-level configuration and operational state.
- Persistent storage for user balances and lock records.

The contract source does not contain any explicit TTL renewal logic such as `extend_ttl` or `bump` calls. In other words, the repository does not implement application-level renewal for storage entries; any retention behavior is determined by the runtime or deployment environment.

## Current storage entries

| Entry | Storage type | Purpose | TTL sensitivity | Renewal expectation in this repo |
| --- | --- | --- | --- | --- |
| `Admin` | Instance | Stores the contract admin address. | High | No explicit renewal logic in source. |
| `Initialized` | Instance | Tracks whether initialization has completed. | High | No explicit renewal logic in source. |
| `Token` | Instance | Stores the token contract address used for transfers. | High | No explicit renewal logic in source. |
| `StorageVersion` | Instance | Tracks the contract storage layout version. | Medium | No explicit renewal logic in source. |
| `Paused` | Instance | Stores the global pause flag. | Medium | No explicit renewal logic in source. |
| `PauseExpiry` | Instance | Stores the pause expiry timestamp. | Medium | No explicit renewal logic in source. |
| `Balance(Address)` | Persistent | Stores the available balance for a user. | High | No explicit renewal logic in source. |
| `NextLockId(Address)` | Persistent | Stores the next lock ID to assign per user. | Medium | No explicit renewal logic in source. |
| `Lock(Address, u64)` | Persistent | Stores a lock record for a specific user and lock ID. | High | No explicit renewal logic in source. |
| `Locks(Address)` | Persistent key variant | Present in the storage enum, but not exercised by the current public implementation. | Low/unclear | No current read or write path in the contract source. |

## Storage inventory

The storage keys are defined in the `DataKey` enum in [contracts/savings_vault/src/lib.rs](../contracts/savings_vault/src/lib.rs). The persistent entries are used by the balance and lock flows:

- `Balance(Address)` is written on deposit, withdrawal, and lock creation.
- `NextLockId(Address)` is written when a new lock is created.
- `Lock(Address, u64)` is written when a lock is created and updated when that lock is withdrawn.

The instance entries are used by contract initialization, pause handling, admin management, and token configuration.

## TTL considerations

### Persistent balance and lock state

The most important TTL-sensitive state in the repository is the persistent user-state data:

- `Balance(Address)` is read in balance queries and used to compute withdrawal eligibility.
- `Lock(Address, u64)` is read to resolve lock history, paginated lock lists, and lock withdrawal operations.
- `NextLockId(Address)` is used to discover existing locks and assign new lock IDs.

If these persistent entries are no longer accessible, the contract can no longer reconstruct the user’s vault state accurately. In the current implementation, missing persistent values typically fall back to defaults such as `0` or `1`, which can cause incorrect balance or lock behavior.

### Instance configuration and control state

The instance entries are also important because they support core contract operations:

- If `Initialized` is missing or unreadable, the contract may behave as if it has not been initialized.
- If `Admin` or `Token` is missing, administrative checks and token transfer flows can fail.
- If `Paused` or `PauseExpiry` is unavailable, pause handling can no longer be reasoned about reliably.

## Renewal expectations

The current repository does not define a renewal policy for storage entries.

For maintainers, the practical expectation is:

- Instance storage should remain available for as long as the contract instance is expected to remain operational.
- Persistent balances and lock records should remain available for as long as users are expected to access them.
- No application-level TTL renewal is currently performed by the contract source.

## Operational recommendations

These recommendations are operational guidance only; they do not change contract behavior.

1. Treat persistent balances and lock records as the highest-priority TTL-sensitive state.
2. If the contract is deployed in an environment with strict retention limits, verify that balances and lock entries remain readable over long idle periods.
3. If a user reports a missing balance or missing lock, confirm whether storage retention is the cause before investigating contract logic.
4. Keep deployment and runtime retention policy aligned with the expected lifetime of vault state.

## Maintenance notes

- The current implementation uses `persistent()` for balances and lock data, and `instance()` for contract configuration and pause state.
- `Locks(Address)` exists in the storage enum but is not used by the current public implementation, so it should not be treated as an active storage path without confirming later code changes.
- This document is intentionally limited to what is directly evidenced by the repository.

## Future considerations

If future maintainers need stronger guarantees for long-lived vault state, that policy should be added explicitly and documented in a follow-up change. Any such change should be a deliberate storage-management decision rather than an implicit assumption in the contract logic.

## References

- [contracts/savings_vault/src/lib.rs](../contracts/savings_vault/src/lib.rs)
- [README.md](../README.md)
