# Changelog

## v0.2.0 — 2026-10-09 (not yet deployed)

The five contracts keep every v0.1.0 function name, argument list, and
behavior, and add the items below. Because storage layout and event shapes are
unchanged or only extended where noted, off-chain callers keep working, but the
contracts must be **redeployed**; the testnet IDs in the README are v0.1.0.

### Fixed
- **Storage lifetime.** Persistent records were extended to only 100,000 ledgers
  (about 6 days) and nothing ever bumped them. Entries now extend to the
  network's maximum, read at call time. (Archived entries can be restored on
  protocol 23+, but restoration costs fees and breaks reads until done.)
- **Panics replaced with typed errors.** Missing records, double registration,
  and failed checks used `unwrap()` and `assert!`. Each contract now has a
  `contracterror` enum, as CONTRIBUTING.md already required.
- Custody sequence arithmetic uses `checked_add`.
- `revoke` on an already revoked grant no longer emits a second event, and a
  retry of an identical anchor, attestation, or transfer stays silent.

### Added
- `bump` keep-alive on every registry (`bump_head` and `bump_event` for
  custody). Anyone can call it; it moves no funds and changes no data.
- `access_grant_registry.has_grant`, which the README already listed.
- `controller()` / `admin()` getters so integrators can verify who is authorized.
- Rustdoc on every public type and function.
- 60 tests (up from 10): authorization failures, immutability, retries,
  boundaries, event counts, and exact TTL values.

### Changed
- Events use the `#[contractevent]` macro instead of the deprecated
  `Events::publish`. Topic names are unchanged (`anchored`, `registered`,
  `status`, `transfer`, `attested`, `grant`, `revoke`); the data payload is now
  a map of named fields. `custody_registry.register` now also emits `registered`.
- CI runs format, clippy, tests, and the wasm release build.

## v0.1.0
- Initial five registries deployed to Stellar testnet.
