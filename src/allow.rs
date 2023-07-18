use soroban_sdk::{ contract, contractimpl, 
    Address, Env };

use crate::storage_types::{ DataKey, };


#[contract]
pub struct Allow;

#[contractimpl]
impl Allow {
    pub fn allow(e: Env, token_addr: Address) {
        let key = DataKey::Allowance(token_addr);
        
        // if e.storage().instance().get::<_, bool>(&key).unwrap() {
        //     // panic!(`current token is already allowed`);
        //     return;
        // }
    
        e.storage().instance().set(&key, &true);
    }
    
    pub fn disallow(e: Env, token_addr: Address) {
        let key = DataKey::Allowance(token_addr);
    
        // if !e.storage().instance().get::<_, bool>(&key).unwrap() {
        //     // panic!("current token isn't allowed");
        //     return;
        // }
    
        e.storage().instance().set(&key, &false);
    }
}


pub fn is_allowed(e: &Env, token: &Address) -> bool {
    let key = DataKey::Allowance(token.clone());
    
    e.storage().instance().get(&key).unwrap()
}
