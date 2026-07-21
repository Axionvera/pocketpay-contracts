# Authorization Boundaries
## Savings Vault Contract
---

## Overview
This document defines the authorization rules for every public function in the Savings Vault contract, documents assumptions, and links to relevant tests.

---
## Public Functions Authorization Rules
| Function | Authorized Caller(s) | Authorization Mechanism | State-Changing? |
|----------|----------------------|-------------------------|-----------------|
| `initialize(env, admin, token)` | Admin (only once!) | `admin.require_auth()` | ✅ Yes |
| `deposit(env, user, amount)` | The `user` address | `user.require_auth()` | ✅ Yes |
| `withdraw(env, user, amount)` | The `user` address | `user.require_auth()` | ✅ Yes |
| `get_balance(env, user)` | Anyone (public) | None | ❌ No |
| `lock_funds(env, user, amount, unlock_time)` | The `user` address | `user.require_auth()` | ✅ Yes |
| `get_locked_balance(env, user)` | Anyone (public) | None | ❌ No |
| `get_lock(env, user, lock_id)` | Anyone (public) | None | ❌ No |
| `list_locks(env, user, offset, limit)` | Anyone (public) | None | ❌ No |
| `can_withdraw(env, user)` | Anyone (public) | None | ❌ No |

---
## Authorization Assumptions
1. **Soroban `require_auth()` is secure**: We rely on Soroban's built-in `Address::require_auth()` to verify that the caller has authorized the operation (via signature, Soroban auth entries, etc.).
2. **Admin address is secure**: The address provided to `initialize()` as admin is assumed to be a secure, controlled address (e.g., multisig, hardware wallet).
3. **User addresses are secure**: Users are responsible for managing their own private keys and not sharing them with unauthorized parties.

---
## Misuse Scenarios & Expected Behavior
### Scenario 1: Call `initialize` again after first initialization
- **Expected Behavior**: Panics with message `Contract is already initialized`
- **Test**: [test_initialize_twice_panics](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L26)

### Scenario 2: Call `deposit` for user A as user B (not authorized)
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [test_withdraw_requires_user_authorization](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L193) (similar, same mechanism)

### Scenario 3: Call `withdraw` for user A as user B
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [test_withdraw_requires_user_authorization](file:///c:/Users/Muhammad/.trae/Grantfox/pocketpay-contracts/contracts/savings_vault/src/test/mod.rs#L193)

### Scenario 4: Call `lock_funds` for user A as user B
- **Expected Behavior**: Panics from `user.require_auth()`

### Scenario 5: Query `get_balance`/`get_locked_balance`/`get_lock`/`list_locks`/`can_withdraw` for any user
- **Expected Behavior**: Returns correct value (no authorization required for read-only queries)
- **Tests**: All get_* tests work for any user!

---
## Test Coverage
| Misuse Scenario | Test Exists? |
|-----------------|--------------|
| Double initialization | ✅ Yes |
| Unauthorized withdraw | ✅ Yes |
| Unauthorized deposit | Implied (same mechanism as withdraw) |
| Unauthorized lock | ❌ No (though mechanism is identical to withdraw/deposit) |
| Cross-user balance queries (allowed) | ✅ Yes |
