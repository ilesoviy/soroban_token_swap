use soroban_sdk::{
    log, token, unwrap::UnwrapOptimized, Address, Env
};
use crate::storage_types::{ FEE_DECIMALS, FeeInfo, OfferStatus, OfferKey, OfferInfo, DataKey };
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
fn calculate_fee(fee_info: &FeeInfo, amount: i128) -> i128 {
    amount * (fee_info.fee_rate as i128) / (i128::pow(10, FEE_DECIMALS))
}

// Creates the offer for offeror for the given token pair and initial amounts.
// See comment above the `Offer` struct for information on swap.
pub fn offer_create(
    e: &Env,
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    timestamp: u64,
    send_amount: i128,
    recv_amount: i128,
    min_recv_amount: i128,
) {
    log!(&e, "I'm here0!");
    if !fee_check(&e) {
        panic!("fee isn't set");
    }
    log!(&e, "I'm here1!");
    if !allow_get(&e, &send_token.clone()) || !allow_get(&e, &recv_token.clone()) {
        panic!("both tokens aren't allowed");
    }

    log!(&e, "I'm here2!");
    let key: OfferKey = OfferKey { 
        offeror: offeror.clone(), 
        send_token: send_token.clone(), 
        recv_token: recv_token.clone(), 
        timestamp };
    if e.storage().instance().has(&DataKey::RegOffers(key.clone())) {
        panic!("offer is already created");
    }
    if send_amount == 0 || recv_amount == 0 {
        panic!("zero amount is not allowed");
    }
    if min_recv_amount > recv_amount {
        panic!("min_recv_amount can't be greater than recv_amount");
    }
    
    // Authorize the `create` call by offeror to verify their identity.
    key.offeror.clone().require_auth();

    let fee_info = fee_get(&e);
    let fee_amount: i128 = calculate_fee(&fee_info.clone(), send_amount);
    
    let contract = e.current_contract_address();
    let send_token_client = token::Client::new(&e, &key.send_token.clone());
    
    // if send_token_client.balance() < (send_amount + fee_amount) {
    //     panic!("insufficient balance");
    // }

    send_token_client.transfer(&key.offeror.clone(), &contract, &(send_amount as i128));
    send_token_client.transfer(&key.offeror.clone(), &fee_info.fee_wallet, &fee_amount);

    offer_write(
        &e,
        &key,
        &OfferInfo {
            offeror: key.offeror.clone(),
            send_token: key.send_token.clone(),
            recv_token: key.recv_token.clone(),
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
pub fn offer_accept(e: &Env, 
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    timestamp: u64,
    acceptor: &Address, 
    amount: i128
) {
    let key: OfferKey = OfferKey { 
        offeror: offeror.clone(), 
        send_token: send_token.clone(), 
        recv_token: recv_token.clone(), 
        timestamp };

    let mut offer = offer_load(&e, &key);

    if !fee_check(&e) {
        panic!("fee isn't set");
    }
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

    let fee_info = fee_get(&e);
    let fee_amount: i128 = calculate_fee(&fee_info.clone(), amount);

    // if recv_token_client.balance() < (amount + fee_amount) {
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
    recv_token_client.transfer(&acceptor, &fee_info.fee_wallet, &fee_amount);
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

    offer_write(&e, &key, &offer);

    // emit OfferAccepted event
}

// Updates offer
// Must be authorized by offeror.
pub fn offer_update(e: &Env, 
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    timestamp: u64,
    recv_amount: i128, 
    min_recv_amount: i128) {
    if recv_amount == 0 {
        panic!("zero amount is not allowed");
    }
    if min_recv_amount > recv_amount {
        panic!("min_recv_amount can't be greater than recv_amount");
    }

    let key: OfferKey = OfferKey{ 
        offeror: offeror.clone(), 
        send_token: send_token.clone(), 
        recv_token: recv_token.clone(), 
        timestamp };
    let mut offer = offer_load(&e, &key);

    if offer.status != OfferStatus::ACTIVE {
        panic!("offer not available");
    }

    offer.offeror.require_auth();
    offer.recv_amount = recv_amount;
    offer.min_recv_amount = min_recv_amount;
    offer_write(&e, &key, &offer);

    // emit OfferUpdated event
}

// Cancel offer
// Must be authorized by offeror.
pub fn offer_close(e: &Env, 
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    timestamp: u64) {
    let key: OfferKey = OfferKey{ 
        offeror: offeror.clone(), 
        send_token: send_token.clone(), 
        recv_token: recv_token.clone(), 
        timestamp };
    let mut offer = offer_load(&e, &key);

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
    offer_write(&e, &key, &offer);

    // emit OfferRevoked event
}


fn offer_load(e: &Env, key: &OfferKey) -> OfferInfo {
    e.storage().instance().get(&DataKey::RegOffers(key.clone())).unwrap()
}

fn offer_write(e: &Env, key: &OfferKey, offer: &OfferInfo) {
    e.storage().instance().set(&DataKey::RegOffers(key.clone()), offer);
}
