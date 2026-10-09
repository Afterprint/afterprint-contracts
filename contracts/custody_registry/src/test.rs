#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

fn bytes(env: &Env, fill: u8) -> BytesN<32> {
    BytesN::from_array(env, &[fill; 32])
}

struct Fixture<'a> {
    env: Env,
    controller: Address,
    alice: Address,
    bob: Address,
    carol: Address,
    client: CustodyRegistryClient<'a>,
}

/// A registered version `v1` held by Alice with initial hash `h0`, at ledger time 1000.
fn fixture<'a>() -> Fixture<'a> {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.timestamp = 1_000);
    let controller = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let id = env.register(CustodyRegistry, (&controller,));
    let client = CustodyRegistryClient::new(&env, &id);
    client.register(&bytes(&env, 1), &alice, &bytes(&env, 10));
    Fixture {
        env,
        controller,
        alice,
        bob,
        carol,
        client,
    }
}

#[test]
fn registering_sets_the_first_custodian_at_sequence_zero() {
    let f = fixture();
    let head = f.client.head(&bytes(&f.env, 1)).unwrap();
    assert_eq!(head.custodian, f.alice);
    assert_eq!(head.hash, bytes(&f.env, 10));
    assert_eq!(head.sequence, 0);
    assert_eq!(head.timestamp, 0);
}

#[test]
fn a_version_cannot_be_registered_twice() {
    let f = fixture();
    let r = f
        .client
        .try_register(&bytes(&f.env, 1), &f.bob, &bytes(&f.env, 99));
    assert_eq!(r.err(), Some(Ok(Error::AlreadyRegistered)));
    assert_eq!(f.client.head(&bytes(&f.env, 1)).unwrap().custodian, f.alice);
}

#[test]
fn a_transfer_moves_custody_and_advances_the_chain() {
    let f = fixture();
    let e = &f.env;
    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &500,
    );

    let head = f.client.head(&bytes(e, 1)).unwrap();
    assert_eq!(head.custodian, f.bob);
    assert_eq!(head.hash, bytes(e, 11));
    assert_eq!(head.sequence, 1);
    assert_eq!(head.timestamp, 500);

    let event = f.client.event(&bytes(e, 20)).unwrap();
    assert_eq!(event.from, f.alice);
    assert_eq!(event.to, f.bob);
    assert_eq!(event.previous_hash, bytes(e, 10));
    assert_eq!(event.sequence, 1);
}

#[test]
fn transfers_chain_through_several_custodians() {
    let f = fixture();
    let e = &f.env;
    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &100,
    );
    f.client.transfer(
        &bytes(e, 21),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 11),
        &bytes(e, 12),
        &200,
    );
    f.client.transfer(
        &bytes(e, 22),
        &bytes(e, 1),
        &f.carol,
        &f.alice,
        &bytes(e, 12),
        &bytes(e, 13),
        &200,
    );

    let head = f.client.head(&bytes(e, 1)).unwrap();
    assert_eq!(head.sequence, 3);
    assert_eq!(head.custodian, f.alice);
    assert_eq!(f.client.event(&bytes(e, 22)).unwrap().sequence, 3);
}

#[test]
fn an_unregistered_version_cannot_be_transferred() {
    let f = fixture();
    let e = &f.env;
    let r = f.client.try_transfer(
        &bytes(e, 20),
        &bytes(e, 77),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &1,
    );
    assert_eq!(r.err(), Some(Ok(Error::NotRegistered)));
}

#[test]
fn only_the_current_custodian_can_hand_over() {
    let f = fixture();
    let e = &f.env;
    // Bob does not hold it yet.
    let r = f.client.try_transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 10),
        &bytes(e, 11),
        &1,
    );
    assert_eq!(r.err(), Some(Ok(Error::NotExpectedCustodian)));
    assert_eq!(f.client.head(&bytes(e, 1)).unwrap().custodian, f.alice);
}

#[test]
fn a_stale_previous_hash_is_rejected() {
    let f = fixture();
    let e = &f.env;
    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &100,
    );

    // Bob tries to build on the old head instead of h1.
    let r = f.client.try_transfer(
        &bytes(e, 21),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 10),
        &bytes(e, 12),
        &200,
    );
    assert_eq!(r.err(), Some(Ok(Error::StaleHead)));
    assert_eq!(f.client.head(&bytes(e, 1)).unwrap().sequence, 1);
}

#[test]
fn a_transfer_to_oneself_is_rejected() {
    let f = fixture();
    let e = &f.env;
    let r = f.client.try_transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.alice,
        &bytes(e, 10),
        &bytes(e, 11),
        &1,
    );
    assert_eq!(r.err(), Some(Ok(Error::SameCustodian)));
}

#[test]
fn timestamps_may_not_go_backwards_or_into_the_future() {
    let f = fixture();
    let e = &f.env;
    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &600,
    );

    // Earlier than the last transfer.
    let r = f.client.try_transfer(
        &bytes(e, 21),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 11),
        &bytes(e, 12),
        &599,
    );
    assert_eq!(r.err(), Some(Ok(Error::InvalidTimestamp)));

    // After the ledger time (1000).
    let r = f.client.try_transfer(
        &bytes(e, 21),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 11),
        &bytes(e, 12),
        &1_001,
    );
    assert_eq!(r.err(), Some(Ok(Error::InvalidTimestamp)));

    // Exactly the same time as the last transfer, and exactly the ledger time, are fine.
    f.client.transfer(
        &bytes(e, 21),
        &bytes(e, 1),
        &f.bob,
        &f.carol,
        &bytes(e, 11),
        &bytes(e, 12),
        &600,
    );
    f.client.transfer(
        &bytes(e, 22),
        &bytes(e, 1),
        &f.carol,
        &f.alice,
        &bytes(e, 12),
        &bytes(e, 13),
        &1_000,
    );
    assert_eq!(f.client.head(&bytes(e, 1)).unwrap().sequence, 3);
}

#[test]
fn an_identical_retry_is_a_silent_no_op() {
    let f = fixture();
    let e = &f.env;
    let send = || {
        f.client.transfer(
            &bytes(e, 20),
            &bytes(e, 1),
            &f.alice,
            &f.bob,
            &bytes(e, 10),
            &bytes(e, 11),
            &500,
        )
    };
    send();
    assert_eq!(e.events().all().events().len(), 1);

    send();
    assert_eq!(e.events().all().events().len(), 0);
    let head = f.client.head(&bytes(e, 1)).unwrap();
    assert_eq!(head.sequence, 1, "a retry must not advance the chain");
    assert_eq!(head.custodian, f.bob);
}

#[test]
fn reusing_an_event_reference_with_different_details_is_rejected() {
    let f = fixture();
    let e = &f.env;
    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &500,
    );

    // Same event_ref, different destination.
    let r = f.client.try_transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.carol,
        &bytes(e, 10),
        &bytes(e, 11),
        &500,
    );
    assert_eq!(r.err(), Some(Ok(Error::ConflictingRetry)));
    // Same event_ref, different hash.
    let r = f.client.try_transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 99),
        &500,
    );
    assert_eq!(r.err(), Some(Ok(Error::ConflictingRetry)));
}

#[test]
fn only_the_controller_can_register() {
    let f = fixture();
    let e = &f.env;
    let stranger = Address::generate(e);
    let result = f
        .client
        .mock_auths(&[MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &f.client.address,
                fn_name: "register",
                args: (bytes(e, 2), f.alice.clone(), bytes(e, 10)).into_val(e),
                sub_invokes: &[],
            },
        }])
        .try_register(&bytes(e, 2), &f.alice, &bytes(e, 10));

    assert!(result.is_err());
    assert!(f.client.head(&bytes(e, 2)).is_none());
}

#[test]
fn the_controller_cannot_move_custody_on_the_custodians_behalf() {
    let f = fixture();
    let e = &f.env;
    // The controller signs, but Alice (the custodian) does not.
    let result = f
        .client
        .mock_auths(&[MockAuth {
            address: &f.controller,
            invoke: &MockAuthInvoke {
                contract: &f.client.address,
                fn_name: "transfer",
                args: (
                    bytes(e, 20),
                    bytes(e, 1),
                    f.alice.clone(),
                    f.bob.clone(),
                    bytes(e, 10),
                    bytes(e, 11),
                    500u64,
                )
                    .into_val(e),
                sub_invokes: &[],
            },
        }])
        .try_transfer(
            &bytes(e, 20),
            &bytes(e, 1),
            &f.alice,
            &f.bob,
            &bytes(e, 10),
            &bytes(e, 11),
            &500,
        );

    assert!(result.is_err());
    assert_eq!(f.client.head(&bytes(e, 1)).unwrap().custodian, f.alice);
}

#[test]
fn unknown_records_read_as_none() {
    let f = fixture();
    assert!(f.client.head(&bytes(&f.env, 42)).is_none());
    assert!(f.client.event(&bytes(&f.env, 42)).is_none());
}

#[test]
fn bump_functions_need_existing_records_and_no_authorization() {
    let f = fixture();
    let e = &f.env;
    assert_eq!(
        f.client.try_bump_head(&bytes(e, 42)).err(),
        Some(Ok(Error::NotFound))
    );
    assert_eq!(
        f.client.try_bump_event(&bytes(e, 42)).err(),
        Some(Ok(Error::NotFound))
    );

    f.client.transfer(
        &bytes(e, 20),
        &bytes(e, 1),
        &f.alice,
        &f.bob,
        &bytes(e, 10),
        &bytes(e, 11),
        &500,
    );
    assert!(f.client.mock_auths(&[]).try_bump_head(&bytes(e, 1)).is_ok());
    assert!(f
        .client
        .mock_auths(&[])
        .try_bump_event(&bytes(e, 20))
        .is_ok());
}

#[test]
fn the_controller_is_readable() {
    let f = fixture();
    assert_eq!(f.client.controller(), f.controller);
}
