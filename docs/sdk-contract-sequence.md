# SDK ↔ Contract Sequence Diagrams

This document shows how a vault request travels from the mobile app, through
the SDK, to Soroban RPC, and into the savings vault contract — and how the
response (or error) travels back. It complements
[architecture.md](architecture.md) (component responsibilities) and
[error-codes.md](error-codes.md) (failure conditions), which describe each
piece individually rather than as one end-to-end flow.

The diagrams are intentionally high level: they show the shape of each flow,
not every parameter or SDK method name, so they should not need updates for
routine contract changes. Update them only if a step is added, removed, or
reordered (for example, a new custody/token-transfer hop).

## Actors

- **Mobile App** — the end-user-facing client (UI, wallet signing prompts).
- **SDK** — the client library the mobile app calls; builds and simulates
  transactions, and abstracts contract invocation details.
- **Soroban RPC** — the network endpoint the SDK talks to for simulation,
  submission, and transaction status.
- **Vault Contract** — the deployed savings vault contract described in
  [architecture.md](architecture.md).

## Read-only balance query

`get_balance` (and similarly `get_locked_balance`, `can_withdraw`) is a
read-only call: it does not require the user's authorization or a signed
transaction.

```mermaid
sequenceDiagram
    actor User
    participant Mobile as Mobile App
    participant SDK
    participant RPC as Soroban RPC
    participant Vault as Vault Contract

    User->>Mobile: Open balance screen
    Mobile->>SDK: getBalance(user)
    SDK->>RPC: simulateTransaction(get_balance, user)
    RPC->>Vault: invoke get_balance(user)
    Vault-->>RPC: balance (i128)
    RPC-->>SDK: simulation result
    SDK-->>Mobile: balance
    Mobile-->>User: Display balance
```

## Deposit

`deposit` is a state-changing call: it requires the user's signature and a
submitted (not just simulated) transaction. A successful deposit transfers tokens
from the user to the vault via the configured Stellar Asset Contract (SAC), then
updates internal accounting. See [architecture.md](architecture.md#token-backed-accounting-and-asset-custody).

```mermaid
sequenceDiagram
    actor User
    participant Mobile as Mobile App
    participant SDK
    participant RPC as Soroban RPC
    participant Vault as Vault Contract

    User->>Mobile: Enter deposit amount
    Mobile->>SDK: deposit(user, amount)
    SDK->>RPC: simulateTransaction(deposit, user, amount)
    RPC->>Vault: invoke deposit(user, amount)
    Vault-->>RPC: simulation result (auth required)
    RPC-->>SDK: simulation result
    SDK-->>Mobile: request wallet signature
    Mobile->>User: Prompt to sign
    User-->>Mobile: Approve & sign
    Mobile->>SDK: signed transaction
    SDK->>RPC: sendTransaction(signed tx)
    RPC->>Vault: apply deposit(user, amount)
    Vault-->>RPC: updated balance / success
    RPC-->>SDK: transaction status (success)
    SDK-->>Mobile: deposit confirmed
    Mobile-->>User: Show updated balance
```

## Withdraw

`withdraw` follows the same signed-transaction shape as deposit, but can fail
against vault-side checks (for example, insufficient available balance) in
addition to token-transfer failures. See
[error-codes.md](error-codes.md#insufficient-balance-errors).

```mermaid
sequenceDiagram
    actor User
    participant Mobile as Mobile App
    participant SDK
    participant RPC as Soroban RPC
    participant Vault as Vault Contract

    User->>Mobile: Enter withdrawal amount
    Mobile->>SDK: withdraw(user, amount)
    SDK->>RPC: simulateTransaction(withdraw, user, amount)
    RPC->>Vault: invoke withdraw(user, amount)
    Vault-->>RPC: simulation result (auth required)
    RPC-->>SDK: simulation result
    SDK-->>Mobile: request wallet signature
    Mobile->>User: Prompt to sign
    User-->>Mobile: Approve & sign
    Mobile->>SDK: signed transaction
    SDK->>RPC: sendTransaction(signed tx)
    RPC->>Vault: apply withdraw(user, amount)
    Vault-->>RPC: updated balance / success
    RPC-->>SDK: transaction status (success)
    SDK-->>Mobile: withdrawal confirmed
    Mobile-->>User: Show updated balance
```

## Error response path

Failures can surface at simulation time (before any signature is requested)
or at submission time (after a signed transaction is sent). The SDK should
retain the full diagnostic and avoid branching on panic text, per
[error-codes.md](error-codes.md).

```mermaid
sequenceDiagram
    actor User
    participant Mobile as Mobile App
    participant SDK
    participant RPC as Soroban RPC
    participant Vault as Vault Contract

    User->>Mobile: Enter amount
    Mobile->>SDK: withdraw(user, amount)
    SDK->>RPC: simulateTransaction(withdraw, user, amount)
    RPC->>Vault: invoke withdraw(user, amount)
    Vault-->>RPC: panic (e.g. "Insufficient balance")
    RPC-->>SDK: simulation error + diagnostic
    SDK-->>Mobile: error result (no signature requested)
    Mobile-->>User: Show friendly error message

    Note over SDK,RPC: A signed transaction can also fail at<br/>submission time (e.g. missing auth,<br/>token transfer failure) after simulation<br/>succeeded. The SDK handles both cases<br/>the same way: surface a friendly message<br/>and retain the diagnostic.
```

## See also

- [Architecture Documentation](architecture.md) — component responsibilities
  and storage design.
- [Savings Vault Error Reference](error-codes.md) — current failure
  conditions and caller guidance.
- [Vault Contract ID Handoff](contract-id-handoff.md) — how the contract ID
  reaches the SDK and mobile app configuration.
