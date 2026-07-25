# Dependency Review Checklist

This checklist should be completed before adding or upgrading Rust and Soroban dependencies in the PocketPay contracts. Dependency changes can affect contract size, security, compatibility, and build reliability.

## General Review

- [ ] **Justification documented**: Explain why this dependency is needed or why the upgrade is necessary.
- [ ] **Latest stable version checked**: Verify you are using the latest stable version unless a specific version is required.
- [ ] **Version pinned or specified**: Use exact versions (e.g., `"22.0.0"`) or appropriate ranges, never `"*"`.
- [ ] **Dependency added to workspace**: For cross-contract dependencies, add to `[workspace.dependencies]` in the root `Cargo.toml`.

## Security Review

- [ ] **Security advisories checked**: Review [crates.io](https://crates.io) and [Rustsec Advisory Database](https://rustsec.org/) for known vulnerabilities in the dependency and its transitive dependencies.
- [ ] **Maintenance status verified**: Confirm the crate is actively maintained and receives security updates.
- [ ] **No suspicious permissions**: If the crate requires unusual OS permissions or network access, review its purpose and implementation.
- [ ] **Transitive dependencies acceptable**: Examine transitive dependencies for unnecessary bloat or security concerns.
- [ ] **Cryptographic functions reviewed**: If the dependency provides cryptographic functions, verify it uses well-established, peer-reviewed implementations.

## Licensing

- [ ] **License is compatible**: Verify the dependency's license is compatible with this project's MIT License.
  - Compatible licenses: MIT, Apache 2.0, BSD, ISC, and similar permissive licenses.
  - Incompatible licenses: GPL, AGPL, or other copyleft licenses that require derivative works to be open-sourced under the same license.
- [ ] **License file exists**: Confirm the crate has a LICENSE or LICENSES file in its repository.
- [ ] **No conflicting licenses in transitive dependencies**: Check that transitive dependencies also use compatible licenses.

## Contract Size Impact

- [ ] **WASM artifact size checked**: Build the release WASM and measure the contract size before and after the dependency change.
  ```bash
  cargo build --release --target wasm32-unknown-unknown
  make wasm-size
  ```
- [ ] **Size increase acceptable**: Confirm the WASM size change is acceptable for deployment costs.
  - Document the before/after sizes in the PR description.
  - Note: Soroban contract size affects upload and deployment costs on the Stellar network.
- [ ] **No unnecessary features enabled**: Review the dependency's feature flags and disable unnecessary ones to minimize binary size.

## Build & Test Verification

- [ ] **Local build succeeds**: Run a clean build and verify all targets compile without warnings.
  ```bash
  cargo clean
  cargo build --workspace
  cargo build --release --target wasm32-unknown-unknown
  ```
- [ ] **All tests pass**: Run the full test suite to ensure no regressions.
  ```bash
  cargo test --workspace
  ```
- [ ] **Code formatting compliant**: Check that the code adheres to project standards.
  ```bash
  cargo fmt --check
  ```
- [ ] **Linting passes**: Run clippy with the project's strict settings.
  ```bash
  cargo clippy --tests -- -D warnings
  ```

## Documentation & PR Preparation

- [ ] **Changelog updated**: Add an entry to `CHANGELOG.md` documenting the dependency addition or upgrade.
- [ ] **PR description includes rationale**: Explain the purpose and impact of the dependency change.
- [ ] **WASM size change noted**: Include before/after sizes in the PR description if the change affects the contract artifact.
- [ ] **Security considerations documented**: Flag any security-sensitive dependencies or changes.
- [ ] **Related issues linked**: Reference any issues this dependency resolves or relates to.

## Examples

### Adding a new dependency

Example PR description for adding a security-focused crate:

```
## Summary
Add `serde` (v1.0.x) for serialization support needed by the new state versioning system.

## Changes
- Add `serde` with default features to `Cargo.toml`
- Update contract to use serde derives on state structs

## Testing
- All tests pass: `cargo test --workspace`
- WASM size: 5.2 KiB (before) → 5.5 KiB (after) — acceptable increase
- Security: No known vulnerabilities in v1.0.x or its transitive dependencies
- License: MIT, compatible with this project

## Checklist
- [x] Security advisories checked
- [x] License compatible
- [x] WASM size impact acceptable
- [x] All tests pass
```

### Upgrading a dependency

Example PR description for a version upgrade:

```
## Summary
Upgrade `soroban-sdk` from 21.0.0 to 22.0.0.

## Changes
- Update `soroban-sdk` version in workspace `Cargo.toml`
- Update contract invocation examples to reflect API changes (if any)

## Testing
- All tests pass: `cargo test --workspace`
- WASM size: No significant change (5.7 KiB before, 5.7 KiB after)
- Compatibility: Reviewed breaking changes; no contract logic updates needed
- Security: No vulnerabilities reported for v22.0.0

## Breaking Changes
None — this is a minor version bump with backward-compatible APIs.

## Checklist
- [x] Latest stable version verified
- [x] Security advisories checked
- [x] License compatible
- [x] All tests pass
- [x] No breaking changes to contract interface
```

## When to Use This Checklist

Use this checklist whenever a PR includes:

- Adding a new dependency to `Cargo.toml` or workspace dependencies
- Upgrading an existing dependency to a newer version
- Changing feature flags on existing dependencies
- Removing a dependency (though removal is less risky)

## Additional Resources

- [Soroban documentation](https://soroban.stellar.org/)
- [Rustsec Advisory Database](https://rustsec.org/) — Check for known vulnerabilities
- [crates.io](https://crates.io/) — Review crate metadata, versions, and dependencies
- [SPDX License List](https://spdx.org/licenses/) — Verify license compatibility
- [Cargo documentation](https://doc.rust-lang.org/cargo/) — Cargo features, dependencies, and workspaces
- [Dependency Resolution](https://doc.rust-lang.org/cargo/guide/dependencies.html) — How Cargo resolves versions

## Integration with Contributing Guidelines

Contributors must complete this checklist before opening a pull request that modifies dependencies. See [CONTRIBUTING.md](../CONTRIBUTING.md) for additional requirements and the PR submission process.

Link to this checklist in the PR description and check off each item that applies. Ensure every checked item includes supporting evidence (security review notes, build logs, size measurements, etc.).
