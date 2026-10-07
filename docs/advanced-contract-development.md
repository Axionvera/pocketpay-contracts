# Advanced Contract Development Guide

This guide provides an advanced reference for developing, compiling, testing, extending, and debugging the `SavingsVault` Soroban smart contracts in `stellar-pocketpay-contracts`.

> **Safety Notice:** This contract is designed for educational and testnet use. Review [Security Considerations](../README.md#security-considerations) and [Threat Models](admin-pause-threat-model.md) before production evaluation.

---

## 1. Prerequisites & Toolchain Setup

To work with Soroban contract code and run the comprehensive test suite, configure your environment with:

```bash
# 1. Stable Rust toolchain
rustup default stable

# 2. WebAssembly target for Soroban
rustup target add wasm32-unknown-unknown

# 3. Soroban CLI (for local network invocations and deployments)
cargo install --locked soroban-cli

# 4. Code quality tools
cargo fmt --check
cargo clippy --tests
```

---

## 2. Build Commands & Workflows

### Standard Compilation

- **Check syntax and type safety:**
  ```bash
  cargo check
  ```

- **Compile WASM (Debug):**
  ```bash
  cargo build --target wasm32-unknown-unknown
  ```

- **Compile WASM (Optimized Release):**
  ```bash
  cargo build --target wasm32-unknown-unknown --release
  ```

### Makefile Automation

The project provides Makefile helpers for routine development tasks:

| Command | Action |
|---|---|
| `make verify` | Runs formatting check, Clippy, unit tests, and release WASM build |
| `make build-release` | Builds release WASM and outputs artifact size analysis |
| `make wasm-size` | Checks size of existing release WASM artifact |

Compiled artifact path:
`target/wasm32-unknown-unknown/release/savings_vault.wasm`

---

## 3. Test Suite Architecture & Organization

All unit and integration tests reside in `contracts/savings_vault/src/test/`:

```
contracts/savings_vault/src/test/
├── mod.rs                        # Test module declarations & re-exports
├── test_helpers.rs               # Test environment setup & client factories
├── initialization.rs             # One-time contract init & re-initialization guards
├── config_read_helpers.rs        # Query helpers (admin, token, pause state)
├── balance_conservation.rs       # Accounting balance conservation invariants
├── token_backed_withdrawals.rs   # SAC token transfers & withdrawals
├── withdrawal_invariant.rs       # Withdrawal bounds and authorization
├── lock_amount_validation.rs     # Lock creation amount checks
├── lock_atomicity.rs             # Atomic state updates on fund locking
├── lock_extension.rs             # Maturity extension verification
├── lock_maturity_boundary.rs     # Timestamp boundaries (t vs t - 1)
├── lock_read_helpers.rs         # Single and batch lock queries
├── pause.rs                      # Emergency pause behavior
├── property_vault_accounting.rs  # Proptest property-based state machine tests
└── token_transfer_rollback.rs    # Token failure transaction rollback
```

> **Note on Test Registration:** Any newly created test file must be explicitly registered via `mod <test_name>;` in `contracts/savings_vault/src/test/mod.rs` for `cargo test` to execute it.

---

## 4. Deterministic Ledger Time in Soroban Tests

Soroban provides full control over ledger environment parameters in tests. Use `env.ledger().with_mut(...)` to manipulate the timestamp deterministically:

```rust
use soroban_sdk::Env;

// Set initial timestamp
env.ledger().with_mut(|li| {
    li.timestamp = 1_000_000;
});

// Fast-forward time past lock maturity
env.ledger().with_mut(|li| {
    li.timestamp += 86_400; // +1 day
});
```

### Boundary Testing Patterns
Always test both sides of a time boundary:
- **At maturity minus 1 (`maturity - 1`):** Withdrawal must fail or lock must report active.
- **At exact maturity (`maturity`):** Withdrawal must succeed.
- **Past maturity (`maturity + 1`):** Withdrawal must succeed.

---

## 5. Token Mocking & Storage Fixtures

In local unit tests, interact with native and SAC tokens using the Soroban SDK mock token client:

```rust
use soroban_sdk::{testutils::Address as _, token, Address, Env};

let env = Env::default();
env.mock_all_auths();

let admin = Address::generate(&env);
let token_admin = Address::generate(&env);

// Register mock Stellar Asset Contract
let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
let token_client = token::Client::new(&env, &token_id.address());
let token_admin_client = token::StellarAssetClient::new(&env, &token_id.address());

// Mint tokens to user
token_admin_client.mint(&user, &10_000);
```

---

## 6. Failure Scenario & Invariant Testing

Testing negative scenarios ensures contracts fail safely without corrupting state:

### Invariant and Auth Failure Verification
```rust
#[test]
#[should_panic(expected = "HostError")]
fn test_unauthorized_withdrawal_fails() {
    let env = Env::default();
    // Do NOT call env.mock_all_auths() to simulate real caller auth failure
    let client = create_test_client(&env);
    client.withdraw(&unauthorized_user, &100);
}
```

### Asserting Specific Contract Errors
When using `try_invoke` patterns or inspecting `Result`:
```rust
let res = client.try_lock_funds(&user, &0, &future_time);
assert!(res.is_err());
```

---

## 7. Common Errors & Troubleshooting

| Symptom | Cause | Remediation |
|---|---|---|
| `UnregisteredContract` | Contract not registered in current `Env` | Ensure `env.register(...)` is called before client creation |
| `HostError: Error(Auth, InvalidAction)` | Missing auth mock in test | Call `env.mock_all_auths()` or provide mock auth entries |
| Test file ignored by `cargo test` | Missing `mod <file>;` in `mod.rs` | Add declaration to `contracts/savings_vault/src/test/mod.rs` |
| WASM binary exceeds size limit | Heavy dependencies or debug symbols | Build with `--release` and verify using `make wasm-size` |

---

## 8. See Also

- [Advanced Development & Testing Reference](advanced-development-and-testing.md)
- [Architecture Overview](architecture.md)
- [Invariant Test Checklist](invariant-test-checklist.md)
- [Failure Mode Catalogue](failure-mode-catalogue.md)
