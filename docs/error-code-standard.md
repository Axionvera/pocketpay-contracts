# Savings Vault Error Code Standard

This document defines the standard error code structure for the Savings Vault contract and provides guidance for SDK mapping and mobile UX implementation.

## Error Code Structure

Error codes are organized by category with stable numeric values to enable reliable SDK mapping and consistent user experience across platforms.

### Category Ranges

- **1000-1999**: Configuration errors
- **2000-2999**: Validation errors  
- **3000-3999**: Authorization errors
- **4000-4999**: Balance errors
- **5000-5999**: Lock errors

### Error Code Reference

| Code | Error Variant | Category | Description |
|------|--------------|----------|-------------|
| 1001 | `AlreadyInitialized` | Configuration | Contract has already been initialized |
| 1002 | `NotInitialized` | Configuration | Contract has not been initialized |
| 2001 | `InvalidDepositAmount` | Validation | Deposit amount must be greater than zero |
| 2002 | `InvalidWithdrawAmount` | Validation | Withdrawal amount must be greater than zero |
| 2003 | `InvalidLockAmount` | Validation | Lock amount must be greater than zero |
| 2004 | `InvalidUnlockTime` | Validation | Unlock time must be in the future |
| 3001 | `Unauthorized` | Authorization | Missing required authorization |
| 4001 | `InsufficientBalance` | Balance | Insufficient balance for withdrawal |
| 4002 | `InsufficientBalanceToLock` | Balance | Insufficient balance to lock funds |
| 4003 | `FundsLockedUntilMaturity` | Balance | Cannot withdraw: funds are locked until maturity |
| 5001 | `LockNotFound` | Lock | No lock found (reserved for future use) |

## SDK Mapping Guidance

### Error Handling Pattern

SDKs should map error codes to platform-specific error types while preserving the numeric code for logging and analytics.

#### Example: JavaScript/TypeScript SDK

```typescript
enum SavingsVaultError {
  AlreadyInitialized = 1001,
  NotInitialized = 1002,
  InvalidDepositAmount = 2001,
  InvalidWithdrawAmount = 2002,
  InvalidLockAmount = 2003,
  InvalidUnlockTime = 2004,
  Unauthorized = 3001,
  InsufficientBalance = 4001,
  InsufficientBalanceToLock = 4002,
  FundsLockedUntilMaturity = 4003,
  LockNotFound = 5001,
}

class VaultError extends Error {
  constructor(
    public code: SavingsVaultError,
    message: string,
    public contractMessage?: string
  ) {
    super(message);
    this.name = 'VaultError';
  }
}

function mapContractError(error: any): VaultError {
  const code = error.code as number;
  const message = error.message as string;
  
  // Map to SDK error type
  if (Object.values(SavingsVaultError).includes(code)) {
    return new VaultError(code, getErrorMessage(code), message);
  }
  
  // Unknown error
  return new VaultError(-1, 'Unknown vault error', message);
}

function getErrorMessage(code: SavingsVaultError): string {
  switch (code) {
    case SavingsVaultError.AlreadyInitialized:
      return 'Vault has already been initialized';
    case SavingsVaultError.InvalidDepositAmount:
      return 'Deposit amount must be greater than zero';
    case SavingsVaultError.FundsLockedUntilMaturity:
      return 'Funds are locked until maturity date';
    // ... other cases
    default:
      return 'An error occurred';
  }
}
```

#### Example: Rust SDK

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("Contract error {code}: {message}")]
    ContractError { code: u32, message: String },
    
    #[error("SDK error: {0}")]
    SdkError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionError(String),
}

impl VaultError {
    pub fn from_contract_error(code: u32, message: &str) -> Self {
        VaultError::ContractError {
            code,
            message: message.to_string(),
        }
    }
    
    pub fn code(&self) -> Option<u32> {
        match self {
            VaultError::ContractError { code, .. } => Some(*code),
            _ => None,
        }
    }
}
```

## Mobile UX Guidance

### User-Friendly Messages

Map error codes to user-friendly messages appropriate for mobile UI:

| Error Code | User Message | Action |
|------------|--------------|--------|
| 2001, 2002, 2003 | "Please enter a valid amount" | Show input validation error |
| 2004 | "Unlock time must be in the future" | Show date picker validation |
| 4001 | "Insufficient balance" | Show current balance, disable action |
| 4002 | "Not enough funds to lock" | Show available balance, adjust lock amount |
| 4003 | "Funds are locked until [date]" | Show unlock date, disable withdrawal |
| 3001 | "Authorization required" | Prompt user to sign transaction |

### Error Recovery Strategies

**Validation Errors (2000-2999)**
- Pre-validate user input before contract invocation
- Show inline validation messages
- Disable invalid actions

**Balance Errors (4000-4999)**
- Display current available balance
- For locked funds (4003), show unlock date/time
- Provide countdown timer for lock maturity
- Enable "Notify when available" feature

**Authorization Errors (3000-3999)**
- Clear sign request prompts
- Explain why authorization is needed
- Handle wallet connection issues gracefully

**Configuration Errors (1000-1999)**
- These are typically deployment/setup issues
- Show clear error to developers/admins
- Provide troubleshooting steps

### Analytics and Logging

Log error codes for monitoring and improvement:

```typescript
function logError(error: VaultError, context: any) {
  analytics.track('vault_error', {
    code: error.code,
    action: context.action,
    user: context.userId,
    timestamp: Date.now(),
  });
  
  // Also log to error tracking service
  errorTracking.captureException(error, {
    tags: {
      vaultError: error.code.toString(),
    },
    extra: context,
  });
}
```

## Error Code Stability

Error codes are **stable** and **backward compatible**:

- Existing error codes will never change
- New error codes will be added within their category ranges
- Deprecated error codes will be marked in documentation but remain functional
- Major version changes may introduce new categories but preserve existing codes

## Testing Guidance

SDKs should include tests for error handling:

```typescript
describe('Vault Error Mapping', () => {
  it('should map insufficient balance error correctly', () => {
    const contractError = { code: 4001, message: 'Insufficient balance' };
    const vaultError = mapContractError(contractError);
    
    expect(vaultError.code).toBe(SavingsVaultError.InsufficientBalance);
    expect(vaultError.message).toContain('Insufficient balance');
  });
  
  it('should handle unknown error codes', () => {
    const contractError = { code: 9999, message: 'Unknown error' };
    const vaultError = mapContractError(contractError);
    
    expect(vaultError.code).toBe(-1);
  });
});
```

## Future Extensions

The error code standard is designed for extensibility:

- **6000-6999**: Reserved for future features (e.g., staking, yield)
- **7000-7999**: Reserved for integration errors (e.g., token bridge)
- **8000-8999**: Reserved for governance errors
- **9000-9999**: Reserved for system-level errors

When adding new error codes:
1. Choose the appropriate category range
2. Document the error in this standard
3. Update SDK mappings
4. Add test coverage
5. Communicate changes to SDK maintainers
