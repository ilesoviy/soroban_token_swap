use soroban_sdk::{ log, Address, Env };

use crate::storage_types::{ DataKey, };


pub fn allow_set(e: &Env, token_addr: &Address) {
    let key = DataKey::Allowance(token_addr.clone());
    
    if e.storage().instance().has(&key) && e.storage().instance().get::<_, bool>(&key).unwrap() {
        log!(&e, "current token was already allowed");
        return;
    }

    e.storage().instance().set(&key, &true);
}

pub fn allow_reset(e: &Env, token_addr: &Address) {
    let key = DataKey::Allowance(token_addr.clone());

    if !e.storage().instance().has(&key) || !e.storage().instance().get::<_, bool>(&key).unwrap() {
        log!(&e, "current token wasn't allowed");
        return;
    }

    e.storage().instance().set(&key, &false);
    e.storage().instance().bump(200000000);
}

pub fn allow_get(e: &Env, token: &Address) -> bool {
    let key = DataKey::Allowance(token.clone());
    
    if e.storage().instance().has(&key) && e.storage().instance().get::<_, bool>(&key).unwrap() {
        true
    }
    else {
        false
    }
}
