#![cfg(test)]
extern crate std;

use soroban_sdk::{ log, token };
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
) -> (token::Client<'a>, token::AdminClient<'a>) {
    let addr = e.register_stellar_asset_contract(admin.clone());
    (
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

    let send_token = create_token_contract(&e, &token_admin);
    let send_token_client = send_token.0;
    let send_token_admin_client = send_token.1;

    let recv_token = create_token_contract(&e, &token_admin);
    let recv_token_client = recv_token.0;
    let recv_token_admin_client = recv_token.1;
    let timestamp: u64 = e.ledger().timestamp();
    const MUL_VAL: i128 = i128::pow(10, TOKEN_DECIMALS);

    // Mint 1000 send_tokens to offeror and 100 recv_tokens to acceptor.
    send_token_admin_client.mint(&offeror, &(1000 * MUL_VAL));
    recv_token_admin_client.mint(&acceptor, &(100 * MUL_VAL));
    
    
    // create contract
    let token_swap = create_token_swap_contract(
        &e,
    );

    // init fee
    let fee_rate = DEF_FEE_RATE;
    let fee_wallet = Address::random(&e);

    token_swap.init_fee(&FeeInfo{ fee_rate, fee_wallet: fee_wallet.clone() });


    // allow tokens
    token_swap.allow_token(&send_token_client.address);
    token_swap.allow_token(&recv_token_client.address);
    
    
    // Initial transaction 1 - create offer
    // 500 send_tokens : 50 recv_tokens (10 min_recv_tokens)
    token_swap.create_offer(
        &offeror,
        &send_token_client.address,
        &recv_token_client.address,
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
                        send_token_client.address.clone(),
                        recv_token_client.address.clone(),
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
                            send_token_client.address.clone(),
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
                            send_token_client.address.clone(),
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
    
    
    // Try accepting 9 recv_token for at least 10 recv_token - that wouldn't
    // succeed because minimum recv amount is 10 recv_token.
    assert!(token_swap.try_accept_offer(&offeror,
        &send_token_client.address,
        &recv_token_client.address,
        &timestamp,
        &acceptor, 
        &(9 * MUL_VAL)).is_err());
    
    // acceptor accepts 10 recv_tokens.
    token_swap.accept_offer(&offeror,
        &send_token_client.address,
        &recv_token_client.address,
        &timestamp,
        &acceptor,
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
    token_swap.update_offer(&offeror,
        &send_token_client.address,
        &recv_token_client.address,
        &timestamp,
        &(80_i128 * MUL_VAL),   // new recv_amount
        &(20_i128 * MUL_VAL)    // new min_recv_amount
    );

    
    // acceptor accepts 40 recv_tokens.
    token_swap.accept_offer(&offeror,
        &send_token_client.address,
        &recv_token_client.address,
        &timestamp,
        &acceptor, 
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
    token_swap.close_offer(&offeror,
        &send_token_client.address,
        &recv_token_client.address,
        &timestamp);

    assert_eq!(send_token_client.balance(&offeror), 700_i128 * MUL_VAL - 12500);
    assert_eq!(send_token_client.balance(&token_swap.address), 0);
    assert_eq!(send_token_client.balance(&acceptor), 300_i128 * MUL_VAL);
    assert_eq!(send_token_client.balance(&fee_wallet), 12500);
    
    assert_eq!(recv_token_client.balance(&offeror), 50_i128 * MUL_VAL);
    assert_eq!(recv_token_client.balance(&token_swap.address), 0);
    assert_eq!(recv_token_client.balance(&acceptor), 50_i128 * MUL_VAL - 1250);
    assert_eq!(recv_token_client.balance(&fee_wallet), 1250);


    // disallow tokens
    token_swap.disallow_token(&send_token_client.address);
    token_swap.disallow_token(&recv_token_client.address);
}
