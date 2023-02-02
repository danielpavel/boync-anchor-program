use anchor_lang::{
    prelude::*,
    solana_program::clock::Clock,
    system_program,
    { AnchorDeserialize, AnchorSerialize },
};

use anchor_spl::{
    token:: { TokenAccount, Token, Mint, Transfer },
    associated_token::AssociatedToken
};

use {
    std::mem::size_of
};

pub const TREASURY_SEED: &[u8] = b"treasury";
pub const WALLET_SEED: &[u8] = b"wallet";
pub const AUCTION_SEED: &[u8] = b"auction";
pub const BIDDER_SEED: &[u8] = b"bidder";
pub const MS_IN_SEC: i64 = 1000;

declare_id!("Ed1gXfSgsu8Cj8ZeKc8hPKXog4dNuBaYagFZPFrVDsmv");

#[program]
pub mod boync_anchor_program {

    use super::*;

    pub fn initialize(ctx: Context<InitializeAuction2>, app_idx: i64, state_bump: u8, fp: u64) -> Result<()> {
        msg!("[BoyncProgram] Initializing new Boync Auction State");

        let clock = Clock::get()?;
        let auction_state = &mut ctx.accounts.state;

        auction_state.id = app_idx;             // App index is UnixTimestamp
        auction_state.end_auction_at = app_idx;
        auction_state.starting_price = (0.05 * fp as f64) as u64;
        auction_state.next_bid = auction_state.starting_price.clone();
        auction_state.authority = ctx.accounts.signer.key().clone();
        auction_state.treasury_mint = ctx.accounts.treasury_mint.key().clone();
        auction_state.treasury = ctx.accounts.treasury.key().clone();
        auction_state.bidders_chest = ctx.accounts.bidders_chest.key().clone();
        auction_state.bump = state_bump;

        msg!("Initialized new Boync Auction State for token: {}",
            auction_state.treasury.key());

        // FIX: [BA-Program-FnWbMVHB]: Fetching bump within anchor context
        //      does not work.
        // let _bump = *ctx.bumps.get("auction").unwrap();
        // let bump = state_bump;
        let treasury_mint = ctx.accounts.treasury_mint.key().clone();
        let app_idx_bytes = app_idx.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            ctx.accounts.signer.key.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump]
        ];
        let signer_seeds = &[&seeds[..]];

        msg!("[BoyncDebug] Created Seeds");

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from:       ctx.accounts.signer_withdraw_wallet.to_account_info(),
            to:         ctx.accounts.treasury.to_account_info(),
            authority:  ctx.accounts.signer.to_account_info(),
        };

        msg!("[BoyncDebug] Created Transfer");

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        msg!("[BoyncDebug] Created CPI context");

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        auction_state.claimed = 0;
        if auction_state.ended(clock.unix_timestamp)? {
            auction_state.state = AuctionState::Ended;
        } else {
            auction_state.state = AuctionState::Created;
        }

        msg!("[BoyncDebug] Done!");

        emit!(BoyncInitializeEvent {
            auction_pubkey: auction_state.key(),
            label:          "initialize".to_string()
        });

       Ok(())
    }

    /* Disabled as part of [BA-Program-5uJBi4jN][MVP] Remove BOYNC token GATE
    pub fn initialize(ctx: Context<InitializeAuction>, app_idx: i64, state_bump: u8) -> Result<()> {
        msg!("[BoyncProgram] Initializing new Boync Auction State");

        let clock = Clock::get()?;
        let auction_state = &mut ctx.accounts.state;

        auction_state.id = app_idx;             // App index is UnixTimestamp
        auction_state.end_auction_at = app_idx;
        auction_state.authority = ctx.accounts.signer.key().clone();
        auction_state.treasury_mint = ctx.accounts.treasury_mint.key().clone();
        auction_state.treasury = ctx.accounts.treasury.key().clone();
        auction_state.collector_mint = ctx.accounts.collector_mint.key().clone();
        auction_state.bidders_chest = ctx.accounts.bidders_chest.key().clone();
        auction_state.bump = state_bump;

        msg!("Initialized new Boync Auction State for token: {}",
            auction_state.treasury.key());

        // FIX: [BA-Program-FnWbMVHB]: Fetching bump within anchor context
        //      does not work.
        // let _bump = *ctx.bumps.get("auction").unwrap();
        // let bump = state_bump;
        let treasury_mint = ctx.accounts.treasury_mint.key().clone();
        let app_idx_bytes = app_idx.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            ctx.accounts.signer.key.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump]
        ];
        let signer_seeds = &[&seeds[..]];

        msg!("[BoyncDebug] Created Seeds");

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from:       ctx.accounts.signer_withdraw_wallet.to_account_info(),
            to:         ctx.accounts.treasury.to_account_info(),
            authority:  ctx.accounts.signer.to_account_info(),
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
    */


    pub fn start(ctx: Context<UpdateAuctionState2>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;

        // // Can't start an Auction that has has already yet started.
        require!(auction.state == AuctionState::Created,
            AuctionError::InvalidState);

        auction.state = auction.state.start()?;

        Ok(())
    }

    pub fn end(ctx: Context<EndAuction>, bidder_chest_bump: u8) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        let bidders_chest = &mut ctx.accounts.bidders_chest;
        let treasury = &mut ctx.accounts.treasury;
        let clock = Clock::get()?;

        // Can't end an Auction that is already ended.
        require!(auction_state.state == AuctionState::Started,
            AuctionError::InvalidState);

        auction_state.state = auction_state.state.end()?;
        auction_state.end_auction_at = clock.unix_timestamp * MS_IN_SEC;

        /* Build bidders_chest PDA to sign transaction */
        // let bump = *ctx.bumps.get("wallet").unwrap();
        let auction_auth = auction_state.authority.clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            WALLET_SEED,
            auction_auth.as_ref(),
            app_idx_bytes.as_ref(),
            &[bidder_chest_bump]
        ];
        let signer_seeds = &[&seeds[..]];

        let total_lamports: u64 = bidders_chest.lamports();

        /* transfer 75% of bidders_chest to authority account */
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer { 
                    from:       bidders_chest.to_account_info(),
                    to:         ctx.accounts.authority.to_account_info()
                },
                signer_seeds
            ),
            (total_lamports as f64 * 0.75) as u64
        )?;

        /* transfer rest (25%) of bidders_chest to treasury account */
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer { 
                    from:       bidders_chest.to_account_info(),
                    to:         treasury.to_account_info()
                },
                signer_seeds
            ),
            bidders_chest.lamports().clone()
        )?;

        emit!(BoyncEndEvent {
            auction_pubkey:         auction_state.key(),
            updated_end_timestamp:  auction_state.end_auction_at,
            label:                  "end".to_string()
        });

        Ok(())
    }

    pub fn bid(ctx: Context<UpdateAuction2>, ts: i64) -> Result<()> {

        let auction_state = &mut ctx.accounts.state;
        let clock = Clock::get()?;

        // Can't bid on an Auction that is expired.
        require!(!auction_state.ended(clock.unix_timestamp)?,
            AuctionError::AuctionExpired);
    
        // Can't bid on an Auction that is not started.
        require!(auction_state.state == AuctionState::Started,
            AuctionError::InvalidState);

        // Can't bid on an Auction that was already claimed.
        require!(auction_state.claimed == 0,
            AuctionError::AuctionClaimed);

        // Can't bid on an Auction you're the authority of.
        require!(auction_state.authority.key() != ctx.accounts.bidder.key(),
            AuctionError::AuctionAuthorityBid);

        // Just transfer SPL Token to bidders_chest
        // let _bump = *ctx.bumps.get("auction").unwrap();
        // let bump = auction_state.bump;
        let auction_auth = auction_state.authority.clone();
        let treasury_mint = auction_state.treasury_mint.key().clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            auction_auth.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump]
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_instruction = anchor_lang::system_program::Transfer { 
            from:       ctx.accounts.bidder.to_account_info(),
            to:         ctx.accounts.bidders_chest.to_account_info()
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        anchor_lang::system_program::transfer(cpi_ctx, auction_state.next_bid.clone())?;

        /* Store bid state */
        let bidder_state = &mut ctx.accounts.bidder_state;
        bidder_state.auction = auction_state.key();
        bidder_state.bidder = ctx.accounts.bidder.key.clone();
        bidder_state.bid_value = auction_state.next_bid;
        bidder_state.ts = ts;

        auction_state.last_bidder = ctx.accounts.bidder.key.clone();
        auction_state.end_auction_at += 60 * MS_IN_SEC; // Add 60 seconds to countdown
        auction_state.next_bid = (1.05 * auction_state.next_bid as f64) as u64;

        emit!(BoyncBidEvent {
            auction_pubkey:         auction_state.key(),
            bidder_pubkey:          auction_state.last_bidder.clone(),
            updated_end_timestamp:  auction_state.end_auction_at,
            label:                  "bid".to_string(),
            ts:                     ts
        });

        Ok(())
    }

    /// Bid
    /* 
    pub fn bid(ctx: Context<UpdateAuction>, amount: u64) -> Result<()> {
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
        // let bump = auction_state.bump;
        let auction_auth = auction_state.authority.clone();
        let treasury_mint = auction_state.treasury_mint.key().clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            auction_auth.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump]
        ];
        let signer_seeds = &[&seeds[..]];

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from:       ctx.accounts.bidder_withdraw_wallet.to_account_info(),
            to:         ctx.accounts.bidders_chest.to_account_info(),
            authority:  ctx.accounts.bidder.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            signer_seeds
        );

        anchor_spl::token::transfer(cpi_ctx, amount)?;

        auction_state.last_bidder = ctx.accounts.bidder.key.clone();
        auction_state.end_auction_at += 60 * MS_IN_SEC; // Add 60 seconds to countdown
        auction_state.tokens_spent += 1;

        Ok(())
    }
    */

    pub fn claim(ctx: Context<ClaimRewards>) -> Result<()> {
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

        // If last_bidder is system program Id => no bids has been placed => claimable only by authority
        if auction_state.last_bidder.key() == system_program::ID.key() {
            require!(auction_state.authority.key() == ctx.accounts.winner.key(),
                AuctionError::YouAreNotTheAuthority);
        } else {
            require!(auction_state.last_bidder.key() == ctx.accounts.winner.key(),
                AuctionError::YouAreNotTheWinner);
        }

        // let _bump = *ctx.bumps.get("auction").unwrap();
        let treasury_mint = ctx.accounts.treasury_mint.key().clone();
        let auction_auth = auction_state.authority.clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            auction_auth.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump]
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

        auction_state.claimed = 1;

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
#[instruction(app_idx: i64, state_bump: u8, fp: u64)]
pub struct InitializeAuction2<'info> {
    /// State of our auction program (up to you)
    #[account(
        init,
        payer = signer,
        space = 8 + BoyncAuction::AUCTION_SIZE,
        seeds =  [AUCTION_SEED, signer.key().as_ref(), treasury_mint.key().as_ref(), app_idx.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Box<Account<'info, BoyncAuction2>>,

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
    pub treasury_mint: Account<'info, Mint>,

    /// Payer's SPL Token account wallet 
    /// (The wallet who will send the NFT being auctioned)
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
pub struct UpdateAuctionState2<'info> {
    #[account(mut, has_one = authority @ AuctionError::InvalidAuthority)]
    pub auction: Account<'info, BoyncAuction2>,
    pub authority: Signer<'info>,
}

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

    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    /// CHECK: only used as a signing PDA
    pub bidders_chest: AccountInfo<'info>,
 
    #[account(
        init_if_needed,
        payer = bidder,
        space = 8 + BoyncUserBid::ACCOUNT_SIZE,
        seeds = [BIDDER_SEED, state.key().as_ref(), bidder.key().as_ref(), ts.to_le_bytes().as_ref()],
        bump 
    )]
    pub bidder_state: Account<'info, BoyncUserBid>,

    // Users and accounts in the system
    #[account(mut)]
    pub bidder: Signer<'info>,

    // Application level accounts
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct EndAuction<'info> {
    #[account(mut, has_one = authority @ AuctionError::InvalidAuthority)]
    pub state: Account<'info, BoyncAuction2>,

    #[account(
        mut,
        seeds = [WALLET_SEED, state.authority.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    /// CHECK: only used as a signing PDA
    pub bidders_chest: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Treasury account will be one random account that only receives SOL.
    pub treasury: AccountInfo<'info>,

    // Application level accounts
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(
        mut,
        seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
        bump
    )]
    pub state: Account<'info, BoyncAuction2>,

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
        init_if_needed,
        payer = winner,
        associated_token::mint = treasury_mint,
        associated_token::authority = winner,
        constraint=winner_withdraw_wallet.owner == winner.key(),
        constraint=winner_withdraw_wallet.mint == treasury_mint.key(),
    )]
    winner_withdraw_wallet: Account<'info, TokenAccount>,

    // Application level accounts
    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
} 

// #[derive(Accounts)]
// pub struct ClaimRewards<'info> {
//     #[account(
//         mut,
//         seeds = [AUCTION_SEED, state.authority.key().as_ref(), state.treasury_mint.key().as_ref(), state.id.to_le_bytes().as_ref()],
//         bump
//     )]
//     pub state: Account<'info, BoyncAuction>,

//     #[account(mut)]
//     /// Account which holds auctioned token(s).
//     pub treasury: Account<'info, TokenAccount>,
//     /// Mint for SPL Token stored in treasury.
//     pub treasury_mint: Account<'info, Mint>,

//     // Users and accounts in the system
//     #[account(mut)]
//     pub winner: Signer<'info>,

//     /// Winner's SPL Token account wallet 
//     /// (The wallet who will receive the auctioned token(s))
//     #[account(
//         init,
//         payer = winner,
//         associated_token::mint = treasury_mint,
//         associated_token::authority = winner,
//         constraint=winner_withdraw_wallet.owner == winner.key(),
//         constraint=winner_withdraw_wallet.mint == treasury_mint.key(),
//     )]
//     winner_withdraw_wallet: Account<'info, TokenAccount>,

//     // Application level accounts
//     associated_token_program: Program<'info, AssociatedToken>,
//     token_program: Program<'info, Token>,
//     system_program: Program<'info, System>,
//     rent: Sysvar<'info, Rent>,
// }

#[account]
pub struct BoyncAuction {
    id:             i64,
    end_auction_at: i64, // 1 + 64
    authority:      Pubkey,
    treasury_mint:  Pubkey,
    collector_mint: Pubkey,
    treasury:       Pubkey,
    bidders_chest:  Pubkey,
    tokens_spent:   u64,
    state:          AuctionState, // 1 + 32
    last_bidder:    Pubkey,
    bump:           u8
}

/**
 * V2 
 * Users use SOL to bid. 
 * [BA-Program-5uJBi4jN][MVP] Remove BOYNC token GATE
 */

#[account]
pub struct BoyncAuction2 {
    id:             i64,
    end_auction_at: i64, // 1 + 64
    authority:      Pubkey,
    treasury_mint:  Pubkey,
    treasury:       Pubkey,
    bidders_chest:  Pubkey,
    starting_price: u64,
    next_bid:       u64,
    claimed:        u8,
    state:          AuctionState, // 1 + 32
    last_bidder:    Pubkey,
    bump:           u8
}

#[account]
pub struct BoyncUserBid {
    auction:        Pubkey,
    bidder:         Pubkey,
    bid_value:      u64,
    ts:             i64, // 1 + 64
}

impl BoyncUserBid {
    pub const ACCOUNT_SIZE: usize = size_of::<BoyncUserBid>();
}

impl BoyncAuction {
    // pub const AUCTION_SIZE: usize = ( 1 + 32 ) + ( 1 + 32 ) + ( 1 + 64 );
    pub const AUCTION_SIZE: usize = size_of::<BoyncAuction>();

    pub fn ended(&self, now: i64) -> Result<bool> {
        Ok((now * MS_IN_SEC) > self.end_auction_at)
    }
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
        Ok((now * MS_IN_SEC) > self.end_auction_at)
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
 * Events
 */
#[event]
pub struct BoyncBidEvent {
    pub auction_pubkey: Pubkey,
    pub bidder_pubkey: Pubkey,
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
    #[msg("Auction has already been claimed!")]
    AuctionClaimed,
    #[msg("You can't bid on an auction you created!")]
    AuctionAuthorityBid,
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
