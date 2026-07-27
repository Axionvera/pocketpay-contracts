# Vault Economic Assumptions and Fee Model Review

## Purpose

A savings vault carries economic assumptions even when it charges no fees:
who bears custody risk, what incentivizes locking funds instead of just
holding them, how the contract behaves if the underlying token misbehaves,
how much power the admin key holds, and what users are likely to assume
that the contract does not actually promise. This document collects those
assumptions in one place for users, integrators, and auditors. It does not
introduce any new contract behavior — it is a review of the current
implementation and its implications.

**Related documents**: this review summarizes and cross-links
[Vault Fee Model](vault-fee-model.md) (the authoritative fee/no-fee
specification), [Vault Custody Assumptions](vault-custody-assumptions.md)
(token custody guarantees), and [Formal Accounting Invariants](accounting-invariants.md)
(the invariants these assumptions depend on). Read those for implementation
detail; read this for the economic reasoning that ties them together.

---

## 1. Fee Model: Summary

**The vault charges no fees of any kind.** Deposits, withdrawals,
`withdraw_lock`, and `extend_lock` all move exactly the stated amount —
no percentage cut, no flat charge, no spread.

The full specification — including the accounting invariants this depends
on, the framework required before fees could ever be added, and a
code-review checklist for verifying the vault stays fee-free — lives in
[Vault Fee Model](vault-fee-model.md). This review does not repeat that
detail; it only draws out the consequences below.

**Consequence**: the vault is not self-sustaining. It has no revenue
mechanism, so operational costs (ledger rent for storage TTL, deployment,
future audits) must be funded externally (e.g., by the deploying team),
not by the vault itself. Anyone integrating this contract should not
assume it will ever generate protocol revenue in its current form.

---

## 2. Custody Assumptions: Summary

Token custody guarantees (deposit/withdrawal atomicity, balance
conservation, what the admin can and cannot touch) are specified in full
in [Vault Custody Assumptions](vault-custody-assumptions.md). The
economically relevant points for this review:

- The vault assumes the configured token is a **standard, well-behaved
  SAC** — no transfer fees, no rebasing, no blacklisting. A misbehaving
  token breaks the 1:1 accounting the whole fee-free model rests on (see
  [Vault Custody Assumptions §2.2](vault-custody-assumptions.md#22-token-behavior-standards)).
- The vault does not verify token solvency at initialization or on each
  call; it trusts that `deposit`'s SAC transfer succeeded and mirrors that
  in internal state.
- There is no yield, interest, or staking reward. Depositing is a pure
  custody/access-control action, not a savings-with-return product,
  despite the "Savings Vault" name (see
  [Vault Custody Assumptions §3.3](vault-custody-assumptions.md#33-no-interest-or-yield)).

---

## 3. Lock Duration Incentives

Locking funds (`lock_funds`) has **no direct financial incentive** in the
current contract — no bonus interest, no fee discount, no governance
weight. The only effect of a lock is that funds become unavailable via
`withdraw` until `unlock_time`, and must instead be claimed one-by-one via
`withdraw_lock` once matured.

Given that, the vault's actual incentive structure is:

| Behavior | What the contract does | Economic implication |
| --- | --- | --- |
| Depositing without locking | Funds sit in `unlocked` balance, withdrawable any time | No reason not to keep everything unlocked unless the user wants self-imposed discipline |
| Locking funds | Funds move to a `LockEntry`, blocked from `withdraw` until maturity | Purely a user-side commitment device (e.g., "don't let me spend this for 90 days") — not a yield product |
| Extending a lock (`extend_lock`) | Pushes `unlock_time` further out, no principal change | Lets a user voluntarily recommit; cannot be used to *shorten* a lock, so it cannot be used to escape a commitment early |
| Admin lock duration bounds (`set_min_lock_duration` / `set_max_lock_duration`) | Constrain the *range* of durations a user may choose | Protects against pathological locks (e.g., 1-second or 100-year locks) but do not change the incentive itself |

**Why this matters economically**: because there is no yield or reward
attached to locking, the vault should not be marketed or perceived as a
"staking" or "interest-bearing" product. Any SDK or mobile copy that
implies a locked balance "grows" or "earns" would misrepresent the
contract. The only user-facing benefit of locking is behavioral
(commitment / delayed access), not financial.

If a future version wants locking to carry real incentive (e.g., an
interest bonus for longer locks), that is a **new economic feature**, not
a parameter change, and it would need its own accounting invariants (see
[Vault Fee Model §Framework for Future Fee Support](vault-fee-model.md#framework-for-future-fee-support)
for the shape that kind of change would need to take — a yield mechanism
has the same "who funds it, how is it tracked, does it break balance
conservation" questions that a fee mechanism does).

---

## 4. Token Behavior Risk

The vault's accounting model assumes the configured SAC token is
transparent and non-adversarial:

- **Fee-on-transfer tokens**: if the configured token deducts a fee on
  `transfer`, the vault will credit the user the full requested amount
  internally while having received less than that from the SAC call,
  silently creating a solvency gap between `contract_token_balance` and
  the sum of internal user balances. The vault does not detect or guard
  against this.
- **Blacklist/pausable tokens**: if the token issuer can freeze an
  address, a user (or the vault contract address itself) being
  blacklisted would cause `deposit`/`withdraw` to fail even though the
  vault's own logic is correct. This is an external dependency risk, not
  a vault bug.
- **Token substitution**: `initialize(admin, token)` accepts any
  `Address` with no validation that it behaves like a SAC. Deploying
  teams are responsible for configuring a trustworthy token; the vault
  provides no on-chain token vetting.

See [Vault Custody Assumptions §2.1–2.2](vault-custody-assumptions.md#2-what-the-vault-does-not-guarantee)
for the full non-guarantee list this risk falls under.

---

## 5. Admin Power and Risk

The admin is a **single key** (no multi-sig) with the following economic
reach:

**Can do**:
- Pause the contract for a bounded duration, blocking new deposits and
  locks (withdrawals remain open the whole time).
- Transfer the admin role to a new address.
- Set `min_deposit_amount`, `max_lock_duration_secs`, and
  `min_lock_duration_secs` — parameters that constrain future user
  actions, not existing balances.

**Cannot do**:
- Withdraw, move, or reduce any user's balance or lock.
- Change the amount or maturity of an existing `LockEntry`.
- Bypass a user's `require_auth()` on their own funds.

**Economic risk from a compromised or malicious admin**: the worst a
compromised admin key can do is deny service (repeated pausing, or
setting `min_deposit_amount`/lock-duration bounds to values that make
normal use impractical) — it cannot directly steal funds. See the
[Emergency Pause and Admin Misuse Threat Model](admin-pause-threat-model.md)
for the full scenario-by-scenario breakdown, and [Admin Role](admin-role.md)
for what `initialize(admin)` actually records.

This still means the admin key is a real economic dependency: users are
trusting that the current admin will not use configuration parameters
(especially `min_deposit_amount` and lock duration bounds) to grief the
vault, and that admin key custody is handled at least as carefully as a
production signing key, even though this contract targets testnet only.

---

## 6. User Misunderstanding Scenarios

These are the gaps between what a typical user might assume and what the
contract actually does — worth calling out explicitly since the contract
is named "Savings Vault":

1. **"My locked funds are earning interest."** False — see §3. Locking
   only restricts access; it does not increase balance.
2. **"The vault takes a small fee like a real bank/exchange."** False —
   see [Vault Fee Model](vault-fee-model.md). Zero fees are charged, and
   none are silently deducted anywhere in the code path.
3. **"If I lock for longer, I get a better rate or priority."** False —
   duration has no effect beyond the timestamp comparison in
   `withdraw_lock`. All matured locks are treated identically regardless
   of how long they were locked.
4. **"The admin can recover my funds if I lose my key."** False — there is
   no admin fund-recovery mechanism (see
   [Vault Custody Assumptions §3.5](vault-custody-assumptions.md#35-no-emergency-token-recovery)).
   Losing access to the signing key that controls a user's address means
   losing access to that user's vault balance and locks, with no
   contract-level recourse.
5. **"Pausing the contract freezes my funds."** False — pause blocks new
   deposits and locks only; `withdraw` and `withdraw_lock` remain callable
   during a pause (see [pause-design.md](pause-design.md)).
6. **"This is audited / production-grade because it's this well
   documented."** False — see the [README's Release Readiness
   table](../README.md#release-readiness). Extensive documentation and
   test coverage do not substitute for an external audit; none has been
   performed.

---

## 7. Summary Table

| Assumption area | Current model | Where it's specified in depth |
| --- | --- | --- |
| Fees | None, on any operation | [vault-fee-model.md](vault-fee-model.md) |
| Custody | 1:1, atomic deposit/withdraw, no yield | [vault-custody-assumptions.md](vault-custody-assumptions.md) |
| Lock incentive | None — commitment device only, no interest | §3 above |
| Token behavior | Assumes standard, non-adversarial SAC | §4 above, [vault-custody-assumptions.md §2.2](vault-custody-assumptions.md#22-token-behavior-standards) |
| Admin power | Cannot move funds; can pause / configure limits / transfer role | [admin-role.md](admin-role.md), [admin-pause-threat-model.md](admin-pause-threat-model.md) |
| User expectations | Six common misconceptions to correct in SDK/UI copy | §6 above |

**Bottom line**: the vault is a fee-free, non-yield-bearing, token-backed
custody contract whose only "economic" feature is a user-chosen,
admin-bounded time lock with no attached reward. Any product copy, SDK
documentation, or mobile UI built on top of this contract should reflect
that explicitly rather than implying banking-like fees, interest, or
admin-backed fund recovery.
