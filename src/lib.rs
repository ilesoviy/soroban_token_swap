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
use crate::storage_types::{ FeeInfo };
use crate::fee::{ fee_init, fee_set };
use crate::allow::{ allow_set, allow_reset };
use crate::offer::{ offer_create, offer_accept, offer_update, offer_close };


#[contract]
pub struct TokenSwap;

#[contractimpl]
impl TokenSwap {
    pub fn init_fee(e: Env, fee_rate: u32, fee_wallet: Address) {
        let fee_info: FeeInfo = FeeInfo {fee_rate, fee_wallet};
        fee_init(&e, &fee_info);
    }

    pub fn allow_token(e: Env, token: Address) {
        allow_set(&e, &token);
    }

    pub fn disallow_token(e: Env, token: Address) {
        allow_reset(&e, &token);
    }

    pub fn create_offer(
        e: Env,
        offeror: Address,
        send_token: Address,
        recv_token: Address,
        timestamp: u64,
        send_amount: i128,
        recv_amount: i128,
        min_recv_amount: i128,
    ) -> BytesN<32> {
        offer_create(&e, &offeror, &send_token, &recv_token, timestamp, send_amount, recv_amount, min_recv_amount)
    }

    pub fn accept_offer(e: Env, 
        offer_id: BytesN<32>, 
        acceptor: Address, 
        amount: i128
    ) {
        offer_accept(&e, &offer_id, &acceptor, amount)
    }

    pub fn update_offer(e: Env, 
        offer_id: BytesN<32>, 
        recv_amount: i128, 
        min_recv_amount: i128
    ) {
        offer_update(&e, &offer_id, recv_amount, min_recv_amount)
    }

    pub fn close_offer(e: Env, 
        offer_id: BytesN<32>
    ) {
        offer_close(&e, &offer_id)
    }
}


mod test;
