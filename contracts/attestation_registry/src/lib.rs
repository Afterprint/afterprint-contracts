#![no_std]
//! Attestation registry.
//!
//! Records an immutable link between a subject reference and the hash of a
//! statement made about it, plus an opaque reference to the issuer. The
//! statement text itself is never stored on-chain.
//!
//! Only the `controller` set at deployment can attest. Attesting identical
//! values again is a harmless no-op, so retries are safe; attesting different
//! values for an existing reference is rejected.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
    Symbol,
};

#[cfg(test)]
mod test;

/// An attestation about a subject.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    /// Opaque reference to what the statement is about.
    pub subject_ref: BytesN<32>,
    /// SHA-256 of the statement held off-chain.
    pub statement_hash: BytesN<32>,
    /// Opaque reference to who made the statement.
    pub issuer_ref: BytesN<32>,
}

/// Errors this contract can return.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The constructor has not run, so there is no controller.
    NotInitialized = 1,
    /// An attestation already exists for this reference with different values.
    ImmutableAttestationConflict = 2,
    /// No attestation exists for this reference.
    NotFound = 3,
}

/// Emitted the first time an attestation is recorded.
#[contractevent(topics = ["attested"])]
pub struct Attested {
    #[topic]
    pub attestation_ref: BytesN<32>,
    pub subject_ref: BytesN<32>,
    pub statement_hash: BytesN<32>,
    pub issuer_ref: BytesN<32>,
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

/// Extend an entry to the network's maximum lifetime (read at call time, since
/// it is a network parameter). The threshold avoids paying on every write.
fn extend(env: &Env, key: &BytesN<32>) {
    let max = env.storage().max_ttl();
    env.storage().persistent().extend_ttl(key, max / 2, max);
}

#[contract]
pub struct AttestationRegistry;

#[contractimpl]
impl AttestationRegistry {
    /// Set the controller, the only address allowed to attest.
    pub fn __constructor(env: Env, controller: Address) {
        env.storage()
            .instance()
            .set(&controller_key(&env), &controller);
    }

    /// Record an attestation. Requires the controller's authorization.
    ///
    /// Repeating identical values succeeds without changing anything. Different
    /// values for an existing `attestation_ref` fail with
    /// `ImmutableAttestationConflict`.
    pub fn attest(
        env: Env,
        attestation_ref: BytesN<32>,
        subject_ref: BytesN<32>,
        statement_hash: BytesN<32>,
        issuer_ref: BytesN<32>,
    ) -> Result<(), Error> {
        controller(&env)?.require_auth();

        let proof = Attestation {
            subject_ref: subject_ref.clone(),
            statement_hash: statement_hash.clone(),
            issuer_ref: issuer_ref.clone(),
        };
        if let Some(prior) = env
            .storage()
            .persistent()
            .get::<_, Attestation>(&attestation_ref)
        {
            if prior == proof {
                return Ok(());
            }
            return Err(Error::ImmutableAttestationConflict);
        }

        env.storage().persistent().set(&attestation_ref, &proof);
        extend(&env, &attestation_ref);

        Attested {
            attestation_ref,
            subject_ref,
            statement_hash,
            issuer_ref,
        }
        .publish(&env);
        Ok(())
    }

    /// Read an attestation, or `None` if it was never recorded.
    pub fn get(env: Env, attestation_ref: BytesN<32>) -> Option<Attestation> {
        env.storage().persistent().get(&attestation_ref)
    }

    /// The address allowed to attest.
    pub fn controller(env: Env) -> Result<Address, Error> {
        controller(&env)
    }

    /// Extend an attestation's storage lifetime. Callable by anyone: it moves
    /// no funds and changes no data.
    pub fn bump(env: Env, attestation_ref: BytesN<32>) -> Result<(), Error> {
        if !env.storage().persistent().has(&attestation_ref) {
            return Err(Error::NotFound);
        }
        extend(&env, &attestation_ref);
        Ok(())
    }
}
