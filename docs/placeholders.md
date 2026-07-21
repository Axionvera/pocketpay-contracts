# Placeholder Value Conventions

This document defines the canonical placeholder names used in examples across
this repository: the README, `CONTRIBUTING.md`, and everything under `docs/`.
It exists so that contract IDs, addresses, transaction hashes, and timestamps
in examples are easy to recognize as placeholders and consistent from one doc
to the next. See [docs/docs-style-guide.md](docs-style-guide.md#placeholders)
for the general rule this document implements.

## Canonical placeholders

Use these names, exactly as written, when a new or edited doc needs a
placeholder value of the corresponding kind:

| Placeholder | Represents | Example usage |
| --- | --- | --- |
| `CONTRACT_ID_PLACEHOLDER` | A deployed savings vault contract ID | `--id CONTRACT_ID_PLACEHOLDER` |
| `USER_PUBLIC_KEY` | A depositor/withdrawer account address | `--user USER_PUBLIC_KEY` |
| `ADMIN_PUBLIC_KEY` | The admin account address passed to `initialize` | `--admin ADMIN_PUBLIC_KEY` |
| `UNLOCK_TIMESTAMP` | A Unix timestamp used with `lock_funds` | `--unlock_time UNLOCK_TIMESTAMP` |

Example command using all four:

```bash
soroban contract invoke \
  --id CONTRACT_ID_PLACEHOLDER \
  --source USER_PUBLIC_KEY \
  --rpc-url RPC_URL \
  --network-passphrase NETWORK_PASSPHRASE \
  -- \
  lock_funds \
  --user USER_PUBLIC_KEY \
  --amount 20000000 \
  --unlock_time UNLOCK_TIMESTAMP
```

## Existing equivalent forms

A few docs predate this convention and use an equivalent form of the same
placeholder, such as `<CONTRACT_ID>`, `YOUR_CONTRACT_ID`, `<ADMIN_ADDRESS>`, or
`<UNIX_TIMESTAMP>`. These refer to the same kind of value described above.
New documentation and future edits to existing docs should use the canonical
names in this file; this document is the single source of truth for the
mapping, so a doc does not need to repeat this explanation.

## Avoiding real-looking secrets

- Never use a real contract ID, secret key, seed phrase, RPC credential, or
  any other value captured from an actual deployment in an example. Use the
  placeholders above, or another clearly synthetic value, instead. See the
  [Security-sensitive contributions](../CONTRIBUTING.md#security-sensitive-contributions)
  section of `CONTRIBUTING.md`.
- A placeholder should be self-explanatory on its own — a reader should be
  able to tell it is a placeholder without reading surrounding prose.
- These rules apply to command examples. Where a doc walks through an already
  deployed contract ID for handoff purposes, follow
  [docs/contract-id-handoff.md](contract-id-handoff.md) instead, which uses a
  clearly-fake example value for that different purpose.

## Scope

This document covers placeholder naming only. For testnet wording, avoiding
production claims, command formatting, and linking conventions, see
[docs/docs-style-guide.md](docs-style-guide.md).
