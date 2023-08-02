const OFFER: Symbol = symbol_short!("OFFER");

use soroban_sdk::{
    log, token, unwrap::UnwrapOptimized, Address, Env, symbol_short, BytesN, Symbol, 
    xdr::{ToXdr}
};
use crate::storage_types::{ FEE_DECIMALS, FeeInfo, OfferStatus, OfferInfo, DataKey };
use crate::fee::{ fee_check, fee_get };
use crate::allow::{ allow_get };


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
fn calculate_fee(fee_info: &FeeInfo, amount: u64) -> u64 {
    amount * (fee_info.fee_rate as u64) / (u64::pow(10, FEE_DECIMALS))
}

pub fn error(
    e: &Env
) -> u32 {
    if !e.storage().instance().has(&DataKey::ERROR_CODE) {
        return 1000;
    }

    let err_code: u32 = e.storage().instance().get(&DataKey::ERROR_CODE).unwrap_or(0);
    err_code

    // 1001
}

pub fn offer_count(
    e: &Env
) -> u32 {
    let offer_count: u32 = e.storage().instance().get(&DataKey::OFFER_COUNT).unwrap_or(0);
    offer_count
}

// Creates the offer for offeror for the given token pair and initial amounts.
// See comment above the `Offer` struct for information on swap.
pub fn offer_create(
    e: &Env,
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    timestamp: u32,
    send_amount: u64,
    recv_amount: u64,
    min_recv_amount: u64,
) -> u32 {
    if !fee_check(e) {
        // panic!("fee wasn't set");
        return 101;
    }
    if !allow_get(e, &send_token.clone()) || !allow_get(e, &recv_token.clone()) {
        // panic!("both tokens aren't allowed");
        return 102;
    }

    let offer_count: u32 = e.storage().instance().get(&DataKey::OFFER_COUNT).unwrap_or(0);
    let offer_id: u32 = offer_count;
    log!(e, "offer_id = {}", offer_id);

    if send_amount == 0 || recv_amount == 0 {
        // panic!("zero amount is not allowed");
        return 104;
    }
    if min_recv_amount > recv_amount {
        // panic!("min_recv_amount can't be greater than recv_amount");
        return 105;
    }
    
    // Authorize the `create` call by offeror to verify their identity.
    // offeror.require_auth();

    let fee_info = fee_get(e);
    let fee_amount: u64 = calculate_fee(&fee_info.clone(), send_amount);
    let transfer_amount = send_amount + fee_amount;
    
    let contract = e.current_contract_address();
    let send_token_client = token::Client::new(e, &send_token.clone());

    if send_token_client.balance(&offeror) < (transfer_amount as i128) {
        // panic!("insufficient balance");
        return 106;
    }
    if send_token_client.allowance(&offeror, &contract) < (transfer_amount as i128) {
        // panic!("insufficient allowance");
        return 107;
    }

    send_token_client.transfer(&offeror, &contract, &(send_amount as i128));
    send_token_client.transfer(&offeror, &fee_info.fee_wallet, &(fee_amount as i128));

    offer_write(
        e,
        offer_id,
        &OfferInfo {
            offeror: offeror.clone(),
            send_token: send_token.clone(),
            recv_token: recv_token.clone(),
            send_amount,
            recv_amount,
            min_recv_amount,
            status: OfferStatus::ACTIVE,
        },
    );
    let new_offer_count: u32 = offer_count + 1;
    e.storage().instance().set(&DataKey::OFFER_COUNT, &new_offer_count);
    e.storage().instance().bump(200000000);

    // emit OfferCreated event
    e.events().publish((OFFER, symbol_short!("OCreate")), 
        (offer_id, offeror.clone(), send_token.clone(), recv_token.clone(), send_amount, recv_amount, min_recv_amount, timestamp)
    );

    offer_id
}

// Swaps `amount` of recv_token from acceptor for `send_token` amount calculated by the amount.
// acceptor needs to authorize the `swap` call and internal `transfer` call to the contract address.
pub fn offer_accept(e: &Env, 
    acceptor: &Address, 
    offer_id: u32,
    amount: u64
) -> u32 {
    if !e.storage().instance().has(&DataKey::RegOffers(offer_id)) {
        // panic!("can't find offer");
        return 110;
    }

    let mut offer = offer_load(e, offer_id);

    if !fee_check(e) {
        // panic!("fee isn't set");
        return 111;
    }
    if offer.status != OfferStatus::ACTIVE {
        // panic!("offer not available");
        return 112;
    }
    if offer.recv_amount < amount {
        // panic!("amount is greater than max_recv_amount");
        return 113;
    }
    if amount < offer.min_recv_amount {
        // panic!("amount must be more than min_recv_amount");
        return 114;
    }
    
    // acceptor needs to authorize the trade.
    // acceptor.require_auth();

    // Load the offer and prepare the token clients to do the trade.
    let send_token_client = token::Client::new(e, &offer.send_token);
    let recv_token_client = token::Client::new(e, &offer.recv_token);

    let fee_info = fee_get(e);
    let fee_amount: u64 = calculate_fee(&fee_info.clone(), amount);
    let contract = e.current_contract_address();
    
    if recv_token_client.balance(&acceptor) < (amount + fee_amount) as i128 {
        // panic!("insufficient balance");
        return 115;
    }
    if recv_token_client.allowance(&acceptor, &contract.clone()) < (amount + fee_amount) as i128 {
        // panic!("insufficient allowance");
        return 116;
    }

    // Compute the amount of send_token that acceptor can receive.
    let prop_send_amount = amount.checked_mul(offer.send_amount).unwrap_optimized() / offer.recv_amount;

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
    recv_token_client.transfer(&acceptor, &fee_info.fee_wallet, &(fee_amount as i128));
    // Transfer the `recv_token` to the offeror immediately.
    recv_token_client.transfer(&acceptor, &offer.offeror, &(amount as i128));
    // Transfer the `send_token` from contract to acceptor.
    send_token_client.transfer(&contract, &acceptor, &(prop_send_amount as i128));

    // Update Offer
    offer.send_amount -= prop_send_amount;
    offer.recv_amount -= amount;

    if offer.recv_amount == 0 {
        offer.status = OfferStatus::COMPLETE;
        // emit OfferCompleted event
        e.events().publish((OFFER, symbol_short!("OComplete")), 
            offer_id
        );
    }
    else if offer.recv_amount < offer.min_recv_amount {
        offer.min_recv_amount = offer.recv_amount;
    }

    offer_write(e, offer_id, &offer);

    // emit OfferAccepted event
    e.events().publish((OFFER, symbol_short!("OAccept")), 
        (acceptor.clone(), offer_id, amount)
    );

    0
}

// Updates offer
// Must be authorized by offeror.
pub fn offer_update(e: &Env, 
    offeror: &Address, 
    offer_id: u32, 
    recv_amount: u64, 
    min_recv_amount: u64
) -> u32 {
    if recv_amount == 0 {
        // panic!("zero amount is not allowed");
        return 121;
    }
    if min_recv_amount > recv_amount {
        // panic!("min_recv_amount can't be greater than recv_amount");
        return 122;
    }

    if !e.storage().instance().has(&DataKey::RegOffers(offer_id)) {
        // panic!("can't find offer");
        // return 123;
        return e.ledger().sequence();
    }

    let mut offer = offer_load(e, offer_id);

    if offer.offeror != offeror.clone() {
        // panic!("invalid offeror");
        return 124;
    }
    if offer.status != OfferStatus::ACTIVE {
        // panic!("offer not available");
        return 125;
    }

    // offeror.clone().require_auth();
    offer.recv_amount = recv_amount;
    offer.min_recv_amount = min_recv_amount;
    offer_write(e, offer_id, &offer);

    // emit OfferUpdated event
    e.events().publish((OFFER, symbol_short!("OUpdate")), 
        (offeror.clone(), offer_id, recv_amount, min_recv_amount)
    );

    0
}

// Cancel offer
// Must be authorized by offeror.
pub fn offer_close(e: &Env, 
    offeror: &Address, 
    offer_id: u32
) -> u32 {
    if !e.storage().instance().has(&DataKey::RegOffers(offer_id)) {
        // panic!("can't find offer");
        return 131;
    }

    let mut offer = offer_load(e, offer_id);

    if offer.offeror != offeror.clone() {
        // panic!("invalid offeror");
        return 132;
    }
    if offer.status != OfferStatus::ACTIVE {
        // panic!("offer not available");
        return 133;
    }

    // offeror.clone().require_auth();
    token::Client::new(e, &offer.send_token).transfer(
        &e.current_contract_address(),
        &offeror,
        &(offer.send_amount as i128),
    );

    offer.status = OfferStatus::CANCEL;
    offer_write(e, offer_id, &offer);

    // emit OfferRevoked event
    e.events().publish((OFFER, symbol_short!("ORevoke")), 
        (offeror.clone(), offer_id)
    );

    0
}

// Check balances
pub fn offer_balances(e: &Env, 
    offeror: &Address, 
    acceptor: &Address, 
    send_token: &Address, 
    recv_token: &Address
) -> (u64, u64, u64, u64) {
    let send_token_client = token::Client::new(e, send_token);
    let recv_token_client = token::Client::new(e, recv_token);

    (send_token_client.balance(offeror) as u64, recv_token_client.balance(offeror) as u64, 
        send_token_client.balance(acceptor) as u64, recv_token_client.balance(acceptor) as u64)
}

fn offer_load(e: &Env, key: u32) -> OfferInfo {
    e.storage().instance().get(&DataKey::RegOffers(key)).unwrap()
}

fn offer_write(e: &Env, key: u32, offer: &OfferInfo) {
    e.storage().instance().set(&DataKey::RegOffers(key), offer);
    e.storage().instance().bump(200000000);
}
