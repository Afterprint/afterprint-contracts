#![no_std]
use soroban_sdk::{contract,contractimpl,contracttype,Address,BytesN,Env,Symbol};
#[contracttype] #[derive(Clone)] pub struct Grant{pub policy_hash:BytesN<32>,pub revoked:bool}
#[contract] pub struct AccessGrantRegistry;
#[contractimpl]impl AccessGrantRegistry{
 pub fn __constructor(env:Env,controller:Address){env.storage().instance().set(&Symbol::new(&env,"controller"),&controller);}
 pub fn grant(env:Env,grant_ref:BytesN<32>,policy_hash:BytesN<32>){let c:Address=env.storage().instance().get(&Symbol::new(&env,"controller")).unwrap();c.require_auth();assert!(!env.storage().persistent().has(&grant_ref),"grant ref already used");env.storage().persistent().set(&grant_ref,&Grant{policy_hash,revoked:false});env.storage().persistent().extend_ttl(&grant_ref,10000,100000);env.events().publish((Symbol::new(&env,"grant"),grant_ref),());}
 pub fn revoke(env:Env,grant_ref:BytesN<32>){let c:Address=env.storage().instance().get(&Symbol::new(&env,"controller")).unwrap();c.require_auth();let mut grant:Grant=env.storage().persistent().get(&grant_ref).unwrap();grant.revoked=true;env.storage().persistent().set(&grant_ref,&grant);env.storage().persistent().extend_ttl(&grant_ref,10000,100000);env.events().publish((Symbol::new(&env,"revoke"),grant_ref),());}
 pub fn get(env:Env,grant_ref:BytesN<32>)->Option<Grant>{env.storage().persistent().get(&grant_ref)}
}
#[cfg(test)]mod tests{use super::*;use soroban_sdk::testutils::Address as _;#[test]fn revocation_cannot_be_reversed(){let e=Env::default();e.mock_all_auths();let a=Address::generate(&e);let id=e.register(AccessGrantRegistry,(&a,));let c=AccessGrantRegistryClient::new(&e,&id);let h=BytesN::from_array(&e,&[1;32]);c.grant(&h,&h);c.revoke(&h);assert!(c.get(&h).unwrap().revoked);assert!(c.try_grant(&h,&h).is_err());}#[test]fn unauthorized(){let e=Env::default();let a=Address::generate(&e);let id=e.register(AccessGrantRegistry,(&a,));let c=AccessGrantRegistryClient::new(&e,&id);let h=BytesN::from_array(&e,&[1;32]);assert!(c.try_grant(&h,&h).is_err());}}
