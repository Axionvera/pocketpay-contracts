## Description

<!-- Provide a clear summary of the changes and why they are necessary. -->

Closes #<!-- Issue Number -->

## Contribution Quality Gate

By opening this PR, I confirm that my work meets the [Contribution Quality Gate](docs/contribution-quality-gate.md):

### Implementation
- [ ] Logic is fully implemented with no placeholders or `TODO`s.
- [ ] Storage usage follows the [Storage Audit Map](docs/storage-audit.md).
- [ ] Authorization checks (`require_auth`) are correctly applied.

### Testing
- [ ] Unit tests cover both success and failure paths.
- [ ] Accounting changes are verified via [Property Tests](contracts/savings_vault/src/test/property_vault_accounting.rs).
- [ ] Event changes have updated [Snapshots](contracts/savings_vault/test_snapshots/).
- [ ] All tests pass locally (`cargo test`).

### Documentation & CI
- [ ] New behavior is documented in `docs/` and `README.md`.
- [ ] Code is formatted (`cargo fmt`) and linted (`cargo clippy`).
- [ ] WASM build succeeds (`make build-release`).

## Acceptance Criteria Coverage

<!-- Explicitly state how each Acceptance Criterion (AC) from the issue was met. -->
- AC 1: ...
- AC 2: ...

## Security & Risk
<!-- Describe any security-sensitive changes (balances, auth, storage). -->

## Screenshots / Evidence
<!-- Include test output or other evidence of successful implementation. -->
