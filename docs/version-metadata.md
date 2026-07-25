# Design Note: Smart Contract Version Metadata

## Summary
This document addresses the design considerations for exposing version metadata in the `savings_vault` smart contract. Providing a standardized approach to version retrieval is essential for SDK consumers and deployment systems to safely check compatibility before executing transactions.

## SDK Compatibility & Use Cases
External SDK consumers and frontend clients need a reliable way to verify contract versions prior to interacting with them. 
- **Graceful Failures:** If a contract is upgraded or incompatible, the SDK can immediately throw a clear warning instead of hitting low-level transaction execution errors.
- **Dynamic Interface Mapping:** Allows client SDKs to adjust interface decoding dynamically depending on the active version deployed on-chain.

## Architectural Trade-offs: Storage vs. Hardcoded

### Approach A: Hardcoded Version Constants
```rust
const VERSION: &str = "1.0.0";

pub fn get_version(env: Env) -> String {
    String::from_str(&env, VERSION)
}

pub fn initialize(env: Env, admin: Address, token: Address, version: String) {
    // ...
    env.storage().instance().set(&Symbol::new(&env, "version"), &version);
}

#[contractimpl]
impl SavingsVaultContract {
    /// Returns the active version of the deployed contract.
    pub fn get_version(env: Env) -> String {
        // Implementation details pending maintainer decision
    }
}
# Version Metadata

The `savings_vault` contract exposes a `get_version` read-only function that
returns the contract's semantic version string. SDKs, deployment tooling, and
monitoring systems can call this to verify which version is deployed before
executing state-changing operations.

## Function signature

```rust
pub fn get_version(env: Env) -> soroban_sdk::String
```

**Returns:** a string matching the contract crate version in `Cargo.toml`
(e.g. `"0.1.0"`).

**Authorization:** none (read-only).

**Storage:** none — the value is baked into the compiled WASM binary at build
time.

## CLI usage

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source deployer \
  --network testnet \
  -- \
  get_version
```

**Expected output:** `"0.1.0"`

No initialization or token setup is required; `get_version` works on a freshly
deployed contract.

## SDK usage

### Stellar SDK (TypeScript)

```typescript
const result = await contract.get_version();
console.log("Deployed contract version:", result);
```

### Rust (soroban-sdk)

```rust
let version = client.get_version();
assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));
```

## Deployment script integration

Deployment scripts can verify the deployed version immediately after
deployment:

```bash
# Deploy and capture contract ID
CONTRACT_ID=$(soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/savings_vault.wasm \
  --source deployer \
  --network testnet)

# Verify version matches expected
DEPLOYED_VERSION=$(soroban contract invoke \
  --id "$CONTRACT_ID" \
  --source deployer \
  --network testnet \
  -- get_version)

EXPECTED_VERSION="0.1.0"
if [ "$DEPLOYED_VERSION" != "\"$EXPECTED_VERSION\"" ]; then
  echo "Version mismatch: expected $EXPECTED_VERSION, got $DEPLOYED_VERSION"
  exit 1
fi
```

## Design rationale

### Why a hardcoded constant?

The version is a compile-time constant baked into the WASM binary. This means:

- **No storage cost** — no on-chain storage is read or written, so calling
  `get_version` has negligible execution cost.
- **No initialization required** — the function works before `initialize` is
  called.
- **Guaranteed consistency** — the version always matches the deployed binary,
  since it is part of the compiled code itself.

### Why not store the version on-chain?

Storing the version in instance or persistent storage would add unnecessary
mutable state and require initialization before the version could be queried.
The hardcoded approach is simpler and sufficient for this contract's needs.

### How to bump the version

When the contract is updated:

1. Update the `version` field in `contracts/savings_vault/Cargo.toml`.
2. Update the string literal in `get_version` in `contracts/savings_vault/src/lib.rs`
   to match.
3. The test `test_get_version` in `contracts/savings_vault/src/test/mod.rs`
   will fail if the two values diverge — use it as a safety net.

## Test coverage

The `test_get_version` test in `contracts/savings_vault/src/test/mod.rs`
verifies that the function returns the expected version string:

```rust
#[test]
fn test_get_version() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let version = client.get_version();
    assert_eq!(version, soroban_sdk::String::from_str(&env, "0.1.0"));
}
```

## Related docs

- [CLI Smoke Test Guide](cli-smoke-test.md) — Quick post-deployment
  verification that includes `get_version`.
- [Contract Invocation Examples](invocation-examples.md) — Full list of CLI
  commands for every contract method.
