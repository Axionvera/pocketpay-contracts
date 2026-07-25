# Security Review Checklist

This checklist should be used when reviewing pull requests that introduce security-sensitive changes to the PocketPay contracts. Verify each item applies to the change and provide evidence or explanations where appropriate.

## Authentication & Authorization
- [ ] All state‑changing functions enforce `require_auth()` for the appropriate address.
- [ ] Only authorized roles (admin, owner, etc.) can perform privileged actions.
- [ ] Access control changes are clearly documented and reviewed.

## Storage Changes
- [ ] New storage entries use persistent storage unless temporary.
- [ ] Existing storage keys are not inadvertently overwritten.
- [ ] Storage layout changes are reflected in `docs/storage-change-checklist.md`.
- [ ] Proper data migration strategy is described if needed.

## Token Transfer & Custody
- [ ] Any token transfer uses the Soroban token standard or appropriate asset contract.
- [ ] Transfers are checked for success and handle failure cases.
- [ ] No implicit token movement occurs without explicit transfer calls.

## Locks & Timed Operations
- [ ] Lock logic correctly validates unlock timestamps (future dates).
- [ ] Edge cases such as lock expiration and multiple locks are addressed.
- [ ] Lock amount does not exceed available balance.

## Admin Behaviour
- [ ] Admin‑only actions are clearly gated and documented.
- [ ] Admin can recover or manage contracts safely; any lack of recovery is noted.
- [ ] Changes to admin privileges are reviewed for potential misuse.

## General
- [ ] No secrets, private keys, or credentials are logged or committed.
- [ ] All new code includes comprehensive tests for failure and edge cases.
- [ ] Documentation is updated to reflect security implications.

*Use this checklist as part of the PR description and ensure each item is addressed before merging.*
