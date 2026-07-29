# Stellar PocketPay — Savings Vault Contract

[CHANGELOG](CHANGELOG.md)
## Security Considerations

> **This contract is for educational and testnet use.** Review the following before any mainnet deployment.

See the [Admin Role](docs/admin-role.md) document for details on what the `initialize(admin)` value records, what the admin can and cannot do today, and future admin design considerations.

See the [Admin & Emergency Mechanism Threat Model](docs/admin-pause-threat-model.md) for a security analysis of malicious admin, compromised admin, accidental pause, and blocked-withdrawal scenarios, including mitigations and trust assumptions.

## Features

| Function | Description |
|---|---|
| `initialize(admin)` | One-time setup; records the admin address |
| `deposit(user, amount)` | Add funds to a user's vault |
| `withdraw(user, amount)` | Remove funds from a user's vault |
| `get_balance(user)` | Query available (unlocked) balance |
| `lock_funds(user, amount, unlock_time)` | Lock funds until a Unix timestamp |
| `get_locked_balance(user)` | Query locked balance |
| `can_withdraw(user)` | Check if locked funds are withdrawable |

---

## Prerequisites

Install the following before you begin:

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Soroban CLI**
   ```bash
   cargo install --locked soroban-cli
   ```

3. **WASM target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

---

## Build

Compile the contract to a WASM binary:

```bash
# Debug build
cargo build --target wasm32-unknown-unknown

# Optimized release build (recommended for deployment)
cargo build --target wasm32-unknown-unknown --release

# Optimized release build with an immediate WASM size report
make build-release
```

The compiled `.wasm` file will be at:
```
target/wasm32-unknown-unknown/release/savings_vault.wasm
```

### Contract size report

Soroban contract size affects upload and deployment costs and can reveal unexpected binary growth. Use the release wrapper above to build and print the artifact size in both human-readable units and exact bytes:

```text
WASM artifact: target/wasm32-unknown-unknown/release/savings_vault.wasm
WASM size: 5.73 KiB (5871 bytes)
```

To report the size of an existing release artifact without rebuilding it, run:

```bash
make wasm-size
```

The reporting command exits with an error and identifies the expected path when the WASM file is missing. CI pipelines should run `make build-release` (or `make wasm-size` after their release build) so contract-size changes remain visible in build logs.

---

## Test

Run the full unit test suite:

```bash
cargo test
```

All tests run natively (no WASM needed) using the Soroban SDK test utilities.

---

## Local verification

Before opening a pull request, run the single local verification command:

```bash
make verify
```

This runs the same checks contributors are asked to confirm in the PR template:

1. `cargo fmt --check`
2. `cargo clippy --tests -- -D warnings`
3. `cargo test --workspace`
4. `cargo build --release --target wasm32-unknown-unknown`

See [CONTRIBUTING.md](CONTRIBUTING.md#build-format-and-test) for details and for running each step individually.

## Task Runner

Common tasks are available via `make`:

```bash
make verify         # Format, lint, test, and release WASM build
make build-release  # Optimized WASM build with size report
make wasm-size      # Report size of an existing release WASM
```
---

## Deploy to Testnet

### 1. Configure the Stellar Testnet

```bash
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

### 2. Create & Fund an Identity

```bash
soroban keys generate --global deployer --network testnet
soroban keys address deployer
```

Fund the account at [Stellar Friendbot](https://friendbot.stellar.org/?addr=YOUR_ADDRESS).

### 3. Deploy the Contract

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source deployer \
  --network testnet
```

Save the returned **Contract ID** — you'll need it to invoke functions.

### 4. Initialize the Contract

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin deployer
```

### 5. Invoke Functions

```bash
# Deposit 1000 units
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  deposit \
  --user deployer \
  --amount 1000

# Check balance
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  get_balance \
  --user deployer
```

---

## Project Structure

```
stellar-pocketpay-contracts/
├── Cargo.toml                          # Workspace root
├── .gitignore
├── README.md
└── contracts/
    └── savings_vault/
        ├── Cargo.toml                  # Contract crate
        └── src/
            ├── lib.rs                  # Contract implementation
            └── test.rs                 # Unit tests
```

---
## Documentation

- [Architecture Documentation](docs/architecture.md) – Overview of project structure, state management, storage, SDK integration, and future extension points.
- [Admin Role](docs/admin-role.md) – Details on the admin address, current capabilities, and future design considerations.
- [Admin & Emergency Mechanism Threat Model](docs/admin-pause-threat-model.md) – Security analysis of malicious admin, compromised admin, accidental pause, and blocked-withdrawal scenarios.
- [Failure Mode Catalogue](docs/failure-mode-catalogue.md) – Summary of safe-failure behavior, expected errors, affected functions, and related tests for vault operations.
- [Contributor Self-Review Template](docs/self-review-template.md) – Copy-paste checklist covering behaviour, tests, CI, security, edge cases, and docs impact — fill it in before opening a PR.
- [Traceability Table Guide](docs/traceability-table.md) – Standard format for mapping PR changes to issue acceptance criteria, with worked examples.
- [Payment-Period Conduct Guidance](docs/payment-period-conduct.md) – Expectations for how contributors raise payment-status questions, and how GrantFox's evaluation process relates to this repository's review process.

---

## Security Considerations

> **This contract is for educational and testnet use.** Review the following before any mainnet deployment.

### Authorization
- Every state-changing function calls `require_auth()` on the user's address.
- Only the signing user can deposit, withdraw, or lock their own funds.

### Input Validation
- Zero and negative amounts are rejected for deposits, withdrawals, and locks.
- Withdrawals exceeding the available balance are rejected.
- Lock amounts exceeding the available balance are rejected.
- Unlock times in the past are rejected.

### Re-initialization Protection
- `initialize()` can only be called once; subsequent calls panic.

### Storage Design
- User balances are stored in **persistent** storage (survives ledger expiry longer).
- Admin and initialization flags use **instance** storage (tied to contract lifetime).

### Known Limitations
- **No real token transfers**: This contract tracks balances internally but does not yet integrate with the Stellar Asset Contract (SAC) for actual XLM/token transfers. A production version should call the token contract's `transfer()` function.
- **Single unlock time**: Locking funds multiple times overwrites the previous unlock timestamp. A production version might use per-lock entries.
- **No admin recovery**: There is no mechanism for the admin to recover or migrate funds.
- **No upgrade mechanism**: The contract does not implement `upgrade()`. Consider adding this for mainnet.

---

## Deployment Notes

- **Testnet RPC**: `https://soroban-testnet.stellar.org:443`
- **Network passphrase**: `Test SDF Network ; September 2015`
- **Friendbot** (free testnet XLM): `https://friendbot.stellar.org`
- **Soroban Explorer**: [stellar.expert](https://stellar.expert/explorer/testnet)
- Always test thoroughly on testnet before considering mainnet deployment.
- Monitor contract storage TTL and extend as needed using `soroban contract extend`.

---

## Contributing

Contributions are welcome! This project is intentionally beginner-friendly.

See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full guide, including:

- How to run local verification (`make verify`)
- How to format code (`cargo fmt`)
- How to lint code (`cargo clippy --tests -- -D warnings`)
- How to run the test suite (`cargo test --workspace`)
- PR checklist and commit message conventions

Before opening a PR, fill in the **[Contributor Self-Review Template](docs/self-review-template.md)** — a checklist covering behaviour, tests, CI, security, edge cases, and docs impact — so requirement gaps are caught before a reviewer looks at the PR.

Every pull request must use the **[PR template](.github/PULL_REQUEST_TEMPLATE.md)**, which requires:

- A reference to the issue being fixed (`Closes #N`)
- A list of contract functions added, modified, or removed
- **Test evidence** — description of tests added, no-test justification, and references to existing contract test patterns
- A **[traceability table](docs/traceability-table.md)** mapping each acceptance criterion to changed functions, tests, and edge cases
- A security considerations section (with checklist for contract changes)
- Confirmation that `make verify` passes (or equivalently `cargo fmt --check`, `cargo clippy --tests -- -D warnings`, and `cargo test --workspace`)
- CI green before requesting review

> **Test evidence requirement:** Every contract PR must include clear testing evidence. Changes without tests require a justification. See the [PR template](.github/PULL_REQUEST_TEMPLATE.md) and [CONTRIBUTING.md](CONTRIBUTING.md#pull-request-expectations) for details.

Before asking about payment status on an issue or PR, read the **[Payment-Period Conduct Guidance](docs/payment-period-conduct.md)** — it explains how to self-review first and how this repository's review process relates to GrantFox's own evaluation process.

Quick start:

```bash
# Fork & clone, then verify everything is green before opening a PR
make verify
```

---

## License

MIT
