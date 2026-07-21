# Storage Change Checklist

Storage layout changes are high-impact for smart contracts. Before modifying storage keys or stored value shapes, confirm each of the following items.

## Required review steps

- [ ] Update storage documentation.
  - Document the changed storage keys, value shapes, and any reason for the layout change.
  - Explain how the new layout differs from the prior version.
- [ ] Add or update tests for the storage change.
  - Cover normal behavior and edge cases.
  - Include tests that verify old state is handled correctly when applicable.
- [ ] Review SDK compatibility.
  - Confirm whether SDKs or external clients depend on existing storage keys or serialization.
  - Add compatibility notes for maintainers, SDK authors, and integrators.
- [ ] Consider migration and upgrade impact.
  - Determine whether existing contract state requires migration, reinitialization, or a compatibility shim.
  - Document migration steps if the change affects deployed state.
- [ ] Update release notes or changelog guidance.
  - Call out storage layout changes explicitly in release notes, upgrade notes, or deployment guidance.

## Why this matters

- Storage layout changes can break existing contract state when the contract is upgraded.
- External tooling and SDKs may assume existing key names, types, or serialization formats.
- Migration gaps can cause data loss, failed upgrades, or runtime errors.

## When to use this checklist

Use this checklist whenever a change touches any of the following:

- storage key definitions
- data enum variants or struct serialization formats
- stored value shapes, schemas, or type representations
- contract state load/save behavior
- storage access patterns that may be observed by external tooling

If a PR includes a storage change, link to this checklist in the description and confirm the completed items.