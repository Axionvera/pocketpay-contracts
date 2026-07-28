# Advanced Local Development and Testing Guide

This guide provides an advanced reference for building, testing, extending, and debugging the `SavingsVault` smart contract in `stellar-pocketpay-contracts`.

> **Safety Notice:** This contract is for educational and testnet use. It is not production- or mainnet-ready. For security guidelines and current scope, see the [Security Considerations](../README.md#security-considerations) section in the main README.

---

## 1. Prerequisites and Environment Setup

Before working with the test suite or compiling contract binaries, ensure your host system has the following tools installed:

1. **Rust Toolchain (Stable)**
   Ensure stable Rust is configured:
   ```bash
   rustup default stable
   ```

2. **WASM Target**
   Add the WebAssembly compilation target for Soroban smart contracts:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Soroban CLI**
   Install the official Soroban CLI tool:
   ```bash
   cargo install --locked soroban-cli
   ```

4. **Code Quality Tools**
   Ensure formatting and linting tools are installed:
   ```bash
   cargo fmt --check
   cargo clippy --tests
   ```
   `cargo clippy --tests` currently reports pre-existing lint warnings across
   the test suite; none are compile errors, but avoid running with
   `-D warnings` until that debt is cleaned up, or it will fail on code you
   didn't touch.

---

## 2. Build Commands and Task Runner

### Core Compilation Commands

- **Check Workspace Code:**
  ```bash
  cargo check
  ```

- **Run Native Test Suite:**
  ```bash
  cargo test
  ```

- **Compile Contract to WASM (Debug):**
  ```bash
  cargo build --target wasm32-unknown-unknown
  ```

- **Compile Contract to WASM (Release):**
  ```bash
  cargo build --target wasm32-unknown-unknown --release
  ```

### Using the `Makefile` Task Runner

The repository includes Makefile targets to streamline common workflow steps:

- **Run all local verification checks** (format, Clippy, tests, release WASM build):
  ```bash
  make verify
  ```

- **Build release WASM and print binary size report:**
  ```bash
  make build-release
  ```

- **Report existing release WASM size:**
  ```bash
  make wasm-size
  ```

The compiled release WebAssembly artifact is created at:
`target/wasm32-unknown-unknown/release/savings_vault.wasm`

See also the README [Local verification](../README.md#local-verification) section and [CONTRIBUTING.md](../CONTRIBUTING.md#build-format-and-test).

---

## 3. Test Suite Architecture and Organization

All contract tests are located under `contracts/savings_vault/src/test/`. The test suite is organized into focused, single-responsibility modules:

```
contracts/savings_vault/src/test/
├── mod.rs                        # Module registration & shared test re-exports
├── test_helpers.rs               # Reusable test environment & fixture factories
├── initialization.rs             # Contract initialization & re-initialization guards
├── config_read_helpers.rs        # Configuration query tests (admin, token, pause)
├── balance_conservation.rs       # Accounting balance conservation invariants
├── token_backed_withdrawals.rs   # SAC token transfer & withdrawal verification
├── withdrawal_invariant.rs       # Withdrawal authorization & balance limits
├── lock_amount_validation.rs     # Lock creation amount validation
├── lock_atomicity.rs             # Atomic state transition on lock creation
├── lock_extension.rs            # Extending active lock maturity timestamps
├── lock_id_generation.rs        # Sequential lock ID allocation
├── lock_maturity_boundary.rs     # Timestamp boundary tests (t = maturity - 1 vs t)
├── lock_read_helpers.rs         # Single & paginated lock query tests
├── zero_duration_lock.rs         # Special zero-duration lock edge cases
├── invalid_lock_id.rs           # Rejection of non-existent or spent lock IDs
├── unauthorized_access.rs        # Unmocked signature & auth rejection tests
├── admin_invariant_guard.rs      # Admin-only capability boundaries
├── admin_rotation.rs            # Admin address update & handoff verification
├── replay_protection.rs          # Invocation sequence replay safety
├── pause.rs                      # Emergency pause execution and withdrawal availability
├── pause_state_read.rs           # `is_paused()` query accuracy
├── pause_transition.rs           # Pause transition boundaries & auto-expiry
├── event_compatibility.rs        # Schema backwards-compatibility rules
├── event_ordering.rs             # Multi-event sequence ordering
├── storage_version.rs            # Contract storage versioning rules
├── property_vault_accounting.rs  # Proptest property-based accounting state machine
├── property_fee_invariants.rs    # Property-based fee model invariant checks
└── token_transfer_rollback.rs    # SAC token transfer failure state rollback
```

`cross_user_isolation.rs`, `total_vault_balance.rs`, `event_schema.rs`, and
`amount_normalization.rs` also exist in this directory but are **not**
declared in `mod.rs`, so `cargo test` never compiles or runs them — see the
next section. Don't assume a file's presence on disk means it's part of the
suite; confirm with `mod <name>;` in `mod.rs`, or `cargo test <name>::` to
see whether anything actually runs.

### Module Aggregation in `mod.rs`

New test files must be declared in `contracts/savings_vault/src/test/mod.rs` to be included in `cargo test`:

```rust
mod my_new_feature;
```

---

## 4. Soroban Environment & Fixture Setup

The Soroban SDK provides an in-memory execution environment (`soroban_sdk::Env`) for unit testing contracts without compiling to WASM or running a node.

### Standard Test Helpers (`test_helpers.rs`)

The test suite provides pre-configured helpers to set up test fixtures rapidly:

#### 1. `test_env()` — Mocked Authorization
`test_env()` initializes an environment with `env.mock_all_auths()` enabled. This automatically approves all `require_auth()` calls, making it ideal for testing internal business logic and accounting state changes without constructing signatures:

```rust
use super::test_helpers::*;

let env = test_env();
let (contract_id, client) = init_contract(&env);
```

#### 2. `strict_test_env()` — Strict Authorization
`strict_test_env()` creates an environment **without** `mock_all_auths()`. Use this fixture when explicitly verifying that unauthorized callers or missing signatures fail:

```rust
use super::test_helpers::*;

let env = strict_test_env();
// Calling contract methods here will trigger require_auth failures unless signed
```

#### 3. Mocking Stellar Asset Contract (SAC) Tokens
`SavingsVault` interacts with a configured SAC token contract for deposits and withdrawals. In tests, a mock SAC token is registered using `env.register_stellar_asset_contract_v2`:

```rust
use soroban_sdk::{token, Address, Env};

let issuer = Address::generate(&env);
let token_address = env.register_stellar_asset_contract_v2(issuer).address();

// Mint mock tokens to a test user
let token_admin = token::StellarAssetClient::new(&env, &token_address);
token_admin.mint(&user, &1_000_000);

// Inspect token balance
let token_client = token::Client::new(&env, &token_address);
let balance = token_client.balance(&user);
```

#### 4. Funding Test Accounts (`deposit_balance` & `deposit_with_sac`)
To fund a test user with both token balance and internal vault balance:

```rust
use super::test_helpers::*;

// Standard deposit helper (mints SAC tokens and calls client.deposit)
deposit_balance(&client, &user, 10_000);
```

---

## 5. Deterministic Ledger Time Simulation

Soroban contracts use the ledger timestamp (`env.ledger().timestamp()`) for time-locking and pause auto-expiry. In tests, ledger time can be set deterministically without real-world delay.

### Advancing Time in Tests

Use `env.ledger().set_timestamp(seconds)` or `set_ledger_timestamp(&env, ts)`:

```rust
use super::test_helpers::*;

// 1. Establish base time
env.ledger().set_timestamp(1_000);

// 2. Create a lock maturing at t = 5_000
let lock_id = client.lock_funds(&user, &500, &5_000);

// 3. Fast-forward past maturity
env.ledger().set_timestamp(5_001);

// 4. Withdrawal succeeds
client.withdraw_lock(&user, &lock_id);
```

### Testing Boundary Conditions

Always test exact timestamp boundaries:
- `t = unlock_time - 1`: Lock is immature; `withdraw_lock` must fail.
- `t = unlock_time`: Lock matures; `withdraw_lock` must succeed.

`lock_maturity_boundary.rs` splits each boundary into its own test rather
than reusing one lock across both checks (a lock that has already failed to
withdraw is still immature, so a single shared lock can't exercise both
sides of the boundary). The pattern:

```rust
// One second before maturity: rejected.
#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_boundary_one_second_before_maturity_rejected() {
    let f = setup_boundary_fixture(1_000);
    let lock_id = f.client.lock_funds(&f.user, &1_000, &5_000);

    set_ledger_timestamp(&f.env, 4_999);
    assert!(!f.client.can_withdraw(&f.user));

    f.client.withdraw_lock(&f.user, &lock_id);
}

// Exact maturity second: succeeds.
#[test]
fn test_boundary_exact_maturity_second_succeeds() {
    let f = setup_boundary_fixture(1_000);
    let lock_id = f.client.lock_funds(&f.user, &1_000, &5_000);

    set_ledger_timestamp(&f.env, 5_000);
    assert!(f.client.can_withdraw(&f.user));

    f.client.withdraw_lock(&f.user, &lock_id);
}
```

---

## 6. Failure Scenario and State Rollback Guidance

Robust smart contract testing requires testing failure cases to prove that failed operations preserve contract state atomically.

### Method 1: Expecting Panics with `#[should_panic]`

Use `#[should_panic]` or `#[should_panic(expected = "...")]` when testing operations that panic on invalid inputs or unauthorized calls:

```rust
#[test]
#[should_panic(expected = "Lock has not matured yet")]
fn test_withdraw_immature_lock_panics() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    deposit_balance(&client, &user, 1_000);
    let lock_id = client.lock_funds(&user, &500, &10_000);

    // Attempting withdrawal before maturity panics
    client.withdraw_lock(&user, &lock_id);
}
```

### Method 2: Inspecting Results with `try_*` Client Methods

Soroban SDK automatically generates fallible `try_<function_name>` client methods. These return `Result<T, Result<Error, InvokeError>>`, enabling tests to inspect errors and verify state without unwinding the test process:

```rust
#[test]
fn test_failed_lock_preserves_state() {
    let env = test_env();
    let (_id, client) = init_contract(&env);
    let user = Address::generate(&env);

    set_ledger_timestamp(&env, 1_000);
    deposit_balance(&client, &user, 1_000);
    let initial_balance = client.get_balance(&user);

    // unlock_time (500) is not strictly after the current ledger time
    // (1_000), so lock_funds must reject it.
    let res = client.try_lock_funds(&user, &500, &500);
    assert!(res.is_err());

    // Verify balance is completely untouched
    assert_eq!(client.get_balance(&user), initial_balance);
    assert_eq!(client.get_locked_balance(&user), 0);
}
```

Watch the ledger clock when writing this kind of test: `Env::default()`
starts at timestamp `0`, so an `unlock_time` that looks "in the past" at a
glance (e.g. `500`) is still in the future relative to an untouched
environment and the call will actually **succeed**. Set the ledger time
explicitly before asserting a time-based rejection.

### Method 3: State Rollback & Atomicity Verification

When testing SAC token transfer rollbacks (e.g., token contract failure), verify that internal balances and storage records remain unchanged if an external token transfer fails. See `token_transfer_rollback.rs` for canonical reference implementations.

To simulate an insufficient-balance transfer failure, don't reach for
`token_admin.mint()` on some other address — `mint` only credits its target,
it never debits the source you're trying to drain. Move the contract's
tokens out directly instead (`token_client.transfer(&contract_id, &somewhere_else, &amount)`);
`mock_all_auths()` satisfies the `require_auth()` the transfer performs
internally, even though the contract itself isn't the one calling it in the
test. See `test_failed_withdraw_lock_token_transfer_failure_preserves_state`
in `token_transfer_rollback.rs`.

### Events Only Cover the Most Recent Top-Level Call

`env.events().all()` does **not** return every event published since the
environment was created — it only returns events from the single most
recent top-level contract call. Every subsequent client call, including a
read-only query like `get_balance`, replaces the buffer.

```rust
client.deposit(&user, &500);
let events = env.events().all();
assert_eq!(events.len(), 2); // SAC transfer + deposit event — correct

client.lock_funds(&user, &100, &2_000);
let events = env.events().all();
assert_eq!(events.len(), 1); // only the lock event — the deposit's 2 are gone

client.get_balance(&user); // read-only, publishes nothing
let events = env.events().all();
assert_eq!(events.len(), 0); // buffer reset again, even though nothing failed
```

When a test needs to assert on events from more than one call in the same
transaction sequence — as in `event_ordering.rs` — capture `events().all()`
immediately after each call and accumulate manually:

```rust
client.deposit(&user, &500);
let mut events = env.events().all();

client.lock_funds(&user, &100, &2_000);
events.append(&env.events().all()); // soroban_sdk::Vec, not std Vec — use append(), not extend()

assert_eq!(events.len(), 3); // 2 from deposit + 1 from lock_funds, in order
```

If you only need the *last* call's events (e.g. asserting on the single
event a function just emitted), read `events().all()` immediately after that
call and before any other client call — including getters used for
unrelated assertions later in the same test. Reordering assertions so the
event read comes first is usually simpler than restructuring the call
sequence; see `test_extend_lock_success` in `lock_extension.rs`.

---

## 7. Property-Based Testing with `proptest`

The test suite includes property-based tests using the `proptest` crate (`property_vault_accounting.rs`). Property tests generate hundreds of pseudo-random sequences of user operations (`Deposit`, `Lock`, `Withdraw`) to verify fundamental invariants:

1. **Total Balance Invariant:** `available_balance + locked_balance == sum(all_deposits) - sum(all_withdrawals)`.
2. **Custody Invariant:** SAC token balance held by vault contract equals total user funds custody.
3. **No Unplanned Solvency Loss:** Operations cannot create negative balances or unauthorized locked funds.

To run property tests specifically:

```bash
cargo test property_vault_accounting
```

---

## 8. Common Developer Pitfalls and Debugging

| Pitfall / Error | Cause | Solution |
| --- | --- | --- |
| `Status(HostError, Error(Auth, InvalidAction))` | Calling contract method in `strict_test_env()` without mock signatures | Use `env.mock_all_auths()` or use `test_env()` helper |
| `Lock has not matured yet` | Forgot to advance `env.ledger().set_timestamp()` | Fast-forward ledger time past `unlock_time` |
| `token should be set during initialization` | Called contract function before `client.initialize(&admin, &token)` | Ensure `client.initialize` is invoked in test setup |
| `error: target 'wasm32-unknown-unknown' not found` | Target omitted from Rust toolchain | Run `rustup target add wasm32-unknown-unknown` |
| `Insufficient balance` | Attempted to lock or withdraw more than `get_balance(user)` | Check available vs locked balances before operation |
| `zero balance is not sufficient to spend` inside `deposit`/`withdraw` | Called `client.deposit(...)` without first minting the user any SAC tokens | Use the `deposit_balance(&client, &user, amount)` helper, which mints then deposits, instead of calling `client.deposit` directly on an unfunded user |
| `error[E0107]: method takes 0 generic arguments but 1 was supplied` on `.try_into_val::<T>(...)` | This SDK version's `try_into_val` takes no turbofish; the type is inferred from context | Drop the `::<T>`; if inference then fails with `E0283`, bind an intermediate `let x: T = ...;` |
| A test with `#[should_panic(expected = "...")]` fails with `HostError: Error(WasmVm, InvalidAction)` instead of matching the message, even though the diagnostic log shows the right panic string | The panic is real, but a `#[should_panic]` mismatch is usually really a missing/wrong attribute, not an SDK quirk — check whether `#[should_panic]` is present at all and whether `expected` matches the literal `panic!(...)` string in `lib.rs` | Add or correct `#[should_panic(expected = "...")]`; grep `lib.rs` for the exact panic string rather than guessing it |
| `env.events().all()` returns fewer events than expected, or `0` where a `- 1` on `.len()` then overflows | It only returns events from the single most recent top-level contract call — see "Events Only Cover the Most Recent Top-Level Call" above | Read `events().all()` immediately after the call you're asserting on, before any other client call, or accumulate across calls with `Vec::append` |

---

## 9. Step-by-Step Example: Adding a New Test Module

1. **Create the test file:** `contracts/savings_vault/src/test/my_feature_test.rs`.
2. **Register the module:** Add `mod my_feature_test;` in `contracts/savings_vault/src/test/mod.rs`.
3. **Write the test:**
   ```rust
   use super::test_helpers::*;

   #[test]
   fn test_my_feature_behaviour() {
       let env = test_env();
       let (_contract_id, client) = init_contract(&env);
       let user = new_user(&env);

       deposit_balance(&client, &user, 1_000);
       assert_eq!(client.get_balance(&user), 1_000);
   }
   ```
4. **Execute your new test:**
   ```bash
   cargo test test_my_feature_behaviour
   ```

---

## Related Documentation

- [Local Development Guide](local-development.md) — Initial environment setup & basic workflow
- [Test Coverage Summary](test-coverage.md) — Comprehensive test coverage matrix
- [Test Naming Conventions](testing.md) — Standardized test function naming rules
- [Failure Mode Catalogue](failure-mode-catalogue.md) — Failure cases and expected contract behaviors
- [Accounting Invariants](accounting-invariants.md) — System accounting laws and invariants
