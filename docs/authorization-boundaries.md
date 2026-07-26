# Authorization Boundaries
## Savings Vault Contract
---

## Overview
This document defines the authorization rules for every public function in the Savings Vault contract, documents assumptions, and links to relevant tests.

---
## Public Functions Authorization Rules
| Function | Authorized Caller(s) | Authorization Mechanism | State-Changing? |
|----------|----------------------|-------------------------|-----------------|
| [initialize(env, admin, token)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L248) | Admin (only once!) | `admin.require_auth()` + initialization check | ✅ Yes |
| [get_version(env)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L300) | Anyone (public) | None | ❌ No |
| [deposit(env, user, amount)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L333) | The `user` address | `user.require_auth()` | ✅ Yes |
| [withdraw(env, user, amount)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L413) | The `user` address | `user.require_auth()` | ✅ Yes |
| [withdraw_lock(env, user, lock_id)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L538) | The `user` address | `user.require_auth()` | ✅ Yes |
| [get_balance(env, user)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L629) | Anyone (public) | None | ❌ No |
| [lock_funds(env, user, amount, unlock_time)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L689) | The `user` address | `user.require_auth()` | ✅ Yes |
| [get_locked_balance(env, user)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L805) | Anyone (public) | None | ❌ No |
| [get_lock(env, user, lock_id)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L895) | Anyone (public) | None | ❌ No |
| [list_locks(env, user, offset, limit)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L921) | Anyone (public) | None | ❌ No |
| [can_withdraw(env, user)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L862) | Anyone (public) | None | ❌ No |
| [get_admin(env)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L961) | Anyone (public) | None | ❌ No |
| [transfer_admin(env, admin, new_admin)](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/lib.rs#L985) | Current admin | `admin.require_auth()` + admin check | ✅ Yes |

---
## Authorization Assumptions
1. **Soroban `require_auth()` is secure**: We rely on Soroban's built-in `Address::require_auth()` to verify that the caller has authorized the operation (via signature, Soroban auth entries, etc.).
2. **Admin address is secure**: The address provided to `initialize()` as admin is assumed to be a secure, controlled address (e.g., multisig, hardware wallet).
3. **User addresses are secure**: Users are responsible for managing their own private keys and not sharing them with unauthorized parties.

---
## Misuse Scenarios & Expected Behavior
### Scenario 1: Call `initialize` again after first initialization
- **Expected Behavior**: Panics with message `Contract is already initialized`
- **Test**: [initialization.rs::test_initialize_twice_panics](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/initialization.rs)

### Scenario 2: Call `deposit` without user authorization
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [unauthorized_access.rs::test_unauthorized_deposit_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs#L10)

### Scenario 3: Call `withdraw` without user authorization
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [unauthorized_access.rs::test_unauthorized_withdraw_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs#L24)

### Scenario 4: Call `withdraw_lock` without user authorization
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [unauthorized_access.rs::test_unauthorized_withdraw_lock_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs#L51)

### Scenario 5: Call `lock_funds` without user authorization
- **Expected Behavior**: Panics from `user.require_auth()`
- **Test**: [unauthorized_access.rs::test_unauthorized_lock_fails](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/unauthorized_access.rs#L37)

### Scenario 6: Query `get_balance`/`get_locked_balance`/`get_lock`/`list_locks`/`can_withdraw` for any user
- **Expected Behavior**: Returns correct value (no authorization required for read-only queries)
- **Tests**: All get_* tests work for any user!

### Scenario 7: Call `transfer_admin` as non-admin
- **Expected Behavior**: Panics with message `Not authorized`
- **Test**: [admin_invariant_guard.rs::test_non_admin_cannot_transfer_admin](file:///c:/Users/muham/.trae/Grantfox%20Coder%20x/pocketpay-contracts/contracts/savings_vault/src/test/admin_invariant_guard.rs)

---
## Test Coverage
| Misuse Scenario | Test Exists? |
|-----------------|--------------|
| Double initialization | ✅ Yes |
| Unauthorized withdraw | ✅ Yes |
| Unauthorized deposit | ✅ Yes |
| Unauthorized lock | ✅ Yes |
| Unauthorized withdraw_lock | ✅ Yes |
| Cross-user balance queries (allowed) | ✅ Yes |
| Unauthorized admin transfer | ✅ Yes |
