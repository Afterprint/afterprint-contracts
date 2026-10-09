#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{
    storage::Persistent as _, Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke,
};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

fn bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn setup<'a>(env: &Env) -> (Address, EvidenceAnchorRegistryClient<'a>) {
    env.mock_all_auths();
    let controller = Address::generate(env);
    let id = env.register(EvidenceAnchorRegistry, (&controller,));
    (controller, EvidenceAnchorRegistryClient::new(env, &id))
}

#[test]
fn anchors_and_reads_back() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.sequence_number = 777);
    let (_c, client) = setup(&env);

    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    let anchor = client.get(&bytes(&env, 1)).unwrap();
    assert_eq!(anchor.manifest_hash, bytes(&env, 2));
    assert_eq!(anchor.submitter_ref, bytes(&env, 3));
    assert_eq!(anchor.ledger, 777);
}

#[test]
fn unknown_reference_reads_as_none() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    assert!(client.get(&bytes(&env, 9)).is_none());
}

#[test]
fn repeating_the_same_anchor_is_a_no_op_and_keeps_the_original_ledger() {
    let env = Env::default();
    env.ledger().with_mut(|l| l.sequence_number = 10);
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    env.ledger().with_mut(|l| l.sequence_number = 500);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    assert_eq!(client.get(&bytes(&env, 1)).unwrap().ledger, 10);
}

#[test]
fn a_retry_does_not_emit_a_second_event() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));
    let after_first = env.events().all().events().len();
    assert_eq!(after_first, 1);

    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));
    assert_eq!(
        env.events().all().events().len(),
        0,
        "a no-op retry must stay silent"
    );
}

#[test]
fn a_different_manifest_for_an_existing_reference_is_rejected() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    let r = client.try_anchor(&bytes(&env, 1), &bytes(&env, 9), &bytes(&env, 3));
    assert_eq!(r.err(), Some(Ok(Error::ImmutableAnchorConflict)));
    assert_eq!(
        client.get(&bytes(&env, 1)).unwrap().manifest_hash,
        bytes(&env, 2)
    );
}

#[test]
fn a_different_submitter_for_an_existing_reference_is_rejected() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    let r = client.try_anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 9));
    assert_eq!(r.err(), Some(Ok(Error::ImmutableAnchorConflict)));
}

#[test]
fn distinct_references_are_independent() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));
    client.anchor(&bytes(&env, 4), &bytes(&env, 5), &bytes(&env, 6));

    assert_eq!(
        client.get(&bytes(&env, 1)).unwrap().manifest_hash,
        bytes(&env, 2)
    );
    assert_eq!(
        client.get(&bytes(&env, 4)).unwrap().manifest_hash,
        bytes(&env, 5)
    );
}

#[test]
fn anchoring_without_any_authorization_fails() {
    let env = Env::default();
    let controller = Address::generate(&env);
    let id = env.register(EvidenceAnchorRegistry, (&controller,));
    let client = EvidenceAnchorRegistryClient::new(&env, &id);

    assert!(client
        .try_anchor(&bytes(&env, 1), &bytes(&env, 1), &bytes(&env, 1))
        .is_err());
    assert!(client.get(&bytes(&env, 1)).is_none());
}

#[test]
fn only_the_controller_can_anchor() {
    let env = Env::default();
    let (_controller, client) = setup(&env);
    let stranger = Address::generate(&env);
    let r = bytes(&env, 1);

    let result = client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "anchor",
                args: (r.clone(), r.clone(), r.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_anchor(&r, &r, &r);

    assert!(
        result.is_err(),
        "a non-controller signature must be refused"
    );
    assert!(client.get(&r).is_none());
}

#[test]
fn the_controller_is_readable() {
    let env = Env::default();
    let (controller, client) = setup(&env);
    assert_eq!(client.controller(), controller);
}

#[test]
fn bump_requires_an_existing_anchor_and_needs_no_authorization() {
    let env = Env::default();
    let (_c, client) = setup(&env);

    let r = client.try_bump(&bytes(&env, 1));
    assert_eq!(r.err(), Some(Ok(Error::NotFound)));

    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));
    assert!(client.mock_auths(&[]).try_bump(&bytes(&env, 1)).is_ok());
}

#[test]
fn anchors_are_kept_alive_to_the_network_maximum() {
    let env = Env::default();
    env.ledger().with_mut(|l| {
        l.sequence_number = 1_000;
        l.min_persistent_entry_ttl = 500;
        l.max_entry_ttl = 100_000;
    });
    let (_c, client) = setup(&env);
    client.anchor(&bytes(&env, 1), &bytes(&env, 2), &bytes(&env, 3));

    let ttl = || {
        env.as_contract(&client.address, || {
            env.storage().persistent().get_ttl(&bytes(&env, 1))
        })
    };
    // Extended to the maximum, not the 500-ledger minimum.
    assert_eq!(ttl(), 100_000 - 1);

    // After half the lifetime passes, a keeper bump restores the full lifetime.
    env.ledger().with_mut(|l| l.sequence_number += 60_000);
    assert!(ttl() < 50_000);
    client.mock_auths(&[]).bump(&bytes(&env, 1));
    assert_eq!(ttl(), 100_000 - 1);
}
