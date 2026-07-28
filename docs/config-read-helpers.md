# Configuration Read API

> Issue: #456 — Add contract configuration read API

## Overview

SDK and mobile clients need a stable way to read the full contract
configuration — accepted token, admin address, version, pause state, and
configurable limits — without issuing multiple separate RPC queries.

The `get_config` read-only function returns a `ContractConfig` struct
containing every configuration field in a single call.

## `get_config() → ContractConfig`

Returns the full contract configuration. No authorization required
(read-only operation).

### Response fields

| Field                | Type      | Description                                                     |
|----------------------|-----------|-----------------------------------------------------------------|
| `token`              | `Address` | Address of the accepted Stellar Asset Contract (SAC).           |
| `admin`              | `Address` | Address of the contract admin.                                  |
| `version`            | `String`  | Hard-coded semantic version of the deployed WASM (`"0.1.0"`).   |
| `paused`             | `bool`    | Whether the emergency pause is currently active.                |
| `pause_expiry`       | `u64`     | Unix timestamp when the current pause expires (0 = no active pause). |
| `min_deposit_amount` | `i128`    | Minimum deposit floor in atomic units (0 = no floor enforced).  |
| `max_lock_duration`  | `u64`     | Maximum lock duration in seconds (0 = unbounded).               |
| `min_lock_duration`  | `u64`     | Minimum lock duration in seconds (0 = no lower bound enforced). |

### Default values

After initialization with no admin configuration applied:

| Field                | Default |
|----------------------|---------|
| `paused`             | `false` |
| `pause_expiry`       | `0`     |
| `min_deposit_amount` | `0`     |
| `max_lock_duration`  | `0`     |
| `min_lock_duration`  | `0`     |

### Pause expiry behavior

If a pause has been set but its expiry timestamp has been reached, `paused`
reports `false` in the returned config (the pause has lapsed). The stored
`pause_expiry` value is still returned so callers can distinguish between
"never paused" (`pause_expiry == 0`) and "was paused but expired"
(`pause_expiry > 0`).

### Errors

| Error                          | Condition                                  |
|--------------------------------|---------------------------------------------|
| `NotInitialized`               | Contract has not been initialized.          |
| `StorageVersionUnsupported`    | Stored version does not match compiled.     |
| `TokenNotConfigured`           | Token address is missing from storage.      |
| `RequiredStorageEntryMissing`  | Admin address is missing from storage.      |

## CLI usage

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source <ACCOUNT> \
  --network testnet \
  -- \
  get_config
```

**Example output:**

```json
{
  "token": "CAS2J7Z3EJMIIFID4JHOV3K5WIEZRE6Q3GQLR7VPCZ5T7RFXI5L6XH6F",
  "admin": "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
  "version": "0.1.0",
  "paused": false,
  "pause_expiry": 0,
  "min_deposit_amount": 0,
  "max_lock_duration": 0,
  "min_lock_duration": 0
}
```

## SDK usage

### Stellar SDK (TypeScript)

```typescript
const config = await contract.get_config();
console.log("Accepted token:", config.token);
console.log("Admin:", config.admin);
console.log("Paused:", config.paused);
console.log("Min deposit:", config.min_deposit_amount);
```

### Rust (soroban-sdk)

```rust
let config = client.get_config();
assert_eq!(config.token, token_address);
assert_eq!(config.admin, admin_address);
assert!(!config.paused);
```

## Non-mutating guarantee

`get_config` is a pure read-only function. It reads from instance storage
and never writes. Calling it repeatedly returns identical results (barring
admin-initiated configuration changes between calls).

## Test coverage

| Scenario                                  | Covered |
|-------------------------------------------|---------|
| Defaults after initialization             | Yes     |
| Configured limits reflected               | Yes     |
| Active pause state and expiry             | Yes     |
| Unpause resets state                       | Yes     |
| Expired pause reports not-paused           | Yes     |
| Non-mutating (idempotent)                  | Yes     |
| Individual field read helpers              | Yes     |
| Uninitialized contract panics              | Yes     |

See `contracts/savings_vault/src/test/config_read_helpers.rs` for the full
test suite.

## Related docs

- [Version Metadata](version-metadata.md) — `get_version` details
- [Pause Design](pause-design.md) — emergency pause model
- [API Reference](api-reference.md) — function naming conventions
- [Read Models](read-models.md) — balance snapshot and lock summary
