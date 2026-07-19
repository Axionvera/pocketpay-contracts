# Stellar PocketPay — Savings Vault Contract

[CHANGELOG](CHANGELOG.md)

## Project Status and Scope

This project is currently intended for development, learning, and Stellar testnet usage. It is **not production-ready or mainnet-ready**.

The savings vault currently uses internal balance tracking: `deposit`, `withdraw`, and locking operations update accounting records stored by the contract, but they do not move or custody XLM or other tokens. The contract should therefore **not be treated as a real token custody contract**.

Supporting real asset deposits and withdrawals in the future may require integration with a Stellar Asset Contract (SAC), including explicit token transfer and custody behavior. See [Known Limitations](#known-limitations) for other current constraints.
## Security Considerations

> **This contract is for educational and testnet use.** Review the following before any mainnet deployment.

See the [Admin Role](docs/admin-role.md) document for details on what the `initialize(admin)` value records, what the admin can and cannot do today, and future admin design considerations.

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

### Deposit and custody limitation

> **Deposits currently update internal contract storage only.** Calling `deposit` increases the user's recorded balance for the vault's accounting, but it does not move real XLM, a Stellar Asset Contract (SAC) asset, or any other token into contract custody.

An **internal balance** is a number maintained by this contract and used by its deposit, withdrawal, and locking logic. **Real token custody** requires an on-chain asset transfer that moves tokens between addresses and ensures the recorded balance is backed by assets held for the user. That transfer and custody layer is not implemented yet, so the current internal balances must not be treated as proof of deposited or custodied assets.

Future SAC integration is planned to support real asset transfers and custody-backed balances.

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

## Task Runner

Common tasks are available via `make`:

```bash
make test        # Run all tests
make build-wasm  # Build the contract WASM in release mode
make clean       # Clean build artifacts
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

3. Deploy the Contract

Use the automated deployment script to build the release WASM and deploy it to the network. Pass your configured identity (e.g., `deployer`) as the first argument:

```bash
./scripts/deploy-testnet.sh deployer
```

The script will verify prerequisites, compile the contract, deploy it, and print your new Contract ID. Save the returned Contract ID — you'll need it to invoke functions.

See the [deployment output example](docs/deployment-output-example.md) to learn what successful output looks like and which Contract ID value to copy.

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
└── docs/
    ├── admin-role.md                   # Admin role documentation
    ├── pause-design.md                 # Pause / emergency stop research
    └── upgrade-strategy.md             # Upgrade strategy research
```

---
## Documentation

- [Audit Preparation Checklist](docs/audit-preparation.md) — Checklist of documentation, tests, threat model, and deployment details required before any external security review or audit.
- [Deployment Environments](docs/deployment-environments.md) — Network configuration for local, testnet, and future mainnet, including RPC URLs, identities, environment variables, and deployment commands.
- [Contract Error Reference](docs/error-codes.md) - Current savings vault failure conditions and guidance for SDK and mobile callers.
- [Architecture Documentation](docs/architecture.md) – Overview of project structure, state management, storage, SDK integration, and future extension points.
- [SDK ↔ Contract Sequence Diagrams](docs/sdk-contract-sequence.md) – Mermaid sequence diagrams for balance query, deposit, withdraw, and error paths across mobile, SDK, Soroban RPC, and the vault contract.
- [Event Schema Documentation](docs/events.md) – Overview of event names, topics, payload schemas, and JSON examples for vault actions.
- [Vault Contract ID Handoff](docs/contract-id-handoff.md) - How to pass a deployed vault contract ID safely to SDK configuration and the mobile app.
- [Documentation Style Guide](docs/docs-style-guide.md) — Conventions for Testnet wording, avoiding production claims, placeholders, command formatting, and linking related docs.
- [Placeholder Value Conventions](docs/placeholders.md) — Canonical placeholder names for contract IDs, user and admin addresses, and timestamps used in examples.

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
- **Internal accounting only; no real token custody**: Deposits update contract storage but do not transfer real XLM, SAC assets, or other tokens into custody. Internal balances are accounting entries and are not proof that the contract holds corresponding assets. Future SAC integration is planned to support real asset transfers and custody-backed balances.
- **Single unlock time**: Locking funds multiple times overwrites the previous unlock timestamp. A production version might use per-lock entries.
- **No admin recovery**: There is no mechanism for the admin to recover or migrate funds.
- **No upgrade mechanism**: The contract does not implement `upgrade()`. See
  [docs/upgrade-strategy.md](docs/upgrade-strategy.md) for research into possible upgrade paths.
- **No pause / emergency stop**: There is no mechanism to halt operations in an emergency.
  See [docs/pause-design.md](docs/pause-design.md) for research and trade-offs.

---

## Deployment Notes

- **Testnet RPC**: `https://soroban-testnet.stellar.org:443`
- **Network passphrase**: `Test SDF Network ; September 2015`
- **Friendbot** (free testnet XLM): `https://friendbot.stellar.org`
- **Soroban Explorer**: [stellar.expert](https://stellar.expert/explorer/testnet)
- Deployment help: see the [troubleshooting guide](docs/troubleshooting.md)
  for common Soroban CLI, Friendbot, WASM, network, and invocation issues.
  For a full breakdown of environment-specific configuration, see the
  [deployment environments guide](docs/deployment-environments.md).
- Always test thoroughly on testnet before considering mainnet deployment.
- Monitor contract storage TTL and extend as needed using `soroban contract extend`. See the [Storage TTL Guide](docs/storage-ttl.md) for persistent vs. instance storage details and example commands.

## Documentation

For a full list of CLI command examples and arguments for each contract method, check out the [Contract Invocation Examples](docs/invocation-examples.md).
---

## Contributing

Contributions are welcome! This project is intentionally beginner-friendly.

See **[CONTRIBUTING.md](CONTRIBUTING.md)** for the full guide, including:

- How to format code (`cargo fmt`)
- How to lint code (`cargo clippy -- -D warnings`)
- How to run the test suite (`cargo test`)
- PR checklist and commit message conventions

Quick start:

```bash
# Fork & clone, then verify everything is green before making changes
cargo fmt --check
cargo clippy --tests -- -D warnings
cargo test
```

---

## License

This project is licensed under the [MIT License](LICENSE).
