# Release WASM Artefacts (Savings Vault)

This document defines a simple, consistent convention for naming and handling the compiled WASM artefacts when you later attach them to GitHub releases.

> **Guiding principles**
> - Keep artefacts **out of git** (use CI/build steps to generate them).
> - Prefer deterministic names that encode the contract + version.
> - Avoid committing large binaries; attach them to releases instead.

---

## Build output path

The contract build process currently produces this file:

- `target/wasm32-unknown-unknown/release/savings_vault.wasm`

This is already documented in `README.md` under the **Build** section.

---

## Suggested filename convention

When copying/renaming the compiled WASM for release attachments, use this format:

```text
savings-vault_${VERSION}.wasm
```

Where:
- `VERSION` should be the project version (for example `0.1.0`) or the git tag/semver you use for releases.

Optional (only if useful):

```text
savings-vault_${VERSION}_${COMMIT_SHA}.wasm
```

---

## Versioning

Pick one source of truth for `VERSION`, for example:
- The Rust crate/workspace version, or
- The git tag used to create the GitHub release.

Whatever you choose, ensure the same `VERSION` value is used in:
- the filename
- the GitHub release tag/name

---

## Checksums (optional but recommended)

If you want extra integrity/traceability, publish SHA-256 checksums alongside the WASM (e.g., in the GitHub release description):

```text
SHA256(savings-vault_${VERSION}.wasm) = <hash>
```

You can generate it locally with:

- PowerShell:
  - `Get-FileHash -Algorithm SHA256 <file>`

---

## Should WASM artefacts be committed?

No.

WASM build outputs are generated into `target/` and should be treated as build artefacts rather than source.

The repo’s `.gitignore` already excludes `target/`, so committing the WASM is not intended.

---

## Example workflow

1. Build in release mode.
2. Copy/rename the WASM into a temporary release directory.
3. Attach `savings-vault_${VERSION}.wasm` to the GitHub release.
4. (Optional) include a checksum in the release notes.

