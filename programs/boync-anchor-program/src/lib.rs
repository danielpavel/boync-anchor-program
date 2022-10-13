use anchor_lang::{
    prelude::*,
    solana_program::clock::Clock,
    { AnchorDeserialize, AnchorSerialize },
};

use anchor_spl::{
    token:: { TokenAccount, Token, Mint, Transfer, CloseAccount },
};

use {
    std::mem::size_of
};

pub const TREASURY_SEED: &[u8] = b"treasury";
pub const WALLET_SEED: &[u8] = b"wallet";
pub const AUCTION_SEED: &[u8] = b"auction";

declare_id!("CTNouVLjqMCabPFdDDoinhfuFLRH5hY5PxoQHsBf6drF");

#[program]
pub mod boync_anchor_program {
    use super::*;

    pub fn initialize(ctx: Context<InitializeAuction>, app_idx: i64, amount: u64, state_bump: u8) -> Result<()> {
        msg!("[BoyncProgram] Initializing new Boync Auction State");

        let clock = Clock::get()?;
        let auction_state = &mut ctx.accounts.state;

        auction_state.end_auction_at = app_idx; // App index is UnixTimestamp
        auction_state.authority = ctx.accounts.signer.key().clone();
        auction_state.treasury_mint = ctx.accounts.treasury_mint.key().clone();
        auction_state.treasury = ctx.accounts.treasury.key().clone();
        auction_state.collector_mint = ctx.accounts.collector_mint.key().clone();
        auction_state.bidders_chest = ctx.accounts.bidders_chest.key().clone();

        msg!("Initialized new Boync Auction State for token: {}",
            auction_state.treasury.key());

        // FIX: [BA-Program-FnWbMVHB]: Fetching bump within anchor context
        //      does not work.
        // let _bump = *ctx.bumps.get("auction").unwrap();
        let _bump = state_bump;
        let mint_of_token_being_sent_pk = ctx.accounts.treasury_mint.key().clone();
        let app_idx_bytes = app_idx.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            ctx.accounts.signer.key.as_ref(),
            mint_of_token_being_sent_pk.as_ref(),
            app_idx_bytes.as_ref(),
            &[_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        msg!("[BoyncDebug] Created Seeds");

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from: ctx.accounts.signer_withdraw_wallet.to_account_info(),
            to: ctx.accounts.treasury.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
        };

        msg!("[BoyncDebug] Created Transfer");

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        msg!("[BoyncDebug] Created CPI context");

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        auction_state.tokens_spent = 0;
        if auction_state.ended(clock.unix_timestamp)? {
            auction_state.state = AuctionState::Ended;
        } else {
            auction_state.state = AuctionState::Created;
        }

        msg!("[BoyncDebug] Done!");

       Ok(())
    }


    pub fn start(ctx: Context<UpdateAuctionState>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        // Can't start an Auction that has has already yet started.
        require!(auction.state == AuctionState::Created,
            AuctionError::InvalidState);

        auction.state = auction.state.start()?;

        Ok(())
    }

    pub fn end(ctx: Context<UpdateAuctionState>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        let clock = Clock::get()?;

        // Can't end an Auction that is already ended.
        require!(auction.state == AuctionState::Started,
            AuctionError::InvalidState);

        auction.state = auction.state.end()?;
        auction.end_auction_at = clock.unix_timestamp;

        Ok(())
    }

    /// Bid
    pub fn bid(ctx: Context<UpdateAuction>, amount: u64, auction_bump: u8) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        let clock = Clock::get()?;

        // Can't bid on an Auction that is expired.
        require!(!auction_state.ended(clock.unix_timestamp)?,
            AuctionError::AuctionExpired);
    
        // Can't bid on an Auction that is not started.
        require!(auction_state.state == AuctionState::Started,
            AuctionError::InvalidState);

        // Just transfer SPL Token to bidders_chest
        // let _bump = *ctx.bumps.get("auction").unwrap();
        let _bump = auction_bump;
        let _auction_auth = auction_state.authority.clone();
        let mint_of_token_being_sent_pk = auction_state.treasury_mint.key().clone();
        let app_idx_bytes = auction_state.end_auction_at.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            _auction_auth.as_ref(),
            mint_of_token_being_sent_pk.as_ref(),
            app_idx_bytes.as_ref(),
            &[_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from: ctx.accounts.bidder_withdraw_wallet.to_account_info(),
            to: ctx.accounts.bidders_chest.to_account_info(),
            authority: ctx.accounts.bidder.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        anchor_spl::token::transfer(cpi_ctx, amount)?;

        auction_state.last_bidder = ctx.accounts.bidder.key.clone();
        auction_state.end_auction_at += 60;
        auction_state.tokens_spent += 1;

        Ok(())
    }

    pub fn claim(ctx: Context<ClaimRewards>, auction_bump: u8) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        let clock = Clock::get()?;

        // Can't withdraw on an Auction that is ongoing.
        require!(auction_state.ended(clock.unix_timestamp)?,
            AuctionError::AuctionOngoing);

        // Can't claim an Auction that is not in ended state.
        require!(auction_state.state == AuctionState::Ended,
            AuctionError::InvalidState);

        // Only Winner can claim rewards.
        // FIX: *MAYBE REDUNDAND because of 
        // #[account(constraint=state.last_bidder == winner.key())
        require!(auction_state.last_bidder.key() == ctx.accounts.winner.key(),
            AuctionError::YouAreNotTheWinner);

        // let _bump = *ctx.bumps.get("auction").unwrap();
        let _bump = auction_bump;
        let mint_of_token_being_sent_pk = ctx.accounts.treasury_mint.key().clone();
        let _auction_auth = auction_state.authority.clone();
        let app_idx_bytes = auction_state.end_auction_at.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            _auction_auth.as_ref(),
            mint_of_token_being_sent_pk.as_ref(),
            app_idx_bytes.as_ref(),
            &[_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from: ctx.accounts.treasury.to_account_info(),
            to: ctx.accounts.winner_withdraw_wallet.to_account_info(),
            authority: auction_state.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        // Use the `reload()` function on an account to reload it's state. Since we performed the
        // transfer, we are expecting the `amount` field to have changed.
        // TODO: *PROPERLY CLOSE TREASURY ACCOUNT*

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(app_idx: i64, state_bump: u8)]
pub struct InitializeAuction<'info> {
    /// State of our auction program (up to you)
    #[account(
        init,
        payer = signer,
        space = 8 + BoyncAuction::AUCTION_SIZE,
        seeds =  [AUCTION_SEED, signer.key().as_ref(), treasury_mint.key().as_ref(), app_idx.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Box<Account<'info, BoyncAuction>>,

    #[account(
        init,
        payer = signer,
        seeds = [TREASURY_SEED, signer.key().as_ref(), treasury_mint.key().as_ref(), app_idx.to_le_bytes().as_ref()],
        bump,
        token::mint=treasury_mint,
        token::authority=state
    )]
    /// Account holding token being auctioned.
    pub treasury: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        seeds = [WALLET_SEED, signer.key().as_ref(), collector_mint.key().as_ref(), app_idx.to_le_bytes().as_ref()],
        bump,
        token::mint=collector_mint,
        token::authority=state
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
        constraint=signer_withdraw_wallet.owner == signer.key(),
        constraint=signer_withdraw_wallet.mint == treasury_mint.key()
    )]
    signer_withdraw_wallet: Account<'info, TokenAccount>,

    // Application level accounts
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateAuctionState<'info> {
    #[account(mut, has_one = authority @ AuctionError::InvalidAuthority)]
    pub auction: Account<'info, BoyncAuction>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAuction<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.end_auction_at.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Account<'info, BoyncAuction>,

    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.collector_mint.key().as_ref(), state.end_auction_at.to_le_bytes().as_ref()],
        bump
    )]
    /// Account which holds tokens bidded by biders
    pub bidders_chest: Account<'info, TokenAccount>,

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

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
        constraint=state.last_bidder == winner.key() @ AuctionError::YouAreNotTheWinner
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
        mut,
        constraint=winner_withdraw_wallet.owner == winner.key(),
        constraint=winner_withdraw_wallet.mint == treasury_mint.key(),
    )]
    winner_withdraw_wallet: Account<'info, TokenAccount>,

    // Application level accounts
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[account]
pub struct BoyncAuction {
    end_auction_at: i64, // 1 + 64
    authority: Pubkey,
    treasury_mint: Pubkey,
    collector_mint: Pubkey,
    treasury: Pubkey,
    bidders_chest: Pubkey,
    tokens_spent: u64,
    state: AuctionState, // 1 + 32

    last_bidder: Pubkey
}

impl BoyncAuction {
    // pub const AUCTION_SIZE: usize = ( 1 + 32 ) + ( 1 + 32 ) + ( 1 + 64 );
    pub const AUCTION_SIZE: usize = size_of::<BoyncAuction>();

    pub fn ended(&self, now: i64) -> Result<bool> {
        Ok(now > self.end_auction_at)
    }

}

/**
 * Boync Auction State
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

/**
 * Errors
 */
#[error_code]
pub enum AuctionError {
    /// Invalid transition, auction state may only transition: Created -> Started -> Stopped
    #[msg("Invalid auction state transition.")]
    AuctionTransitionInvalid,
    /// Auction is not currently running.
    #[msg("Auction is not currently running.")]
    InvalidState,
    #[msg("Auction expired.")]
    AuctionExpired,
    #[msg("Auction ongoing")]
    AuctionOngoing,
    #[msg("You Are not the winner")]
    YouAreNotTheWinner,
    /// Bid is too small.
    #[msg("Bid is too small.")]
    BidTooSmall,
    #[msg("You are not the authority for this auction!")]
    InvalidAuthority,
}
