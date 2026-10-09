#![no_std]
//! Access grant registry.
//!
//! Records that an access policy (identified by its hash) was granted under an
//! opaque reference, and whether it has since been revoked. Revocation is
//! permanent: a reference can never be granted again, so a revoked grant
//! cannot be quietly reinstated.
//!
//! Only the `controller` set at deployment can grant or revoke.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
    Symbol,
};

#[cfg(test)]
mod test;

/// A recorded access grant.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grant {
    /// SHA-256 of the access policy held off-chain.
    pub policy_hash: BytesN<32>,
    /// True once the grant has been revoked. Never reset.
    pub revoked: bool,
}

/// Errors this contract can return.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The constructor has not run, so there is no controller.
    NotInitialized = 1,
    /// This grant reference was already used, even if since revoked.
    GrantRefAlreadyUsed = 2,
    /// No grant exists for this reference.
    NotFound = 3,
}

/// Emitted when an access grant is created.
#[contractevent(topics = ["grant"])]
pub struct Granted {
    #[topic]
    pub grant_ref: BytesN<32>,
    pub policy_hash: BytesN<32>,
}

/// Emitted when an access grant is revoked.
#[contractevent(topics = ["revoke"])]
pub struct Revoked {
    #[topic]
    pub grant_ref: BytesN<32>,
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
pub struct AccessGrantRegistry;

#[contractimpl]
impl AccessGrantRegistry {
    /// Set the controller, the only address allowed to grant or revoke.
    pub fn __constructor(env: Env, controller: Address) {
        env.storage()
            .instance()
            .set(&controller_key(&env), &controller);
    }

    /// Record a grant. Requires the controller's authorization. A reference can
    /// be used once, including after revocation.
    pub fn grant(env: Env, grant_ref: BytesN<32>, policy_hash: BytesN<32>) -> Result<(), Error> {
        controller(&env)?.require_auth();
        if env.storage().persistent().has(&grant_ref) {
            return Err(Error::GrantRefAlreadyUsed);
        }

        let grant = Grant {
            policy_hash: policy_hash.clone(),
            revoked: false,
        };
        env.storage().persistent().set(&grant_ref, &grant);
        extend(&env, &grant_ref);

        Granted {
            grant_ref,
            policy_hash,
        }
        .publish(&env);
        Ok(())
    }

    /// Revoke a grant. Requires the controller's authorization. Revoking an
    /// already revoked grant succeeds without emitting a second event.
    pub fn revoke(env: Env, grant_ref: BytesN<32>) -> Result<(), Error> {
        controller(&env)?.require_auth();
        let mut grant: Grant = env
            .storage()
            .persistent()
            .get(&grant_ref)
            .ok_or(Error::NotFound)?;
        if grant.revoked {
            return Ok(());
        }

        grant.revoked = true;
        env.storage().persistent().set(&grant_ref, &grant);
        extend(&env, &grant_ref);

        Revoked { grant_ref }.publish(&env);
        Ok(())
    }

    /// Read a grant, or `None` if the reference was never used.
    pub fn get(env: Env, grant_ref: BytesN<32>) -> Option<Grant> {
        env.storage().persistent().get(&grant_ref)
    }

    /// True if the grant exists and has not been revoked.
    pub fn has_grant(env: Env, grant_ref: BytesN<32>) -> bool {
        env.storage()
            .persistent()
            .get::<_, Grant>(&grant_ref)
            .map(|grant| !grant.revoked)
            .unwrap_or(false)
    }

    /// The address allowed to grant and revoke.
    pub fn controller(env: Env) -> Result<Address, Error> {
        controller(&env)
    }

    /// Extend a grant's storage lifetime. Callable by anyone: it moves no funds
    /// and changes no data.
    pub fn bump(env: Env, grant_ref: BytesN<32>) -> Result<(), Error> {
        if !env.storage().persistent().has(&grant_ref) {
            return Err(Error::NotFound);
        }
        extend(&env, &grant_ref);
        Ok(())
    }
}
