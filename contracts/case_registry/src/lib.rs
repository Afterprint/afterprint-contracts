#![no_std]
//! Case registry.
//!
//! Registers opaque 32-byte case references with the address that controls
//! each case, a metadata hash, and a status. No case content, names, or
//! personal data are stored: only references and digests.
//!
//! Registration needs both the contract `admin` and the case `controller` to
//! authorize, so an admin cannot assign a case to someone who did not agree.
//! A reference can be registered once. Only the case controller can change its
//! status afterwards.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
    Symbol,
};

#[cfg(test)]
mod test;

/// Case is open.
pub const STATUS_OPEN: u32 = 0;
/// Case is closed.
pub const STATUS_CLOSED: u32 = 1;
/// Case is archived.
pub const STATUS_ARCHIVED: u32 = 2;

/// What is recorded for a registered case.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseProof {
    /// The address that controls this case and may change its status.
    pub controller: Address,
    /// SHA-256 of the case metadata held off-chain.
    pub metadata_hash: BytesN<32>,
    /// 0 = open, 1 = closed, 2 = archived.
    pub status: u32,
}

/// Errors this contract can return.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The constructor has not run, so there is no admin.
    NotInitialized = 1,
    /// This case reference is already registered.
    AlreadyRegistered = 2,
    /// No case is registered under this reference.
    NotFound = 3,
    /// Status must be 0, 1, or 2.
    InvalidStatus = 4,
}

/// Emitted when a case is registered.
#[contractevent(topics = ["registered"])]
pub struct Registered {
    #[topic]
    pub case_ref: BytesN<32>,
}

/// Emitted when a case status changes.
#[contractevent(topics = ["status"])]
pub struct StatusChanged {
    #[topic]
    pub case_ref: BytesN<32>,
    pub status: u32,
}

fn admin_key(env: &Env) -> Symbol {
    Symbol::new(env, "admin")
}

fn admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&admin_key(env))
        .ok_or(Error::NotInitialized)
}

/// Extend an entry to the network's maximum lifetime (read at call time, since
/// it is a network parameter). The threshold avoids paying on every write.
fn extend(env: &Env, key: &BytesN<32>) {
    let max = env.storage().max_ttl();
    env.storage().persistent().extend_ttl(key, max / 2, max);
}

fn load(env: &Env, case_ref: &BytesN<32>) -> Result<CaseProof, Error> {
    env.storage()
        .persistent()
        .get(case_ref)
        .ok_or(Error::NotFound)
}

#[contract]
pub struct CaseRegistry;

#[contractimpl]
impl CaseRegistry {
    /// Set the admin that must co-sign every registration.
    pub fn __constructor(env: Env, admin: Address) {
        env.storage().instance().set(&admin_key(&env), &admin);
    }

    /// Register a case. Requires both the admin and `controller` to authorize.
    /// Fails with `AlreadyRegistered` if the reference exists.
    pub fn register(
        env: Env,
        case_ref: BytesN<32>,
        controller: Address,
        metadata_hash: BytesN<32>,
    ) -> Result<(), Error> {
        admin(&env)?.require_auth();
        controller.require_auth();
        if env.storage().persistent().has(&case_ref) {
            return Err(Error::AlreadyRegistered);
        }

        let proof = CaseProof {
            controller,
            metadata_hash,
            status: STATUS_OPEN,
        };
        env.storage().persistent().set(&case_ref, &proof);
        extend(&env, &case_ref);

        Registered { case_ref }.publish(&env);
        Ok(())
    }

    /// Change a case status (0 open, 1 closed, 2 archived). Requires the case
    /// controller's authorization.
    pub fn set_status(env: Env, case_ref: BytesN<32>, status: u32) -> Result<(), Error> {
        if status > STATUS_ARCHIVED {
            return Err(Error::InvalidStatus);
        }
        let mut proof = load(&env, &case_ref)?;
        proof.controller.require_auth();

        proof.status = status;
        env.storage().persistent().set(&case_ref, &proof);
        extend(&env, &case_ref);

        StatusChanged { case_ref, status }.publish(&env);
        Ok(())
    }

    /// Read a case, or `None` if it was never registered.
    pub fn get(env: Env, case_ref: BytesN<32>) -> Option<CaseProof> {
        env.storage().persistent().get(&case_ref)
    }

    /// The admin that must co-sign registrations.
    pub fn admin(env: Env) -> Result<Address, Error> {
        admin(&env)
    }

    /// Extend a case's storage lifetime. Callable by anyone: it moves no funds
    /// and changes no data.
    pub fn bump(env: Env, case_ref: BytesN<32>) -> Result<(), Error> {
        load(&env, &case_ref)?;
        extend(&env, &case_ref);
        Ok(())
    }
}
