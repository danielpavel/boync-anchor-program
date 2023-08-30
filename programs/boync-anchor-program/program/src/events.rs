use anchor_lang::prelude::*;
use crate::account::AuctionState;

/**
 * Boync Events
 */
#[event]
pub struct BoyncBidEvent {
    pub auction_pubkey: Pubkey,
    pub bidder_pubkey: Pubkey,
    pub updated_bid_value: u64,
    pub updated_end_timestamp: i64,
    pub ts: i64,
    #[index]
    pub label: String,
}

#[event]
pub struct BoyncInitializeEvent {
    pub auction_pubkey: Pubkey,
    #[index]
    pub label: String,
}

#[event]
pub struct BoyncEndEvent {
    pub auction_pubkey: Pubkey,
    pub updated_end_timestamp: i64,
    #[index]
    pub label: String,
}

#[event]
pub struct BoyncStartEvent {
    pub auction_pubkey: Pubkey,
    pub updated_auction_state: AuctionState,
    #[index]
    pub label: String
}

#[event]
pub struct BoyncClaimEvent {
    pub auction_pubkey: Pubkey,
    pub claimed: u8,
    #[index]
    pub label: String
}
