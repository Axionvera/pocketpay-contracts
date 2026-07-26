# Contributing to PocketPay Contracts

Thank you for contributing to PocketPay Contracts. This repository contains Soroban smart contracts written in Rust. Keep contributions focused and add tests whenever contract behavior changes.

## Prerequisites

Install these tools before working on the project:

- [Git](https://git-scm.com/) for version control.
- [Rust](https://www.rust-lang.org/tools/install) through `rustup`, including `rustc` and Cargo.
- The `wasm32-unknown-unknown` target used to compile this repository's contracts.
- The Soroban CLI used by this repository:

  ```bash
  cargo install --locked soroban-cli
  ```

Verify the Rust tools and install the WASM target:

```bash
rustup --version
rustc --version
cargo --version
rustup target add wasm32-unknown-unknown
```

The repository does not currently require `wasm32v1-none`. If its toolchain changes to use that target, install it with:

```bash
rustup target add wasm32v1-none
```

## Local setup

1. Fork `Axionvera/pocketpay-contracts` on GitHub.
2. Clone your fork:

   ```bash
   git clone https://github.com/YOUR-USERNAME/pocketpay-contracts.git
   cd pocketpay-contracts
   ```

3. Add and fetch the original repository as `upstream`:

   ```bash
   git remote add upstream https://github.com/Axionvera/pocketpay-contracts.git
   git fetch upstream
   ```

4. Create a feature branch from the latest upstream branch:

   ```bash
   git switch main
   git pull --ff-only upstream main
   git switch -c your-feature-name
   ```

## Build, format, and test

Check formatting:

```bash
cargo fmt --check
```

Run the full workspace test suite:

```bash
cargo test --workspace
```

Build the optimized contract WASM with the command used by this repository's CI workflow:

```bash
cargo build --release --target wasm32-unknown-unknown
```

The artifact is written under `target/wasm32-unknown-unknown/release/`. Run all three commands before opening a pull request. Logic changes must include tests for the changed behavior and relevant failure and edge cases.

Follow the [test naming convention](docs/testing.md) when adding or updating tests under `contracts/savings_vault/src/test/`.

## Pull request expectations

- Keep each pull request focused on one issue or related change.
- Reference the issue number, for example `Closes #3`.
- Explain what changed and why.
- Include test notes listing the commands run and their results.
- Call out contract behavior, storage, authorization, or interface changes.
- Avoid changing contract logic in documentation-only pull requests.
- Add or update tests for every logic change.
- When changing storage layout, follow the storage change checklist in `docs/storage-change-checklist.md`.
- When adding or upgrading dependencies, follow the dependency review checklist in `docs/dependency-review.md`.

## License

This project is licensed under the MIT License; see the [LICENSE](LICENSE) file at
the repository root for the full text. That repository-level file is the single
source of truth for licensing — **individual source files should not include
their own license headers**. Do not add license headers to new or existing
`.rs` files or other source files; keep new contributions consistent with the
current codebase, which has no per-file headers.

If you believe a specific file needs its own header (for example, code
adapted from another project under a different license), raise it with the
maintainers in your pull request description before adding one.

## Documentation style

When writing or editing the README, this file, or anything under `docs/`,
follow the [Documentation Style Guide](docs/docs-style-guide.md) for Testnet
wording, avoiding production claims, placeholder values, command formatting,
and linking between docs.

## Security-sensitive contributions

Changes involving balances, access control, signatures, storage, upgrades, or external calls are security-sensitive. Describe their risks and assumptions clearly in the pull request.

- Do not log secrets or sensitive credentials.
- Never commit private keys, seed phrases, RPC keys, wallet secrets, or populated secret configuration files.
- Use test accounts and non-sensitive placeholders in examples and tests.
- Keep documentation-only changes separate from contract logic changes.
- Add tests for logic changes, especially authorization failures, boundary conditions, and invalid inputs.
- Report vulnerabilities privately to the maintainers rather than publishing exploitable details in a public issue.

Before pushing, review the staged diff for credentials and unrelated files.
- Review the [security checklist](docs/security-checklist.md) for auth, storage, token transfer, locks, and admin behavior.
