use soroban_sdk::{ Env };
use crate::storage_types::{ DataKey, FeeInfo };


pub fn fee_check(e: &Env) -> bool {
    let key = DataKey::FEE;

    if e.storage().instance().has(&key) {
        true
    }
    else {
        false
    }
}

pub fn fee_get(e: &Env) -> FeeInfo {
    let key = DataKey::FEE;

    if !e.storage().instance().has(&key) {
        panic!("FeeInfo wasn't initialized");
    }
    
    e.storage().instance().get(&key).unwrap()
}

pub fn fee_set(e: &Env, fee_info: &FeeInfo) {
    let key = DataKey::FEE;

    e.storage().instance().set(&key, fee_info);
    e.storage().instance().bump(200000000);
}
