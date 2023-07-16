//! This contract implements swap of one token pair between one offeror and
//! multiple acceptors.
//! It demonstrates one of the ways of how swap might be implemented.
#![no_std]

use soroban_sdk::{
    // xdr::{AccountId, Hash, PublicKey, ScAddress, Uint256},
    contract, contractimpl, contracttype, token, unwrap::UnwrapOptimized, Address, Env, IntoVal, TryFromVal
};

#[derive(Clone, Copy, PartialEq)]
#[contracttype]
pub enum OfferStatus {
    ACTIVE = 1,
    COMPLETE = 2,
    CANCEL = 3
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    // TokenA = 0,
    // TokenB = 1,
    AdminState,
    Offer,
}

// Represents an offer managed by the SingleOffer contract.
// If an offeror wants to swap 1000 XLM for 100 USDC, the `send_amount` would be 1000
// and `recv_amount` would be 100
#[derive(Clone)]
#[contracttype]
struct AdminState {
    pub fee_wallet: Address,
    pub owner: Address,
}

#[derive(Clone)]
#[contracttype]
struct Offer {
    // Owner of this offer. Swaps send_token with recv_token.
    pub offeror: Address,
    
    pub send_token: Address,
    pub recv_token: Address,
    
    // offeror-defined amount of the send token
    pub send_amount: i128,
    // offeror-defined amount of the recv token
    pub recv_amount: i128,
    pub min_recv_amount: i128,

    pub status: OfferStatus
}

#[contract]
pub struct SingleOffer;

/*
How this contract should be used:

1. Call `create` once to create an offer and register its offeror.
2. Offeror transfers send_amount of the `send_token` to the
   contract address for swap. He may also update the recv_amount and/or min_recv_amount.
3. Acceptors may call `accept` to accept the offer. The contract will
   immediately perform the swap and send the respective amounts of `recv_token`
   and `send_token` to the offeror and acceptor respectively.
4. Offeror may call `close` to claim any remaining `send_token` balance.
*/
#[contractimpl]
impl SingleOffer {
    // calculate fee
    pub fn calculate_fee(amount: i128) -> i128 {
        // fee is 0.025%
        amount * 25 / 10000
    }

    // Creates the offer for offeror for the given token pair and initial amounts.
    // See comment above the `Offer` struct for information on swap.
    pub fn create(
        e: Env,
        offeror: Address,
        send_token: Address,
        recv_token: Address,
        send_amount: i128,
        recv_amount: i128,
        min_recv_amount: i128,
    ) {
        if e.storage().instance().has(&DataKey::Offer) {
            panic!("offer is already created");
        }
        
        // check if both tokens are allowed
        // if !e.storage().instance().has(&DataKey::TokenA) || !e.storage().instance().has(&DataKey::TokenB) {
        //     panic!("tokens aren't allowed");
        // }
        if send_amount == 0 || recv_amount == 0 {
            panic!("zero amount is not allowed");
        }
        if min_recv_amount > recv_amount {
            panic!("min_recv_amount can't be greater than recv_amount");
        }
        
        // Authorize the `create` call by offeror to verify their identity.
        offeror.require_auth();

        let fee: i128 = /*calculate_fee(send_amount)*/ send_amount * 25 / 10000;
        // let fee_wallet: Address = Address::unchecked_new(&e, "GBHNNZGD7UUSOIV3J3VH2PC7LPDRLRWJ4SMMBKDBHRIPDWLXRRZ6NA2Q");
        
        let contract = e.current_contract_address();
        let send_token_client = token::Client::new(&e, &send_token);
        
        // if send_token_client.balance() < (send_amount + fee) {
        //     panic!("insufficient balance");
        // }

        send_token_client.transfer(&offeror, &contract, &send_amount);
        // send_token_client.transfer(&offeror, &fee_wallet, &fee);

        write_offer(
            &e,
            &Offer {
                offeror,
                send_token,
                recv_token,
                send_amount,
                recv_amount,
                min_recv_amount,
                status: OfferStatus::ACTIVE,
            },
        );

        // emit OfferCreated event
    }

    // Swaps `amount` of recv_token from acceptor for `send_token` amount calculated by the amount.
    // acceptor needs to authorize the `swap` call and internal `transfer` call to the contract address.
    pub fn accept(e: Env, acceptor: Address, amount: i128) {
        let mut offer = load_offer(&e);

        if offer.status != OfferStatus::ACTIVE {
            panic!("offer not available");
        }
        if offer.recv_amount < amount {
            panic!("amount is greater than max_recv_amount");
        }
        if amount < offer.min_recv_amount {
            panic!("amount must be more than min_recv_amount");
        }
        
        // acceptor needs to authorize the trade.
        acceptor.require_auth();

        // Load the offer and prepare the token clients to do the trade.
        let send_token_client = token::Client::new(&e, &offer.send_token);
        let recv_token_client = token::Client::new(&e, &offer.recv_token);

        let fee: i128 = /*calculate_fee(amount)*/ amount * 25 / 10000;
        // let fee_wallet: Address = Address::unchecked_new(&e, "GBHNNZGD7UUSOIV3J3VH2PC7LPDRLRWJ4SMMBKDBHRIPDWLXRRZ6NA2Q");

        // if recv_token_client.balance() < (amount + fee) {
        //     panic!("insufficient balance");
        // }

        // Compute the amount of send_token that acceptor can receive.
        let prop_send_amount = amount.checked_mul(offer.send_amount as i128).unwrap_optimized() / offer.recv_amount as i128;

        let contract = e.current_contract_address();

        // Perform the trade in 3 `transfer` steps.
        // Note, that we don't need to verify any balances - the contract would
        // just trap and roll back in case if any of the transfers fails for
        // any reason, including insufficient balance.

        // Transfer the `recv_token` from acceptor to this contract.
        // This `transfer` call should be authorized by acceptor.
        // This could as well be a direct transfer to the offeror, but sending to
        // the contract address allows building more transparent signature
        // payload where the acceptor doesn't need to worry about sending token to
        // some 'unknown' third party.
        // recv_token_client.transfer(&acceptor, &fee_wallet, &fee);
        // Transfer the `recv_token` to the offeror immediately.
        recv_token_client.transfer(&acceptor, &offer.offeror, &amount);
        // Transfer the `send_token` from contract to acceptor.
        send_token_client.transfer(&contract, &acceptor, &prop_send_amount);

        // Update Offer
        offer.send_amount -= prop_send_amount;
        offer.recv_amount -= amount;

        if offer.recv_amount == 0 {
            offer.status = OfferStatus::COMPLETE;
            // emit OfferCompleted event
        }
        else if offer.recv_amount < offer.min_recv_amount {
            offer.min_recv_amount = offer.recv_amount;
        }

        write_offer(&e, &offer);

        // emit OfferAccepted event
    }

    // Cancel offer
    // Must be authorized by offeror.
    pub fn close(e: Env) {
        let mut offer = load_offer(&e);

        if offer.status != OfferStatus::ACTIVE {
            panic!("offer not available");
        }

        offer.offeror.require_auth();
        token::Client::new(&e, &offer.send_token).transfer(
            &e.current_contract_address(),
            &offer.offeror,
            &offer.send_amount,
        );

        offer.status = OfferStatus::CANCEL;
        write_offer(&e, &offer);

        // emit OfferRevoked event
    }

    // Updates offer
    // Must be authorized by offeror.
    pub fn update(e: Env, recv_amount: i128, min_recv_amount: i128) {
        if recv_amount == 0 {
            panic!("zero amount is not allowed");
        }
        if min_recv_amount > recv_amount {
            panic!("min_recv_amount can't be greater than recv_amount");
        }

        let mut offer = load_offer(&e);

        if offer.status != OfferStatus::ACTIVE {
            panic!("offer not available");
        }

        offer.offeror.require_auth();
        offer.recv_amount = recv_amount;
        offer.min_recv_amount = min_recv_amount;
        write_offer(&e, &offer);

        // emit OfferUpdated event
    }

    // Returns the current state of the offer.
    fn get_offer(e: Env) -> Offer {
        load_offer(&e)
    }
}


fn load_offer(e: &Env) -> Offer {
    e.storage().instance().get(&DataKey::Offer).unwrap()
}

fn write_offer(e: &Env, offer: &Offer) {
    e.storage().instance().set(&DataKey::Offer, offer);
}


mod test;
