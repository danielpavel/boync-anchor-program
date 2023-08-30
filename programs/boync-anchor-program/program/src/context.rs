use anchor_lang::{
    prelude::*,
    solana_program::sysvar,
};

use anchor_spl::{
    token::{ TokenAccount, Token, Mint },
    associated_token::AssociatedToken,
};

use crate::constants::*;
use crate::utils::TokenMetadataProgram;
use crate::errors::AuctionError;
use crate::account::{BoyncAuction2, BoyncAuction, BoyncUserBid};

#[derive(Accounts)]
#[instruction(app_idx: i64, state_bump: u8)]
pub struct InitializeAuction<'info> {
    /// State of our auction program (up to you)
    #[account(
        init,
        payer = signer,
        space = 8 + BoyncAuction::AUCTION_SIZE,
        seeds = [
            AUCTION_SEED,
            signer.key().as_ref(),
            treasury_mint.key().as_ref(),
            app_idx.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub state: Box<Account<'info, BoyncAuction>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            TREASURY_SEED,
            signer.key().as_ref(),
            treasury_mint.key().as_ref(),
            app_idx.to_le_bytes().as_ref(),
        ],
        bump,
        token::mint = treasury_mint,
        token::authority = state
    )]
    /// Account holding token being auctioned.
    pub treasury: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        seeds = [
            WALLET_SEED,
            signer.key().as_ref(),
            collector_mint.key().as_ref(),
            app_idx.to_le_bytes().as_ref(),
        ],
        bump,
        token::mint = collector_mint,
        token::authority = state
    )]
    /// Account which holds tokens bidded by biders
    pub bidders_chest: Account<'info, TokenAccount>,

    // Users and accounts in the system
    #[account(mut)]
    pub signer: Signer<'info>,
    /// Mint for SPL Token stored in treasury.
    pub treasury_mint: Account<'info, Mint>,
    /// Mint for SPL Token stored in bidder's chest.
    pub collector_mint: Account<'info, Mint>,

    /// Payer's SPL Token account wallet
    /// (The wallet who will send the token(s) being auctioned)
    #[account(
        mut,
        constraint=signer_ata.owner == signer.key(),
        constraint=signer_ata.mint == treasury_mint.key()
    )]
    signer_ata: Account<'info, TokenAccount>,

    // Application level accounts
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(app_idx: i64, state_bump: u8, fp: u64, start_at: i64, end_at: i64)]
pub struct InitializeAuction2<'info> {
    /// State of our auction program (up to you)
    #[account(
        init,
        payer = signer,
        space = 8 + BoyncAuction::AUCTION_SIZE,
        seeds = [
            AUCTION_SEED,
            signer.key().as_ref(),
            treasury_mint.key().as_ref(),
            app_idx.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub state: Box<Account<'info, BoyncAuction2>>,

    #[account(
        init,
        payer = signer,
        seeds = [
            TREASURY_SEED,
            signer.key().as_ref(),
            treasury_mint.key().as_ref(),
            app_idx.to_le_bytes().as_ref(),
        ],
        bump,
        token::mint = treasury_mint,
        token::authority = state
    )]
    /// Token Account holding token being auctioned.
    pub treasury: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [WALLET_SEED, signer.key().as_ref(), app_idx.to_le_bytes().as_ref()],
        bump
    )]
    /// Account which holds tokens bidded by biders
    /// CHECK: only used as a signing PDA
    pub bidders_chest: AccountInfo<'info>,

    // Users and accounts in the system
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Mint for SPL Token stored in treasu2ry.
    pub treasury_mint: Box<Account<'info, Mint>>,

    /// CHECK: Metadata Account
    /// verified in `initialize_auction_2`
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,

    //// CHECK: Edition Account
    /// verified in `initialize_auction_2`
    pub edition: UncheckedAccount<'info>,

    //// CHECK: Owner Token Record Account
    /// verified in `initialize_auction_2`
    #[account(mut)]
    pub owner_token_record: UncheckedAccount<'info>,

    //// CHECK: Owner Token Record Account
    /// verified in `initialize_auction_2`
    #[account(mut)]
    pub destination_token_record: UncheckedAccount<'info>,

    /// CHECK: PDA checked by anchor
    pub auth_rules: UncheckedAccount<'info>,

    /// SPL Token account for Signer wallet
    /// (The wallet who will send the Token being auctioned)
    #[account(
        init_if_needed,
        associated_token::mint = treasury_mint,
        associated_token::authority = signer,
        payer = signer
    )]
    pub signer_token_account: Box<Account<'info, TokenAccount>>,

    // Application level accounts
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(address = mpl_token_auth_rules::id())]
    pub auth_rules_token_program: UncheckedAccount<'info>,
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: Program<'info, TokenMetadataProgram>,
    rent: Sysvar<'info, Rent>,
    #[account(address = sysvar::instructions::id())]
    pub sysvar_instructions: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Box<Account<'info, BoyncAuction2>>,

    /// Token Account holding token being auctioned.
    #[account(
        mut,
        seeds = [TREASURY_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump,
        token::mint=treasury_mint,
        token::authority=state
    )]
    pub treasury: Box<Account<'info, TokenAccount>>,

    /// Mint for SPL Token stored in treasury.
    pub treasury_mint: Account<'info, Mint>,

    // Users and accounts in the system
    #[account(mut)]
    pub winner: Signer<'info>,

    /// Winner's SPL Token account wallet
    /// (The wallet who will receive the auctioned token(s))
    #[account(
        init_if_needed,
        payer = winner,
        associated_token::mint = treasury_mint,
        associated_token::authority = winner,
        constraint = winner_token_account.owner == winner.key(),
        constraint = winner_token_account.mint == treasury_mint.key()
    )]
    pub winner_token_account: Account<'info, TokenAccount>,

    /// CHECK: Metadata Account
    /// verified part of the mpl_metadata_token::transfer
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,

    //// CHECK: Edition Account
    /// verified part of the mpl_metadata_token::transfer
    pub edition: UncheckedAccount<'info>,

    //// CHECK: Owner Token Record Account
    /// verified part of the mpl_metadata_token::transfer
    #[account(mut)]
    pub owner_token_record: UncheckedAccount<'info>,

    //// CHECK: Owner Token Record Account
    /// verified part of the mpl_metadata_token::transfer
    #[account(mut)]
    pub destination_token_record: UncheckedAccount<'info>,

    /// CHECK: Authorization Rules account
    /// verified part of the mpl_metadata_token::transfer
    pub auth_rules: UncheckedAccount<'info>,

    // Application level accounts
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(address = mpl_token_auth_rules::id())]
    pub auth_rules_token_program: UncheckedAccount<'info>,
    #[account(address = mpl_token_metadata::id())]
    pub token_metadata_program: Program<'info, TokenMetadataProgram>,
    pub rent: Sysvar<'info, Rent>,

    #[account(address = sysvar::instructions::id())]
    pub sysvar_instructions: UncheckedAccount<'info>,
}

/*
 * [DEPRECATED]
 *
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Account<'info, BoyncAuction>,

    #[account(mut)]
    /// Account which holds auctioned token(s).
    pub treasury: Account<'info, TokenAccount>,
    /// Mint for SPL Token stored in treasury.
    pub treasury_mint: Account<'info, Mint>,

    // Users and accounts in the system
    #[account(mut)]
    pub winner: Signer<'info>,

    /// Winner's SPL Token account wallet
    /// (The wallet who will receive the auctioned token(s))
    #[account(
        init,
        payer = winner,
        associated_token::mint = treasury_mint,
        associated_token::authority = winner,
        constraint=winner_token_account.owner == winner.key(),
        constraint=winner_token_account.mint == treasury_mint.key(),
    )]
    winner_token_account: Account<'info, TokenAccount>,

    // Application level accounts
    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}
*/

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(mut, has_one = authority @ AuctionError::InvalidAuthority)]
    pub state: Account<'info, BoyncAuction2>,

    /// CHECK: only used as a signing PDA
    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub bidders_chest: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Treasury account will be one random account that only receives SOL.
    pub treasury: AccountInfo<'info>,

    // Application level accounts
    pub system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

/**
 * V2
 * Users use SOL to bid.
 * [BA-Program-5uJBi4jN][MVP] Remove BOYNC token GATE
 */
#[derive(Accounts)]
#[instruction(ts: i64)]
pub struct UpdateAuction2<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Account<'info, BoyncAuction2>,

    /// CHECK: only used as a signing PDA
    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub bidders_chest: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = bidder,
        space = 8 + BoyncUserBid::ACCOUNT_SIZE,
        seeds = [
            BIDDER_SEED,
            state.key().as_ref(),
            bidder.key().as_ref(),
            ts.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub bidder_state: Account<'info, BoyncUserBid>,

    // Users and accounts in the system
    #[account(mut)]
    pub bidder: Signer<'info>,

    // Application level accounts
    pub system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

/*
 * [DEPRECATED]
 *
 */
#[derive(Accounts)]
pub struct UpdateAuction<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Account<'info, BoyncAuction>,

    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.collector_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    /// Account which holds tokens bidded by biders
    /// CHECK: only used as a signing PDA
    pub bidders_chest: AccountInfo<'info>,

    // Users and accounts in the system
    #[account(mut)]
    pub bidder: Signer<'info>,
    /// Mint for SPL Token stored in bidder's chest.
    pub collector_mint: Account<'info, Mint>,

    /// Payer's SPL Token account wallet
    /// (The wallet who will send the token(s) being auctioned)
    #[account(
        mut,
        constraint=bidder_withdraw_wallet.owner == bidder.key(),
        constraint=bidder_withdraw_wallet.mint == collector_mint.key(),
    )]
    bidder_withdraw_wallet: Account<'info, TokenAccount>,

    // Application level accounts
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

/*
 * [DEPRECATED]
 *
 */
#[derive(Accounts)]
pub struct UpdateAuctionState2<'info> {
    #[account(mut, has_one = authority @ AuctionError::InvalidAuthority)]
    pub auction: Account<'info, BoyncAuction2>,
    pub authority: Signer<'info>,
}