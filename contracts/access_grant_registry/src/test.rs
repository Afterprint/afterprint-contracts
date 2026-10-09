#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

fn bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

fn setup<'a>(env: &Env) -> (Address, AccessGrantRegistryClient<'a>) {
    env.mock_all_auths();
    let controller = Address::generate(env);
    let id = env.register(AccessGrantRegistry, (&controller,));
    (controller, AccessGrantRegistryClient::new(env, &id))
}

#[test]
fn a_new_grant_is_active() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));

    let grant = client.get(&bytes(&env, 1)).unwrap();
    assert_eq!(grant.policy_hash, bytes(&env, 2));
    assert!(!grant.revoked);
    assert!(client.has_grant(&bytes(&env, 1)));
}

#[test]
fn an_unknown_reference_is_not_granted() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    assert!(client.get(&bytes(&env, 1)).is_none());
    assert!(!client.has_grant(&bytes(&env, 1)));
}

#[test]
fn revoking_turns_the_grant_off() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));

    client.revoke(&bytes(&env, 1));

    assert!(client.get(&bytes(&env, 1)).unwrap().revoked);
    assert!(!client.has_grant(&bytes(&env, 1)));
}

#[test]
fn a_revoked_reference_can_never_be_granted_again() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));
    client.revoke(&bytes(&env, 1));

    let r = client.try_grant(&bytes(&env, 1), &bytes(&env, 2));
    assert_eq!(r.err(), Some(Ok(Error::GrantRefAlreadyUsed)));
    assert!(client.get(&bytes(&env, 1)).unwrap().revoked);
}

#[test]
fn an_active_reference_cannot_be_granted_twice() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));

    let r = client.try_grant(&bytes(&env, 1), &bytes(&env, 9));
    assert_eq!(r.err(), Some(Ok(Error::GrantRefAlreadyUsed)));
    assert_eq!(
        client.get(&bytes(&env, 1)).unwrap().policy_hash,
        bytes(&env, 2)
    );
}

#[test]
fn revoking_twice_succeeds_but_emits_only_one_event() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));

    client.revoke(&bytes(&env, 1));
    assert_eq!(env.events().all().events().len(), 1);
    client.revoke(&bytes(&env, 1));
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
fn revoking_an_unknown_grant_is_not_found() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    let r = client.try_revoke(&bytes(&env, 1));
    assert_eq!(r.err(), Some(Ok(Error::NotFound)));
}

#[test]
fn grants_are_independent_of_each_other() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));
    client.grant(&bytes(&env, 3), &bytes(&env, 4));
    client.revoke(&bytes(&env, 1));

    assert!(!client.has_grant(&bytes(&env, 1)));
    assert!(client.has_grant(&bytes(&env, 3)));
}

#[test]
fn granting_and_revoking_without_authorization_fail() {
    let env = Env::default();
    let controller = Address::generate(&env);
    let id = env.register(AccessGrantRegistry, (&controller,));
    let client = AccessGrantRegistryClient::new(&env, &id);

    assert!(client.try_grant(&bytes(&env, 1), &bytes(&env, 2)).is_err());
    assert!(client.get(&bytes(&env, 1)).is_none());
}

#[test]
fn only_the_controller_can_revoke() {
    let env = Env::default();
    let (_controller, client) = setup(&env);
    client.grant(&bytes(&env, 1), &bytes(&env, 2));
    let stranger = Address::generate(&env);

    let result = client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "revoke",
                args: (bytes(&env, 1),).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_revoke(&bytes(&env, 1));

    assert!(result.is_err());
    assert!(client.has_grant(&bytes(&env, 1)));
}

#[test]
fn the_controller_is_readable() {
    let env = Env::default();
    let (controller, client) = setup(&env);
    assert_eq!(client.controller(), controller);
}

#[test]
fn bump_needs_an_existing_grant_and_no_authorization() {
    let env = Env::default();
    let (_c, client) = setup(&env);
    assert_eq!(
        client.try_bump(&bytes(&env, 1)).err(),
        Some(Ok(Error::NotFound))
    );

    client.grant(&bytes(&env, 1), &bytes(&env, 2));
    assert!(client.mock_auths(&[]).try_bump(&bytes(&env, 1)).is_ok());
}
