# Contract Release Checklist

Repeatable checklist for maintainers cutting a release of the `savings_vault`
contract. Work through every section before tagging a version or redeploying to
testnet. This contract is an educational / testnet project — nothing in this
checklist implies mainnet readiness. See the README's
[Release Readiness](../README.md#release-readiness) table for the current
maturity posture.

Copy the checklist into the release pull request description and tick each item
with evidence (command output, file path, or a short "N/A because …" note).

## How to use this document

- Run it for **every** change to `contracts/savings_vault/src/lib.rs`, not only
  for version bumps. Storage, event, and error changes are the ones that break
  downstream consumers silently.
- Sections 2, 3 and 4 define the **compatibility contract** with the SDK and the
  mobile app. Any "no" answer there means the change is breaking and must be
  coordinated across repositories before merge.
- This checklist complements, and does not replace, the narrower checklists
  already in the repo:
  [storage change](storage-change-checklist.md),
  [security review](security-checklist.md),
  [dependency review](dependency-review.md), and
  [audit preparation](audit-preparation.md).

---

## 1. Tests

There is **no CI workflow that builds or tests this repository**. The only
workflow, `.github/workflows/trigger-auto-merge.yml`, dispatches an event to the
automation repo and runs neither `cargo test` nor the WASM build. Every item
below must therefore be run locally and its output pasted into the release PR.

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --tests -- -D warnings` passes.
- [ ] `cargo test --workspace` passes, with the summary line (test counts)
      recorded in the PR.
- [ ] `cargo build --release --target wasm32-unknown-unknown` succeeds, or
      `make build-release`, which also prints the WASM size.
- [ ] The reported WASM size is compared against the previous release and any
      unexpected growth is explained (`make wasm-size`,
      `scripts/report-wasm-size.sh`).
- [ ] Every new or changed behaviour has a test under
      `contracts/savings_vault/src/test/`, following the naming rules in
      [testing.md](testing.md).
- [ ] **Every new test file is declared in
      `contracts/savings_vault/src/test/mod.rs`.** A test file that is not
      declared there is never compiled and silently provides no coverage. At the
      time of writing, `amount_normalization.rs`, `cross_user_isolation.rs`,
      `event_schema.rs`, `lock_summary.rs`, and `total_vault_balance.rs` exist on
      disk but are **not** declared, so they do not run — confirm the release does
      not depend on coverage that is not actually executing.
- [ ] Snapshot changes under `contracts/savings_vault/test_snapshots/` are
      reviewed deliberately, not accepted blindly; a diff there means observable
      contract behaviour (events, auth, storage footprint) changed.
- [ ] Property-test regressions in
      `contracts/savings_vault/proptest-regressions/` are still green, and any
      newly recorded failing case is either fixed or documented.
- [ ] The invariant suites still pass and still assert what they claim:
      `balance_conservation.rs`, `property_vault_accounting.rs`,
      `property_fee_invariants.rs`, `withdrawal_invariant.rs`,
      `admin_invariant_guard.rs`.
- [ ] Coverage claims in [test-coverage.md](test-coverage.md) and
      `tests/README.md` still match reality, including their references to test
      files (see the undeclared-module note above).
- [ ] The TypeScript integration test `tests/atomicity/transfer-atomicity.test.ts`
      is either run and reported, or explicitly marked as not run — the repo
      currently ships no `package.json` or runner configuration for it.

## 2. Storage compatibility

Storage layout is the hardest thing to fix after deployment. The contract is
**not upgradeable** ([upgradeability.md](upgradeability.md)), so a layout change
means a fresh deployment and a state migration story for users.

- [ ] Diff `DataKey` in `contracts/savings_vault/src/lib.rs` against the previous
      release. Confirm no variant was **removed, reordered, or retyped** — this
      changes the serialized key and orphans existing entries.
- [ ] New variants are appended at the end of the enum, never inserted between
      existing ones.
- [ ] The instance/persistent split is unchanged and still intentional:
      - **Instance:** `Admin`, `Initialized`, `Token`, `StorageVersion`,
        `Paused`, `PauseExpiry`, `MinDepositAmount`, `MaxLockDurationSecs`,
        `MinLockDurationSecs`.
      - **Persistent:** `Balance(Address)`, `Locks(Address)`,
        `Lock(Address, u64)`, `NextLockId(Address)`.
- [ ] Any change to the shape of a stored value (`LockEntry`, `BalanceSnapshot`,
      `LockSummary`) is treated as a breaking storage change, not a refactor.
- [ ] `STORAGE_VERSION` in `lib.rs` is bumped if and only if the layout changed,
      and `try_migrate` gains an explicit arm for the new
      `previous_version -> STORAGE_VERSION` step.
- [ ] Downgrade protection still holds: a stored version greater than the
      compiled `STORAGE_VERSION` must return `StorageVersionUnsupported` (6001)
      rather than silently proceeding.
- [ ] `contracts/savings_vault/src/test/storage_version.rs` covers the new
      migration path, including the legacy "no version marker" (v0) case.
- [ ] [storage-change-checklist.md](storage-change-checklist.md) is completed and
      linked from the PR.
- [ ] [storage-audit.md](storage-audit.md), [storage-migration.md](storage-migration.md),
      and [storage-versioning.md](storage-versioning.md) are updated to match the
      new layout.
- [ ] TTL impact is reviewed. The contract makes **no `extend_ttl` calls**, so
      every persistent entry depends on manual, operational renewal — confirm the
      release does not add persistent entries whose expiry would destroy user
      funds or lock records. See [storage-ttl.md](storage-ttl.md).

## 3. Event compatibility

Events are consumed by the SDK, the mobile client, and any indexer. Per
[events.md](events.md), changing topic order, field types, field count, or the
event name is **breaking**; adding a new event type is not.

The events emitted at the time of writing, all published from
`contracts/savings_vault/src/lib.rs`:

| Event | Topic 0 form | Data payload |
| --- | --- | --- |
| `initialize` | `Symbol::new` | `token: Address` |
| `deposit` | `symbol_short!` | `(amount, new_balance)` |
| `withdraw` | `symbol_short!` | `(amount, new_balance)` |
| `withdraw_lock` | `Symbol::new` | `(lock_id, amount)` |
| `lock` | `symbol_short!` | `(amount, unlock_time, available, locked)` |
| `extend_lock` | `Symbol::new` | `(lock_id, old_unlock_time, new_unlock_time, amount)` |
| `pause` | `symbol_short!` | `expiry` |
| `unpause` | `symbol_short!` | `()` |
| `cfg_min` | `symbol_short!` | `min_amount` |
| `cfg_maxlk` | `symbol_short!` | `max_duration_secs` |
| `cfg_minlk` | `symbol_short!` | `min_duration_secs` |
| `xferadmin` | `symbol_short!` | `new_admin: Address` |

- [ ] Every `env.events().publish(...)` site in `lib.rs` is reviewed against the
      previous release, using the table above as the baseline.
- [ ] Topic 1 is still the subject address (user, admin, or old admin) for every
      event, so consumers can keep filtering by address.
- [ ] Any new event name of 10 characters or more uses `Symbol::new`, not
      `symbol_short!` (which is limited to 9 characters).
- [ ] Event **count per call** is unchanged, or the change is documented — a
      duplicate or dropped emission breaks indexers even when field shapes match.
- [ ] `contracts/savings_vault/src/test/event_compatibility.rs` and
      `event_ordering.rs` are updated and passing.
- [ ] Event documentation is updated **in every place it lives**. The repo
      currently carries three overlapping event documents:
      [event-schema.md](event-schema.md) (most complete),
      [events.md](events.md), and [vault-events.md](vault-events.md). Confirm the
      release does not widen the drift between them, and note that the
      admin-config events (`cfg_min`, `cfg_maxlk`, `cfg_minlk`) are currently
      undocumented in all three.
- [ ] New or changed payload fields are reviewed against
      [event-privacy-review.md](event-privacy-review.md) — no new data is exposed
      beyond what indexing and mobile display actually require.
- [ ] Diagnostic `log!` output is still treated as debug-only and is not relied
      on as part of the SDK contract.

## 4. Error compatibility

`ContractError` in `lib.rs` is a `#[contracterror] #[repr(u32)]` enum, and
[error-codes.md](error-codes.md) declares the numeric codes to be part of the
cross-repo SDK interface.

- [ ] No existing variant was renumbered, removed, or reused for a different
      meaning. Renumbering is breaking and requires a coordinated SDK and mobile
      release.
- [ ] New variants are allocated inside the correct category range and use a gap
      in that range rather than shifting existing codes:
      1000s validation, 2000s authorisation, 3000s lifecycle, 4000s accounting,
      5000s locks, 6000s storage, 7000s token, 8000s admin rotation.
- [ ] Each new variant has a doc comment on the enum saying which function raises
      it and under what condition.
- [ ] [error-codes.md](error-codes.md) is updated with meaning, likely cause, and
      caller action for every new or changed code.
- [ ] [sdk-error-mapping-guide.md](sdk-error-mapping-guide.md) is updated so the
      SDK and mobile have user-facing copy for the new code.
- [ ] [failure-mode-catalogue.md](failure-mode-catalogue.md) covers the new
      failure mode and points at the test that proves it.
- [ ] `contracts/savings_vault/src/test/contract_error_codes.rs` asserts the exact
      numeric code, not just that the call failed.
- [ ] Failed calls still leave no partial state: token transfer and vault
      accounting either both apply or both roll back
      (`token_transfer_rollback.rs`, `lock_atomicity.rs`,
      [atomicity.md](atomicity.md)).

## 5. Documentation

- [ ] `CHANGELOG.md` — the `Unreleased` section is filled in under the right
      headings (Added / Changed / Fixed / Security) and, for a tagged release,
      promoted to a version heading. Storage, event, and error changes are called
      out explicitly.
- [ ] Version is bumped **in both places** and they match:
      `version` in `contracts/savings_vault/Cargo.toml`, and the string literal
      returned by `get_version` in `contracts/savings_vault/src/lib.rs`. The
      `test_get_version` test in `contracts/savings_vault/src/test/mod.rs` is the
      safety net — confirm it ran. See [version-metadata.md](version-metadata.md).
- [ ] README is updated where the change touches it: the function table, the
      [Release Readiness](../README.md#release-readiness) table, Known
      Limitations, and the documentation index. New docs are added to the index
      per [docs-style-guide.md](docs-style-guide.md).
- [ ] Known Limitations in the README still describe the contract as it is today
      (upgrade path, admin model, audit status, TTL dependency).
- [ ] Behavioural docs that mirror contract logic are re-read, not assumed:
      [architecture.md](architecture.md), [state-machine.md](state-machine.md),
      [api-reference.md](api-reference.md),
      [accounting-invariants.md](accounting-invariants.md),
      [authorisation-rules.md](authorisation-rules.md).
- [ ] CLI examples still reflect the real signatures:
      [invocation-examples.md](invocation-examples.md),
      [state-changing-invocations.md](state-changing-invocations.md),
      [cli-smoke-test.md](cli-smoke-test.md), [walkthrough.md](walkthrough.md).
- [ ] Wording follows [docs-style-guide.md](docs-style-guide.md): testnet only,
      no production or audited claims, `UPPER_SNAKE_CASE` placeholders, no real
      contract IDs or secrets.
- [ ] Documentation-only changes are kept in separate pull requests from contract
      logic changes, per `CONTRIBUTING.md`.

## 6. SDK compatibility

The SDK lives outside this repository; the compatibility surface is defined by
the contract's public functions plus the documents below. Treat every item as a
question to answer *before* the release, not after the SDK breaks.

- [ ] The public function surface in `lib.rs` is diffed against the previous
      release: no removed function, no renamed parameter, no changed argument
      order or return type without a version bump and an SDK ticket.
- [ ] Returned struct shapes are unchanged, or the change is announced:
      `LockEntry`, `BalanceSnapshot`, `LockSummary`.
- [ ] Pagination behaviour is unchanged: `list_locks` and `list_matured_locks`
      still clamp to `MAX_LOCK_PAGE_SIZE` (50) and still handle out-of-range
      offsets the way [lock-read-helpers.md](lock-read-helpers.md) and
      [matured-lock-discovery.md](matured-lock-discovery.md) describe.
- [ ] Read-only helpers remain read-only and callable without a signature, so
      SDKs can keep simulating them speculatively. Re-check against
      [simulation-compatibility.md](simulation-compatibility.md) and
      `contracts/savings_vault/src/test/simulation_compatibility.rs`.
- [ ] Which calls require `require_auth()` is unchanged, or
      [simulation-compatibility.md](simulation-compatibility.md) and
      [sdk-contract-sequence.md](sdk-contract-sequence.md) are updated.
- [ ] `get_version` returns the new version, so SDKs performing a version
      handshake see the bump.
- [ ] Error-code mapping is in sync (see section 4) — an unmapped code surfaces
      to users as a raw number.
- [ ] Read-model expectations still hold: [read-models.md](read-models.md).
- [ ] The new contract ID is handed off through the documented path only —
      `deployment output -> VAULT_CONTRACT_ID -> SDK configuration -> mobile app`
      — per [contract-id-handoff.md](contract-id-handoff.md). Never commit
      deployment identities or secret keys.
- [ ] A breaking change has an SDK-side issue opened and linked from the release
      PR before merge.

## 7. Mobile compatibility

The mobile app consumes the contract through the SDK, so most breakage reaches
it indirectly. These items cover what is mobile-specific.

- [ ] The mobile app receives the contract ID via SDK configuration only, not
      hardcoded in mobile source ([contract-id-handoff.md](contract-id-handoff.md)).
- [ ] The network the ID belongs to is stated in the handoff — a valid contract
      ID for the wrong network fails confusingly at runtime.
- [ ] Balance display still works from a single call: `get_balance_snapshot`
      (`unlocked` / `locked` / `total` / `withdrawable`) and `get_lock_summary`
      still return the fields the UI binds to.
- [ ] Lock countdown and maturity UI still work: `can_withdraw`, `unlock_time`
      semantics, and ledger-timestamp assumptions are unchanged
      ([ledger-time-locks.md](ledger-time-locks.md)).
- [ ] Every new or changed error code has user-facing copy, so the app never
      shows a bare numeric code
      ([sdk-error-mapping-guide.md](sdk-error-mapping-guide.md)).
- [ ] Pause behaviour is unchanged and still communicable in the UI: `deposit`
      and `lock_funds` blocked, `withdraw` and `withdraw_lock` always available,
      auto-expiry after `duration_secs` ([pause-design.md](pause-design.md)).
- [ ] Amount handling assumptions are unchanged — `i128` in the asset's smallest
      unit, never floats ([amount-normalization.md](amount-normalization.md)).
- [ ] Any event payload the app or its indexer decodes is still shaped as before
      (see section 3).
- [ ] Admin-only functions remain clearly separated from user-facing ones, so
      nothing admin-gated leaks into the user UI
      ([admin-role.md](admin-role.md), [authorization-boundaries.md](authorization-boundaries.md)).

## 8. Audit and security notes

- [ ] [security-checklist.md](security-checklist.md) is completed for the change
      (auth, storage, token transfer, locks, admin behaviour).
- [ ] Every state-changing function still calls `require_auth()` on the right
      address, and admin-gated functions still call `assert_admin`
      ([authorisation-rules.md](authorisation-rules.md),
      `unauthorized_access.rs`).
- [ ] Accounting invariants still hold under the change
      ([accounting-invariants.md](accounting-invariants.md),
      [balance-reconciliation.md](balance-reconciliation.md)).
- [ ] Admin-power changes are re-assessed against
      [admin-pause-threat-model.md](admin-pause-threat-model.md) and
      [admin-rotation-design.md](admin-rotation-design.md); the single-key admin
      limitation is still disclosed.
- [ ] Custody and economic assumptions are unchanged, or updated:
      [vault-custody-assumptions.md](vault-custody-assumptions.md),
      [vault-fee-model.md](vault-fee-model.md),
      [economic-assumptions-review.md](economic-assumptions-review.md).
- [ ] Dependency changes went through [dependency-review.md](dependency-review.md);
      `Cargo.lock` diff is reviewed and `soroban-sdk` version changes are called
      out in the changelog.
- [ ] Audit-facing documents are refreshed so an external reviewer is not sent to
      stale evidence: [audit-evidence-index.md](audit-evidence-index.md),
      [audit-preparation.md](audit-preparation.md),
      [audit-readiness.md](audit-readiness.md), `tests/README.md`.
- [ ] Newly discovered gaps are written down as known limitations rather than
      left implicit — in the README and in
      [audit-readiness.md](audit-readiness.md).
- [ ] No secrets, keys, seed phrases, or populated `.env` files are in the diff;
      examples use placeholders only.
- [ ] The release notes state plainly that the contract is unaudited,
      non-upgradeable, and testnet-only.

## 9. Deployment verification (testnet)

- [ ] Deploy with `./scripts/deploy-testnet.sh YOUR_IDENTITY` and record the
      returned contract ID.
- [ ] Initialize with the intended admin and SAC token address, and confirm a
      second `initialize` call fails with `AlreadyInitialized` (3001).
- [ ] `get_version` on the deployed contract returns the version being released.
- [ ] Run the [CLI smoke test](cli-smoke-test.md) end to end and confirm every
      function responds as documented.
- [ ] Record the deployed contract ID, network, WASM size, and version in the
      release notes, and hand the ID off per section 6.
- [ ] Schedule TTL extension for persistent entries; the contract will not do it
      for you ([storage-ttl.md](storage-ttl.md)).

---

## Related checklists

- [Storage Change Checklist](storage-change-checklist.md)
- [Security Review Checklist](security-checklist.md)
- [Dependency Review Checklist](dependency-review.md)
- [Audit Preparation Checklist](audit-preparation.md)
- [Contract ID Handoff](contract-id-handoff.md)
- [CLI Smoke Test Guide](cli-smoke-test.md)
