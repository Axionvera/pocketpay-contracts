# Vault Contract ID Handoff

This guide describes how to pass a deployed savings vault contract ID from this contracts repository to the SDK and mobile app configuration.

## Get the contract ID from deployment

Deploy the vault using the repository deployment command described in the README. After a successful deployment, the deployment output prints and returns the new contract ID. Copy that value from the output; do not copy an account address, transaction hash, or deployment identity.

The examples below use this fake value only:

```text
VAULT_CONTRACT_ID_PLACEHOLDER_ABC123
```

Replace the placeholder with the contract ID returned by the deployment output for the intended network. Testnet and mainnet deployments have different contract IDs, so label the value with its environment when handing it off.

## Update the SDK configuration

Configure the SDK to read the deployed vault contract ID from an environment variable or the SDK's environment-specific configuration. Use `VAULT_CONTRACT_ID` as the configuration key unless the SDK already defines an equivalent key.

For example, a local environment file may contain:

```dotenv
VAULT_CONTRACT_ID=VAULT_CONTRACT_ID_PLACEHOLDER_ABC123
```

The SDK should read `VAULT_CONTRACT_ID` through its normal configuration layer and use the value when it builds vault contract calls. Verify both the selected Stellar network and the contract ID; a valid ID for the wrong network will not identify the intended deployment.

## Mobile integration path

The mobile app should obtain the vault contract ID through the SDK configuration path. The app may select an SDK environment or supply public runtime configuration, but it should not duplicate the ID across source files or hardcode any deployment credentials.

```text
deployment output -> VAULT_CONTRACT_ID -> SDK configuration -> mobile app
```

Keeping the value behind the SDK configuration provides one integration point for changing deployments and reduces the risk that mobile and SDK builds target different vaults.

## Public IDs and secret material

A deployed contract ID is public on the network and may be stored in non-secret application configuration. Deployment identities, private keys, seed phrases, signing keys, access tokens, and other secrets are different: never commit them to this repository, the SDK repository, the mobile repository, or an example environment file.

Keep secret material in an approved secret manager or local environment file that is excluded from version control. If a secret is committed accidentally, revoke or rotate it immediately and follow the affected repository's incident process; deleting it in a later commit is not enough.

## Safe and unsafe examples

Safe: commit a documented variable name with a fake placeholder in an example file.

```dotenv
# .env.example
VAULT_CONTRACT_ID=VAULT_CONTRACT_ID_PLACEHOLDER_ABC123
```

Safe: provide the real public contract ID through the SDK's environment-specific deployment configuration, following that repository's configuration policy.

Unsafe: commit deployment credentials or a seed phrase alongside the ID.

```dotenv
VAULT_CONTRACT_ID=VAULT_CONTRACT_ID_PLACEHOLDER_ABC123
DEPLOYER_SECRET_KEY=DO_NOT_COMMIT_A_REAL_SECRET_KEY
DEPLOYER_SEED_PHRASE=DO_NOT_COMMIT_A_REAL_SEED_PHRASE
```

Unsafe: bypass the SDK configuration by embedding deployment values directly in mobile source code.

```text
mobile source -> hardcoded contract ID and deployment secret
```

## Handoff checklist

- [ ] Deploy the contract.
- [ ] Copy the contract ID returned in the deployment output.
- [ ] Update `VAULT_CONTRACT_ID` in the SDK environment or configuration for the correct network.
- [ ] Verify the SDK reads the expected contract ID.
- [ ] Verify the mobile app consumes the SDK configuration.
