#![no_std]
use soroban_sdk::{contract,contractimpl,contracttype,Address,BytesN,Env,Symbol};
#[contracttype] #[derive(Clone)] pub struct CaseProof {pub controller:Address,pub metadata_hash:BytesN<32>,pub status:u32}
#[contract] pub struct CaseRegistry;
#[contractimpl]
impl CaseRegistry {
 pub fn __constructor(env:Env,admin:Address){env.storage().instance().set(&Symbol::new(&env,"admin"),&admin);}
 pub fn register(env:Env,case_ref:BytesN<32>,controller:Address,metadata_hash:BytesN<32>){let admin:Address=env.storage().instance().get(&Symbol::new(&env,"admin")).unwrap();admin.require_auth();controller.require_auth();assert!(!env.storage().persistent().has(&case_ref),"case already registered");env.storage().persistent().set(&case_ref,&CaseProof{controller,metadata_hash,status:0});env.storage().persistent().extend_ttl(&case_ref,10000,100000);env.events().publish((Symbol::new(&env,"registered"),case_ref),());}
 pub fn set_status(env:Env,case_ref:BytesN<32>,status:u32){assert!(status<=2,"invalid status");let mut proof:CaseProof=env.storage().persistent().get(&case_ref).unwrap();proof.controller.require_auth();proof.status=status;env.storage().persistent().set(&case_ref,&proof);env.storage().persistent().extend_ttl(&case_ref,10000,100000);env.events().publish((Symbol::new(&env,"status"),case_ref),status);}
 pub fn get(env:Env,case_ref:BytesN<32>)->Option<CaseProof>{env.storage().persistent().get(&case_ref)}
}
#[cfg(test)] mod tests{use super::*;use soroban_sdk::testutils::Address as _;#[test]fn immutable_registration(){let e=Env::default();e.mock_all_auths();let a=Address::generate(&e);let id=e.register(CaseRegistry,(&a,));let c=CaseRegistryClient::new(&e,&id);let r=BytesN::from_array(&e,&[1;32]);let h=BytesN::from_array(&e,&[2;32]);c.register(&r,&a,&h);assert!(c.try_register(&r,&a,&h).is_err());c.set_status(&r,&1);assert_eq!(c.get(&r).unwrap().status,1);}#[test]fn unauthorized_registration_fails(){let e=Env::default();let a=Address::generate(&e);let id=e.register(CaseRegistry,(&a,));let c=CaseRegistryClient::new(&e,&id);assert!(c.try_register(&BytesN::from_array(&e,&[1;32]),&a,&BytesN::from_array(&e,&[2;32])).is_err());}}
