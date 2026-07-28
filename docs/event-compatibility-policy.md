# Event Backward-Compatibility Policy

## Status

**Authors:** PocketPay Contributors  
**Last updated:** 2026-07-28  
**Applies to:** All Soroban contract events emitted by pocketpay-contracts.

---

## 1. Purpose

Contract events are a public API. SDKs, mobile apps, indexers, and monitoring
tools consume event topics and payloads by their shape. This policy defines
what constitutes a breaking event change, how to deprecate safely, and how
consumers should prepare.

---

## 2. Event Stability Levels

| Level | Meaning | Consumers | Builder |
|-------|---------|-----------|---------|
| **Stable** | Shape is fixed. Breaking changes require 1-release deprecation notice. | Must not break on upgrade. | Semver-major bump + migration guide. |
| **Experimental** | May change without notice. Opt-in for SDK/mobile consumers. | Guard reads behind feature flags. | Use `_experimental` topic prefix. Remove only in minor releases. |
| **Internal** | For tooling only. Not documented for external use. | Ignore unless explicitly consuming for diagnostics. | Prefix topic with `_`. |

---

## 3. Breaking vs Non-Breaking Changes

### Breaking (semver-major)

- Topic key renamed or removed.
- Field removed from event payload.
- Field type changed (e.g. `i128` → `u64`).
- Payload ordering restructured.
- Event removed entirely without deprecation window.

### Non-Breaking (semver-minor or patch)

- New field appended to the end of the payload.
- New event added (consumers that filter by topic are unaffected).
- Field semantics clarified in docs without shape change.
- New topic added alongside existing one.

---

## 4. Deprecation Process

1. **Announce** — Document deprecated event in CHANGELOG with `[DEPRECATED]` tag.
2. **Grace period** — Keep the deprecated event for at least 1 minor release.
3. **Migration** — Provide a before/after table and migration example in docs.
4. **Remove** — Drop the event in the next major release.

### Example CHANGELOG Entry

```
## [2.0.0] - 2026-08-01

### Breaking

- [#459] Removed `VaultEvent::BalanceUpdated` (deprecated since 1.3.0).
  Use `VaultEvent::BalanceChanged` with the same payload shape.
```

---

## 5. Consumer Expectations

### SDK

- SDKs must parse events by topic key, not positional index (where possible).
- Test SDK against both old and new event shapes during the deprecation window.

### Mobile

- Filter events by topic string — unknown topics are safely ignored.
- Never assume event payload length is fixed; pad optional fields.
- Log warnings when consuming a `[DEPRECATED]` event.

---

## 6. Reviewer Checklist

When reviewing a PR that modifies events, check:

- [ ] Are existing topics/fields changed? → **Breaking** → Deprecation applied?
- [ ] Are new topics added with `_experimental` prefix? → OK if experimental.
- [ ] Are deprecated topics still emitted? → Must not be removed before grace period.
- [ ] Is the CHANGELOG updated with the change?
- [ ] README points to this policy?

---

## 7. Current Events Map

_(Maintainers: keep this table in sync with `src/vault.rs` and related event modules.)_

| Topic Key | Stability | Payload | Notes |
|-----------|-----------|---------|-------|
| `VaultEvent::Deposit` | Stable | (from: Address, amount: i128) | |
| `VaultEvent::Withdraw` | Stable | (to: Address, amount: i128) | |
| `VaultEvent::BalanceChanged` | Stable | (account: Address, new_balance: i128) | Replaces `BalanceUpdated` |
| `VaultEvent::AccessGranted` | Experimental | (account: Address, role: u32) | Prefix may stabilise |

---

## 8. References

- [Soroban Events Documentation](https://soroban.stellar.org/docs)
- CONTRIBUTING.md — PR guidelines
- `tests/README.md` — Event test patterns
