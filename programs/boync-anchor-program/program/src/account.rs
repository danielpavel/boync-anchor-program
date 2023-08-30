use anchor_lang::prelude::*;
use crate::constants::*;
use crate::errors::AuctionError;

use std::mem::size_of;

/**
 * [Deprecated]
 */
#[account]
pub struct BoyncAuction {
    pub id: i64,
    pub end_auction_at: i64, // 1 + 64
    pub authority: Pubkey,
    pub treasury_mint: Pubkey,
    pub collector_mint: Pubkey,
    pub treasury: Pubkey,
    pub bidders_chest: Pubkey,
    pub tokens_spent: u64,
    pub state: AuctionState, // 1 + 32
    pub last_bidder: Pubkey,
    pub bump: u8,
}

/**
 * [Deprecated]
 */
impl BoyncAuction {
    // pub const AUCTION_SIZE: usize = ( 1 + 32 ) + ( 1 + 32 ) + ( 1 + 64 );
    pub const AUCTION_SIZE: usize = size_of::<BoyncAuction>();

    pub fn ended(&self, now: i64) -> Result<bool> {
        Ok(now * MS_IN_SEC > self.end_auction_at)
    }
}

/**
 * V2
 * Users use SOL to bid.
 * [BA-Program-5uJBi4jN][MVP] Remove BOYNC token GATE
 */
#[account]
pub struct BoyncAuction2 {
    pub id: i64,
    pub start_auction_at: i64, // 1 + 64
    pub end_auction_at: i64, // 1 + 64
    pub authority: Pubkey,
    pub treasury_mint: Pubkey,
    pub treasury: Pubkey,
    pub bidders_chest: Pubkey,
    pub starting_price: u64,
    pub next_bid: u64,
    pub claimed: u8,
    pub state: AuctionState, // 1 + 32
    pub last_bidder: Pubkey,
    pub bump: u8,
}

/**
 * V2
 * Users use SOL to bid.
 * [BA-Program-5uJBi4jN][MVP] Remove BOYNC token GATE
 */
impl BoyncAuction2 {
    // pub const AUCTION_SIZE: usize = ( 1 + 32 ) + ( 1 + 32 ) + ( 1 + 64 );
    pub const AUCTION_SIZE: usize = size_of::<BoyncAuction>();

    pub fn ended(&self, now: i64) -> Result<bool> {
        Ok(now * MS_IN_SEC > self.end_auction_at)
    }
}

#[account]
pub struct BoyncUserBid {
    pub auction: Pubkey,
    pub bidder: Pubkey,
    pub bid_value: u64,
    pub ts: i64, // 1 + 64
}

impl BoyncUserBid {
    pub const ACCOUNT_SIZE: usize = size_of::<BoyncUserBid>();
}

/*
 * Boync Auction State
 *
 */
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Copy)]
pub enum AuctionState {
    Created,
    Started,
    Ended,
}

impl AuctionState {
    pub fn create() -> Self {
        AuctionState::Created
    }

    #[inline(always)]
    pub fn start(self) -> Result<Self> {
        match self {
            AuctionState::Created => Ok(AuctionState::Started),
            _ => Err(AuctionError::AuctionTransitionInvalid.into()),
        }
    }

    #[inline(always)]
    pub fn end(self) -> Result<Self> {
        match self {
            AuctionState::Started => Ok(AuctionState::Ended),
            AuctionState::Created => Ok(AuctionState::Ended),
            _ => Err(AuctionError::AuctionTransitionInvalid.into()),
        }
    }
}