# Documentation Style Guide

This guide sets shared conventions for documentation and examples in this
repository: the README, `CONTRIBUTING.md`, and everything under `docs/`. It
exists so that contributor-facing docs stay consistent, safe, and easy to
maintain as more of them are added. Follow it for new documentation and when
editing existing docs.

## Testnet wording

This project targets Stellar **testnet** only; it is not production- or
mainnet-ready. Say so explicitly and consistently:

- Refer to the network as **testnet** (lowercase, one word), matching the
  `soroban network add --global testnet` / `stellar network ls` usage
  elsewhere in this repo.
- When a doc discusses deployment, invocation, or funding, state that the
  instructions are for testnet unless the section is explicitly about a
  different environment (e.g. local development).
- Don't imply mainnet support exists or is close. If mainnet is relevant,
  describe it as a future target, not a current capability — see
  [Avoiding production claims](#avoiding-production-claims).
- Point readers to [docs/deployment-environments.md](deployment-environments.md)
  for the canonical RPC URL, network passphrase, and Friendbot details instead
  of restating them; if a doc needs to show them inline for a worked example,
  keep them consistent with that document.

## Avoiding production claims

Docs must not overstate what the contract does or how safe it is to use with
real funds.

- Do not describe the contract as production-ready, audited, or safe for
  mainnet use. If a doc discusses limitations, security, or deployment, add or
  keep a clear statement such as "this contract is for educational and
  testnet use," matching the wording already used in the README's
  [Security Considerations](../README.md#security-considerations) section.
- Be precise about custody: this contract currently tracks balances
  internally and does not transfer or hold real XLM, SAC assets, or other
  tokens. Do not describe `deposit` or `withdraw` as moving real funds. See
  the README's
  [Deposit and custody limitation](../README.md#deposit-and-custody-limitation)
  section for the wording to reuse.
- When describing a feature that does not exist yet (e.g. upgrades, pausing,
  admin recovery), say so plainly and link to the relevant research doc
  (for example [docs/upgrade-strategy.md](upgrade-strategy.md) or
  [docs/pause-design.md](pause-design.md)) instead of describing planned
  behavior as if it were implemented.
- If in doubt about a claim's accuracy, prefer the more conservative wording
  and flag it for maintainer review in the pull request description.

## Placeholders

Use clearly fake, self-explanatory placeholder values in examples so nobody
mistakes them for real data. See
[docs/placeholders.md](placeholders.md) for the canonical placeholder names
(contract ID, user address, admin address, and timestamp) to use in new or
edited docs.

- Use `UPPER_SNAKE_CASE` placeholders such as `YOUR_CONTRACT_ID`,
  `YOUR_ADDRESS`, or `YOUR-USERNAME`, matching existing usage in the README
  and `CONTRIBUTING.md`.
- Never use real contract IDs, secret keys, seed phrases, RPC credentials, or
  other values captured from an actual deployment. Use placeholders or
  clearly synthetic example values instead, per the
  [Security-sensitive contributions](../CONTRIBUTING.md#security-sensitive-contributions)
  section of `CONTRIBUTING.md`.
- Named CLI identities in examples should use generic names already used in
  this repo, such as `deployer`, rather than a name tied to a real account.
- When showing example command output (not a command to run), make that
  explicit — for instance by introducing it as "example output" — so readers
  don't copy it as-is. See
  [docs/deployment-output-example.md](deployment-output-example.md) for the
  pattern to follow.

## Command formatting

- Put every runnable command in a fenced code block with the `bash` language
  tag; use `text` for non-runnable example output.
- One logical command (or a short, clearly related sequence) per code block.
  Don't mix commentary prose into a code block beyond short `#` comments.
- Use the CLI commands and flags already used elsewhere in this repo
  (`cargo build --target wasm32-unknown-unknown`, `cargo test`,
  `cargo fmt --check`, `cargo clippy --tests -- -D warnings`, `soroban ...`)
  rather than introducing new equivalent forms.
- This repository's examples use the `soroban` CLI. If a doc needs to
  mention the newer `stellar` CLI, follow the pattern in
  [docs/troubleshooting.md](troubleshooting.md#quick-pre-deployment-checklist):
  note it as an alternative and keep the primary examples on `soroban` unless
  maintainers update the README to switch.
- Show file paths as they would actually appear (e.g.
  `target/wasm32-unknown-unknown/release/savings_vault.wasm`), not abbreviated
  or guessed paths.

## Linking related docs

- Prefer linking to an existing doc over duplicating its content. If two docs
  would otherwise repeat the same explanation (e.g. custody limitations,
  network configuration), link to the single source of truth instead.
- Use relative Markdown links (`docs/foo.md` from the README,
  `foo.md` or `../README.md#section` from within `docs/`) so links keep
  working when the repository is cloned or viewed on a fork.
- When adding a new document under `docs/`, add it to the README's
  [Documentation](../README.md#documentation) list and to the
  [Project Structure](../README.md#project-structure) tree if it's the kind
  of doc a new contributor would look for.
- Keep section anchors stable where practical, since other docs link to them
  by heading (e.g. `README.md#known-limitations`); if you rename a heading
  that other docs link to, update those links in the same pull request.

## Scope

This guide covers wording and formatting conventions for documentation. It is
not a grammar or tone-of-voice policy beyond what's written above, and it does
not cover Rust doc comments (`///`, `//!`) in contract source, which follow
normal Rust conventions instead.
