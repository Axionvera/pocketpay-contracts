# Audit Evidence Index

This index organizes and consolidates all security-relevant documentation for the **PocketPay Savings Vault** contract. It provides an exhaustive map of the repository's threat models, storage schemas, accounting invariants, public APIs, and test coverage to facilitate external security audits.

> **Status**: In preparation for audit. The contract is considered experimental and is currently oriented for Testnet usage.

---

## 1. High-Level Architecture & Threat Models

Documents covering the overarching design, state transitions, and specific threat vectors analyzed.

- [Security Review](SECURITY_REVIEW.md): Architectural analysis of potential vulnerabilities.
- [Architecture](architecture.md): Core component design and interaction flow.
- [State Machine](state-machine.md): Vault lifecycle states and transitions.
- [Admin Pause Threat Model](admin-pause-threat-model.md): Evaluation of attack risks regarding the emergency pause mechanism.
- [Failure Mode Catalogue](failure-mode-catalogue.md): Known edge cases and failure states.
- [Comprehensive Analysis](comprehensive-analysis.md): In-depth review of protocol mechanics.

## 2. Custody & Accounting

Details on how the contract handles user funds, custody assumptions, economic models, and the mathematical invariants that guarantee solvency.

- [Vault Custody Assumptions](vault-custody-assumptions.md): Custody responsibilities and access limits to funds.
- [Token-Backed Withdrawals](token-backed-withdrawals.md): Mechanism ensuring all withdrawals are backed 1:1 by real tokens.
- [Accounting Invariants](accounting-invariants.md): Fundamental mathematical rules (e.g., `total_balance >= locked_balance + withdrawable_balance`).
- [Balance Reconciliation](balance-reconciliation.md): Processes to verify and reconcile balances internally.
- [Vault Fee Model](vault-fee-model.md): Clarification on fee assumptions and accounting implications.
- [Economic Assumptions Review](economic-assumptions-review.md): Consolidated look at fees, custody, and token behavior risk.
- [Amount Normalization](amount-normalization.md): Handling of decimals and token amounts.

## 3. Storage

Documentation on the layout, lifecycle, limitations, and persistence of state on the blockchain.

- [Storage Audit](storage-audit.md): General map of the Soroban storage architecture used in the contract.
- [Vault Storage Audit Map](vault_storage_audit_map.md): Technical detail of the `DataKey` structures used.
- [Multi-Lock Storage](multi-lock-storage.md): Structure for handling multiple time locks per user.
- [Ledger Time Locks](ledger-time-locks.md): Ledger-level mechanisms for time-based locks.
- [TTL Management](storage-ttl.md): Renewal and expiration policies for state entries.
- [Storage Change Checklist](storage-change-checklist.md): Rules for modifying storage structures safely.

## 4. Authorization & Admin Controls

Rules governing who can execute actions in the contract, including the emergency pause model and administrative powers.

- [Authorisation Rules](authorisation-rules.md): Soroban authorization context and invocation patterns.
- [Authorization Boundaries](authorization-boundaries.md): Which operations require user signatures and why.
- [Admin Role](admin-role.md): Explicit capabilities and restrictions of the `admin` role.
- [Admin Rotation Design](admin-rotation-design.md): Mechanism and security considerations for rotating the admin.
- [Pause Design](pause-design.md): Emergency pause and resume controls mechanics.

## 5. Public API, Events & Errors

Detailed specifications for external integrations, SDKs, and off-chain indexers.

- [API Reference](api-reference.md): Public functions, their behavior, and parameters.
- [Contract Events](events.md) & [Vault Events](vault-events.md): Design and purpose of event emission for the Vault.
- [Event Schema](event-schema.md): Structure of the data (`topics` and `data`) emitted to the network.
- [Event Privacy Review](event-privacy-review.md): Analysis of sensitive information leakage via events.
- [Error Codes](error-codes.md): List of all possible errors and their triggering conditions.
- [Error Code Standard](error-code-standard.md) & [SDK Error Mapping Guide](sdk-error-mapping-guide.md): Taxonomies for structured error handling.

## 6. Migration Assumptions & Upgradeability

Strategic approach and technical patterns for contract code upgrades and state migrations.

- [Upgrade Strategy](upgrade-strategy.md): Strategic approach and assumptions for contract code upgrades.
- [Upgradeability](upgradeability.md): Technical limitations and requirements for WASM upgrades.
- [Storage Migration](storage-migration.md): Patterns for handling storage version changes across upgrades.
- [Storage Versioning](storage-versioning.md): How storage versions are tracked and asserted.

## 7. Test Coverage

Evidence that invariants and security rules are enforced in the code through tests.

- [Test Coverage](test-coverage.md): Main document describing the scope of testing.
- [Advanced Development and Testing](advanced-development-and-testing.md): Complex testing scenarios and setups.
- [Testing](testing.md): General testing guidelines.
- [Atomicity](atomicity.md): Overview of atomicity guarantees.
- [Atomicity Tests](../tests/atomicity/): Concrete test cases ensuring that state transitions occur in a single transaction or fail together.
- **Test Matrix**: See the [`tests/README.md`](../tests/README.md) for a complete breakdown of test execution and environment structure.

---

## 8. Access Control & Lifecycle Traceability (Code Evidence)

This section maps the Vault's lifecycle states and explicit evidence in the codebase showing that only authorized roles can trigger state transitions.

- **Initialization**: `initialize(admin, token)` limits are enforced directly in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs).
- **Core Operations**: State transitions for `deposit`, `withdraw`, and `lock` require Soroban `require_auth()` in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs).
- **Emergency Pauses**: Admin boundaries for `pause` are enforced in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs).
- **Upgrades & Migrations**: Addressed via the `try_migrate` function in [`contracts/savings_vault/src/lib.rs`](../contracts/savings_vault/src/lib.rs), documenting version advancement (see Section 9 for known limitations).
- **Deployment & Provisioning**: Traceable in [`scripts/deploy-testnet.sh`](../scripts/deploy-testnet.sh) and network deployment guides.

---

## 9. Known Audit Gaps & Limitations (Value Add)

> [!WARNING]  
> **Transparent Honesty**: The development team acknowledges the following incomplete or immature areas in the current architecture. These must be resolved prior to any Mainnet deployment.

1. **Lack of Multi-Sig Admin (Single Point of Failure)**: The contract currently relies on a single `Admin` address. A compromise of this key would allow an attacker to abuse the pause function or future administrative operations. A multi-sig based access control implementation is required.
2. **Immature Upgrade Mechanism & Proxy Pattern**: The contract has an initial state migration pattern (`try_migrate`), but lacks a robust, proven Smart Contract Upgrade Path, as well as a live-upgrade integration test suite.
3. **Lack of Exhaustive Formal Verification**: Although unit tests exist, the repository has not yet implemented exhaustive formal verification or property-based testing (fuzzing) that mathematically guarantees the absence of overflows and state violations under any condition.
4. **Structured Errors Integration**: Current errors are functional, but a standardized structured error taxonomy required for seamless integrations with frontend clients and mobile SDKs is not fully implemented across all reverting paths.
5. **Pending Third-Party Audit**: This codebase has not been subjected to an exhaustive formal review by an independent smart contract security firm. (See [Audit Readiness](audit-readiness.md) and [Audit Preparation](audit-preparation.md) for current internal review status).
