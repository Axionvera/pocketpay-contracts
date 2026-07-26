# Formal Accounting Invariants

## Savings Vault Contract

## Purpose

This document defines the accounting properties that must hold for the Savings Vault contract after every successful operation and after every reverted or failed operation. It is written for audit review and describes the current implementation rather than a future design.

> **Scope:** This contract is intended for development, learning, and Stellar testnet use. It is not production-ready or mainnet-ready. These invariants document current behavior and audit expectations; they do not constitute an external audit or a production-safety guarantee.

The contract tracks two forms of user liability:

- **Available balance**: the value stored under `DataKey::Balance(user)` and returned by `get_balance`.
- **Locked balance**: the sum of every non-withdrawn `LockEntry` for the user and returned by `get_locked_balance`.

A matured lock remains part of `get_locked_balance` until it is withdrawn through `withdraw_lock`. Maturity alone does not move value back into the available balance.

## Accounting Model

For each user `u`, define:

- `A(u)`: available balance returned by `get_balance(u)`.
- `L(u)`: sum of all non-withdrawn lock amounts returned by `get_locked_balance(u)`.
- `D(u)`: cumulative amount from successful deposits.
- `W(u)`: cumulative amount from successful `withdraw` calls.
- `WL(u)`: cumulative amount from successful `withdraw_lock` calls.
- `C`: token balance held by the vault contract.
- `T`: total internal user liabilities known to the test or audit model.

The per-user accounting identity is:

```text
A(u) + L(u) = D(u) - W(u) - WL(u)
```

The aggregate liability is:

```text
T = sum over all users of (A(u) + L(u))
```

The contract does not currently maintain an enumerable on-chain registry of all users. Therefore, `T` is a model-level value used by tests and auditors, not a directly queryable contract field.

## 1. Non-Negativity

For every user:

```text
A(u) >= 0
L(u) >= 0
```

No successful operation may create a negative available balance or a negative locked balance.

Relevant coverage:

- `contracts/savings_vault/src/test/balance_conservation.rs`
- `contracts/savings_vault/src/test/property_vault_accounting.rs`
- `contracts/savings_vault/src/test/multi_lock_invariants.rs`
- `contracts/savings_vault/src/test/maximum_amount_boundary.rs`

## 2. Deposit Invariant

For a successful deposit of amount `x`, where `x > 0`:

```text
delta A(u) = +x
delta L(u) = 0
delta C    = +x
```

The user token transfer into the vault occurs before the internal balance is credited. A successful deposit must therefore increase both vault custody and the user's available balance by the same amount.

A failed deposit must not change:

- available balance;
- lock entries;
- next lock ID;
- vault custody;
- emitted events.

Relevant coverage:

- `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
- `contracts/savings_vault/src/test/token_transfer_rollback.rs`
- `contracts/savings_vault/src/test/property_fee_invariants.rs`
- `contracts/savings_vault/src/test/property_vault_accounting.rs`

## 3. Available Withdrawal Invariant

For a successful `withdraw(u, x)`:

```text
0 < x <= A(u)
delta A(u) = -x
delta L(u) = 0
delta C    = -x
```

`withdraw` may consume only the available balance. It does not consume matured or unmatured lock entries.

A withdrawal that is zero, negative, unauthorized, or greater than the available balance must fail without changing accounting state.

Relevant coverage:

- `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
- `contracts/savings_vault/src/test/balance_conservation.rs`
- `contracts/savings_vault/src/test/token_transfer_rollback.rs`
- `contracts/savings_vault/src/test/unauthorized_access.rs`

## 4. Locking Invariant

For a successful `lock_funds(u, x, unlock_time)`:

```text
x > 0
x <= A(u)
unlock_time > current ledger timestamp

delta A(u) = -x
delta L(u) = +x
delta C    = 0
```

Locking is an internal reclassification. It must not create or destroy value and must not transfer tokens.

Each successful lock creates one independently addressable `LockEntry` with:

- the correct owner;
- a positive amount;
- `withdrawn = false`;
- a per-user lock ID that is unique and monotonically increasing.

A failed lock must not change balances, existing lock entries, custody, events, or the next lock ID.

Relevant coverage:

- `contracts/savings_vault/src/test/independent_lock_creation.rs`
- `contracts/savings_vault/src/test/multi_lock_invariants.rs`
- `contracts/savings_vault/src/test/balance_conservation.rs`

## 5. Lock Maturity Invariant

Advancing ledger time does not itself modify accounting state:

```text
delta A(u) = 0
delta L(u) = 0
delta C    = 0
```

A matured, non-withdrawn lock remains included in `get_locked_balance`. The maturity condition only changes whether `withdraw_lock` is permitted.

This distinction is important for auditors: the current implementation does not automatically reclassify matured locks into `A(u)`.

Relevant coverage:

- `contracts/savings_vault/src/test/withdraw_lock.rs`
- `contracts/savings_vault/src/test/multi_lock_invariants.rs`
- `contracts/savings_vault/src/test/balance_conservation.rs`

## 6. Locked Withdrawal Invariant

For a successful `withdraw_lock(u, lock_id)` with lock amount `x`:

```text
lock exists
lock.owner = u
lock.withdrawn = false
current timestamp >= lock.unlock_time

delta A(u) = 0
delta L(u) = -x
delta C    = -x
```

After success, the lock remains addressable but must be marked:

```text
withdrawn = true
amount = 0
```

The same lock cannot be withdrawn twice.

A failed `withdraw_lock` must not alter balances, the lock record, custody, or events.

Relevant coverage:

- `contracts/savings_vault/src/test/withdraw_lock.rs`
- `contracts/savings_vault/src/test/token_backed_withdrawals.rs`
- `contracts/savings_vault/src/test/token_transfer_rollback.rs`
- `contracts/savings_vault/src/test/multi_lock_invariants.rs`

## 7. Per-User Conservation

After every successful or failed operation:

```text
A(u) + L(u) = D(u) - W(u) - WL(u)
```

Deposits increase the right-hand side. Available withdrawals and locked withdrawals decrease it. Lock creation and time advancement do not change it.

Relevant coverage:

- deterministic sequence tests in `balance_conservation.rs`;
- randomized property tests in `property_vault_accounting.rs`;
- multi-lock sequence tests in `multi_lock_invariants.rs`.

## 8. Token Custody and Solvency

The required solvency property is:

```text
C >= T
```

The vault must hold enough tokens to satisfy all recorded internal liabilities.

Under the closed-system assumption used by current tests, where tokens enter or leave the vault only through its public accounting operations:

```text
C = T
```

Exact equality is not a universal production invariant because an external account may transfer tokens directly to the vault without calling `deposit`. Such an unsolicited transfer increases `C` without increasing `T`. It creates excess custody, not user credit.

Therefore:

- `C < T` is an accounting or solvency failure.
- `C = T` is exact reconciliation.
- `C > T` represents unassigned excess tokens and must not be credited to any user automatically.

Relevant coverage:

- `prop_global_token_custody` in `property_vault_accounting.rs`;
- token transfer assertions in `token_backed_withdrawals.rs`;
- one-to-one fee-free accounting in `property_fee_invariants.rs`.

## 9. User Isolation

For distinct users `u` and `v`, an operation authorized by `u` must not modify:

```text
A(v)
L(v)
v's lock records
v's next lock ID
```

Storage keys are namespaced by user address, and state-changing user operations require that user's authorization.

Relevant coverage:

- `conservation_cross_user_isolation` in `balance_conservation.rs`;
- `prop_cross_user_isolation` in `property_vault_accounting.rs`;
- `multi_lock_cross_user_isolation` in `multi_lock_invariants.rs`;
- isolation snapshots under `contracts/savings_vault/test_snapshots/`.

## 10. Failed Operation Atomicity

A failed contract call must be observationally equivalent to a no-op for accounting state.

At minimum, the following must remain unchanged:

- all affected available balances;
- all affected lock entries;
- all affected next lock ID counters;
- token custody;
- accounting events.

The implementation performs SAC transfers before persistent accounting writes in deposit and withdrawal paths. Soroban transaction rollback is relied upon so that a host-call failure does not leave partial state.

Tests distinguish between:

- validation failures before a token transfer;
- SAC transfer failures;
- repeated failures with no cumulative drift.

Relevant coverage:

- `contracts/savings_vault/src/test/token_transfer_rollback.rs`
- `contracts/savings_vault/src/test/balance_conservation.rs`
- `contracts/savings_vault/src/test/multi_lock_invariants.rs`
- `contracts/savings_vault/src/test/event_compatibility.rs`

## 11. Admin and Pause Isolation

Admin transfer, pause, unpause, and automatic pause expiry must not modify user balances, lock amounts, lock ownership, or token custody.

During a pause:

- deposits and new locks are blocked;
- available withdrawals remain permitted;
- matured lock withdrawals remain permitted;
- existing lock maturity continues to depend only on ledger time.

Relevant coverage:

- `contracts/savings_vault/src/test/admin_invariant_guard.rs`
- `contracts/savings_vault/src/test/pause.rs`

## 12. Withdrawn Funds

The contract does not store a global or per-user cumulative `withdrawn_funds` counter.

Withdrawn value is represented as an accounting flow:

```text
W(u) + WL(u)
```

and by the corresponding reduction in internal liabilities and vault token custody. Auditors must reconstruct cumulative withdrawals from trusted events, transaction history, or an external indexer.

## 13. Known Coverage Gaps

The following areas are not fully proven by the current test suite and should remain visible to reviewers:

1. **Unsolicited token transfers**
   There is no dedicated test proving that direct token transfers to the vault produce `C > T` without crediting a user, while preserving solvency.

2. **Custody-deficit withdrawal rollback**
   Existing tests cover invalid withdrawal amounts and several rollback paths, but there should be an explicit test where internal accounting permits a withdrawal while the SAC transfer fails because contract custody is artificially reduced.

3. **Global liability enumeration**
   The contract has no on-chain user registry, so full-system reconciliation cannot calculate `T` exclusively through contract queries.

4. **Every-key rollback proof**
   Most failure tests snapshot balances, locks, events, or selected counters. They do not formally snapshot every persistent and instance storage key.

5. **TTL-expiry accounting behavior**
   Accounting tests do not simulate persistent-storage expiry or partial expiry of user balance, lock, or lock-counter entries.

6. **Migration preservation**
   Storage-version tests exist, but there is no exhaustive migration property proving preservation of all balances, lock records, counters, and custody across every future storage migration.

7. **Unassigned excess-token handling**
   The contract does not expose a reconciliation or recovery mechanism for tokens transferred directly to the vault outside `deposit`.

## 14. Audit Review Checklist

An auditor should verify that:

- every successful deposit is token-backed;
- available and locked balances are never negative;
- lock creation conserves total user value;
- maturity does not silently reclassify balances;
- `withdraw` cannot consume locked value;
- `withdraw_lock` cannot execute early or twice;
- token custody never falls below aggregate liabilities;
- failed operations produce no state or event drift;
- operations remain isolated by user address;
- admin and pause operations preserve accounting state;
- documented coverage gaps are either accepted or remediated before any future mainnet use.

## Related Documentation

- [Balance Reconciliation](balance-reconciliation.md)
- [Token-Backed Withdrawals](token-backed-withdrawals.md)
- [Multi-Lock Storage](multi-lock-storage.md)
- [Failure Mode Catalogue](failure-mode-catalogue.md)
- [Test Coverage](test-coverage.md)
- [Vault Custody Assumptions](vault-custody-assumptions.md)
- [Admin Pause Threat Model](admin-pause-threat-model.md)
