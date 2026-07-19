# Audit Preparation Checklist — Savings Vault

This document is a checklist of items that should be completed before requesting
or undergoing an external security review or formal audit of the Savings Vault
contract.

> **Note:** This checklist documents what is needed for audit readiness. It does
> **not** mean the contract is currently audit-ready. Many items below are
> incomplete or not yet implemented. Each section notes the current status.

---

## 1. Code Freeze and Scope Definition

Before engaging an auditor, the codebase and scope must be stable.

- [ ] All planned features for the audit scope are fully implemented and merged.
- [ ] A git tag or commit SHA is designated as the audit target (e.g., `v1.0.0-audit`).
- [ ] No unreviewed or untested changes exist in the audit branch.
- [ ] The audit scope is clearly defined: which contracts, which functions, which
      storage keys, and which external integrations (e.g., SAC token contract) are in scope.
- [ ] All TODO/FIXME comments in in-scope code are either resolved or explicitly
      documented as out-of-scope deferred items.

---

## 2. Public API Reference

The contract's external interface must be fully documented before review.

- [ ] Every public function is documented with:
  - [ ] Purpose and behavior description.
  - [ ] All arguments and their types.
  - [ ] Return type and meaning.
  - [ ] All panic / error conditions (including host-level errors from `require_auth`).
  - [ ] Authorization requirements (who must sign).
- [ ] The current functions in scope:
  - [ ] `initialize(admin: Address, token: Address)`
  - [ ] `deposit(user: Address, amount: i128)`
  - [ ] `withdraw(user: Address, amount: i128)`
  - [ ] `get_balance(user: Address) -> i128`
  - [ ] `lock_funds(user: Address, amount: i128, unlock_time: u64) -> u64`
  - [ ] `get_locked_balance(user: Address) -> i128`
  - [ ] `can_withdraw(user: Address) -> bool`
- [ ] A stable, machine-readable error enum (`#[contracterror]`) replaces panic
      strings, or the use of panic strings is explicitly accepted and documented
      as a known limitation. See [error-codes.md](error-codes.md).

**Current status:** Inline Rust doc comments exist. A `#[contracterror]` enum is
not yet implemented; failures currently use panic strings.

---

## 3. Storage Model Documentation

Auditors must understand what is stored on-chain, how long it persists, and what
its access controls are.

- [ ] All storage keys (`DataKey` variants) are documented with:
  - [ ] Key name and type.
  - [ ] Storage tier (persistent vs. instance) and rationale.
  - [ ] Ledger entry TTL behavior and extension strategy.
  - [ ] Who reads and who writes each key.
- [ ] The storage model table is up to date:

  | Key | Type | Storage Tier | Writers | Notes |
  |---|---|---|---|---|
  | `Balance(user)` | `i128` | Persistent | `deposit`, `withdraw` | Available (unlocked) balance |
  | `Locks(user)` | `Vec<LockEntry>` | Persistent | `lock_funds`, `withdraw` | Per-user lock entries |
  | `NextLockId(user)` | `u64` | Persistent | `lock_funds` | Monotonic lock ID counter |
  | `Admin` | `Address` | Instance | `initialize` | Set once; no admin functions today |
  | `Token` | `Address` | Instance | `initialize` | Token contract address for transfers |
  | `Initialized` | `bool` | Instance | `initialize` | One-time init guard |

- [ ] The TTL extension strategy for persistent entries is documented and
      operationally tested. See [storage-ttl.md](storage-ttl.md).
- [ ] Risks of storage expiry (lost balances, inaccessible locks) are documented
      and a mitigation or monitoring plan exists.

**Current status:** Storage keys are documented in [architecture.md](architecture.md)
and [storage-ttl.md](storage-ttl.md). TTL extension commands are provided but no
automated monitoring exists.

---

## 4. Threat Model

A written threat model helps auditors focus their effort on the highest-risk areas.

- [ ] A threat model document exists covering:
  - [ ] Trust boundaries (user, admin, token contract, Soroban host).
  - [ ] Assets at risk (user balances, lock state, admin authority).
  - [ ] Identified attack surfaces (re-initialization, authorization bypass,
        integer overflow/underflow, reentrancy, storage expiry, token contract interaction).
  - [ ] Known mitigations for each threat.
  - [ ] Residual risks that are accepted or deferred.
- [ ] The token integration threat surface is covered:
  - [ ] Behavior when an incompatible or malicious token contract is configured.
  - [ ] Behavior when the token contract's `transfer` call fails.
  - [ ] Accounting consistency if a transfer fails after state is written.
- [ ] Integer arithmetic is reviewed for overflow/underflow (`i128` range for
      balances and `u64` range for timestamps and lock IDs).

**Current status:** No dedicated threat model document exists. Security
considerations are scattered across [README.md](../README.md),
[admin-role.md](admin-role.md), and the Known Limitations section. A consolidated
threat model must be written before audit.

---

## 5. Test Coverage

Tests serve as executable specifications and reduce auditor time spent on basic
correctness checks.

- [ ] Unit tests exist for all public functions covering:
  - [ ] Happy-path (normal successful execution).
  - [ ] All documented panic/error conditions.
  - [ ] Authorization checks (unauthorized callers must be rejected).
  - [ ] Edge cases: zero amounts, amounts equal to balance, balance at maximum
        `i128`, timestamps at the exact unlock boundary.
- [ ] Tests cover multi-lock scenarios: creating multiple lock entries, partial
      maturation, withdrawal consuming matured locks.
- [ ] Test coverage report exists and a minimum threshold (e.g., 80% line
      coverage) is defined and met.
- [ ] Integration or simulation tests exist for the token transfer path in
      `withdraw`.
- [ ] All tests pass against the audit-target commit with no warnings suppressed.

Run the full test suite:

```bash
cargo test
```

**Current status:** Unit tests exist in `contracts/savings_vault/src/test.rs`.
Coverage reporting is not yet configured. Integration tests for the token transfer
path in `withdraw` may be incomplete.

---

## 6. Known Limitations

All known limitations must be documented, accepted, and communicated to the auditor
so they can be assessed rather than re-discovered.

- [ ] Each known limitation is documented with: description, risk impact, and
      whether it is accepted, deferred, or planned for resolution.
- [ ] Current known limitations to document and resolve or accept:

  | Limitation | Risk | Status |
  |---|---|---|
  | Internal accounting only; no real token custody in `deposit` | Deposits do not transfer tokens; recorded balance is not backed by real assets held in contract | Accepted / planned for SAC integration |
  | `withdraw` transfers real tokens but `deposit` does not move tokens | Accounting mismatch; contract may try to transfer more than it holds | Must be resolved before mainnet |
  | Single unlock time per user overwritten by new `lock_funds` call | Old lock context lost; user confusion possible | Known; should be redesigned before audit |
  | No admin recovery mechanism | No way to recover user funds if state is corrupted or lost | Accepted; documented in [admin-role.md](admin-role.md) |
  | No upgrade mechanism | Contract cannot be patched after deployment | Accepted; documented in [upgrade-strategy.md](upgrade-strategy.md) |
  | No pause / emergency stop | Cannot halt operations in an emergency | Accepted; documented in [pause-design.md](pause-design.md) |
  | Error handling via panic strings (no `#[contracterror]` enum) | No stable machine-readable error codes for callers | Should be resolved before audit |
  | Events defined in schema but not emitted | Off-chain indexers cannot observe contract state changes | Should be resolved before audit |
  | No admin functions implemented despite admin address being recorded | Admin role is inert; provides false sense of admin control | Acceptable if documented; confirm with auditor |

- [ ] The distinction between internal accounting and real token custody is
      clearly communicated in all user-facing documentation.

**Current status:** Known limitations are documented in the README. The deposit/withdraw
token-custody asymmetry is a significant issue that must be resolved before any
mainnet or production use.

---

## 7. Deployment and Network Assumptions

Auditors need to understand the deployment context, especially assumptions about
the network, token contracts, and operational environment.

- [ ] The target deployment network is specified (testnet, mainnet, or both).
- [ ] The token contract address to be used in production is identified, reviewed,
      and documented. The risks of using an incorrect or malicious token contract
      address are acknowledged.
- [ ] The admin key management strategy is documented:
  - [ ] Who controls the admin key.
  - [ ] How it is stored and protected.
  - [ ] What happens if the admin key is lost or compromised (see
        [admin-role.md](admin-role.md): currently no recovery path).
- [ ] The initialization process is documented and tested end-to-end on testnet.
      See [deployment-environments.md](deployment-environments.md).
- [ ] Contract ID handoff to SDKs and client applications is documented.
      See [contract-id-handoff.md](contract-id-handoff.md).
- [ ] Storage TTL monitoring and extension procedures are operational.
      See [storage-ttl.md](storage-ttl.md).
- [ ] Network RPC URLs, network passphrases, and environment-specific config
      are documented. See [deployment-environments.md](deployment-environments.md).
- [ ] The deployment script and initialization steps are reviewed for security
      (no secrets in logs, no unvalidated inputs).

**Current status:** Testnet deployment is documented and the deployment script
exists. No mainnet deployment plan exists. Admin key management beyond recording
the address is not yet defined.

---

## 8. Unresolved Design Questions

Open questions must be answered or explicitly deferred before audit so the auditor
can assess finalized design intent.

- [ ] All open design questions are listed, and a resolution or deferral decision
      is recorded for each. Current open questions:

  - **Token custody model:** Will `deposit` be updated to call the SAC `transfer`
    and move real tokens into contract custody before audit? If not, the
    internal-accounting-only model must be fully accepted and documented.
  - **Lock model:** Will multiple concurrent locks per user be supported, or will
    the single-overwrite behavior be kept? The current multi-lock implementation
    should be validated as the intended design.
  - **Admin capabilities:** Will any admin-only functions (pause, recovery,
    upgrade) be added before audit? If yes, they must be in scope.
  - **Error handling:** Will a `#[contracterror]` enum be added before audit?
    Panic strings are not a stable API surface.
  - **Event emission:** Will events be implemented before audit? The schema
    exists in [events.md](events.md) but no events are emitted.
  - **Upgrade path:** Will the contract include an `upgrade()` function before
    audit? If so, the access control and migration strategy must be documented.

---

## 9. Supporting Documentation Summary

Confirm all supporting documents are complete and up to date before requesting an audit.

- [ ] [README.md](../README.md) — Project overview, build, test, deploy instructions.
- [ ] [architecture.md](architecture.md) — Project structure, state model, storage design.
- [ ] [admin-role.md](admin-role.md) — Admin address meaning, current capabilities, future design.
- [ ] [error-codes.md](error-codes.md) — All failure conditions and caller guidance.
- [ ] [events.md](events.md) — Event schema (note: not yet implemented in contract).
- [ ] [upgrade-strategy.md](upgrade-strategy.md) — Upgrade path research and trade-offs.
- [ ] [pause-design.md](pause-design.md) — Emergency pause research and trade-offs.
- [ ] [storage-ttl.md](storage-ttl.md) — Storage TTL behavior and extension guide.
- [ ] [deployment-environments.md](deployment-environments.md) — Environment config for local, testnet, and future mainnet.
- [ ] [contract-id-handoff.md](contract-id-handoff.md) — How to pass deployed contract ID to client SDK.
- [ ] Threat model document — **Does not yet exist; must be created.**
- [ ] `CHANGELOG.md` — All significant changes since initial commit are logged.

---

## Quick Readiness Summary

Use this table for a fast status check. Update it as items are resolved.

| Area | Status |
|---|---|
| Code freeze / audit tag | ❌ Not yet |
| Public API fully documented | ⚠️ Partial (inline docs exist; no stable error enum) |
| Storage model documented | ✅ Documented |
| Threat model written | ❌ Not yet |
| Test coverage meets threshold | ⚠️ Tests exist; no coverage report |
| Known limitations documented | ✅ Documented |
| Token custody model resolved | ❌ Deposit/withdraw asymmetry unresolved |
| Events implemented | ❌ Schema only |
| Deployment config documented | ✅ Testnet documented |
| Admin key management defined | ❌ Not yet |
| All open design questions answered | ❌ Several open |

---

*Last updated: 2026-07-19*
