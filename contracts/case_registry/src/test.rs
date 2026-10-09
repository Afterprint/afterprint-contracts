#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

fn bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn setup<'a>(env: &Env) -> (Address, CaseRegistryClient<'a>) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let id = env.register(CaseRegistry, (&admin,));
    (admin, CaseRegistryClient::new(env, &id))
}

#[test]
fn registers_a_case_as_open() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let controller = Address::generate(&env);

    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));

    let proof = client.get(&bytes(&env, 1)).unwrap();
    assert_eq!(proof.controller, controller);
    assert_eq!(proof.metadata_hash, bytes(&env, 2));
    assert_eq!(proof.status, STATUS_OPEN);
}

#[test]
fn unknown_case_reads_as_none() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    assert!(client.get(&bytes(&env, 7)).is_none());
}

#[test]
fn a_reference_cannot_be_registered_twice() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let controller = Address::generate(&env);
    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));

    // Even by the same controller with the same hash, and with a new controller.
    let r = client.try_register(&bytes(&env, 1), &controller, &bytes(&env, 2));
    assert_eq!(r.err(), Some(Ok(Error::AlreadyRegistered)));
    let other = Address::generate(&env);
    let r = client.try_register(&bytes(&env, 1), &other, &bytes(&env, 9));
    assert_eq!(r.err(), Some(Ok(Error::AlreadyRegistered)));

    // The original registration is untouched.
    let proof = client.get(&bytes(&env, 1)).unwrap();
    assert_eq!(proof.controller, controller);
    assert_eq!(proof.metadata_hash, bytes(&env, 2));
}

#[test]
fn the_controller_can_move_a_case_through_every_status() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let controller = Address::generate(&env);
    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));

    for status in [STATUS_CLOSED, STATUS_ARCHIVED, STATUS_OPEN] {
        client.set_status(&bytes(&env, 1), &status);
        assert_eq!(client.get(&bytes(&env, 1)).unwrap().status, status);
    }
}

#[test]
fn an_out_of_range_status_is_rejected_and_changes_nothing() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let controller = Address::generate(&env);
    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));

    let r = client.try_set_status(&bytes(&env, 1), &3);
    assert_eq!(r.err(), Some(Ok(Error::InvalidStatus)));
    assert_eq!(client.get(&bytes(&env, 1)).unwrap().status, STATUS_OPEN);
}

#[test]
fn status_of_an_unregistered_case_is_not_found() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    let r = client.try_set_status(&bytes(&env, 1), &1);
    assert_eq!(r.err(), Some(Ok(Error::NotFound)));
}

#[test]
fn registration_without_authorization_fails() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let id = env.register(CaseRegistry, (&admin,));
    let client = CaseRegistryClient::new(&env, &id);

    let r = client.try_register(&bytes(&env, 1), &admin, &bytes(&env, 2));
    assert!(r.is_err());
    assert!(client.get(&bytes(&env, 1)).is_none());
}

#[test]
fn the_admin_alone_cannot_register_a_case_for_someone_else() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let controller = Address::generate(&env);
    let case_ref = bytes(&env, 1);
    let hash = bytes(&env, 2);

    // Only the admin signs; the named controller never agreed.
    let result = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "register",
                args: (case_ref.clone(), controller.clone(), hash.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_register(&case_ref, &controller, &hash);

    assert!(result.is_err());
    assert!(client.get(&case_ref).is_none());
}

#[test]
fn only_the_case_controller_can_change_status() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    let controller = Address::generate(&env);
    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));

    // Not even the admin may change a case status.
    let result = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_status",
                args: (bytes(&env, 1), 1u32).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_set_status(&bytes(&env, 1), &1);

    assert!(result.is_err());
    assert_eq!(client.get(&bytes(&env, 1)).unwrap().status, STATUS_OPEN);
}

#[test]
fn the_admin_is_readable() {
    let env = Env::default();
    let (admin, client) = setup(&env);
    assert_eq!(client.admin(), admin);
}

#[test]
fn bump_needs_an_existing_case_and_no_authorization() {
    let env = Env::default();
    let (_admin, client) = setup(&env);
    assert_eq!(
        client.try_bump(&bytes(&env, 1)).err(),
        Some(Ok(Error::NotFound))
    );

    let controller = Address::generate(&env);
    client.register(&bytes(&env, 1), &controller, &bytes(&env, 2));
    assert!(client.mock_auths(&[]).try_bump(&bytes(&env, 1)).is_ok());
}
