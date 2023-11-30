use soroban_sdk::{ contracttype, Address/* , BytesN */ };


pub(crate) const FEE_DECIMALS: u32 = 4;

pub(crate) const DAY_IN_LEDGERS: u32 = 17280;
pub(crate) const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS; // 7 days
pub(crate) const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS; // 6 days
pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS; // 30 days


#[derive(Clone)]
#[contracttype]
pub struct FeeInfo {
    pub fee_rate: u32,
    pub fee_wallet: Address,
}

#[derive(Clone, Copy, PartialEq)]
#[contracttype]
pub enum OfferStatus {
    INIT = 0,
    ACTIVE = 1,
    COMPLETE = 2,
    CANCEL = 3
}

// Represents an offer managed by the TokenSwap contract.
// If an offeror wants to swap 1000 XLM for 100 USDC, the `send_amount` would be 1000
// and `recv_amount` would be 100
#[derive(Clone)]
#[contracttype]
pub struct OfferInfo {
    // Owner of this offer. Swaps send_token with recv_token.
    pub offeror: Address,
    
    pub send_token: Address,
    pub recv_token: Address,
    
    // offeror-defined amount of the send token
    pub send_amount: u64,
    // offeror-defined amount of the recv token
    pub recv_amount: u64,
    pub min_recv_amount: u64,

    pub status: OfferStatus
}

#[derive(Clone)]
#[contracttype]
pub struct OfferKey {
    pub offeror: Address,
    pub send_token: Address,
    pub recv_token: Address,
    pub timestamp: u32,
}


#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    FEE,
    Allowance(Address),
    OfferCount,
    RegOffers(u32),
    ErrorCode,
}
