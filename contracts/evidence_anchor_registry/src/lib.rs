#![no_std]
//! Evidence anchor registry.
//!
//! Stores one immutable `Anchor` per evidence version: the SHA-256 of a
//! manifest describing the evidence, plus an opaque reference to whoever
//! submitted it. Raw evidence never touches the chain; only 32-byte digests
//! and references do.
//!
//! Only the `controller` set at deployment can anchor. Anchoring the same
//! values again is a harmless no-op (so retries are safe); anchoring
//! different values for an existing reference is rejected.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
    Symbol,
};

#[cfg(test)]
mod test;

/// An anchored manifest hash and where in the ledger it was recorded.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Anchor {
    /// SHA-256 of the canonical evidence manifest.
    pub manifest_hash: BytesN<32>,
    /// Opaque reference to the submitting party.
    pub submitter_ref: BytesN<32>,
    /// Ledger sequence at which the anchor was first recorded.
    pub ledger: u32,
}

/// Errors this contract can return.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The constructor has not run, so there is no controller.
    NotInitialized = 1,
    /// An anchor already exists for this reference with different values.
    ImmutableAnchorConflict = 2,
    /// No anchor exists for this reference.
    NotFound = 3,
}

/// Emitted the first time an evidence version is anchored.
#[contractevent(topics = ["anchored"])]
pub struct Anchored {
    #[topic]
    pub evidence_version_ref: BytesN<32>,
    pub manifest_hash: BytesN<32>,
    pub submitter_ref: BytesN<32>,
    pub ledger: u32,
}

fn controller_key(env: &Env) -> Symbol {
    Symbol::new(env, "controller")
}

fn controller(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&controller_key(env))
        .ok_or(Error::NotInitialized)
}

/// Extend an entry to the network's maximum lifetime. The maximum is a network
/// parameter, so it is read at call time instead of being hard-coded. The
/// threshold avoids paying for an extension on every write.
fn extend(env: &Env, key: &BytesN<32>) {
    let max = env.storage().max_ttl();
    env.storage().persistent().extend_ttl(key, max / 2, max);
}

#[contract]
pub struct EvidenceAnchorRegistry;

#[contractimpl]
impl EvidenceAnchorRegistry {
    /// Set the controller, the only address allowed to anchor.
    pub fn __constructor(env: Env, controller: Address) {
        env.storage()
            .instance()
            .set(&controller_key(&env), &controller);
    }

    /// Anchor `manifest_hash` for `evidence_version_ref`.
    ///
    /// Requires the controller's authorization. Repeating the exact same call
    /// succeeds without changing anything. Calling again with a different
    /// `manifest_hash` or `submitter_ref` fails with `ImmutableAnchorConflict`.
    pub fn anchor(
        env: Env,
        evidence_version_ref: BytesN<32>,
        manifest_hash: BytesN<32>,
        submitter_ref: BytesN<32>,
    ) -> Result<(), Error> {
        controller(&env)?.require_auth();

        if let Some(prior) = env
            .storage()
            .persistent()
            .get::<_, Anchor>(&evidence_version_ref)
        {
            if prior.manifest_hash == manifest_hash && prior.submitter_ref == submitter_ref {
                return Ok(());
            }
            return Err(Error::ImmutableAnchorConflict);
        }

        let ledger = env.ledger().sequence();
        let proof = Anchor {
            manifest_hash: manifest_hash.clone(),
            submitter_ref: submitter_ref.clone(),
            ledger,
        };
        env.storage()
            .persistent()
            .set(&evidence_version_ref, &proof);
        extend(&env, &evidence_version_ref);

        Anchored {
            evidence_version_ref,
            manifest_hash,
            submitter_ref,
            ledger,
        }
        .publish(&env);
        Ok(())
    }

    /// Read an anchor, or `None` if the reference was never anchored.
    pub fn get(env: Env, evidence_version_ref: BytesN<32>) -> Option<Anchor> {
        env.storage().persistent().get(&evidence_version_ref)
    }

    /// The address allowed to anchor.
    pub fn controller(env: Env) -> Result<Address, Error> {
        controller(&env)
    }

    /// Extend an anchor's storage lifetime. Callable by anyone: it moves no
    /// funds and changes no data, so a keeper job needs no signature.
    pub fn bump(env: Env, evidence_version_ref: BytesN<32>) -> Result<(), Error> {
        if !env.storage().persistent().has(&evidence_version_ref) {
            return Err(Error::NotFound);
        }
        extend(&env, &evidence_version_ref);
        Ok(())
    }
}
