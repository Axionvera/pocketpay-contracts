# Smart Contract Invocation Examples

This document provides CLI invocation examples for all public functions in the `savings_vault` contract using the Soroban CLI.

### Prerequisites

Before running these commands, ensure you have set up your environment variables or replace the placeholders:
- `--source`: Your local account alias or secret key.
- `--rpc-url`: The RPC network endpoint URL.
- `--network-passphrase`: The target network passphrase.
- `--id`: The deployed contract ID.

---

## 1. Initialize Contract
Initializes the contract with an administrator and the token address to handle.
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <ADMIN_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  initialize \
  --admin <ADMIN_ADDRESS> \
  --token <TOKEN_ADDRESS>

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  deposit \
  --user <USER_ADDRESS> \
  --amount 10000000

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  withdraw \
  --user <USER_ADDRESS> \
  --amount 5000000

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  get_balance \
  --user <USER_ADDRESS>

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  lock_funds \
  --user <USER_ADDRESS> \
  --amount 20000000 \
  --unlock_time <UNIX_TIMESTAMP>

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  get_locked_balance \
  --user <USER_ADDRESS>

  soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <USER_ACCOUNT> \
  --rpc-url <RPC_URL> \
  --network-passphrase <NETWORK_PASSPHRASE> \
  -- \
  can_withdraw \
  --user <USER_ADDRESS>