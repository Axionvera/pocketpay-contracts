## Summary

<!-- What changed? Keep it short, but precise. -->

## Affected components

- [ ] On-chain contract code
- [ ] Storage layout / keys
- [ ] Auth / authorization behavior
- [ ] Token / transfer integration
- [ ] Lock/unlock logic
- [ ] Tests
- [ ] Documentation

## Affected functions / entrypoints

<!-- List contract functions and any parameter changes. -->

## Behavior changes (storage / token / locks)

- **Storage impact** (keys, persistence/TTL, new/removed entries):

- **Auth impact** (what `require_auth()` covers and who signs):

- **Token / transfer impact** (e.g., SAC integration, real transfers vs internal balances):

- **Lock/unlock impact** (single vs multiple locks, unlock semantics, edge cases):

## Tests

- Commands run locally/CI:
  - `cargo test`
  - (optional) other commands:

- Test results / notes:

## Security notes

<!-- Call out any new trust assumptions, admin powers, upgrade/recovery implications, and reentrancy/ledger-time edge cases as applicable. -->

## Checklist

- [ ] I updated/added tests for the behavior change
- [ ] I reviewed storage/auth implications
- [ ] I documented any new limitations or changed assumptions
