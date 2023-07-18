use soroban_sdk::{ Env };
use crate::storage_types::{ DataKey, FeeInfo };


pub fn fee_init(e: &Env, fee: &FeeInfo) {
    if fee_check(&e) {
        panic!("FeeInfo is already initialized");
    }
    
    fee_write(
        &e,
        &fee,
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

    e.storage().instance().get(&key).unwrap()
}

pub fn fee_set(e: &Env, fee: &FeeInfo) {
    if !fee_check(e) {
        panic!("FeeInfo isn't initialized");
    }

    fee_write(e, &fee);
}


fn fee_write(e: &Env, fee: &FeeInfo) {
    let key = DataKey::FEE;

    e.storage().instance().set(&key, fee);
}    
