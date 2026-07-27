# Deposit Amount Normalisation & Precision Reference

This document defines the rules, unit representations, precision assumptions, minimum values, and invalid amount behavior for deposits, withdrawals, and fund locks in the **Stellar PocketPay Savings Vault Contract**.

---

## 1. Overview & Unit Definitions

All token and balance amounts in the Savings Vault contract are passed, processed, and stored as signed 128-bit integers (`i128`).

- **Raw Atomic Base Units:** The contract does **not** store floating-point numbers or human-readable decimal representations (e.g. `1.5` XLM). All values represent **raw atomic base units** (e.g. stroops for native XLM).
- **Base Unit Conversion:**
  $$\text{Contract Base Units} = \text{Human Display Value} \times 10^{\text{Decimals}}$$
  *Example for XLM (7 decimals / stroops):*
  - $1.0\text{ XLM} = 10,000,000\text{ stroops}$ (`10_000_000` base units)
  - $0.0000001\text{ XLM} = 1\text{ stroop}$ (`1` base unit)

---

## 2. Minimum Amount & Precision Assumptions

### Minimum Valid Amount
- **Minimum Value:** `1` atomic base unit (`amount >= 1`).
- The smallest deposit, withdrawal, or fund lock allowed by the contract is `1` raw base unit.

### Precision Assumptions
- Contract precision is governed entirely by the underlying Stellar Asset Contract (SAC) token decimals.
- Standard Stellar native asset (XLM) uses 7 decimal places ($10^7$).
- Custom Stellar Asset Contracts (SAC) may use custom decimals (e.g., 6 or 9 decimals).
- Sub-atomic fractions (amounts smaller than 1 atomic base unit) are truncated/not expressible in `i128` base unit representation.

---

## 3. Invalid Amount Behaviour

The contract enforces validation on all amount arguments (`deposit`, `withdraw`, `lock_funds`):

| Condition | Failure Type | Contract Error / Message | Action Required |
|---|---|---|---|
| `amount == 0` | Panic | `"Deposit amount must be greater than zero"` / `"Withdrawal amount must be greater than zero"` / `"Lock amount must be greater than zero"` | Reject input in client/SDK prior to invocation. |
| `amount < 0` | Panic | Same as zero amount error above. | Ensure amounts are positive before invocation. |
| `balance + amount > i128::MAX` | Panic | `"Deposit balance overflow"` | Enforce max limit checks in off-chain transaction builder. |
| `amount > available_balance` | Panic | `"Insufficient balance"` / `"Insufficient balance to lock"` | Check `get_balance(user)` before invoking withdrawal or lock. |

---

## 4. Client & SDK Integration Normalisation Rules

Mobile wallet apps and client SDKs calling the contract must adhere to the following normalisation pipeline before building transaction invocations:

1. **Input Truncation:** Truncate user inputs exceeding the token's decimal precision (e.g., more than 7 decimal places for XLM).
2. **Atomic Conversion:** Multiply the truncated display value by $10^{\text{decimals}}$ to produce the `i128` integer.
3. **Range Check:** Ensure `1 <= amount <= i128::MAX`.
4. **Invocation:** Pass the resulting integer base unit as `amount` to `deposit`, `withdraw`, or `lock_funds`.
