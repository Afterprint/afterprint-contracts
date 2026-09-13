# Afterprint — Soroban Smart Contracts

Five Soroban smart contracts on the Stellar network providing decentralized, tamper-evident trust stamps for evidence hashes, chain of custody, and attestations.

## Architecture

In strict compliance with the **Afterprint Master Build Specification** (`AFTERPRINT_MASTER_BUILD.md`), raw case evidence and personally identifying information are **never stored on-chain**. Only opaque cryptographic references and SHA-256 digests are anchored.

### Registry Contracts

| Contract | Purpose | Methods |
|---|---|---|
| `case_registry` | Registers opaque case references, controller address, and metadata hashes. | `register`, `set_status`, `get` |
| `evidence_anchor_registry` | Anchors immutable evidence manifests per version. Once anchored, cannot be overwritten. | `anchor`, `get` |
| `custody_registry` | Append-only custody transfer logs with monotonic timestamps and actor signatures. | `transfer`, `get_event` |
| `attestation_registry` | Immutable attestation proofs linking subject references to statement hashes. | `attest`, `get` |
| `access_grant_registry` | Cross-organization access policies and revocation proofs. | `grant`, `revoke`, `has_grant` |

## Prerequisites

- Rust stable with `wasm32-unknown-unknown` target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- Stellar CLI (`stellar`):
  ```bash
  cargo install --locked stellar-cli
  ```

## Running Unit Tests

Each contract includes native Soroban unit tests verifying authorized execution and testing that unauthorized calls fail:

```bash
cargo test
```

## Building WASM Contracts

Build optimized WASM binaries for all five contracts:

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Deploying to Stellar Network

Deploy all five contracts using the deployment script:

```bash
# Testnet deployment
STELLAR_NETWORK=testnet STELLAR_SECRET_KEY=<your-secret-key> ./scripts/deploy.sh
```

Contract addresses are automatically saved to `.contracts.env` for seamless consumption by `afterprint-api`.
