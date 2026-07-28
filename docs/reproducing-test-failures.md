# Reproducing Test Failures Locally

This guide provides step-by-step instructions for reproducing and debugging contract test failures on your local machine. Following these steps ensures your PR passes the remote CI checks (dispatched to `Axionvera/pocketpay-issue-automation`).

## 1. Toolchain Prerequisites

Ensure your local environment matches the project requirements:

- **Rust**: Latest stable (Edition 2021).
- **WASM Target**: `wasm32-unknown-unknown`.
  ```powershell
  rustup target add wasm32-unknown-unknown
  ```
- **Soroban CLI**: Required for WASM size reporting and deployment tests.
  ```powershell
  cargo install --locked soroban-cli
  ```

## 2. Standard Test Execution

### Running All Tests
Run the entire suite to catch side effects or regressions:
```powershell
cargo test
```

### Running Specific Modules
If you are working on a specific feature, run only relevant tests to save time:
```powershell
# Examples
cargo test initialization
cargo test balance_isolation
cargo test pause_mechanism
```

### Running with Output
To see `log!` and `println!` output from failing tests:
```powershell
cargo test -- --nocapture
```

---

## 3. Reproducing Proptest Failures

The property-based tests (`property_vault_accounting` and `property_fee_invariants`) use random inputs. If a failure occurs in CI, you must reproduce it using the specific seed.

### Step 1: Get the Seed
In the CI failure log, look for a line like:
`proptest: failing seed was 0x1234567890abcdef...`

### Step 2: Run with the Seed
Set the `PROPTEST_SEED` environment variable and run the test:
```powershell
# PowerShell
$env:PROPTEST_SEED="0x1234567890abcdef..."
cargo test property_vault_accounting
```

### Step 3: Check Regressions
The suite also checks `contracts/savings_vault/proptest-regressions/`. If you've fixed a bug, ensure the regression files are updated or that the test now passes with the failing seed.

---

## 4. Handling Snapshot Mismatches

Tests in `event_schema.rs` and `event_compatibility.rs` compare contract outputs against JSON snapshots in `contracts/savings_vault/test_snapshots/`.

### Identifying a Mismatch
A snapshot failure looks like:
`Error: snapshot mismatch in 'deposit_event_v1.json'`

### Resolving the Mismatch
1. **Analyze the Diff**: Review the test output to see what changed in the event payload or state.
2. **Intentional Changes**: If the change is intentional (e.g., you added a new field to an event), delete the old snapshot file and re-run the test to generate a new one:
   ```powershell
   Remove-Item contracts/savings_vault/test_snapshots/deposit_event_v1.json
   cargo test event_schema
   ```
3. **Unintentional Changes**: If the change was accidental, fix your code logic until the test matches the existing snapshot.

---

## 5. Build and Makefile Failures

### WASM Build Failure
If `make build-release` fails, ensure the `wasm32-unknown-unknown` target is installed and that you are not using `std` features (the contract is `no_std`).

### Size Regression
If CI fails due to WASM size limits:
1. Run `make wasm-size` locally.
2. Check `lib.rs` for large dependencies or inefficient code patterns.
3. Ensure you are building in `--release` mode.

---

## 6. Common CI Mismatch Issues

| Symptom | Cause | Solution |
| :--- | :--- | :--- |
| **Passes locally, fails in CI** | Toolchain version mismatch | Ensure `rustup update stable` is run. |
| **Formatting error** | `cargo fmt` not run | Run `cargo fmt` before pushing. |
| **Clippy warnings** | Linter violations | Run `cargo clippy --tests -- -D warnings`. |
| **Missing files** | `.gitignore` blocking files | Check if `test_snapshots` or `proptest-regressions` were committed. |

## 7. Troubleshooting Toolchain

If you encounter strange compilation errors:
1. **Clean build artifacts**: `cargo clean` or `make clean`.
2. **Update dependencies**: `cargo update`.
3. **Check toolchain**: `rustc --version` and `soroban --version`.
