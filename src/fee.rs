use soroban_sdk::{ Env };
use crate::storage_types::{ DataKey, FeeInfo };


pub fn fee_init(e: &Env, fee_info: &FeeInfo) {
    if fee_check(&e) {
        panic!("FeeInfo was already initialized");
    }
    
    fee_write(
        &e,
        &fee_info,
    );
}

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

    if !fee_check(&e) {
        panic!("FeeInfo wasn't initialized");
    }
    
    e.storage().instance().get(&key).unwrap()
}

pub fn fee_set(e: &Env, fee_info: &FeeInfo) {
    if !fee_check(e) {
        panic!("FeeInfo wasn't initialized");
    }

    fee_write(e, &fee_info);
}


fn fee_write(e: &Env, fee_info: &FeeInfo) {
    let key = DataKey::FEE;

    e.storage().instance().set(&key, fee_info);
}
