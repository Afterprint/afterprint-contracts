#![no_std]
use soroban_sdk::{contract,contractimpl,contracttype,Address,BytesN,Env,Symbol};
#[contracttype] #[derive(Clone,Debug,Eq,PartialEq)] pub struct Anchor {pub manifest_hash:BytesN<32>,pub submitter_ref:BytesN<32>,pub ledger:u32}
#[contract] pub struct EvidenceAnchorRegistry;
#[contractimpl]
impl EvidenceAnchorRegistry {
 pub fn __constructor(env:Env,controller:Address){env.storage().instance().set(&Symbol::new(&env,"controller"),&controller);}
 pub fn anchor(env:Env,evidence_version_ref:BytesN<32>,manifest_hash:BytesN<32>,submitter_ref:BytesN<32>){let controller:Address=env.storage().instance().get(&Symbol::new(&env,"controller")).unwrap();controller.require_auth();if let Some(prior)=env.storage().persistent().get::<_,Anchor>(&evidence_version_ref){assert!(prior.manifest_hash==manifest_hash&&prior.submitter_ref==submitter_ref,"immutable anchor conflict");return;}let proof=Anchor{manifest_hash,submitter_ref,ledger:env.ledger().sequence()};env.storage().persistent().set(&evidence_version_ref,&proof);env.storage().persistent().extend_ttl(&evidence_version_ref,10000,100000);env.events().publish((Symbol::new(&env,"anchored"),evidence_version_ref),proof);}
 pub fn get(env:Env,evidence_version_ref:BytesN<32>)->Option<Anchor>{env.storage().persistent().get(&evidence_version_ref)}
}
#[cfg(test)] mod tests{use super::*;use soroban_sdk::testutils::Address as _;#[test]fn anchor_immutable_and_idempotent(){let e=Env::default();e.mock_all_auths();let a=Address::generate(&e);let id=e.register(EvidenceAnchorRegistry,(&a,));let c=EvidenceAnchorRegistryClient::new(&e,&id);let r=BytesN::from_array(&e,&[1;32]);let h=BytesN::from_array(&e,&[2;32]);c.anchor(&r,&h,&h);c.anchor(&r,&h,&h);assert_eq!(c.get(&r).unwrap().manifest_hash,h);assert!(c.try_anchor(&r,&r,&h).is_err());}#[test]fn unauthorized_anchor_fails(){let e=Env::default();let a=Address::generate(&e);let id=e.register(EvidenceAnchorRegistry,(&a,));let c=EvidenceAnchorRegistryClient::new(&e,&id);let r=BytesN::from_array(&e,&[1;32]);assert!(c.try_anchor(&r,&r,&r).is_err());}}
