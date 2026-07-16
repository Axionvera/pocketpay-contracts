# Contributing to Stellar PocketPay Contracts

Thank you for contributing! This guide covers everything you need to know before
opening a pull request against this Soroban smart-contract repository.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Getting Started](#getting-started)
3. [Code Style & Formatting](#code-style--formatting)
4. [Linting](#linting)
5. [Testing](#testing)
6. [Running All Checks Locally](#running-all-checks-locally)
7. [Pull Request Checklist](#pull-request-checklist)
8. [Commit Messages](#commit-messages)

---

## Prerequisites

Make sure you have the following installed before working on the project:

| Tool | Purpose | Install |
|---|---|---|
| Rust (stable) | Compile contracts | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `wasm32-unknown-unknown` target | WASM builds | `rustup target add wasm32-unknown-unknown` |
| `rustfmt` component | Auto-formatting | `rustup component add rustfmt` |
| `clippy` component | Linting | `rustup component add clippy` |
| Soroban CLI | Deploy & invoke | `cargo install --locked soroban-cli` |

All Rust toolchain components (`rustfmt`, `clippy`) are included in the default
`rustup` installation. If you installed Rust via `rustup`, you already have them.

---

## Getting Started

```bash
# Clone the repository
git clone https://github.com/Stellar-PocketPay/pocketpay-contracts.git
cd pocketpay-contracts

# Verify the build compiles cleanly
cargo build --target wasm32-unknown-unknown --release

# Run the test suite
cargo test
```

---

## Code Style & Formatting

This project uses **`rustfmt`** with the default Rust style settings. All
submitted code must be formatted before opening a PR.

### Format the entire workspace

```bash
cargo fmt
```

### Check formatting without modifying files (useful in CI)

```bash
cargo fmt --check
```

If `cargo fmt --check` exits with a non-zero status, re-run `cargo fmt` and
commit the changes.

> Tip: configure your editor to run `rustfmt` on save so you never have to
> think about it. Most Rust-aware editors (VS Code with `rust-analyzer`,
> IntelliJ IDEA, Zed) do this automatically.

---

## Linting

This project uses **`clippy`**, the official Rust linter, to catch common
mistakes and enforce idiomatic Rust patterns.

### Run clippy across the workspace

```bash
cargo clippy -- -D warnings
```

The `-D warnings` flag treats every clippy warning as an error. PRs must pass
this check with zero warnings.

### Clippy for the test suite as well

```bash
cargo clippy --tests -- -D warnings
```

### Common clippy fixes

| Warning | What to do |
|---|---|
| `clippy::unwrap_used` | Prefer `unwrap_or_default()` or an explicit panic message |
| `clippy::clone_on_ref_ptr` | Use `.clone()` only where semantically needed |
| `clippy::pedantic` (informational) | Not enforced, but worth reviewing |

Because this is a `#![no_std]` Soroban contract, some standard-library clippy
lints will not apply. Clippy is smart enough to detect the target automatically.

---

## Testing

Tests live in `contracts/savings_vault/src/test.rs` and run natively (no WASM
needed) using the Soroban SDK test utilities.

### Run all tests

```bash
cargo test
```

### Run a specific test by name

```bash
cargo test test_withdraw
```

### Run tests with output visible (useful for debugging `log!` calls)

```bash
cargo test -- --nocapture
```

### Guidelines for writing tests

- Every new public function must have at least one happy-path test.
- Every validation branch (zero amount, negative amount, insufficient balance,
  etc.) must have a `#[should_panic(expected = "...")]` test whose expected
  message matches the exact panic string in the contract.
- Test names should describe the scenario, not the implementation:
  prefer `test_withdraw_from_empty_balance_panics` over `test_withdraw_err`.
- Use the `setup()` helper in `test.rs` to avoid boilerplate.
- Tests that involve time-sensitive logic (locking, unlock checks) must set
  `env.ledger().with_mut(|li| { li.timestamp = ...; })` explicitly.

---

## Running All Checks Locally

Run this sequence before pushing — it mirrors what a CI pipeline would check:

```bash
# 1. Format check
cargo fmt --check

# 2. Lint (warnings = errors)
cargo clippy --tests -- -D warnings

# 3. Full test suite
cargo test
```

You can also wrap these in a small shell script for convenience:

```bash
#!/usr/bin/env bash
set -e
cargo fmt --check
cargo clippy --tests -- -D warnings
cargo test
echo "All checks passed!"
```

Save it as `scripts/check.sh`, make it executable (`chmod +x scripts/check.sh`),
and run `./scripts/check.sh` before every PR.

---

## Pull Request Checklist

Before requesting a review, confirm that every item below is true:

- [ ] `cargo fmt --check` passes with no diff
- [ ] `cargo clippy --tests -- -D warnings` passes with zero warnings
- [ ] `cargo test` passes — all tests green
- [ ] New or changed behaviour is covered by tests
- [ ] Test names clearly describe the scenario being tested
- [ ] No new `unwrap()` calls without a comment explaining why panicking is safe
- [ ] Commit messages follow the [Conventional Commits](#commit-messages) format
- [ ] PR title is concise (≤ 70 characters)
- [ ] PR description explains **what** changed and **why**

---

## Commit Messages

Use the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <short summary>

[optional body]

[optional footer — e.g. Closes #32]
```

Common types:

| Type | When to use |
|---|---|
| `feat` | A new feature or contract function |
| `fix` | A bug fix |
| `test` | Adding or updating tests only |
| `docs` | Documentation changes only |
| `refactor` | Code change that is neither a fix nor a feature |
| `chore` | Build, CI, or tooling changes |

Examples:

```
feat(savings_vault): add multi-lock support per user

test(savings_vault): add insufficient balance tests for withdraw (closes #26)

docs: add contributing guide with formatting and linting guidance (closes #32)
```

---

## Questions?

Open an issue or start a discussion in the repository. All skill levels are
welcome — this project is intentionally beginner-friendly.
