# Contributing to afterprint-contracts

## Setup

```bash
rustup target add wasm32v1-none
cargo install --locked stellar-cli
cargo test --workspace
```

## Workflow

1. Pick an issue from the [tracker](https://github.com/Afterprint/afterprint-contracts/issues) — issues are labeled by contract and complexity.
2. Branch from `main`: `git checkout -b feat/short-description`.
3. One logical change per commit, [Conventional Commits](https://www.conventionalcommits.org/) format: `type(scope): description` (e.g. `feat(custody-registry): add revoke method`).
4. Every public function needs a unit test covering both an authorized call and an unauthorized call that should fail.
5. `cargo test --workspace` and `cargo build --workspace --target wasm32v1-none --release` must both pass before opening a PR.
6. Open a PR against `main`. CI must pass.

## Code standards

- No `unwrap()` outside test code — use `Result` and `contracterror` for anything that can fail in a live contract call.
- No floating point — all amounts are integers (stroops / basis points).
- `require_auth()` on every function that mutates state, called on the address that should be authorizing the action — never assume the caller is who they claim without it.
- Storage: use `instance()` for small per-contract config (like `admin`/`controller`), `persistent()` for anything that scales with usage, and extend TTL on persistent writes.
- Emit an event for every state-changing call so off-chain services can index without polling.

## Reporting a security issue

See [SECURITY.md](./SECURITY.md) — do not open a public issue for a vulnerability.
