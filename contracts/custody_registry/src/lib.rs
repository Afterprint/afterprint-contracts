#![no_std]
//! Custody registry.
//!
//! An append-only log of who holds each evidence version. Every transfer
//! commits to the previous head hash, so the history forms a chain: a missed,
//! reordered, or forged step is detected because its `previous_hash` will not
//! match the current head.
//!
//! The `controller` registers a version with its first custodian. After that,
//! the *current custodian* must sign each transfer. Retrying an identical
//! transfer is a harmless no-op; a conflicting retry is rejected.

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, BytesN, Env,
    Symbol,
};

#[cfg(test)]
mod test;

/// Storage keys for custody state.
#[contracttype]
#[derive(Clone)]
pub enum Key {
    /// Current custody head for an evidence version.
    Head(BytesN<32>),
    /// A recorded transfer, keyed by its event reference.
    Event(BytesN<32>),
}

/// The current state of custody for one evidence version.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Head {
    /// Who holds the evidence now.
    pub custodian: Address,
    /// Hash of the latest custody event (or the initial hash).
    pub hash: BytesN<32>,
    /// Number of transfers so far. Starts at 0.
    pub sequence: u32,
    /// Timestamp of the latest transfer. Starts at 0.
    pub timestamp: u64,
}

/// One recorded custody transfer.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transfer {
    pub version: BytesN<32>,
    pub from: Address,
    pub to: Address,
    pub previous_hash: BytesN<32>,
    pub event_hash: BytesN<32>,
    pub timestamp: u64,
    /// Position in the chain, starting at 1.
    pub sequence: u32,
}

/// Errors this contract can return.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The constructor has not run, so there is no controller.
    NotInitialized = 1,
    /// This evidence version already has a custody head.
    AlreadyRegistered = 2,
    /// This evidence version has not been registered.
    NotRegistered = 3,
    /// An event with this reference exists but with different details.
    ConflictingRetry = 4,
    /// `from` is not the current custodian.
    NotExpectedCustodian = 5,
    /// `previous_hash` is not the current head hash.
    StaleHead = 6,
    /// Timestamp is before the last transfer or after the ledger time.
    InvalidTimestamp = 7,
    /// `from` and `to` are the same address.
    SameCustodian = 8,
    /// The chain is too long to count.
    Overflow = 9,
    /// No such record exists.
    NotFound = 10,
}

/// Emitted when an evidence version is registered.
#[contractevent(topics = ["registered"])]
pub struct Registered {
    #[topic]
    pub version: BytesN<32>,
    pub custodian: Address,
}

/// Emitted for each new custody transfer.
#[contractevent(topics = ["transfer"])]
pub struct Transferred {
    #[topic]
    pub event_ref: BytesN<32>,
    pub version: BytesN<32>,
    pub from: Address,
    pub to: Address,
    pub previous_hash: BytesN<32>,
    pub event_hash: BytesN<32>,
    pub timestamp: u64,
    pub sequence: u32,
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
fn extend(env: &Env, key: &Key) {
    let max = env.storage().max_ttl();
    env.storage().persistent().extend_ttl(key, max / 2, max);
}

#[contract]
pub struct CustodyRegistry;

#[contractimpl]
impl CustodyRegistry {
    /// Set the controller, the only address allowed to register versions.
    pub fn __constructor(env: Env, controller: Address) {
        env.storage()
            .instance()
            .set(&controller_key(&env), &controller);
    }

    /// Register an evidence version with its first custodian and initial hash.
    /// Requires the controller's authorization.
    pub fn register(
        env: Env,
        version: BytesN<32>,
        custodian: Address,
        initial_hash: BytesN<32>,
    ) -> Result<(), Error> {
        controller(&env)?.require_auth();

        let key = Key::Head(version.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AlreadyRegistered);
        }
        let head = Head {
            custodian: custodian.clone(),
            hash: initial_hash,
            sequence: 0,
            timestamp: 0,
        };
        env.storage().persistent().set(&key, &head);
        extend(&env, &key);

        Registered { version, custodian }.publish(&env);
        Ok(())
    }

    /// Record a custody transfer. Requires `from` (the current custodian) to
    /// authorize.
    ///
    /// Checks, in order: the version is registered; an identical earlier call
    /// with the same `event_ref` is a no-op, a different one is rejected; `from`
    /// is the current custodian; `previous_hash` is the current head; the
    /// timestamp is not before the last transfer and not in the future;
    /// `from` and `to` differ.
    #[allow(clippy::too_many_arguments)]
    pub fn transfer(
        env: Env,
        event_ref: BytesN<32>,
        version: BytesN<32>,
        from: Address,
        to: Address,
        previous_hash: BytesN<32>,
        event_hash: BytesN<32>,
        timestamp: u64,
    ) -> Result<(), Error> {
        from.require_auth();

        let head_key = Key::Head(version.clone());
        let mut head: Head = env
            .storage()
            .persistent()
            .get(&head_key)
            .ok_or(Error::NotRegistered)?;

        let event_key = Key::Event(event_ref.clone());
        if let Some(old) = env.storage().persistent().get::<_, Transfer>(&event_key) {
            let same = old.version == version
                && old.from == from
                && old.to == to
                && old.event_hash == event_hash
                && old.previous_hash == previous_hash
                && old.timestamp == timestamp;
            return if same {
                Ok(())
            } else {
                Err(Error::ConflictingRetry)
            };
        }

        if head.custodian != from {
            return Err(Error::NotExpectedCustodian);
        }
        if head.hash != previous_hash {
            return Err(Error::StaleHead);
        }
        if timestamp < head.timestamp || timestamp > env.ledger().timestamp() {
            return Err(Error::InvalidTimestamp);
        }
        if from == to {
            return Err(Error::SameCustodian);
        }

        let sequence = head.sequence.checked_add(1).ok_or(Error::Overflow)?;
        let transfer = Transfer {
            version: version.clone(),
            from: from.clone(),
            to: to.clone(),
            previous_hash: previous_hash.clone(),
            event_hash: event_hash.clone(),
            timestamp,
            sequence,
        };
        head.sequence = sequence;
        head.custodian = to.clone();
        head.hash = event_hash.clone();
        head.timestamp = timestamp;

        env.storage().persistent().set(&event_key, &transfer);
        env.storage().persistent().set(&head_key, &head);
        extend(&env, &event_key);
        extend(&env, &head_key);

        Transferred {
            event_ref,
            version,
            from,
            to,
            previous_hash,
            event_hash,
            timestamp,
            sequence,
        }
        .publish(&env);
        Ok(())
    }

    /// Current custody head for a version, or `None` if not registered.
    pub fn head(env: Env, version: BytesN<32>) -> Option<Head> {
        env.storage().persistent().get(&Key::Head(version))
    }

    /// A recorded transfer, or `None` if the reference is unknown.
    pub fn event(env: Env, event_ref: BytesN<32>) -> Option<Transfer> {
        env.storage().persistent().get(&Key::Event(event_ref))
    }

    /// The address allowed to register versions.
    pub fn controller(env: Env) -> Result<Address, Error> {
        controller(&env)
    }

    /// Extend a version's custody head lifetime. Callable by anyone: it moves
    /// no funds and changes no data.
    pub fn bump_head(env: Env, version: BytesN<32>) -> Result<(), Error> {
        let key = Key::Head(version);
        if !env.storage().persistent().has(&key) {
            return Err(Error::NotFound);
        }
        extend(&env, &key);
        Ok(())
    }

    /// Extend a transfer record's lifetime. Callable by anyone.
    pub fn bump_event(env: Env, event_ref: BytesN<32>) -> Result<(), Error> {
        let key = Key::Event(event_ref);
        if !env.storage().persistent().has(&key) {
            return Err(Error::NotFound);
        }
        extend(&env, &key);
        Ok(())
    }
}
