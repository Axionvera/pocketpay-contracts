# Token-Backed Withdrawals Architecture & Specification

## Overview

The **Savings Vault Contract** implements **token-backed withdrawals** integrating directly with the **Stellar Asset Contract (SAC)** interface (`soroban_sdk::token::Client`). 

When users deposit funds into or withdraw funds from the vault, real tokens are transferred into and out of the contract address on the Stellar Soroban ledger. Accounting balances (`Balance(user)` and `Locks(user)`) reconcile 1:1 with real SAC token balances held under contract custody.

---

## Acceptance Criteria & Key Mechanics

### 1. Token Transfer to User (`withdraw`)
When a user calls `withdraw(env, user, amount)`:
- The contract invokes `token_client.transfer(&contract_address, &user, &amount)`.
- Real tokens are transferred from contract custody back to the user's wallet address on the Stellar ledger.
- If the SAC transfer fails (e.g. frozen asset or contract balance mismatch), the entire Soroban transaction reverts with zero on-chain state changes.

```
┌──────────────┐     withdraw(user, amount)      ┌─────────────────────────────┐
│ Mobile User  ├────────────────────────────────►│ SavingsVault Smart Contract │
└──────┬───────┘                                 └──────────────┬──────────────┘
       │                                                        │
       │                                  token_client.transfer │
       │◀───────────────────────────────────────────────────────┘
       │                SAC Tokens Handed Back
```

---

### 2. Strict Authorization Requirement
- `withdraw` explicitly enforces `user.require_auth()`.
- Only the authenticated address matching `user` can initiate withdrawals of their deposited or matured funds. Third parties or unauthorized callers cannot withdraw user assets.

---

### 3. Protection of Locked Funds (Maturity Check)
- A user's total withdrawable funds = regular deposited `Balance(user)` + sum of matured lock amounts (`current_time >= unlock_time`).
- Active/unmatured locks (`current_time < unlock_time`) are **excluded** from withdrawable balance.
- If `amount > available_unlocked_balance`, `withdraw` panics with `"Insufficient balance"`.
- This guarantees that **locked funds cannot be withdrawn prior to maturity**.

---

### 4. Deduction Sequence & State Accounting
When a valid withdrawal is executed:
1. `amount` is deducted from liquid `Balance(user)` first.
2. If `amount > Balance(user)`, remaining deduction is applied against matured entries in `Locks(user)`.
3. Liquid `Balance(user)` and `Locks(user)` vector are updated in persistent storage.

---

## Security & Invariant Guarantees

1. **Conservation of Assets**:
   $$\text{Contract Token Custody} = \sum_{\text{users}} \left( \text{Balance}(\text{user}) + \text{Locked}(\text{user}) \right)$$
2. **Revert Atomicity**: State changes and SAC token transfers are executed in a single atomic transaction. Reverts leave balances unaltered.
3. **Multi-User Isolation**: Balance deductions operate strictly on the caller's key (`Balance(user)`), preventing cross-user balance interference.
