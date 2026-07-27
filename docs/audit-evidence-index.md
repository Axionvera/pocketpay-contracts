# Audit Evidence Index

This index organizes security-relevant documentation for external audit review. All documents are linked to facilitate verification of contract invariants, storage safety, and threat mitigation.

## Contract Specification

| Document | Description | Status |
|----------|-------------|--------|
| [Contract Specification](contract-specification.md) | Formal specification of the PocketPay Savings Vault contract | ✅ Complete |
| [API Reference](api-reference.md) | Public API documentation with error codes and behavior | ✅ Complete |
| [Storage Layout](storage-layout.md) | Storage key structure and access patterns | ✅ Complete |

## Invariants and Safety Properties

| Document | Description | Status |
|----------|-------------|--------|
| [Invariants](invariants.md) | Balance invariants and ledger consistency properties | ✅ Complete |
| [Error Handling](error-handling.md) | Error codes and exception handling strategy | ✅ Complete |
| [Reentrancy Protection](reentrancy-protection.md) | Reentrancy guards and callback safety | ✅ Complete |

## Storage Safety

| Document | Description | Status |
|----------|-------------|--------|
| [Storage Layout](storage-layout.md) | Storage structure and access control | ✅ Complete |
| [TTL Management](ttl-management.md) | Ledger TTL policy and renewal strategy | ✅ Complete |
| [Ledger Footprint](ledger-footprint.md) | Read/write sets and transaction footprint | ✅ Complete |

## Access Control

| Document | Description | Status |
|----------|-------------|--------|
| [Authorization Model](authorization-model.md) | Soroban auth context and invocation patterns | ✅ Complete |
| [Admin Operations](admin-operations.md) | Administrative functions and privilege boundaries | ✅ Complete |
| [Pause Mechanism](pause-mechanism.md) | Emergency pause and resume controls | ✅ Complete |

## Threat Models

| Document | Description | Status |
|----------|-------------|--------|
| [Threat Model](threat-model.md) | Attack vectors and mitigation strategies | ✅ Complete |
| [Known Limitations](known-limitations.md) | Documented security limitations | ✅ Complete |
| [Security Recommendations](security-recommendations.md) | Operational security guidance | ✅ Complete |

## Test Coverage

| Document | Description | Status |
|----------|-------------|--------|
| [Test Strategy](test-strategy.md) | Testing approach and coverage goals | ✅ Complete |
| [Unit Tests](../contracts/savings-vault/tests/) | Contract unit test suite | ✅ Complete |
| [Property Tests](../contracts/savings-vault/tests/properties/) | Property-based testing for invariants | ✅ Complete |
| [Integration Tests](../integration-tests/) | Cross-contract integration tests | ✅ Complete |

## Deployment and Operations

| Document | Description | Status |
|----------|-------------|--------|
| [Deployment Guide](deployment-guide.md) | Deployment procedures and verification | ✅ Complete |
| [Deployment Checklist](deployment-checklist.md) | Pre-deployment verification steps | ✅ Complete |
| [Upgrade Path](upgrade-path.md) | Contract upgrade strategy and migration | ✅ Complete |
| [Monitoring](monitoring.md) | Operational monitoring and alerting | ✅ Complete |

## External Dependencies

| Document | Description | Status |
|----------|-------------|--------|
| [Dependencies](dependencies.md) | Third-party dependency audit | ✅ Complete |
| [Soroban SDK Version](../Cargo.toml) | SDK version and compatibility notes | ✅ Complete |

## Audit Trail

| Document | Description | Status |
|----------|-------------|--------|
| [Changelog](../CHANGELOG.md) | Version history and security-relevant changes | ✅ Complete |
| [Security Advisories](security-advisories.md) | Published security advisories | ✅ Complete |

## Related Documentation

- [Security Overview](security-overview.md) - High-level security architecture
- [Local Development](local-development.md) - Development environment setup
- [Deployment Environments](deployment-environments.md) - Network configuration details

---

**Last Updated**: 2026-01-27  
**Maintained by**: Core Development Team  
**Review Cadence**: Updated with each release
