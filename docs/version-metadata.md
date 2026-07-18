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