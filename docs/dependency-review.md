# Dependency Review Policy

This document outlines the repository policy for managing `Cargo.lock` and reviewing dependency changes in the Stellar PocketPay contracts workspace.

---

## Cargo.lock Policy

**The `Cargo.lock` file must be committed and tracked in this repository.**

### Why we commit the lockfile
Soroban smart contracts compile to WebAssembly (`wasm32-unknown-unknown`) binaries. The exact bytecode, build determinism, and WASM binary size directly affect ledger upload costs, transaction footprint, and execution fees.

Committing `Cargo.lock` ensures:
1. **Deterministic Builds**: Every developer, CI runner, and automated build system compiles the contracts using the exact same dependency tree.
2. **Reproducible Bytecode**: The compiled WASM binary matches consistently across environments, preventing silent issues caused by minor/patch version updates in transitive dependencies.
3. **Stable Testing**: Tests run against the identical versions of testing utilities and mock frameworks.

---

## When to Expect Lockfile Changes

Changes to `Cargo.lock` are expected under the following scenarios:
1. **Adding or Removing Dependencies**: When a crate is added or removed from any `Cargo.toml` in the workspace.
2. **Explicit Dependency Upgrades**: When a version requirement in `Cargo.toml` is modified to target a newer version.
3. **Routine Dependency Updates**: When running `cargo update -p <crate>` to pull in minor/patch updates for a specific package, or a workspace-wide `cargo update` for security/maintenance patches.

---

## Reviewing Lockfile Changes

Reviewers and contributors must carefully inspect any modifications to `Cargo.lock`. Unintended lockfile changes can bloat the WASM binary or introduce insecure dependencies.

### Review Guidelines
* **Match with Cargo.toml**: Ensure that all changes in `Cargo.lock` align directly with corresponding edits in a `Cargo.toml` file. If `Cargo.lock` contains updates but no `Cargo.toml` was changed, verify if a deliberate `cargo update` was performed.
* **No Unrelated Crates**: Verify that no unrelated dependencies were updated during the change. Refrain from performing blanket `cargo update` runs in a feature branch unless it is specifically dedicated to dependency maintenance.
* **WASM Size Impact**: Be mindful of dependency additions. If a new dependency is added, compile the release target (`make build-release`) and compare the WASM size against the main branch to ensure it does not exceed the Soroban size limits (or inflate costs unnecessarily).
* **License Compliance**: Ensure that any new dependency (and its transitive dependencies) uses a permissive license compatible with the project's **MIT** license (e.g., MIT, Apache 2.0, BSD).
* **Vulnerability Scanning**: Any new dependencies should be checked for known security advisories. Running `cargo audit` locally is highly recommended before proposing dependency upgrades.

---

## Dependency Review Checklist

Use the following checklist to guide pull request creation and review.

### For Contributors (PR Authors)
- [ ] The `Cargo.lock` changes are strictly necessary for the feature or fix.
- [ ] No unrelated dependencies were modified or updated.
- [ ] Any newly introduced crates are compatible with the MIT license.
- [ ] If a dependency was added, the contract was compiled with `make build-release` and the WASM size increase was verified to be minimal/acceptable.
- [ ] The build compiles successfully and `cargo test` passes.

### For Code Reviewers
- [ ] Verify that `Cargo.lock` diff matches the intent described in the PR.
- [ ] Ensure no transitive dependency updates are unexpected or unrelated to the PR's goal.
- [ ] Confirm that no prohibited or copyleft licenses (e.g. GPL, AGPL) are introduced by the dependency tree changes.
- [ ] Check that build and CI runs succeed.

---

## Navigation

- [Architecture Documentation](architecture.md) – Overview of state management and SDK boundaries.
- [Admin Role](admin-role.md) – Overview of admin storage and privileges.
- [Contributing Guide](../CONTRIBUTING.md) – Standards for formatting, testing, and PR submissions.
