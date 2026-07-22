# SDK Error Mapping Guide

This guide maps savings vault contract errors to expected SDK-level handling.
It is intended for SDK and mobile app developers integrating with the vault
contract on testnet.

> **Not production-ready.** This contract is for educational and testnet use.
> Error behavior may change before any mainnet deployment.

## How errors surface

The contract uses Rust `panic!()` messages for validation failures and
Soroban host-level traps for authorization and token errors. There are no
stable numeric error codes. The SDK should **not** branch on panic message
text; instead, treat any failed invocation as a general failure and surface a
user-friendly message while retaining the diagnostic for debugging.

A failed invocation does **not** commit that invocation's state changes.

## Error categories

| Category | Contract errors | SDK action |
|---|---|---|
| Already initialized | `"Contract is already initialized"` | Show "Vault is already set up." Do not retry. |
| Not initialized | Storage unwrap trap (no message) | Show "Vault is not initialized." Prompt admin to run `initialize`. |
| Invalid amount | `"Deposit amount must be greater than zero"`, `"Withdrawal amount must be greater than zero"`, `"Lock amount must be greater than zero"` | Validate amount > 0 before submitting. Show "Please enter a valid amount." |
| Insufficient balance | `"Insufficient balance"`, `"Insufficient balance to lock"` | Refresh `get_balance` and compare. Show "Not enough available funds." |
| Invalid unlock time | `"Unlock time must be in the future"` | Validate unlock_time > current ledger timestamp before submitting. Show "Unlock time must be in the future." |
| Unauthorized | Soroban host auth failure (no contract message) | Request the correct wallet signature. Show "Transaction not authorized." |
| Token transfer failure | Error from the configured token contract | Inspect nested diagnostic. Show "Transfer failed." Verify token address and vault token balance. |

## User-facing examples

These are the messages the mobile app should show for each failure.

| Trigger | User message |
|---|---|
| Re-initialization | "Vault is already set up." |
| Deposit/withdraw/lock with zero or negative amount | "Please enter a valid amount." |
| Withdraw more than available | "Not enough available funds." |
| Lock more than available | "Not enough available funds." |
| Unlock time in the past | "Unlock time must be in the future." |
| Missing wallet signature | "Transaction not authorized." |
| Token transfer error | "Transfer failed. Please try again." |
| Vault not initialized | "Vault is not initialized." |

## Developer-facing examples

### Pre-flight validation

Validate inputs before submitting to avoid unnecessary round trips:

```rust
// Reject non-positive amounts before invocation
if amount <= 0 {
    return Err(SdkError::InvalidAmount);
}

// Reject past unlock times
if unlock_time <= current_ledger_timestamp {
    return Err(SdkError::InvalidUnlockTime);
}
```

### Balance check before withdraw or lock

```rust
let available = contract.get_balance(user.clone());
if amount > available {
    return Err(SdkError::InsufficientBalance);
}
```

### General error handling

Since the contract has no stable error codes, catch failures generically:

```rust
match contract.try_deposit(user.clone(), amount) {
    Ok(_) => show_success("Deposit confirmed."),
    Err(_) => show_error("Something went wrong. Please try again."),
}
```

For production-quality SDKs, log the full diagnostic (including any panic
message or host error) for debugging while showing the user a generic
friendly message.

### Authorization flow

The SDK must ensure the user signs the transaction before submission. If the
contract returns an authorization error, the SDK should prompt for a wallet
signature rather than retrying silently:

```rust
match contract.try_withdraw(user.clone(), amount) {
    Ok(tx_hash) => show_success("Withdrawal confirmed."),
    Err(SdkError::Unauthorized) => prompt_wallet_signature(),
    Err(SdkError::InsufficientBalance) => {
        let available = contract.get_balance(user.clone());
        show_error(format!("You can withdraw up to {}.", available));
    }
    Err(e) => show_error("Something went wrong."),
}
```

## Further reading

- [Contract Error Reference](error-codes.md) — full list of current failure
  conditions
- [SDK Sequence Diagrams](sdk-contract-sequence.md) — interaction flows
  including error paths
- [Architecture Documentation](architecture.md) — storage model and contract
  structure
