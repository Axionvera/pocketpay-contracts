# Deployment Troubleshooting

## Overview

This guide covers common issues when building, deploying, funding, and invoking
the PocketPay Savings Vault contract on Stellar testnet.

The commands in this repository use the older `soroban` CLI style. Newer
Stellar documentation may show the `stellar` CLI for Soroban smart contracts.
Follow the command style used in this repository unless maintainers update the
README and deployment instructions.

This guide does not cover every possible deployment error, but it should help
with the issues contributors most often hit while getting started.

## Quick pre-deployment checklist

Run these checks before deploying:

```bash
rustup --version
rustc --version
cargo --version
rustup target list --installed
soroban --version
soroban network ls
soroban keys ls
cargo build --target wasm32-unknown-unknown --release
```

Confirm that:

- `wasm32-unknown-unknown` is installed.
- `soroban --version` works.
- `testnet` is listed in `soroban network ls`.
- Your deployer identity is listed in `soroban keys ls`.
- The release WASM exists at
  `target/wasm32-unknown-unknown/release/savings_vault.wasm`.

If you use a newer Stellar CLI locally, these commands may also be useful:

```bash
stellar --version
stellar network ls
stellar keys ls
```

## Missing WASM target

### Symptom

`cargo build --target wasm32-unknown-unknown --release` fails with an error
about a missing standard library, missing target, or `can't find crate for
core`.

### Likely cause

Rust is installed, but the WebAssembly target used by this repository is not
installed for your active Rust toolchain.

### Fix

Install the target used by this repository, then rebuild:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

Check installed targets with:

```bash
rustup target list --installed
```

### Notes

This repository currently builds with `wasm32-unknown-unknown`. Newer Stellar
smart contract setup guides may use:

```bash
rustup target add wasm32v1-none
```

Do not replace the repository's existing build target unless maintainers update
the project build instructions.

## Soroban/Stellar CLI not installed or not found

### Symptom

Your terminal prints `soroban: command not found`, `stellar: command not found`,
or a Windows message that the command is not recognized.

### Likely cause

The CLI is not installed, or Cargo's binary directory is not on your `PATH`.

### Fix

Install the CLI used by this repository:

```bash
cargo install --locked soroban-cli
soroban --version
```

If the install succeeds but the command is still not found, restart your
terminal and check that Cargo's bin directory is on your path:

```bash
cargo --version
```

On many systems, Cargo installs binaries under `~/.cargo/bin`.

### Notes

Newer Stellar documentation may show:

```bash
stellar --version
```

That is useful for contributors using the newer CLI, but this repository's
README currently uses `soroban` commands.

## CLI version mismatch

### Symptom

A command copied from the README fails with an unknown flag, missing subcommand,
or different argument format than expected.

### Likely cause

Your installed CLI version does not match the command style in the README, or
you are mixing newer `stellar` examples with this repository's older `soroban`
commands.

### Fix

Check your CLI versions:

```bash
soroban --version
stellar --version
```

Use the command family consistently. For this repository, start with the
`soroban` commands from the README:

```bash
soroban network ls
soroban keys ls
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source deployer \
  --network testnet
```

### Notes

Avoid mixing `soroban` and `stellar` commands in the same deployment attempt
unless you know both CLIs are configured with the same network and identity.

## Friendbot funding issues

### Symptom

Deployment or invocation fails because the source account cannot pay fees, or
Friendbot returns an error when funding your testnet address.

### Likely cause

The deployer identity is not funded, the wrong address was funded, Friendbot is
temporarily unavailable, or Friendbot rate limits are being hit.

### Fix

Check the deployer address:

```bash
soroban keys address deployer
```

Fund it with Friendbot in a browser:

```text
https://friendbot.stellar.org/?addr=YOUR_ADDRESS
```

If your CLI supports funding directly, you can also try:

```bash
soroban keys fund deployer --network testnet
```

For newer CLI setups, the equivalent may be:

```bash
stellar keys fund deployer --network testnet
```

### Notes

Friendbot is only for testnet. If funding fails, wait a few minutes and try
again. Make sure you fund the public address returned by `soroban keys address
deployer`, not a contract ID.

## Network or RPC configuration issues

### Symptom

Deployment or invocation fails with RPC errors, network passphrase errors,
connection failures, or messages suggesting the account does not exist even
after funding.

### Likely cause

The `testnet` network is missing, points to the wrong RPC URL, uses the wrong
passphrase, or your identity was funded on a different network than the one
used by the command.

### Fix

List configured networks:

```bash
soroban network ls
```

Add the testnet configuration used by the README:

```bash
soroban network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

Then confirm your identity exists:

```bash
soroban keys ls
soroban keys address deployer
```

### Notes

If you use the newer `stellar` CLI, inspect its network config separately:

```bash
stellar network ls
stellar keys ls
```

Network and key configuration may not be shared between older and newer CLI
tools.

## Contract build failures

### Symptom

The contract does not compile, or the expected WASM file is not generated.

### Likely cause

Common causes include a missing WASM target, running the command outside the
repository root, stale build output, or Rust dependency/toolchain issues.

### Fix

From the repository root, run:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
```

You can also use the project task runner:

```bash
make build-release
```

Check the expected output path:

```text
target/wasm32-unknown-unknown/release/savings_vault.wasm
```

### Notes

Run `cargo test` separately to check contract behavior. Tests run natively and
do not require a WASM build:

```bash
cargo test
```

## Contract deployment failures

### Symptom

`soroban contract deploy` fails, cannot find the WASM file, reports an invalid
source account, or fails with insufficient balance or network errors.

### Likely cause

The release WASM has not been built, the deployer identity is missing or
unfunded, the wrong network is selected, or the command is being run from the
wrong directory.

### Fix

Build the WASM and verify your deployer:

```bash
cargo build --target wasm32-unknown-unknown --release
soroban keys ls
soroban keys address deployer
soroban keys fund deployer --network testnet
```

Deploy using the README command style:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source deployer \
  --network testnet
```

### Notes

Save the returned contract ID. You need that exact value for later
`soroban contract invoke` commands.

## Contract invocation failures

### Symptom

`soroban contract invoke` fails with an invalid contract ID, authorization
error, insufficient balance, missing argument, wrong argument type, or contract
panic.

### Likely cause

The command may use the wrong contract ID, the source identity may not match the
address argument requiring authorization, the identity may be unfunded, or the
function arguments may not match the contract function signature.

### Fix

Confirm your identity and network:

```bash
soroban keys ls
soroban keys address deployer
soroban network ls
```

Initialize with the contract ID returned by deploy:

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin deployer
```

Call functions using the argument names from the README:

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  deposit \
  --user deployer \
  --amount 1000
```

```bash
soroban contract invoke \
  --id YOUR_CONTRACT_ID \
  --source deployer \
  --network testnet \
  -- \
  get_balance \
  --user deployer
```

If the account may be unfunded, fund it again:

```bash
soroban keys fund deployer --network testnet
```

### Notes

`YOUR_CONTRACT_ID` is not your public key and not the WASM hash. It is the
contract ID returned by the deploy command. Also, `initialize` can only be run
once for a deployed contract.

## Windows terminal issues

### Symptom

Commands fail because line continuations are not recognized, `make` or `sh` is
missing, paths are not found, or the terminal says scripts are disabled.

### Likely cause

The README examples use Unix-style shell formatting. Windows PowerShell, Command
Prompt, Git Bash, and WSL handle paths and multi-line commands differently.

### Fix

Use Git Bash or WSL for the README commands when possible. If using PowerShell,
run commands on one line when backslash continuations fail:

```bash
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm --source deployer --network testnet
```

If `make build-release` is unavailable, use Cargo directly:

```bash
cargo build --target wasm32-unknown-unknown --release
```

Check the WASM path from the repository root:

```text
target/wasm32-unknown-unknown/release/savings_vault.wasm
```

### Notes

PowerShell uses backticks for line continuation, not backslashes. Copying the
README's multi-line Bash commands into PowerShell may require converting them
to one-line commands.

## When to ask for help

### Symptom

You have checked the target, CLI, network, funding, WASM path, and invocation
arguments, but deployment or invocation still fails.

### Likely cause

The problem may be a CLI version change, a temporary testnet/RPC issue, a
contract-specific panic, or a documentation gap.

### Fix

Collect the exact command and error output before asking for help:

```bash
rustup --version
rustc --version
cargo --version
soroban --version
soroban network ls
soroban keys ls
cargo build --target wasm32-unknown-unknown --release
```

Include:

- Your operating system and terminal.
- The exact command you ran.
- The full error message.
- Whether you are using `soroban` or `stellar`.
- The network name you used, such as `testnet`.

### Notes

Do not share secret keys or seed phrases. Public addresses and contract IDs are
safe to include in issue comments.
