# Vault Misuse Threat Model

> **Status:** Analysis and documentation — no contract logic changes.
>
> **Scope:** `contracts/savings_vault`
>
> **Network posture:** This contract is intended for development, educational,
> and Stellar testnet use. It is **not production-ready or mainnet-ready**.

## Purpose

This threat model identifies and documents misuse and abuse scenarios for the
Savings Vault contract. It covers scenarios that span users, administrators,
malicious callers, compromised keys, incorrect configuration, misleading UI
assumptions, and future token-custody risks. The goal is to help maintainers,
auditors, SDK integrators, and frontend developers understand what can go wrong
before deploying beyond testnet.

## Scope and related documents

This document covers vault-wide misuse scenarios. It complements the existing
[Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md),
which focuses specifically on the administrator role and pause mechanism.

Related documentation:

- [Authorization Boundaries](authorization-boundaries.md) — Per-function auth rules
- [Authorisation Rules & Security Matrix](authorisation-rules.md) — Security reference
- [Security Review](SECURITY_REVIEW.md) — Full security review
- [Vault Custody Assumptions](vault-custody-assumptions.md) — Token custody analysis
- [Failure Mode Catalogue](failure-mode-catalogue.md) — Contract failure modes
- [Security Checklist](security-checklist.md) — Operational security checklist
- [Audit Readiness Review](audit-readiness.md) — Pre-audit assessment

---

## 1. Assets to Protect

| Asset | Description | Impact if compromised |
| --- | --- | --- |
| User tokens | SAC tokens transferred into the vault through `deposit` | Direct financial loss |
| Internal balances | Per-user available and locked balance entries | Accounting corruption, incorrect withdrawals |
| Lock entries | Per-user lock records with amount and unlock time | Incorrect maturity, loss of locked funds |
| Withdrawal capability | Ability to withdraw available and matured balances | User funds trapped in contract |
| Administrative control | The administrator address and its signing authority | Unauthorized pause, admin transfer |
| SAC token address | The token address configured during `initialize` | Irreversible if wrong; must be correct at init |

---

## 2. Trust Assumptions

These assumptions anchor the threat model. If any is violated, the contract's
security properties may not hold.

1. **Soroban host security is sound.** The contract relies on the Stellar
   Soroban runtime's `require_auth()`, storage isolation, and transaction
   execution guarantees. A vulnerability in the host would affect all contracts
   on the network.

2. **The admin address is properly secured.** The address passed to
   `initialize()` as admin is assumed to be controlled by a trusted party using
   adequate key management (hardware wallet, multi-sig for mainnet). A
   compromised admin key can pause operations indefinitely and transfer
   administration.

3. **Users control their own keys.** The contract assumes each user is the sole
   controller of their Stellar account. Key loss or theft is outside the
   contract's scope.

4. **The SAC token is legitimate.** The token address provided during
   `initialize` is assumed to be a valid Stellar Asset Contract (SAC) with
   standard `transfer`, `balance`, and `approve` semantics. A malicious or
   malfunctioning SAC can break custody guarantees.

5. **Callers initiate correct operations.** The contract does not validate
   intent — it only checks authorization and preconditions. If a user signs a
   transaction with an unintended amount, the contract processes it.

6. **Off-chain UIs display accurate information.** The contract emits events
   and provides read-only queries, but it cannot control how a frontend
   renders balance, lock, or maturity information.

---

## 3. Misuse Scenarios

### 3.1 User Mistakes

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| User deposits to the wrong contract ID | Tokens sent to an unintended contract; the vault's `deposit` rejects the call if the vault SAC transfer fails | Users must verify contract IDs; the contract enforces SAC transfer preconditions |
| User withdraws more than available balance | Transaction fails with panic; no state change | Precondition check in contract |
| User locks funds with an unintended unlock time | Funds locked longer or shorter than intended; no undo | No mitigation in contract; UI should confirm unlock time |
| User calls `withdraw_lock` with the wrong `lock_id` | Fails if lock ID does not exist or belongs to another user | `require_auth()` prevents cross-user access |
| User accidentally re-initializes the contract | `initialize` panics on second call | One-time initialization flag |

### 3.2 Malicious Callers

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| Attacker attempts to withdraw another user's funds | Panics — `withdraw` and `withdraw_lock` call `user.require_auth()` | `require_auth()` on all state-changing functions |
| Attacker attempts to lock another user's funds | Panics — `lock_funds` calls `user.require_auth()` | `require_auth()` on all state-changing functions |
| Attacker calls `deposit` with another user's address | Panics — `deposit` requires the target user's authorization and a SAC transfer from that user | Dual check: auth + SAC transfer from same address |
| Attacker attempts to replay a signed transaction | Not possible — Soroban transactions are validated by the network and each transaction has a unique sequence number | Network-level replay protection |
| Attacker calls functions before initialization | Panics with `Contract is not initialized` | Initialization guard on every public function |
| Attacker initializes the contract with a malicious SAC token | Admin controls `initialize`; only the deployer can set the token address | Only the admin can call `initialize`, and only once |
| Attacker attempts to initialize the contract twice | Panics with `Contract is already initialized` | One-time initialization flag |

### 3.3 Compromised Keys

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| User's Stellar key is compromised | Attacker can withdraw all user's available and matured balances | No on-chain mitigation; key management is user responsibility |
| Admin key is compromised | Attacker can pause/unpause, transfer admin, modify admin | See [Admin Misuse Threat Model](admin-pause-threat-model.md) for detailed analysis |
| SAC token admin key is compromised | Token admin could freeze or seize vault-held tokens | Outside vault contract scope; depends on SAC token implementation |

### 3.4 Incorrect Contract Configuration

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| Deployer initializes with wrong SAC token address | Vault interacts with unintended token; irreversible | UI/scripts should confirm token address before invoking `initialize` |
| Deployer initializes with wrong admin address | Admin authority assigned to incorrect address; irreversible | Confirm admin address before deployment |
| Frontend points to a different (malicious) contract ID | User interacts with a fake vault that may steal funds | Users should verify contract IDs; frontend should use a verified contract registry |
| SDK misconfigured with wrong network passphrase | Transactions fail or are submitted to wrong network | SDK validation of network configuration |

### 3.5 Lock-Related Misuse

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| User creates many small locks | Storage grows linearly; gas costs increase for `list_locks` and `get_locked_balance` | No hard limit; `list_locks` supports pagination |
| User locks the same funds multiple times | Each lock deducts from available balance; repeated locks consume available balance until unlocked | Contract enforces that locked amount is deducted once; only available balance is lockable |
| Lock is created with unlock time far in the future | Funds are locked for an extended period | No maximum unlock time; UI should show lock duration |
| User attempts to withdraw a lock before maturity | `withdraw` checks available balance (excludes unmatured locks); `withdraw_lock` checks maturity | Both paths enforce lock maturity |
| User withdraws a matured lock, then attempts to withdraw it again | Panics — `withdraw_lock` checks lock existence and maturity; lock is consumed on first withdrawal | Replay protection on lock consumption |

### 3.6 Misleading UI and Off-Chain Assumptions

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| UI displays pending deposits that have not been confirmed on-chain | User assumes balance is available when it is not | Applications should wait for transaction confirmation before updating displayed balances |
| UI rounds or truncates i128 amounts | User sees a different amount than what the contract records | Contract uses exact i128 arithmetic; UI should use the raw values from events and queries |
| UI assumes all locks are withdrawable immediately | User attempts early withdrawal and gets a panic | UI should check `get_lock()` unlock_time and `can_withdraw()` before offering withdrawal |
| UI does not display pause status | User tries to deposit during a pause and gets a panic | UI should check `is_paused()` and show a warning before deposit/lock flows |
| Indexer or explorer misreads event topics | Balance or lock events attributed to wrong user or amount | Documented event schema ([docs/events.md](events.md)) for correct parsing |
| User relies on stale `get_balance()` or `get_locked_balance()` | Performs an action based on outdated state | UI should re-query before submitting transactions |

### 3.7 Future Token Custody Risks

These risks apply to the current SAC-backed custody model and may change with
future integration changes.

| Scenario | Risk | Mitigation |
| --- | --- | --- |
| SAC token contract is upgraded or frozen upstream | Vault may lose the ability to transfer tokens | No on-chain mitigation; requires monitoring the token contract |
| SAC token implements non-standard transfer semantics | Vault's transfer logic may behave unexpectedly | Vault assumes standard SAC interface; non-standard tokens may fail or behave incorrectly |
| SAC token admin freezes the vault's token balance | User withdrawals fail because the vault cannot transfer tokens | Outside vault scope; depends on the token's freeze mechanism |
| Token contract changes its decimals or metadata | Displayed amounts may be misleading | Vault uses raw i128 values; frontends should query token metadata independently |
| Vault holds multiple token types | Current design supports one configured SAC token; multi-token not supported | Explicit in documentation; future work item |

---

## 4. Admin-Related Risks

The administrator role introduces centralized control and trust. Key risks are
summarised below; a full analysis is in the
[Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md).

| Risk | Impact | Current status |
| --- | --- | --- |
| Malicious admin pauses indefinitely (by repeatedly calling `pause`) | Blocks deposits and locks; withdrawals remain available | Pause is time-bounded individually but extendable |
| Compromised admin key | Same as malicious admin; attacker gains pause and admin-transfer authority | Single-key trust model; multi-sig recommended for mainnet |
| Accidental admin transfer to incorrect address | Admin authority lost; cannot be recovered | No recovery mechanism; admin transfer is not reversible |
| Lost admin key | Admin functions (pause, unpause, admin transfer) become unavailable | No recovery mechanism documented |
| Admin cannot recover user funds | If token transfer is misconfigured, no emergency withdrawal exists | Explicit design choice; documented in known limitations |

---

## 5. Future SAC Transfer Risks

As the contract uses SAC for token custody, additional risks emerge from the
interaction between vault accounting and real token transfers:

| Scenario | Impact | Notes |
| --- | --- | --- |
| SAC transfer fails during `deposit` | Deposit panics; user's internal balance unchanged | Tested and documented in [Failure Mode Catalogue](failure-mode-catalogue.md) |
| SAC transfer fails during `withdraw` | Withdrawal panics; user's balance unchanged | Tested and documented in [Failure Mode Catalogue](failure-mode-catalogue.md) |
| SAC transfer fails during `withdraw_lock` | Withdrawal panics; lock entry unchanged | Tested and documented in [Failure Mode Catalogue](failure-mode-catalogue.md) |
| Vault SAC token balance is insufficient for an approved withdrawal | Internal check passes but SAC transfer fails | Handled by SAC-level failure propagation |
| Off-chain transfers to the vault address (not through `deposit`) | Tokens held by vault but not credited to any user | No credit path for unsolicited transfers |

For a detailed analysis of custody assumptions and SAC integration, see:
- [Vault Custody Assumptions](vault-custody-assumptions.md)
- [Token-Backed Withdrawals Design Note](token-backed-withdrawals.md)
- [Balance Reconciliation Design Note](balance-reconciliation.md)

---

## 6. Risk Summary

| Risk area | Severity | Exploitability | Notes |
| --- | --- | --- | --- |
| Unauthorized withdrawals | **Low** | Not exploitable | Protected by `require_auth()` on all state-changing functions |
| Replay attacks | **Low** | Not exploitable | Network-level replay protection |
| Compromised user key | **High** | Exploitable | No on-chain mitigation |
| Compromised admin key | **High** | Exploitable (for pause/admin transfer) | Mitigated by multi-sig recommendation |
| Incorrect token address on init | **Critical** | Once (during deployment) | Irreversible; requires careful deployment process |
| Malicious SAC token | **Critical** | Once (during init) | Admin controls token selection |
| Misleading UI | **Medium** | Common | Mitigated by documentation and event schema |
| SAC token malfunction | **Medium** | Rare | Outside vault control; monitored externally |
| Lock manipulation | **Low** | Not exploitable | Enforced by contract logic |
| Storage exhaustion (many locks) | **Low** | Limited | Pagination prevents query exhaustion; storage costs are paid by the user |

---

## References

- [Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md)
- [Authorization Boundaries](authorization-boundaries.md)
- [Authorisation Rules & Security Matrix](authorisation-rules.md)
- [Security Review](SECURITY_REVIEW.md)
- [Vault Custody Assumptions](vault-custody-assumptions.md)
- [Failure Mode Catalogue](failure-mode-catalogue.md)
- [Security Checklist](security-checklist.md)
- [Audit Readiness Review](audit-readiness.md)
- [Balance Reconciliation Design Note](balance-reconciliation.md)
- [Event Schema Documentation](events.md)
