//! This contract implements swap of one token pair between one offeror and
//! multiple acceptors.
//! It demonstrates one of the ways of how swap might be implemented.
#![no_std]

mod storage_types;
mod fee;
mod allow;
mod offer;


use soroban_sdk::{
    contract, contractimpl, Address, Env, BytesN
};
use crate::storage_types::{ FeeInfo, DataKey };
use crate::fee::{ fee_set };
use crate::allow::{ allow_set, allow_reset };
use crate::offer::{ error, offer_count, offer_create, offer_accept, offer_update, offer_close, offer_balances };


#[contract]
pub struct TokenSwap;

#[contractimpl]
impl TokenSwap {
    pub fn set_fee(e: Env, fee_rate: u32, fee_wallet: Address) {
        let fee_info: FeeInfo = FeeInfo {fee_rate, fee_wallet};
        fee_set(&e, &fee_info);
    }

    pub fn allow_token(e: Env, token: Address) {
        allow_set(&e, &token);
    }

    pub fn disallow_token(e: Env, token: Address) {
        allow_reset(&e, &token);
    }

    pub fn get_error(e: Env) -> u32 {
        error(&e)
    }

    pub fn count_offers(e: Env) -> u32 {
        offer_count(&e)
    }

    pub fn create_offer(e: Env, 
        offeror: Address, 
        send_token: Address, 
        recv_token: Address, 
        timestamp: u32, 
        send_amount: u64, 
        recv_amount: u64, 
        min_recv_amount: u64
    ) -> u32 {
        let ret: u32 = offer_create(&e, &offeror, &send_token, &recv_token, timestamp, send_amount, recv_amount, min_recv_amount);

        e.storage().instance().set(&DataKey::ERROR_CODE, &ret);
        e.storage().instance().bump(200000000);

        ret
    }

    pub fn accept_offer(e: Env, 
        acceptor: Address, 
        offer_id: u32, 
        amount: u64
    ) -> u32 {
        let ret: u32 = offer_accept(&e, &acceptor, offer_id, amount);

        e.storage().instance().set(&DataKey::ERROR_CODE, &ret);
        e.storage().instance().bump(200000000);

        ret
    }

    pub fn update_offer(e: Env, 
        offeror: Address, 
        offer_id: u32, 
        recv_amount: u64, 
        min_recv_amount: u64
    ) -> u32 {
        let ret: u32 = offer_update(&e, &offeror, offer_id, recv_amount, min_recv_amount);

        e.storage().instance().set(&DataKey::ERROR_CODE, &ret);
        e.storage().instance().bump(200000000);

        ret
    }

    pub fn close_offer(e: Env, 
        offeror: Address,
        offer_id: u32
    ) -> u32 {
        let ret: u32 = offer_close(&e, &offeror, offer_id);

        e.storage().instance().set(&DataKey::ERROR_CODE, &ret);
        e.storage().instance().bump(200000000);

        ret
    }

    pub fn check_balances(e: Env, 
        offeror: Address, 
        acceptor: Address, 
        send_token: Address, 
        recv_token: Address
    ) -> (u64, u64, u64, u64) {
        offer_balances(&e, &offeror, &acceptor, &send_token, &recv_token)
    }
}


mod test;
