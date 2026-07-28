# Invariant Test Checklist — Savings Vault Contract

## Purpose

This checklist documents the critical invariants that contributors must preserve when making changes to the Savings Vault contract. Contract changes can appear small but still affect critical invariants such as balances, locks, withdrawals, and authorization.

> **Scope:** This contract is intended for development, learning, and Stellar testnet use. It is not production-ready or mainnet-ready. These invariants document current behavior and audit expectations; they do not constitute an external audit or a production-safety guarantee.

## How to Use This Checklist

- **Contributors**: Review this checklist before opening any PR that touches contract logic. Use it to identify which invariants your change affects and ensure you have appropriate test coverage.
- **Maintainers**: Reference this checklist during PR review to verify that all relevant invariants are preserved and tested.

For detailed formal definitions and test references, see [Formal Accounting Invariants](accounting-invariants.md).

---

## 1. Balance Consistency Invariants

### 1.1 Non-Negativity
- **Invariant**: Available balance and locked balance must never be negative for any user.
- **Formula**: `A(u) >= 0` and `L(u) >= 0` for all users `u`
- **When to check**: Any change affecting `deposit`, `withdraw`, `lock_funds`, `withdraw_lock`, or balance storage
- **Test coverage**: 
  - `contracts/savings_vault/src/test/balance_conservation.rs`
  - `contracts/savings_vault/src/test/property_vault_accounting.rs`
  - `contracts/savings_vault/src/test/multi_lock_invariants.rs`
  - `contracts/savings_vault/src/test/maximum_amount_boundary.rs`

### 1.2 Per-User Conservation
- **Invariant**: For each user, available plus locked balance equals net deposited amount.
- **Formula**: `A(u) + L(u) = D(u) - W(u) - WL(u)`
  - `A(u)`: available balance
  - `L(u)`: locked balance
  - `D(u)`: cumulative deposits
  - `W(u)`: cumulative available withdrawals
  - `WL(u)`: cumulative locked withdrawals
- **When to check**: Any change affecting balance accounting or lock operations
- **Test coverage**:
  - Deterministic sequence tests in `balance_conservation.rs`
  - Randomized property tests in `property_vault_accounting.rs`
  - Multi-lock sequence tests in `multi_lock_invariants.rs`

### 1.3 Token Custody and Solvency
- **Invariant**: Vault must hold enough tokens to satisfy all recorded internal liabilities.
- **Formula**: `C >= T` where `C` is contract token balance and `T` is aggregate user liabilities
- **Closed-system assumption**: `C = T` when tokens only enter/leave through contract operations
- **When to check**: Any change affecting token transfers or balance accounting
- **Test coverage**:
  - `prop_global_token_custody` in `property_vault_accounting.rs`
  - Token transfer assertions in `token_backed_withdrawals.rs`
  - Fee-free accounting tests in `property_fee_invariants.rs`

---

## 2. Lock and Withdrawal Invariants

### 2.1 Lock Creation Conservation
- **Invariant**: Locking is an internal reclassification that must not create or destroy value.
- **Formula**: For successful `lock_funds(u, x, unlock_time)`:
  - `x > 0`
  - `x <= A(u)`
  - `unlock_time > current ledger timestamp`
  - `delta A(u) = -x`
  - `delta L(u) = +x`
  - `delta C = 0` (no token transfer)
- **When to check**: Any change affecting `lock_funds` or lock storage
- **Test coverage**:
  - `contracts/savings_vault/src/test/independent_lock_creation.rs`
  - `contracts/savings_vault/src/test/multi_lock_invariants.rs`
  - `contracts/savings_vault/src/test/balance_conservation.rs`

### 2.2 Lock Maturity Behavior
- **Invariant**: Time advancement alone does not modify accounting state.
- **Formula**: `delta A(u) = 0`, `delta L(u) = 0`, `delta C = 0`
- **Critical detail**: Matured locks remain in `get_locked_balance` until withdrawn via `withdraw_lock`
- **When to check**: Any change affecting time-based logic or lock maturity checks
- **Test coverage**:
  - `contracts/savings_vault/src/test/withdraw_lock.rs`
  - `contracts/savings_vault/src/test/multi_lock_invariants.rs`
  - `contracts/savings_vault/src/test/lock_maturity_boundary.rs`

### 2.3 Available Withdrawal Constraints
- **Invariant**: Available withdrawals can only consume available balance, not locked value.
- **Formula**: For successful `withdraw(u, x)`:
  - `0 < x <= A(u)`
  - `delta A(u) = -x`
  - `delta L(u) = 0`
  - `delta C = -x`
- **When to check**: Any change affecting `withdraw` function
- **Test coverage**:
  - `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
  - `contracts/savings_vault/src/test/balance_conservation.rs`
  - `contracts/savings_vault/src/test/unauthorized_access.rs`

### 2.4 Locked Withdrawal Constraints
- **Invariant**: Locked withdrawals require lock existence, ownership, non-withdrawn state, and maturity.
- **Formula**: For successful `withdraw_lock(u, lock_id)` with lock amount `x`:
  - Lock exists and is owned by `u`
  - `lock.withdrawn = false`
  - `current timestamp >= lock.unlock_time`
  - `delta A(u) = 0`
  - `delta L(u) = -x`
  - `delta C = -x`
  - After: `lock.withdrawn = true`, `lock.amount = 0`
- **When to check**: Any change affecting `withdraw_lock` or lock state
- **Test coverage**:
  - `contracts/savings_vault/src/test/withdraw_lock.rs`
  - `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
  - `contracts/savings_vault/src/test/multi_lock_invariants.rs`

---

## 3. Authorization Invariants

### 3.1 User Authorization Required
- **Invariant**: Every state-changing user operation requires authorization from that user.
- **Functions requiring user auth**: `deposit`, `withdraw`, `lock_funds`, `withdraw_lock`
- **Mechanism**: `user.require_auth()` enforced by Soroban Host
- **When to check**: Any change adding or modifying user-facing functions
- **Test coverage**:
  - `contracts/savings_vault/src/test/unauthorized_access.rs`
  - Cross-user isolation tests in `balance_conservation.rs`
  - `prop_cross_user_isolation` in `property_vault_accounting.rs`

### 3.2 User Isolation
- **Invariant**: Operations authorized by user `u` must not modify user `v`'s state.
- **Protected state**: `A(v)`, `L(v)`, `v`'s lock records, `v`'s next lock ID
- **Storage mechanism**: Keys are namespaced by user address (`DataKey::Balance(user)`)
- **When to check**: Any change affecting storage access or user operations
- **Test coverage**:
  - `conservation_cross_user_isolation` in `balance_conservation.rs`
  - `prop_cross_user_isolation` in `property_vault_accounting.rs`
  - `multi_lock_cross_user_isolation` in `multi_lock_invariants.rs`

### 3.3 Admin Boundaries
- **Invariant**: Admin operations must not modify user balances, lock amounts, or token custody.
- **Admin functions**: `initialize`, `transfer_admin`, `pause`, `unpause`
- **During pause**: Deposits and new locks blocked; withdrawals still permitted
- **When to check**: Any change affecting admin functions or pause logic
- **Test coverage**:
  - `contracts/savings_vault/src/test/admin_invariant_guard.rs`
  - `contracts/savings_vault/src/test/pause.rs`

---

## 4. Atomicity and Rollback Invariants

### 4.1 Failed Operation Atomicity
- **Invariant**: Failed contract calls must be observationally equivalent to no-ops for accounting state.
- **Unchanged state**: Balances, lock entries, next lock IDs, token custody, events
- **Implementation**: Token transfers before persistent writes; Soroban rollback on failure
- **When to check**: Any change affecting state-changing operations
- **Test coverage**:
  - `contracts/savings_vault/src/test/token_transfer_rollback.rs`
  - `contracts/savings_vault/src/test/balance_conservation.rs`
  - `contracts/savings_vault/src/test/multi_lock_invariants.rs`

### 4.2 Token-Backed Operations
- **Invariant**: Every deposit must be token-backed; every withdrawal must transfer tokens out.
- **Deposit**: Token transfer in, then balance credit
- **Withdrawal**: Balance debit, then token transfer out
- **When to check**: Any change affecting `deposit`, `withdraw`, or `withdraw_lock`
- **Test coverage**:
  - `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
  - `contracts/savings_vault/src/test/token_transfer_rollback.rs`

---

## 5. Example Invariant Test Patterns

### Pattern 1: Balance Conservation Test
```rust
#[test]
fn test_balance_conservation_after_operations() {
    let fixture = new_fixture();
    let user = fixture.user;
    
    // Track net deposited
    let mut net_deposited = 0i128;
    
    // Deposit
    let deposit_amount = 1000i128;
    fixture.token_client.mint(&user, &deposit_amount);
    fixture.client.deposit(&user, &deposit_amount);
    net_deposited += deposit_amount;
    
    // Verify invariant
    let available = fixture.client.get_balance(&user);
    let locked = fixture.client.get_locked_balance(&user);
    assert_eq!(available + locked, net_deposited);
    
    // Lock funds
    let lock_amount = 300i128;
    let unlock_time = 2000u64;
    fixture.client.lock_funds(&user, &lock_amount, &unlock_time);
    
    // Verify invariant still holds
    let available = fixture.client.get_balance(&user);
    let locked = fixture.client.get_locked_balance(&user);
    assert_eq!(available + locked, net_deposited);
}
```

### Pattern 2: Cross-User Isolation Test
```rust
#[test]
fn test_cross_user_isolation() {
    let fixture = new_fixture();
    let user_a = Address::generate(&fixture.env);
    let user_b = Address::generate(&fixture.env);
    
    // Setup user A
    fixture.token_client.mint(&user_a, &1000);
    fixture.client.deposit(&user_a, &1000);
    
    // Setup user B
    fixture.token_client.mint(&user_b, &500);
    fixture.client.deposit(&user_b, &500);
    
    // User A locks funds
    fixture.client.lock_funds(&user_a, &200, &2000);
    
    // Verify user B's state unchanged
    assert_eq!(fixture.client.get_balance(&user_b), 500);
    assert_eq!(fixture.client.get_locked_balance(&user_b), 0);
}
```

### Pattern 3: Authorization Failure Test
```rust
#[test]
fn test_withdraw_unauthorized_caller_fails() {
    let fixture = new_fixture();
    let user = Address::generate(&fixture.env);
    let attacker = Address::generate(&fixture.env);
    
    // Setup user with balance
    fixture.token_client.mint(&user, &1000);
    fixture.client.deposit(&user, &1000);
    
    // Attacker tries to withdraw user's funds
    let result = fixture.client.try_withdraw(&user, &500);
    assert!(result.is_err()); // Should fail due to auth
}
```

### Pattern 4: Failed Operation Atomicity Test
```rust
#[test]
fn test_failed_deposit_leaves_state_unchanged() {
    let fixture = new_fixture();
    let user = Address::generate(&fixture.env);
    
    // Snapshot initial state
    let initial_balance = fixture.client.get_balance(&user);
    
    // Attempt deposit with insufficient token balance
    let result = fixture.client.try_deposit(&user, &1000);
    assert!(result.is_err());
    
    // Verify state unchanged
    assert_eq!(fixture.client.get_balance(&user), initial_balance);
}
```

---

## 6. Invariant Test Coverage Map

| Invariant Category | Key Test Files | Property Tests |
|---|---|---|
| Balance Consistency | `balance_conservation.rs`, `withdrawal_invariant.rs` | `property_vault_accounting.rs` |
| Lock Operations | `independent_lock_creation.rs`, `withdraw_lock.rs`, `lock_extension.rs` | `multi_lock_invariants.rs` |
| Authorization | `unauthorized_access.rs`, `admin_invariant_guard.rs` | `property_vault_accounting.rs` (isolation) |
| Atomicity | `token_transfer_rollback.rs`, `lock_atomicity.rs` | `property_vault_accounting.rs` |
| Token Custody | `token_backed_withdrawals.rs`, `total_vault_balance.rs` | `property_vault_accounting.rs` |
| Pause/Admin | `pause.rs`, `admin_rotation.rs`, `admin_invariant_guard.rs` | N/A |

---

## 7. Quick Reference for Contributors

When changing contract logic, ask yourself:

1. **Does this affect balances?** → Check balance consistency invariants
2. **Does this affect locks?** → Check lock conservation and maturity invariants
3. **Does this affect authorization?** → Check auth and isolation invariants
4. **Does this affect token transfers?** → Check custody and atomicity invariants
5. **Can this operation fail?** → Check rollback atomicity invariants

For each affected invariant:
- Add or update tests to verify the invariant still holds
- Run the full test suite: `cargo test --workspace`
- Document any invariant changes in the PR description

---

## Related Documentation

- [Formal Accounting Invariants](accounting-invariants.md) — Detailed formal definitions
- [Authorisation Rules & Security Matrix](authorisation-rules.md) — Authorization requirements
- [Contract Contributor Security Checklist](security-checklist.md) — Comprehensive PR checklist
- [Balance Reconciliation](balance-reconciliation.md) — Reconciliation procedures
- [Token-Backed Withdrawals](token-backed-withdrawals.md) — Token transfer guarantees
- [Failure Mode Catalogue](failure-mode-catalogue.md) — Known failure modes
