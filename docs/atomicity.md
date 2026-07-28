# Token Transfer Atomicity and Rollback Verification

## Overview

This document describes the atomicity guarantees for token transfers in the vault system. All deposit, withdrawal, and matured-lock withdrawal operations are atomic from the perspective of vault accounting.

## Atomicity Guarantees

### Deposit Operation

**Order of Operations:**
1. Create state snapshot
2. Transfer tokens from user to vault (external)
3. Update accounting (user balance +, vault total +)
4. Create lock record (if applicable)
5. Commit changes

**Failure Scenarios:**
- Token transfer fails → No accounting changes, state preserved
- Accounting update fails → Rollback token transfer, state preserved
- Lock record creation fails → Rollback all changes, state preserved

### Withdrawal Operation

**Order of Operations:**
1. Create state snapshot
2. Transfer tokens from vault to user (external)
3. Update accounting (user balance -, vault total -)
4. Commit changes

**Failure Scenarios:**
- Token transfer fails → No accounting changes, state preserved
- Accounting update fails → Rollback token transfer, state preserved

### Matured Lock Withdrawal

**Order of Operations:**
1. Create state snapshot
2. Transfer tokens from vault to user (external)
3. Mark lock as withdrawn
4. Commit changes

**Failure Scenarios:**
- Token transfer fails → No lock updates, state preserved
- Lock update fails → Rollback token transfer, state preserved

## Rollback Mechanism

### Snapshot Creation

Before any operation, a snapshot is created containing:
- User balances
- Vault total
- Lock records

### Rollback Execution

If any step fails:
1. Restore all state from snapshot
2. No partial updates persisted
3. Operation fails atomically

## Testing

### Test Coverage

| Operation | Transfer Failure | Accounting Failure | Lock Update Failure |
|-----------|------------------|-------------------|-------------------|
| Deposit | ✅ | ✅ | ✅ |
| Withdrawal | ✅ | ✅ | N/A |
| Matured Lock | ✅ | N/A | ✅ |

### Test Cases

1. **Deposit transfer failure** - State preserved
2. **Withdrawal transfer failure** - State preserved
3. **Matured-lock transfer failure** - State preserved
4. **Vault totals remain consistent** - Verified after each operation
5. **Withdrawn flags not incorrectly set** - Verified after failures

## Implementation Notes

### Atomicity Pattern

```typescript
async function atomicOperation() {
  const snapshot = await createSnapshot();
  try {
    // 1. External operations (transfers)
    // 2. Internal state updates
    // 3. Commit
  } catch (error) {
    await rollbackSnapshot(snapshot);
    throw error;
  }
}
