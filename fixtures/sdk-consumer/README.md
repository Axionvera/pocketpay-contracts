# Savings Vault SDK-consumer fixtures (v1)

These versioned, deterministic examples describe the **expected contract-level
results** of the Savings Vault operations implemented in
[`contracts/savings_vault/src/lib.rs`](../../contracts/savings_vault/src/lib.rs).
They support cross-repository SDK/mobile tests without calling Horizon,
Friendbot, Soroban RPC or a deployed contract.

**Source:** [`vault-scenarios.v1.json`](./vault-scenarios.v1.json)

## Coverage

The single fixture file has ten named scenarios, including the successive
states of deposit → lock → early-withdrawal rejection → maturity → lock
withdrawal → ordinary withdrawal, plus insufficient balance, zero amount,
already-withdrawn lock and uninitialized-contract errors.

Each entry includes:

- `id`, `operation`, `ledger_timestamp`, and normalized `args`.
- `before` state and `expected.after` state, except an uninitialized
  contract where no balance state can be queried.
- `expected.ok` and either `expected.return` or the contract's
  `#[contracterror]` numeric code and Rust variant.
- `expected.events` with exact **logical** event topic names/order and
  normalized payload values for successful state-changing calls. Rejected
  calls must emit no contract events and must not mutate state.
- The lock creation/withdrawal response, including the withdrawn lock's
  retained entry with amount zero and `withdrawn: true`.

Balances and amounts are **base-unit i128 values encoded as decimal
strings**, not Stellar network amounts or JavaScript floating-point numbers.
Timestamps are fixed Unix seconds. Public test addresses are synthetic and
safe to expose; **no real credentials or private key values** are included.

The fixture expresses source-derived **expected behavior**. It has *not* been
captured from a live chain or asserted by executing the contract. It is also
**not raw RPC, SCVal or XDR**: `events[].topics` and `events[].value` are
already-normalized logical arrays. An SDK integration layer must decode
Soroban events into the documented logical representation before comparing.

## Consumer example

A Node.js consumer can read the file without importing the Rust contract:

```js
const assert = require('node:assert/strict');
const scenarios = require('./vault-scenarios.v1.json');

const byId = Object.fromEntries(scenarios.scenarios.map(row => [row.id, row]));

assert.equal(scenarios.schema_version, 1);
assert.equal(byId.early_lock_withdrawal_rejected.expected.error.code, 5003);
assert.deepEqual(
  byId.deposit_1000.expected.events[0].topics,
  ['deposit', scenarios.public_test_addresses.user],
);

// Your own SDK decoder supplies the actual event/snapshot:
// assert.deepEqual(normalizeDecodedEvent(actual), byId.deposit_1000.expected.events[0]);
```

Integrators should also use the contract's generated bindings or RPC client
to distinguish `Option<LockEntry>` from a missing lock, and interpret errors
from the **contract** separately from RPC/transport failures. See
[Contract Error Codes](../../docs/error-codes.md) and
[Contract Event Compatibility](../../docs/event-compatibility-policy.md).

## Updating after contract changes

1. Check the public function signatures, `#[contracterror]` discriminants,
   event topics/payload construction, and balance/lock semantics in `lib.rs`.
2. Add/update explicit cases and keep `schema_version` stable only for
   backward-compatible fixture shape changes; create v2 for breaking changes.
3. Run only focused contract acceptance cases impacted by the change before
   treating expected results as observed runtime evidence.
4. Update corresponding SDK/mobile consumers and cross-repo compatibility
   notes when payloads or errors change.

The fixtures do not authorize network transfers, enforce KYC, prove deployed
ABI compatibility, or replace Soroban contract tests.
