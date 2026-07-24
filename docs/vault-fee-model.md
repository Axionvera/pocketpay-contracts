# Vault Fee Model

## Overview

The Stellar PocketPay Savings Vault contract operates under a **no-fee assumption**: all deposits are credited at 1:1, and withdrawals deduct the exact amount requested. The contract does not support charging fees on user balances, lock operations, or administrative functions.

This document clarifies the current no-fee design, explains how it affects accounting invariants, and defines the framework for potential future fee support.

---

## Current State: No Fees

### What Is Charged?

**The contract charges no fees.**

- **Deposits**: User deposits 100 units → available balance increases by exactly 100 units
- **Withdrawals**: User withdraws 50 units → available balance decreases by exactly 50 units
- **Lock Operations**: Locking funds does not deduct any amount
- **Unlock Operations**: Unlocking matured locks does not charge any fee
- **Admin Operations**: Admin pause/unpause operations do not charge user fees

### How Is This Guaranteed?

The no-fee invariant is enforced through:

1. **Code Design**:
   - The `deposit` function updates user balance without deducting any fee
   - The `withdraw` function deducts exactly the requested amount
   - Lock and unlock operations perform no balance arithmetic beyond moving funds between available and locked states

2. **Test Coverage**:
   - [property_fee_invariants.rs](../contracts/savings_vault/src/test/property_fee_invariants.rs) — Property-based tests that verify deposits credit the exact amount and withdrawals deduct the exact amount across randomized sequences of operations
   - `test_deposit_credits_exact_amount()` — Explicit test of 1:1 deposit accounting
   - `prop_fee_free_one_to_one_accounting()` — Invariant: `available_balance + locked_balance = net_deposited` holds for all operation sequences

3. **Accounting Invariants**:
   - Core invariant: For every user U, `available_balance(U) + locked_balance(U) = net_deposited(U)` (see [Formal Accounting Invariants](accounting-invariants.md#invariant-11-individual-user-balance-conservation))
   - Global invariant: `contract_token_balance = sum_of_all_user_balances` — every token in the contract is accounted for (see [Global Token Custody Invariant](accounting-invariants.md#invariant-12-global-token-custody-invariant))
   - These invariants depend entirely on the no-fee assumption

---

## Accounting Implications

### Invariants That Depend on No Fees

All core accounting invariants assume zero fees:

1. **Individual User Balance Conservation**
   ```
   available_balance(U, T) + locked_balance(U, T) = net_deposited(U, T)
   ```
   This invariant **breaks** if fees are deducted. Example: User deposits 100, is charged 5 fee → balance is 95, but net_deposited is 100.

2. **Global Token Custody**
   ```
   contract_token_balance = sum_of_all_users(available_balance + locked_balance)
   ```
   This invariant **breaks** if fees are collected into an admin account or fee pool. Example: If 10 units are charged as fees and held by the contract, the sum of user balances will be 10 units less than the actual contract balance.

3. **User Isolation**
   Fees must not affect other users' balances. A fee mechanism that transfers units between users would violate isolation (see [Invariant 3.1](accounting-invariants.md#invariant-31-user-balance-isolation)).

4. **Failed Operations Are No-Ops**
   If fees are charged but an operation later fails (e.g., a withdrawal succeeds but a follow-up lock fails), the no-op property is violated. Example: Charge a 1-unit fee upfront, then fail the operation → user is down 1 unit even though the operation failed.

### Impact on Design Decisions

Because of the no-fee assumption:

- **Vault is not self-sustaining**: The contract has no revenue mechanism. Operational costs (storage, versioning, upgrades) must be paid through other means (e.g., foundation funding, admin sponsor).
- **Simple accounting model**: The contract's balance model matches user expectations: "What I deposit is available for withdrawal."
- **No admin privilege over balances**: Fees are not an admin function; the admin cannot selectively charge users.
- **Migration complexity**: Any future fee model will require:
  - Storage schema changes to track accumulated fees
  - New invariants to define fee distribution
  - Backward-compatible initialization and upgrade handling

---

## User Transparency Requirements

### Current User Expectations

Users should understand:

1. **Deposits are 1:1**: If I deposit 100 units, I can withdraw up to 100 units (until it is locked).
2. **Locks are free**: Locking funds does not reduce my balance; it only restricts when I can withdraw.
3. **Withdrawals are exact**: If I request 50 units, exactly 50 units are deducted from my account.

### SDK and Dapp Responsibilities

SDK and dapp implementations must communicate:

1. **No hidden costs**: Clearly document that the vault charges no fees on any operation.
2. **Network fees only**: Fees charged by the Stellar network (transaction submission fees) are separate from vault fees and are out of scope for this contract.
3. **Expected behavior**: Show users the exact amount credited or debited before confirming an operation.

### Example SDK Messaging

```
Deposit: You will deposit 100 USDC.
Expected vault balance after: current_balance + 100 USDC
(Network fees apply separately)

Withdraw: You will withdraw 50 USDC.
Expected vault balance after: current_balance - 50 USDC
(Network fees apply separately)

Lock: You will lock 100 USDC until [date].
Expected available balance: current_available - 100 USDC
Expected locked balance: current_locked + 100 USDC
(No fees charged)
```

---

## Design Rationale: Why No Fees Now?

The contract does not support fees because:

1. **Scope clarity**: Fees add complexity to accounting, admin roles, and upgrade paths. The current design prioritizes simplicity and correctness of core balance operations.

2. **Operational model unclear**: It is not yet decided:
   - Who receives fee revenue (admin, foundation, user governance)?
   - How are fees calculated (flat, percentage, tiered)?
   - Are fees applied equally to all users or based on governance?
   - How are fees handled during emergencies or pauses?

3. **Test coverage**: Extensive tests verify that the vault is **fee-free**, making it safe to add fees later only after explicit design.

4. **Audit readiness**: A fee-free design is simpler to audit. Adding fees later requires explicit protocol review.

---

## Framework for Future Fee Support

If fees are added in a future version, the following must be true:

### Design Requirements

1. **Explicit fee invariants**: Define new accounting invariants that describe:
   - How fees are calculated and deducted
   - Where accumulated fees are stored
   - Under what conditions fees are redistributed
   - How fees interact with locks, withdrawals, and the pause mechanism

2. **Fee impact on user balance conservation**:
   ```
   available_balance(U, T) + locked_balance(U, T) + fees_charged(U, T) = net_deposited(U, T)
   ```
   The sum must still equal the amount the user provided.

3. **No cross-user fee transfer**: Fees must not transfer value between users. Fees may accumulate in:
   - An explicit "fee pool" account, or
   - A per-admin account, or
   - A separate fee contract
   
   But they must not reduce User A's balance to increase User B's balance.

4. **Fees must be transparent**:
   - Users can query accumulated fees on their account
   - Fee calculation is deterministic and documented
   - Users are notified before operations that incur fees

### Implementation Requirements

1. **Storage Schema Changes**:
   - Add `fees_charged: i128` per user, or
   - Add a contract-level `total_fees_collected: i128` and `fee_destination: Address`, or
   - Add a separate "fee_account" user entry

2. **New Operations** (examples):
   - `get_fees_charged(user)` — Query fees accumulated on a user's account
   - `claim_fees(admin, destination)` — Admin withdraws accumulated fees
   - `set_fee_model(admin, config)` — Admin configures fee behavior (if governance is designed)

3. **Backward Compatibility**:
   - Existing users and locks must not retroactively have fees applied
   - Storage versioning and migration must be planned (see [Storage Migration Guide](storage-migration.md))
   - Upgrade path must not break existing contracts or SDKs

4. **Test Coverage**:
   - Property tests verifying the new fee invariant across all operation sequences
   - Tests verifying fees do not violate user isolation
   - Tests verifying failed operations do not charge fees
   - Tests verifying fee calculation matches specification

5. **Audit Requirements**:
   - All fee calculations must be formally specified
   - All new invariants must be tested with the same rigor as current invariants
   - Admin capabilities with respect to fees must be threat-modeled

---

## Checking for Violations

To verify that the vault remains fee-free:

1. **Code review checklist**:
   - [ ] No deposit operation reduces the credited amount
   - [ ] No withdraw operation charges a fee before deducting
   - [ ] No lock or unlock operation deducts from balance
   - [ ] No new storage keys accumulate a "fee pool" value
   - [ ] All balance updates are addition or subtraction, with no other arithmetic

2. **Test verification**:
   ```bash
   cargo test property_fee_invariants
   cargo test prop_fee_free_one_to_one_accounting
   cargo test prop_no_fee_token_custody
   cargo test test_deposit_credits_exact_amount
   ```

3. **Audit criteria**:
   - All invariants in [Formal Accounting Invariants](accounting-invariants.md) must pass
   - No new fields in storage that track fees or per-user fee states
   - All property tests must pass with no fee-related regressions

---

## Related Documentation

- [Formal Accounting Invariants](accounting-invariants.md) — Complete specification of balance invariants
- [Balance Reconciliation Design Note](balance-reconciliation.md) — How internal accounting reconciles with real token balances
- [Storage Migration Guide](storage-migration.md) — How to safely version and upgrade storage
- [Failure Mode Catalogue](failure-mode-catalogue.md) — All contract failure modes and their expected behavior
- [SDK Error Mapping Guide](sdk-error-mapping-guide.md) — How SDKs should communicate vault errors to users

---

## Summary

The Savings Vault **has no fees**. This is:

- ✅ **Guaranteed by code**: Deposit and withdraw functions use exact arithmetic
- ✅ **Verified by tests**: Property tests ensure 1:1 accounting invariants hold
- ✅ **Documented for users**: SDKs and dapps should communicate exact amounts
- ✅ **Open to future extension**: If fees are added, they must preserve invariants and go through explicit design review

Do not add fees to the vault without:
1. Explicitly designing the fee model
2. Updating all affected invariants
3. Adding comprehensive test coverage
4. Planning storage versioning and upgrades
5. Conducting threat modeling for fee-related admin capabilities
