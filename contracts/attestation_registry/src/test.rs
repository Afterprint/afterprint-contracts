#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

fn bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn setup<'a>(env: &Env) -> (Address, AttestationRegistryClient<'a>) {
    env.mock_all_auths();
    let controller = Address::generate(env);
    let id = env.register(AttestationRegistry, (&controller,));
    (controller, AttestationRegistryClient::new(env, &id))
}

#[test]
fn records_and_reads_back_an_attestation() {
    let env = Env::default();
    let (_c, client) = setup(&env);

    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );

    let proof = client.get(&bytes(&env, 1)).unwrap();
    assert_eq!(proof.subject_ref, bytes(&env, 2));
    assert_eq!(proof.statement_hash, bytes(&env, 3));
    assert_eq!(proof.issuer_ref, bytes(&env, 4));
}

#[test]
fn unknown_attestation_reads_as_none() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    assert!(client.get(&bytes(&env, 1)).is_none());
}

#[test]
fn repeating_identical_values_is_a_silent_no_op() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );
    assert_eq!(env.events().all().events().len(), 1);

    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
fn changing_any_single_field_is_rejected() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );

    let attempts = [
        (9, 3, 4), // subject differs
        (2, 9, 4), // statement differs
        (2, 3, 9), // issuer differs
    ];
    for (subject, statement, issuer) in attempts {
        let r = client.try_attest(
            &bytes(&env, 1),
            &bytes(&env, subject),
            &bytes(&env, statement),
            &bytes(&env, issuer),
        );
        assert_eq!(r.err(), Some(Ok(Error::ImmutableAttestationConflict)));
    }
    assert_eq!(
        client.get(&bytes(&env, 1)).unwrap().statement_hash,
        bytes(&env, 3)
    );
}

#[test]
fn attestations_with_different_references_coexist() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );
    client.attest(
        &bytes(&env, 5),
        &bytes(&env, 2),
        &bytes(&env, 6),
        &bytes(&env, 4),
    );

    assert_eq!(
        client.get(&bytes(&env, 1)).unwrap().statement_hash,
        bytes(&env, 3)
    );
    assert_eq!(
        client.get(&bytes(&env, 5)).unwrap().statement_hash,
        bytes(&env, 6)
    );
}

#[test]
fn attesting_without_authorization_fails() {
    let env = Env::default();
    let controller = Address::generate(&env);
    let id = env.register(AttestationRegistry, (&controller,));
    let client = AttestationRegistryClient::new(&env, &id);

    let h = bytes(&env, 1);
    assert!(client.try_attest(&h, &h, &h, &h).is_err());
    assert!(client.get(&h).is_none());
}

#[test]
fn only_the_controller_can_attest() {
    let env = Env::default();
    let (_controller, client) = setup(&env);
    let stranger = Address::generate(&env);
    let h = bytes(&env, 1);

    let result = client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "attest",
                args: (h.clone(), h.clone(), h.clone(), h.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_attest(&h, &h, &h, &h);

    assert!(result.is_err());
    assert!(client.get(&h).is_none());
}

#[test]
fn the_controller_is_readable() {
    let env = Env::default();
    let (controller, client) = setup(&env);
    assert_eq!(client.controller(), controller);
}

#[test]
fn bump_needs_an_existing_attestation_and_no_authorization() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    assert_eq!(
        client.try_bump(&bytes(&env, 1)).err(),
        Some(Ok(Error::NotFound))
    );

    client.attest(
        &bytes(&env, 1),
        &bytes(&env, 2),
        &bytes(&env, 3),
        &bytes(&env, 4),
    );
    assert!(client.mock_auths(&[]).try_bump(&bytes(&env, 1)).is_ok());
}
