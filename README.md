<p align="center">
  <img src="https://raw.githubusercontent.com/Afterprint/afterprint-web/main/public/afterprint-logo.png" width="72" alt="Afterprint logo" />
</p>

<h1 align="center">Afterprint — Soroban Smart Contracts</h1>

<p align="center">
  Five Soroban contracts anchoring evidence integrity, chain-of-custody, and attestations on Stellar — without ever putting evidence content on-chain.
</p>

<p align="center">
  <a href="https://github.com/Afterprint/afterprint-contracts/actions/workflows/ci.yml"><img src="https://github.com/Afterprint/afterprint-contracts/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/network-Stellar%20Testnet-blue" alt="Stellar Testnet">
  <img src="https://img.shields.io/badge/soroban--sdk-26.1.1-orange" alt="soroban-sdk 26.1.1">
  <img src="https://img.shields.io/github/license/Afterprint/afterprint-contracts" alt="License">
</p>

---

## Why this exists

Investigators and legal reviewers working with digital evidence (recordings, documents, sensor logs) need a way to prove a file hasn't been altered since it was collected, and to prove a documented chain of custody — without exposing the evidence itself, or trusting a single company's database not to have quietly edited a record. Afterprint anchors cryptographic digests and custody events on Stellar so that integrity claims can be checked by anyone, independent of Afterprint's own servers.

Raw evidence, personal data, and case content are **never stored on-chain** — only SHA-256 digests and opaque references.

## Architecture

| Contract | Purpose | Methods |
|---|---|---|
| `case_registry` | Registers opaque case references with a controller, a metadata hash, and a status. Registration needs both the admin and the case controller to sign. | `register`, `set_status`, `get`, `admin`, `bump` |
| `evidence_anchor_registry` | Anchors an immutable manifest hash per evidence version. Identical retries are no-ops; conflicting values are rejected. | `anchor`, `get`, `controller`, `bump` |
| `custody_registry` | Append-only custody log. Each transfer commits to the previous head hash and must be signed by the current custodian. | `register`, `transfer`, `head`, `event`, `controller`, `bump_head`, `bump_event` |
| `attestation_registry` | Immutable attestations linking a subject reference to a statement hash. | `attest`, `get`, `controller`, `bump` |
| `access_grant_registry` | Access policy grants with permanent revocation: a revoked reference can never be granted again. | `grant`, `revoke`, `get`, `has_grant`, `controller`, `bump` |

Each contract is deployed independently and owned by a single `admin`/`controller` address set at construction, so `afterprint-api` (the off-chain service that calls into these) can rotate its own signing key without redeploying contracts.

## Versions and deployments

| Version | Status |
|---|---|
| **v0.2.0** (this source) | Typed errors instead of panics, `#[contractevent]` events, network-maximum storage lifetimes, keep-alive `bump` functions, `has_grant`, and 60 tests. **Not yet deployed.** |
| v0.1.0 | The contracts deployed below. They are live on testnet but do not match this source. |

Redeploy with `./scripts/deploy.sh` and update the table below before pointing `afterprint-api` at v0.2.0. The v0.2.0 event shapes differ from v0.1.0 (see [CHANGELOG.md](./CHANGELOG.md)).

## Deployed contracts — Stellar Testnet (v0.1.0)

| Contract | Contract ID |
|---|---|
| `case_registry` | [`CD7FA44VPGLEXDGBKXKXIYJN4VB3T365LIP7RBSS6D4D3PPIRBY7UTRI`](https://stellar.expert/explorer/testnet/contract/CD7FA44VPGLEXDGBKXKXIYJN4VB3T365LIP7RBSS6D4D3PPIRBY7UTRI) |
| `evidence_anchor_registry` | [`CC5K23WCGQVA2XSAU6ZYDEKUQGKBVQD3EUA2GH6GRNPZLOK7NWX4ZJVI`](https://stellar.expert/explorer/testnet/contract/CC5K23WCGQVA2XSAU6ZYDEKUQGKBVQD3EUA2GH6GRNPZLOK7NWX4ZJVI) |
| `custody_registry` | [`CADJOSDUL232LHJ5RMYPDIE6IJBSVUNY762MJGGDNAFGWPW6QTP3Z4WC`](https://stellar.expert/explorer/testnet/contract/CADJOSDUL232LHJ5RMYPDIE6IJBSVUNY762MJGGDNAFGWPW6QTP3Z4WC) |
| `attestation_registry` | [`CANWBSU6OUK7QAL5NS2CGD5BCL4ZBFZARSD572S6AXQXWZLQMLX6SK3G`](https://stellar.expert/explorer/testnet/contract/CANWBSU6OUK7QAL5NS2CGD5BCL4ZBFZARSD572S6AXQXWZLQMLX6SK3G) |
| `access_grant_registry` | [`CAYMFABIOZ7GYOGAQENLUFYZXPSKJNV6UWPNZSJH3GSPL6FAPSXKP6X4`](https://stellar.expert/explorer/testnet/contract/CAYMFABIOZ7GYOGAQENLUFYZXPSKJNV6UWPNZSJH3GSPL6FAPSXKP6X4) |

## Quick start

```bash
# Rust + the wasm target Soroban actually needs (not wasm32-unknown-unknown —
# soroban-sdk 26+ rejects that target on Rust 1.82+)
rustup target add wasm32v1-none

# Stellar CLI
cargo install --locked stellar-cli

# Run the unit tests (each contract tests both authorized and unauthorized calls)
cargo test --workspace --lib

# Build optimized WASM for all five contracts
cargo build --workspace --target wasm32v1-none --release
```

### Deploying

```bash
stellar keys add deployer --secure-store   # imports your seed phrase into the OS keystore
STELLAR_ADMIN=<your-G...-address> ./scripts/deploy.sh deployer
```

Contract IDs are written to `.contracts.env` for copying into `afterprint-api`'s environment.

## Contributing

Issues are scoped by contract and labeled by complexity — see the [issue tracker](https://github.com/Afterprint/afterprint-contracts/issues). Read [CONTRIBUTING.md](./CONTRIBUTING.md) before opening a PR, and [SECURITY.md](./SECURITY.md) if you're reporting a vulnerability.

## Maintainer

| | |
|---|---|
| **GitHub** | [@helloworld1-star](https://github.com/helloworld1-star) |
| **Email** | devt14985@gmail.com |

---

<p align="center">
  <a href="https://github.com/Afterprint/afterprint-contracts/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=Afterprint/afterprint-contracts" alt="Contributors" />
  </a>
</p>
