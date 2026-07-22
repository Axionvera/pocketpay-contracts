# Savings Vault — Event Schema

This document describes every event emitted by the Savings Vault contract so that
SDK authors, frontend developers, and indexers can parse and react to on‑chain
activity.

---

## Overview

The Savings Vault emits events through the Soroban host‑environment `log!`
macro.  Each event includes a **topic** (the first string literal) followed by
key‑value pairs that form the **payload**.  SDKs should filter events by topic
and then decode the payload fields.

| Topic                          | Emitted By       | Type       |
|--------------------------------|------------------|------------|
| `Savings Vault initialized…`   | `initialize`     | Lifecycle  |
| `Deposit: user=…`              | `deposit`        | State      |
| `Withdraw: user=…`             | `withdraw`       | State      |
| `Lock: user=…`                 | `lock_funds`     | State      |

---

## Event Reference

### 1. `Savings Vault initialized with admin`

Emitted once when the contract is initialized.

**Topic pattern**

```
Savings Vault initialized with admin: {admin}
```

**Payload**

| Field   | Type      | Description                          |
|---------|-----------|--------------------------------------|
| `admin` | `Address` | The admin address set during setup.  |

**Example raw event**

```
"Savings Vault initialized with admin: GABC…XYZ"
```

> **Note**: The token address passed to `initialize` is stored on‑chain but
> does not appear in this event.  Query the instance storage key `Token` if
> the token address is needed.

---

### 2. `Deposit`

Emitted every time a user deposits funds into the vault.

**Topic pattern**

```
Deposit: user={user}, amount={amount}, new_balance={new_balance}
```

**Payload**

| Field         | Type      | Description                                      |
|---------------|-----------|--------------------------------------------------|
| `user`        | `Address` | The depositor's address.                         |
| `amount`      | `i128`    | The amount deposited (always > 0).               |
| `new_balance` | `i128`    | The user's available balance after the deposit.  |

**Example raw event**

```
"Deposit: user=GABC…XYZ, amount=500, new_balance=1500"
```

---

### 3. `Withdraw`

Emitted every time a user withdraws funds from the vault.  The contract
transfers the underlying token (via the Stellar Asset Contract) as part of this
call.

**Topic pattern**

```
Withdraw: user={user}, amount={amount}, new_balance={new_balance}
```

**Payload**

| Field         | Type      | Description                                         |
|---------------|-----------|-----------------------------------------------------|
| `user`        | `Address` | The withdrawer's address.                           |
| `amount`      | `i128`    | The amount withdrawn (always > 0).                  |
| `new_balance` | `i128`    | The user's available balance after the withdrawal.  |

**Example raw event**

```
"Withdraw: user=GABC…XYZ, amount=200, new_balance=1300"
```

---

### 4. `Lock`

Emitted when a user locks a portion of their balance until a future timestamp.

**Topic pattern**

```
Lock: user={user}, amount={amount}, unlock_time={unlock_time}, available={available}, locked={locked}
```

**Payload**

| Field         | Type      | Description                                                    |
|---------------|-----------|----------------------------------------------------------------|
| `user`        | `Address` | The user's address.                                            |
| `amount`      | `i128`    | The amount moved from available → locked (always > 0).         |
| `unlock_time` | `u64`     | Unix timestamp (seconds) when the funds become withdrawable.   |
| `available`   | `i128`    | The user's available balance after locking.                    |
| `locked`      | `i128`    | The user's total locked balance after locking.                 |

**Example raw event**

```
"Lock: user=GABC…XYZ, amount=100, unlock_time=1710000000, available=1200, locked=300"
```

---

## Consumer Examples

### JavaScript / TypeScript (Soroban SDK)

The `@stellar/stellar-sdk` (v12+) and `@stellar/soroban-client` packages
expose raw events from transaction results.  Use the topic string to dispatch
the correct parser.

```ts
import { SorobanRpc, xdr } from "@stellar/stellar-sdk";

// After submitting a transaction, retrieve the result:
const txResult = SorobanRpc.Api.GetTransactionResponse; // simplified
const meta = txResult.resultMetaXdr.v3().sorobanMeta();

if (meta?.events) {
  for (const event of meta.events) {
    const topic = event.topic;      // Vec<ScVal>
    const data  = event.body;       // Vec<ScVal>

    // Soroban log! events use topic[0] = "fmt string", data = args
    const topicStr = scValToString(topic[0]);

    if (topicStr.startsWith("Deposit:")) {
      const payload = parseDepositEvent(topicStr);
      console.log("Deposit detected:", payload);
    } else if (topicStr.startsWith("Lock:")) {
      const payload = parseLockEvent(topicStr);
      console.log("Lock detected:", payload);
    }
    // … handle other events
  }
}
```

### Parsing helpers

```ts
interface DepositEvent {
  user: string;
  amount: bigint;
  newBalance: bigint;
}

function parseDepositEvent(raw: string): DepositEvent {
  // "Deposit: user=G…, amount=500, new_balance=1500"
  const userMatch      = raw.match(/user=([^,]+)/);
  const amountMatch    = raw.match(/amount=(\d+)/);
  const balanceMatch   = raw.match(/new_balance=(\d+)/);

  return {
    user:       userMatch![1],
    amount:     BigInt(amountMatch![1]),
    newBalance: BigInt(balanceMatch![1]),
  };
}

interface LockEvent {
  user: string;
  amount: bigint;
  unlockTime: number;  // Unix seconds
  available: bigint;
  locked: bigint;
}

function parseLockEvent(raw: string): LockEvent {
  const userMatch       = raw.match(/user=([^,]+)/);
  const amountMatch     = raw.match(/amount=(\d+)/);
  const unlockMatch     = raw.match(/unlock_time=(\d+)/);
  const availableMatch  = raw.match(/available=(\d+)/);
  const lockedMatch     = raw.match(/locked=(\d+)/);

  return {
    user:       userMatch![1],
    amount:     BigInt(amountMatch![1]),
    unlockTime: Number(unlockMatch![1]),
    available:  BigInt(availableMatch![1]),
    locked:     BigInt(lockedMatch![1]),
  };
}
```

### React Native / Mobile (conceptual)

For a React Native wallet app, listen for events from the Horizon or RPC
ingestion layer and update local state:

```ts
function handleVaultEvent(event: ParsedEvent) {
  switch (event.topic) {
    case "Deposit":
      updateBalance(event.user, event.newBalance);
      showToast(`Deposited ${event.amount} — new balance ${event.newBalance}`);
      break;
    case "Withdraw":
      updateBalance(event.user, event.newBalance);
      break;
    case "Lock":
      updateAvailableBalance(event.user, event.available);
      updateLockedBalance(event.user, event.locked, event.unlockTime);
      break;
  }
}
```

---

## Future Considerations

- **Structured events**: Future contract versions may emit Soroban
  `contractevent!`‑style events with typed topics and data vectors, which are
  easier for indexers to consume than formatted strings.

- **Additional events**: Planned features (multi‑lock, admin recovery, token
  swaps) will add new event topics.  Consumers should code defensively so that
  unknown topics are silently ignored rather than erroring.

- **Indexing**: For production, consider running a Soroban‑aware indexer
  (e.g. Mercury, a custom Dapplo backend) that filters events by contract ID
  and persists parsed payloads into a database for fast querying.
