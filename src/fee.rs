use soroban_sdk::{ contract, contractimpl, log, Address, Env };
use crate::storage_types::{ DataKey, FeeInfo };

#[contract]
pub struct Fee;

#[contractimpl]
impl Fee {
    pub fn init(e: Env, fee_rate: u32, fee_wallet: Address) {
        if has_fee(&e) {
            panic!("FeeInfo is already initialized");
        }
        
        write_fee(
            &e,
            &FeeInfo {
                fee_rate,
                fee_wallet,
            },
        );
    }    
}


pub fn has_fee(e: &Env) -> bool {
    let key = DataKey::FEE;

    if e.storage().persistent().has(&key) {
        true
    }
    else {
        false
    }
}

pub fn load_fee(e: &Env) -> FeeInfo {
    e.storage().persistent().get(&DataKey::FEE).unwrap()
}

pub fn update_fee(e: &Env, fee_rate: u32, fee_wallet: Address) {
    if !has_fee(e) {
        panic!("FeeInfo isn't initialized");
    }

    let mut fee: FeeInfo = load_fee(e);

    fee.fee_rate = fee_rate;
    fee.fee_wallet = fee_wallet;

    write_fee(e, &fee);
}


fn write_fee(e: &Env, fee: &FeeInfo) {
    let key = DataKey::FEE;

    log!(e, "writing fee_info...");
    e.storage().persistent().set(&key, fee);
}    
