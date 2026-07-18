# Deployment Output Example

This example shows what a successful testnet deployment can look like and helps contributors identify the deployed contract ID in the command output. All values below are intentionally fake placeholders.

## Example successful output

```text
Building release WASM...
WASM artifact: target/wasm32-unknown-unknown/release/savings_vault.wasm
Deploying with account: TESTNET_ACCOUNT_PLACEHOLDER
RPC URL: https://soroban-testnet.example-rpc.invalid
Transaction status: SUCCESS_PLACEHOLDER
Contract ID (copy this value): CONTRACT_ID_PLACEHOLDER_ABC123
Deployment completed successfully.
```

The clearly labeled `Contract ID (copy this value)` line contains the deployed contract ID. Actual command wording may vary slightly with the Soroban CLI version.

## What to copy after deployment

Copy and save the contract ID printed by the deployment command. Use that value anywhere a later command asks for `YOUR_CONTRACT_ID`, including contract initialization and invocation commands.

You may also record the network name and the public deploying account address when they are useful for your deployment notes. Confirm that the contract ID came from the intended network before using it.

## What not to share publicly

Never share secret keys, private keys, seed phrases, wallet recovery phrases, wallet secrets, signing credentials, access tokens, or live credentials. Deployment logs should be reviewed and sensitive values removed before they are posted in an issue, pull request, chat, or other public location.