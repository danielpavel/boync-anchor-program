use super::constants::*;
use crate::id;
use anchor_lang::prelude::Pubkey;

pub fn find_boync_auction_address(
    authority: &Pubkey,
    mint_address: &Pubkey,
    ts: &i64,
) -> (Pubkey, u8) {
    let ts_bytes = ts.to_le_bytes();
    let seeds = &[
        AUCTION_PREFIX.as_bytes(),
        authority.as_ref(),
        mint_address.as_ref(),
        ts_bytes.as_ref(),
    ];
    Pubkey::find_program_address(seeds, &id())
}

pub fn find_boync_treasury_address(
    authority: &Pubkey,
    mint_address: &Pubkey,
    ts: &i64,
) -> (Pubkey, u8) {
    let ts_bytes = ts.to_le_bytes();
    let seeds = &[
        TREASURY_PREFIX.as_bytes(),
        authority.as_ref(),
        mint_address.as_ref(),
        ts_bytes.as_ref(),
    ];
    Pubkey::find_program_address(seeds, &id())
}

pub fn find_boync_bidders_chest_address(authority: &Pubkey, ts: &i64) -> (Pubkey, u8) {
    let ts_bytes = ts.to_le_bytes();
    let seeds = &[
        WALLET_PREFIX.as_bytes(),
        authority.as_ref(),
        ts_bytes.as_ref(),
    ];
    Pubkey::find_program_address(seeds, &id())
}

pub fn find_boync_bidder_state_address(
    authority: &Pubkey,
    bidder: &Pubkey,
    ts: &i64,
) -> (Pubkey, u8) {
    let ts_bytes = ts.to_le_bytes();
    let seeds = &[
        BIDDER_PREFIX.as_bytes(),
        authority.as_ref(),
        bidder.as_ref(),
        ts_bytes.as_ref(),
    ];
    Pubkey::find_program_address(seeds, &id())
}
