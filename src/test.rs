#![cfg(test)]
extern crate std;

use crate::{token, SingleOfferClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    Address, Env, IntoVal, Symbol,
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

fn create_single_offer_contract<'a>(
    e: &Env,
    offeror: &Address,
    send_token: &Address,
    recv_token: &Address,
    send_amount: i128,
    recv_amount: i128,
    min_recv_amount: i128,
) -> SingleOfferClient<'a> {
    let offer = SingleOfferClient::new(e, &e.register_contract(None, crate::SingleOffer {}));
    offer.create(offeror, send_token, recv_token, &(send_amount as i128), &(recv_amount as i128), &min_recv_amount);
    
    // Verify that authorization is required for the offeror.
    assert_eq!(
        e.auths(),
        std::vec![(
            offeror.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    offer.address.clone(),
                    symbol_short!("create"),
                    (
                        offeror,
                        send_token.clone(),
                        recv_token.clone(),
                        send_amount,
                        recv_amount,
                        min_recv_amount
                    )
                        .into_val(e)
                )),
                sub_invocations: std::vec![
                    AuthorizedInvocation {
                        function: AuthorizedFunction::Contract((
                            send_token.clone(),
                            symbol_short!("transfer"),
                            (
                                offeror,
                                offer.address.clone(),
                                send_amount,
                            )
                                .into_val(e)
                        )),
                        sub_invocations: std::vec![],
                    },
                ]
            }
        )]
    );

    offer
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

    // Mint 1000 send_tokens to offeror and 100 recv_tokens to acceptor.
    send_token_admin_client.mint(&offeror, &1000);
    recv_token_admin_client.mint(&acceptor, &100);
    
    // Initial transaction 1
    // 500 send_tokens : 50 recv_tokens (10 min_recv_tokens)
    let offer = create_single_offer_contract(
        &e,
        &offeror,
        &send_token_client.address,
        &recv_token_client.address,
        500,
        50,
        10,
    );
    
    
    // Try accepting 9 recv_token for at least 10 recv_token - that wouldn't
    // succeed because minimum recv amount is 10 recv_token.
    assert!(offer.try_accept(&acceptor, &9_i128).is_err());
    
    // acceptor accepts 10 recv_tokens.
    offer.accept(&acceptor, &10_i128);
    // Verify that authorization is required for the acceptor.
    assert_eq!(
        e.auths(),
        std::vec![(
            acceptor.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    offer.address.clone(),
                    symbol_short!("accept"),
                    (&acceptor, 10_i128).into_val(&e)
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        recv_token_client.address.clone(),
                        symbol_short!("transfer"),
                        (&acceptor, &offeror, 10_i128).into_val(&e)
                    )),
                    sub_invocations: std::vec![]    // ???
                }]
            }
        )]
    );

    assert_eq!(send_token_client.balance(&offeror), 500);
    assert_eq!(send_token_client.balance(&offer.address), 400);
    assert_eq!(send_token_client.balance(&acceptor), 100);
    
    assert_eq!(recv_token_client.balance(&offeror), 10);
    assert_eq!(recv_token_client.balance(&acceptor), 90);
    assert_eq!(recv_token_client.balance(&offer.address), 0);


    // recv_amount = 80, min_recv_amount = 20
    offer.update(&80_i128, &20_i128);
    // Verify that the seller has to authorize this.
    assert_eq!(
        e.auths(),
        std::vec![(
            offeror.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    offer.address.clone(),
                    Symbol::new(&e, "update"),
                    (80_i128, 20_i128).into_val(&e)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );


    // acceptor accepts 40 recv_tokens.
    offer.accept(&acceptor, &40_i128);
    
    assert_eq!(send_token_client.balance(&offeror), 500);
    assert_eq!(send_token_client.balance(&offer.address), 200);
    assert_eq!(send_token_client.balance(&acceptor), 300);
    
    assert_eq!(recv_token_client.balance(&offeror), 50);
    assert_eq!(recv_token_client.balance(&offer.address), 0);
    assert_eq!(recv_token_client.balance(&acceptor), 50);


    // offeror closes offer
    offer.close();
    // Verify that authorization is required for the acceptor.
    assert_eq!(
        e.auths(),
        std::vec![(
            offeror.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    offer.address.clone(),
                    symbol_short!("close"),
                    ().into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}
