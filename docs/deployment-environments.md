# Deployment Environments

This document describes how deployment configuration differs between local development, Stellar testnet, and future planned network targets.

The Savings Vault contract is **not production-ready**. See [Project Status](../README.md#project-status-and-scope) and [Known Limitations](../README.md#known-limitations) in the README.

## Quick reference

| Setting | Local (tests) | Testnet | Future Mainnet |
|---|---|---|---|
| RPC URL | Not needed | `https://soroban-testnet.stellar.org:443` | Not yet |
| Network passphrase | Not needed | `Test SDF Network ; September 2015` | Not yet |
| Fund source | Not needed | Friendbot | Not yet |
| Deploy command | Not needed | `./scripts/deploy-testnet.sh <identity>` | Not yet |
| Soroban CLI version | Any | `soroban` (older) or `stellar` (newer) | TBD |

## Local development

### What it's for

Building the contract, running unit tests, and verifying WASM compilation. No network connection is required.

### Configuration

No RPC URL, network passphrase, or funded account is needed for local development. All tests run natively using the Soroban SDK test utilities.

### Commands

```bash
# Build
cargo build --target wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release

# Or using the project task runner
make build-release

# Test
cargo test
cargo test --workspace

# Format and lint
cargo fmt --check
cargo clippy --tests -- -D warnings
```

### Environment variables

| Variable | Required | Description |
|---|---|---|
| `CARGO_TERM_COLOR` | No | Set to `always` for coloured CI output |

A `.env` file is not required for local development.

## Testnet

### What it's for

Deploying the contract to the Stellar testnet, invoking functions, and integration testing with real network conditions.

### Network configuration

Add the testnet network to your Soroban CLI:

```bash
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

Verify it was added:

```bash
soroban network ls
```

### Identity

Create and fund a deployer identity:

```bash
soroban keys generate --global deployer --network testnet
soroban keys address deployer
```

Fund it using Friendbot. Open this URL in a browser, replacing `YOUR_ADDRESS` with the address from the command above:

```text
https://friendbot.stellar.org/?addr=YOUR_ADDRESS
```

Or if your CLI supports it:

```bash
soroban keys fund deployer --network testnet
```

### Deploy

Use the automated deployment script:

```bash
./scripts/deploy-testnet.sh deployer
```

The script builds the release WASM, deploys it, and prints the contract ID. Save that value for contract invocation.

Alternatively, deploy manually:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source deployer \
  --network testnet
```

### Contract invocation

Initialize the contract:

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin deployer
```

Call functions:

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

### Environment variables

| Variable | Required | Description |
|---|---|---|
| `SOROBAN_RPC_URL` | No | Override the default testnet RPC URL for the `soroban` CLI |
| `SOROBAN_NETWORK_PASSPHRASE` | No | Override the default testnet network passphrase |
| `VAULT_CONTRACT_ID` | If integrating with SDK | Contract ID returned by the deploy step, used by the SDK configuration |

These values should not be committed to the repository. Use them in a local `.env` file or a secret manager if the SDK requires them.

### Testnet tools

| Tool | URL |
|---|---|
| Stellar testnet RPC | `https://soroban-testnet.stellar.org:443` |
| Friendbot | `https://friendbot.stellar.org` |
| Stellar Expert (testnet) | `https://stellar.expert/explorer/testnet` |

### Deployment output example

See [deployment-output-example.md](deployment-output-example.md) for what a successful testnet deployment looks like and how to identify the contract ID in the output.

### Troubleshooting

See [troubleshooting.md](troubleshooting.md) for common issues with:
- Missing WASM target or Soroban CLI
- Friendbot funding failures
- RPC or network configuration
- Contract build, deploy, and invocation errors

## Future mainnet (not yet supported)

### Status

Mainnet deployment is **not supported** at this stage. The contract uses internal balance tracking rather than real token custody, and it has not been audited for production use. See the README [Project Status and Scope](../README.md#project-status-and-scope) section for details.

### When mainnet is ready

When the contract supports real asset custody (likely through Stellar Asset Contract integration) and has been audited, this section will be updated with:

- Production RPC URL
- Mainnet network passphrase
- Identity management guidance (no Friendbot — real funding required)
- Deployment commands
- Environment variable documentation

### What not to do

- Do not deploy the current contract to mainnet
- Do not use real XLM or tokens with the current contract
- Do not commit real secrets, private keys, or seed phrases anywhere in this repository

## Environment variable reference

These variables are referenced across the project documentation and SDK configuration.

| Variable | Environments | Description |
|---|---|---|
| `VAULT_CONTRACT_ID` | Testnet, Future Mainnet | Deployed contract ID for the savings vault |
| `DEPLOYER_SECRET_KEY` | None — do not use | Example of what not to commit (see contract-id-handoff.md) |
| `SOROBAN_RPC_URL` | Testnet | Override the default Soroban RPC endpoint |
| `SOROBAN_NETWORK_PASSPHRASE` | Testnet | Override the default network passphrase |
| `CARGO_TERM_COLOR` | Local | Cargo output colour setting |

### Safe vs unsafe

Safe to commit in example files:

```dotenv
VAULT_CONTRACT_ID=VAULT_CONTRACT_ID_PLACEHOLDER_ABC123
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org:443
```

Never commit, even in example files:

```dotenv
DEPLOYER_SECRET_KEY=any-real-or-placeholder-secret-key
```
