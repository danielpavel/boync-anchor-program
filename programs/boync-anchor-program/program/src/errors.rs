use anchor_lang::prelude::*;

/**
 * Errors
 */
#[error_code]
pub enum AuctionError {
    /// Invalid transition, auction state may only transition: Created -> Started -> Stopped
    #[msg("Invalid auction state transition.")]
    AuctionTransitionInvalid,

    #[msg("Auction is not currently running.")]
    InvalidState,

    #[msg("Auction has not started yet")]
    AuctionNotStarted,

    #[msg("Auction has ended")]
    AuctionEnded,

    #[msg("Auction has not ended yet")]
    AuctionActive,

    #[msg("Auction has already been claimed!")]
    AuctionClaimed,

    #[msg("You can't bid on an auction you created!")]
    AuctionAuthorityBid,

    #[msg("You can't bid on an auction if you're already the last bidder!")]
    AuctionAlreadyLastBidder,

    #[msg("You Are not the winner")]
    YouAreNotTheWinner,

    #[msg("You Are not the authority")]
    YouAreNotTheAuthority,

    /// Bid is too small.
    #[msg("Bid is too small.")]
    BidTooSmall,

    #[msg("You are not the authority for this auction!")]
    InvalidAuthority,
}
