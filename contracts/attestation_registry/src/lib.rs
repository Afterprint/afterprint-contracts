#![no_std]
use soroban_sdk::{contract,contractimpl,contracttype,Address,BytesN,Env,Symbol};
#[contracttype] #[derive(Clone,Debug,Eq,PartialEq)] pub struct Attestation{pub subject_ref:BytesN<32>,pub statement_hash:BytesN<32>,pub issuer_ref:BytesN<32>}
#[contract] pub struct AttestationRegistry;
#[contractimpl] impl AttestationRegistry{
 pub fn __constructor(env:Env,controller:Address){env.storage().instance().set(&Symbol::new(&env,"controller"),&controller);}
 pub fn attest(env:Env,attestation_ref:BytesN<32>,subject_ref:BytesN<32>,statement_hash:BytesN<32>,issuer_ref:BytesN<32>){let controller:Address=env.storage().instance().get(&Symbol::new(&env,"controller")).unwrap();controller.require_auth();let proof=Attestation{subject_ref,statement_hash,issuer_ref};if let Some(prior)=env.storage().persistent().get::<_,Attestation>(&attestation_ref){assert!(prior==proof,"immutable attestation conflict");return;}env.storage().persistent().set(&attestation_ref,&proof);env.storage().persistent().extend_ttl(&attestation_ref,10000,100000);env.events().publish((Symbol::new(&env,"attested"),attestation_ref),proof);}
 pub fn get(env:Env,attestation_ref:BytesN<32>)->Option<Attestation>{env.storage().persistent().get(&attestation_ref)}
}
#[cfg(test)]mod tests{use super::*;use soroban_sdk::testutils::Address as _;#[test]fn immutable(){let e=Env::default();e.mock_all_auths();let a=Address::generate(&e);let id=e.register(AttestationRegistry,(&a,));let c=AttestationRegistryClient::new(&e,&id);let h=BytesN::from_array(&e,&[1;32]);let b=BytesN::from_array(&e,&[2;32]);c.attest(&h,&h,&h,&h);assert!(c.try_attest(&h,&h,&b,&h).is_err());}#[test]fn unauthorized(){let e=Env::default();let a=Address::generate(&e);let id=e.register(AttestationRegistry,(&a,));let c=AttestationRegistryClient::new(&e,&id);let h=BytesN::from_array(&e,&[1;32]);assert!(c.try_attest(&h,&h,&h,&h).is_err());}}
