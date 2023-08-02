#![cfg(test)]
extern crate std;

use soroban_sdk::{ log, token, BytesN };
use crate::storage_types::{ DEF_FEE_RATE, TOKEN_DECIMALS, FeeInfo };
use crate::{ TokenSwap, TokenSwapClient };


use soroban_sdk::{
    symbol_short, Symbol,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal,
};


fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::AdminClient<'a>) {
    let addr = e.register_stellar_asset_contract(admin.clone());
    (
        addr.clone(),
        token::Client::new(e, &addr),
        token::AdminClient::new(e, &addr),
    )
}

fn create_token_swap_contract<'a>(
    e: &Env,
) -> TokenSwapClient<'a> {
    let token_swap = TokenSwapClient::new(e, &e.register_contract(None, TokenSwap {}));

    token_swap
}


#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();


    let token_admin = Address::random(&e);
    let offeror = Address::random(&e);
    let acceptor = Address::random(&e);
    const MUL_VAL: u64 = u64::pow(10, TOKEN_DECIMALS);
    
    
    // create contract
    let token_swap = create_token_swap_contract(
        &e,
    );


    let send_token = create_token_contract(&e, &token_admin);
    let send_token_id = send_token.0;
    let send_token_client = send_token.1;
    let send_token_admin_client = send_token.2;
    send_token_admin_client.mint(&offeror.clone(), &(1000 * MUL_VAL));
    log!(&e, "send_token_id = {}", send_token_id);
    
    let recv_token = create_token_contract(&e, &token_admin);
    let recv_token_id = recv_token.0;
    let recv_token_client = recv_token.1;
    let recv_token_admin_client = recv_token.2;
    recv_token_admin_client.mint(&acceptor.clone(), &(100 * MUL_VAL));
    
    
    // init fee
    let fee_rate = DEF_FEE_RATE;
    let fee_wallet = Address::random(&e);

    token_swap.set_fee(&fee_rate, &fee_wallet);
    

    // allow tokens
    token_swap.allow_token(&send_token_id);
    token_swap.allow_token(&recv_token_id);

    send_token_client.approve(&offeror.clone(), &token_swap.address.clone(), &(1000 * MUL_VAL), &2000000);
    recv_token_client.approve(&acceptor.clone(), &token_swap.address.clone(), &(100 * MUL_VAL), &2000000);
    
    
    // Initial transaction 1 - create offer
    // 500 send_tokens : 50 recv_tokens (10 min_recv_tokens)
    let timestamp: u32 = e.ledger().timestamp();
    
    let offer_id: BytesN<32> = token_swap.create_offer(
        &offeror,
        &send_token_id,
        &recv_token_id,
        &timestamp,
        &(500 * MUL_VAL),
        &(50 * MUL_VAL),
        &(10 * MUL_VAL));
    
    // Verify that authorization is required for the offeror.
    assert_eq!(
        e.auths(),
        std::vec![(
            offeror.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    token_swap.address.clone(),
                    Symbol::new(&e, "create_offer"),
                    (
                        offeror.clone(),
                        &send_token_id,
                        &recv_token_id,
                        timestamp,
                        500 * MUL_VAL,
                        50 * MUL_VAL,
                        10 * MUL_VAL
                    )
                        .into_val(&e)
                )),
                sub_invocations: std::vec![
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            send_token_id.clone(),
                            symbol_short!("transfer"),
                            (
                                offeror.clone(),
                                token_swap.address.clone(),
                                500 * MUL_VAL,
                            )
                                .into_val(&e)
                        )),
                        sub_invocations: std::vec![]
                    },
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            send_token_id.clone(),
                            symbol_short!("transfer"),
                            (
                                offeror.clone(),
                                fee_wallet.clone(),
                                12500_i128,
                            )
                                .into_val(&e)
                        )),
                        sub_invocations: std::vec![],
                    }
                ]
            }
        )]
    );

    // trying to create an offer with same params - fail
    assert!(token_swap.try_create_offer(
        &offeror,
        &send_token_id,
        &recv_token_id,
        &timestamp,
        &(500 * MUL_VAL),
        &(50 * MUL_VAL),
        &(10 * MUL_VAL)).is_err());

    // trying to create an offer with different timestamp - fails due to insufficient balance
    let timestamp2: u64 = timestamp + 127;
    assert!(token_swap.try_create_offer(
        &offeror,
        &send_token_id,
        &recv_token_id,
        &timestamp2,
        &(500 * MUL_VAL),
        &(50 * MUL_VAL),
        &(10 * MUL_VAL)).is_err());
    
    
    // Try accepting 9 recv_token for at least 10 recv_token - that wouldn't
    // succeed because minimum recv amount is 10 recv_token.
    assert!(token_swap.try_accept_offer(
        &acceptor, 
        &offer_id, 
        &(9 * MUL_VAL)).is_err());
    
    // acceptor accepts 10 recv_tokens.
    token_swap.accept_offer(
        &acceptor,
        &offer_id,
        &(10_i128 * MUL_VAL));
    
    assert_eq!(send_token_client.balance(&offeror), 500_i128 * MUL_VAL - 12500);
    assert_eq!(send_token_client.balance(&token_swap.address), 400_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&acceptor), 100_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&fee_wallet), 12500);
    
    assert_eq!(recv_token_client.balance(&offeror), 10_i128 * MUL_VAL);
    assert_eq!(recv_token_client.balance(&token_swap.address), 0);
    assert_eq!(recv_token_client.balance(&acceptor), 90_i128 * MUL_VAL - 250);
    assert_eq!(recv_token_client.balance(&fee_wallet), 250);
    
    
    // update (recv_amount, min_recv_amount) from (40, 10) to (80, 20)
    token_swap.update_offer(
        &offeror,
        &offer_id,
        &(80_i128 * MUL_VAL),   // new recv_amount
        &(20_i128 * MUL_VAL)    // new min_recv_amount
    );

    
    // acceptor accepts 40 recv_tokens.
    token_swap.accept_offer(
        &acceptor, 
        &offer_id, 
        &(40 * MUL_VAL));
    
    assert_eq!(send_token_client.balance(&offeror), 500_i128 * MUL_VAL - 12500);
    assert_eq!(send_token_client.balance(&token_swap.address), 200_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&acceptor), 300_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&fee_wallet), 12500);

    assert_eq!(recv_token_client.balance(&offeror), 50_i128 * MUL_VAL);
    assert_eq!(recv_token_client.balance(&token_swap.address), 0);
    assert_eq!(recv_token_client.balance(&acceptor), 50_i128 * MUL_VAL - 1250);
    assert_eq!(recv_token_client.balance(&fee_wallet), 1250);
    
    
    // offeror closes offer
    token_swap.close_offer(
        &offeror,
        &offer_id
    );

    assert_eq!(send_token_client.balance(&offeror), 700_i128 * MUL_VAL - 12500);
    assert_eq!(send_token_client.balance(&token_swap.address), 0);
    assert_eq!(send_token_client.balance(&acceptor), 300_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&fee_wallet), 12500);
    
    assert_eq!(recv_token_client.balance(&offeror), 50_i128 * MUL_VAL);
    assert_eq!(recv_token_client.balance(&token_swap.address), 0);
    assert_eq!(recv_token_client.balance(&acceptor), 50_i128 * MUL_VAL - 1250);
    assert_eq!(recv_token_client.balance(&fee_wallet), 1250);


    // disallow tokens
    token_swap.disallow_token(&send_token_id);
    token_swap.disallow_token(&recv_token_id);
}
