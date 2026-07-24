# Savings Vault — Event Schema

This document is the source of truth for the on-chain events emitted by
`SavingsVault`. SDK authors, mobile clients, and indexers can filter, decode,
and route these events safely without reading the contract source.

The event contract is locked in by
`contracts/savings_vault/src/test/event_compatibility.rs`. Every field named
below is asserted there, and any drift will fail CI before merge.

---

## Two output streams, one on-chain contract

`SavingsVault` writes two different kinds of output during a transaction:

| Kind                   | Producer                       | Where it lives                             | SDK-facing?               |
|------------------------|--------------------------------|--------------------------------------------|---------------------------|
| Structured event       | `env.events().publish(...)`    | Transaction `SorobanTransactionMeta.events` | **Yes.** Consume these.  |
| Diagnostic log         | `log!(env, "...")` macro       | Diagnostic events (usually stripped in production) | No. Debug only.  |

The rest of this document describes the structured events. Diagnostic logs may
appear alongside them but are not part of the SDK contract, their format can
change without notice, and production RPC endpoints usually filter them out.

Every structured event uses the shape:

```
topics = (Symbol, Address)     // (event_kind, subject)
data   = <typed payload>       // per-event tuple, see below
```

Topic 0 is a Soroban `Symbol` naming the event. Topic 1 is the on-chain
subject the event is *about* (user, admin, or old admin). The data payload is
a typed tuple decoded via `scValToNative` (JS/TS) or the codegen'd bindings
(Rust/Go).

---

## Event Reference

### 1. `initialize`

Emitted exactly once, when the contract is initialized. Prior versions
double-emitted this event (`Symbol("initialize")` then `symbol_short!("init")`);
that was a bug and has been removed.

**Topics**

| Position | Type      | Value                                          |
|----------|-----------|------------------------------------------------|
| 0        | `Symbol`  | `"initialize"` (long-form symbol)              |
| 1        | `Address` | Admin address recorded at init                 |

**Data**

| Field   | Type      | Description                                    |
|---------|-----------|------------------------------------------------|
| `token` | `Address` | Address of the SAC used for deposits/withdraws |

---

### 2. `deposit`

Emitted after a successful deposit. The internal balance has already been
updated and the SAC transfer has already succeeded when this event fires; a
reverted deposit emits **no** event.

**Topics**

| Position | Type      | Value                          |
|----------|-----------|--------------------------------|
| 0        | `Symbol`  | `"deposit"` (short symbol)     |
| 1        | `Address` | Depositor                      |

**Data**

| Field         | Type   | Description                                             |
|---------------|--------|---------------------------------------------------------|
| `amount`      | `i128` | Amount deposited this call (always `> 0`)               |
| `new_balance` | `i128` | User's available (unlocked) balance **after** deposit   |

---

### 3. `withdraw`

Emitted after a successful withdrawal. The withdrawal path can pull from both
the unlocked deposit balance and matured locks; the emitted `new_locked`
reflects only the still-unmatured lock total.

**Topics**

| Position | Type      | Value                       |
|----------|-----------|-----------------------------|
| 0        | `Symbol`  | `"withdraw"` (short symbol) |
| 1        | `Address` | Withdrawer                  |

**Data**

| Field         | Type   | Description                                             |
|---------------|--------|---------------------------------------------------------|
| `amount`      | `i128` | Amount withdrawn this call (always `> 0`)               |
| `new_balance` | `i128` | User's available balance **after** withdrawal            |
| `new_locked`  | `i128` | User's remaining unmatured locked total after withdrawal |

---

### 4. `lock`

Emitted when a user moves funds from available to a time-locked entry.

**Topics**

| Position | Type      | Value                    |
|----------|-----------|--------------------------|
| 0        | `Symbol`  | `"lock"` (short symbol)  |
| 1        | `Address` | Lock owner               |

**Data**

| Field         | Type   | Description                                                          |
|---------------|--------|----------------------------------------------------------------------|
| `amount`      | `i128` | Amount moved from available to locked (always `> 0`)                 |
| `unlock_time` | `u64`  | Unix-seconds timestamp when the lock matures                         |
| `new_balance` | `i128` | User's available balance after locking                               |
| `new_locked`  | `i128` | User's total unmatured locked balance after this lock is added       |

---

### 5. `withdraw_lock`

Emitted when a specific matured lock entry is withdrawn by id.

**Topics**

| Position | Type      | Value                                    |
|----------|-----------|------------------------------------------|
| 0        | `Symbol`  | `"withdraw_lock"` (long-form, 13 chars)  |
| 1        | `Address` | Lock owner                               |

**Data**

| Field     | Type   | Description                          |
|-----------|--------|--------------------------------------|
| `lock_id` | `u64`  | Id of the matured lock being closed  |
| `amount`  | `i128` | Amount released from the lock         |

---

### 6. `pause`

Emitted when the admin activates the emergency pause.

**Topics**

| Position | Type      | Value                   |
|----------|-----------|-------------------------|
| 0        | `Symbol`  | `"pause"` (short symbol) |
| 1        | `Address` | Admin who paused         |

**Data**

| Field    | Type  | Description                                             |
|----------|-------|---------------------------------------------------------|
| `expiry` | `u64` | Unix-seconds timestamp when the pause auto-clears        |

---

### 7. `unpause`

Emitted when the admin lifts the pause early.

**Topics**

| Position | Type      | Value                       |
|----------|-----------|-----------------------------|
| 0        | `Symbol`  | `"unpause"` (short symbol)  |
| 1        | `Address` | Admin who unpaused          |

**Data**

Empty tuple `()`. `unpause` conveys no additional state beyond the fact that
it happened; subject is on topic1.

---

### 8. `xferadmin`

Emitted on admin rotation. Topic1 is the *old* admin so subscribers filtering
by outgoing admin can find the transition; the new admin is in data.

**Topics**

| Position | Type      | Value                             |
|----------|-----------|-----------------------------------|
| 0        | `Symbol`  | `"xferadmin"` (short symbol)      |
| 1        | `Address` | Previous admin                    |

**Data**

| Field       | Type      | Description        |
|-------------|-----------|--------------------|
| `new_admin` | `Address` | Newly-set admin    |

---

## Consumer Examples

### TypeScript / JavaScript (`@stellar/stellar-sdk`)

The stellar-sdk exposes Soroban structured events on transaction results.
Decode topic0 as a `Symbol`, topic1 as an `Address`, and the data payload with
`scValToNative` for typed access.

```ts
import { scValToNative, xdr } from "@stellar/stellar-sdk";

type VaultEvent =
  | { kind: "initialize"; admin: string; token: string }
  | { kind: "deposit"; user: string; amount: bigint; newBalance: bigint }
  | { kind: "withdraw"; user: string; amount: bigint; newBalance: bigint; newLocked: bigint }
  | { kind: "lock"; user: string; amount: bigint; unlockTime: bigint; newBalance: bigint; newLocked: bigint }
  | { kind: "withdraw_lock"; user: string; lockId: bigint; amount: bigint }
  | { kind: "pause"; admin: string; expiry: bigint }
  | { kind: "unpause"; admin: string }
  | { kind: "xferadmin"; oldAdmin: string; newAdmin: string };

export function decodeVaultEvent(topics: xdr.ScVal[], data: xdr.ScVal): VaultEvent | null {
  if (topics.length < 2) return null;
  const kind = scValToNative(topics[0]) as string;
  const subject = scValToNative(topics[1]) as string;
  const payload = scValToNative(data);

  switch (kind) {
    case "initialize":
      return { kind, admin: subject, token: payload as string };
    case "deposit": {
      const [amount, newBalance] = payload as [bigint, bigint];
      return { kind, user: subject, amount, newBalance };
    }
    case "withdraw": {
      const [amount, newBalance, newLocked] = payload as [bigint, bigint, bigint];
      return { kind, user: subject, amount, newBalance, newLocked };
    }
    case "lock": {
      const [amount, unlockTime, newBalance, newLocked] =
        payload as [bigint, bigint, bigint, bigint];
      return { kind, user: subject, amount, unlockTime, newBalance, newLocked };
    }
    case "withdraw_lock": {
      const [lockId, amount] = payload as [bigint, bigint];
      return { kind, user: subject, lockId, amount };
    }
    case "pause":
      return { kind, admin: subject, expiry: payload as bigint };
    case "unpause":
      return { kind, admin: subject };
    case "xferadmin":
      return { kind, oldAdmin: subject, newAdmin: payload as string };
    default:
      return null;
  }
}
```

### Filtering by contract on Soroban RPC

Use `soroban-rpc.getEvents` with a contract-scoped filter. The topic filter is
matched by ScVal equality, so pass the `Symbol` you care about as an ScVal.

```ts
const events = await server.getEvents({
  startLedger: fromLedger,
  filters: [
    {
      type: "contract",
      contractIds: [savingsVaultContractId],
      topics: [[nativeToScVal("deposit", { type: "symbol" }).toXDR("base64")]],
    },
  ],
});
```

Repeat with the topic string you need (`"initialize"`, `"withdraw"`, `"lock"`,
`"withdraw_lock"`, `"pause"`, `"unpause"`, `"xferadmin"`).

---

## Stability guarantees

- **Topic 0 symbols are stable.** Renames are breaking changes and will bump
  the contract's public version.
- **Topic 1 subject stays an Address.** The specific subject (user, admin,
  or previous admin) per event is documented in each section above and asserted
  in `event_compatibility.rs`.
- **Data payload shape is stable.** Adding fields is a breaking change to
  tuple positions and will bump the contract version. Consumers should decode
  strictly and error on unknown shapes rather than tolerating them silently.
- **Diagnostic `log!` output is not stable.** Do not build parsers against it.

## Future considerations

- **Named-fields payloads.** Migrating from positional tuples to
  `#[contracttype]` structs would give SDKs field names without a positional
  contract. This is a breaking change and will need a versioned rollout.
- **Additional events.** New features may add topics. Consumers should treat
  unknown `topic0` symbols as forward-compatible and ignore them rather than
  erroring.
- **Structured indexer output.** A production indexer (Mercury, a bespoke
  ingester, etc.) can parse and persist these events using the shapes above
  and expose queryable views to the mobile client.
