pub mod utils;
pub mod errors;
pub mod pda;
pub mod constants;
pub mod events;
pub mod context;
pub mod account;

use anchor_lang::{
    prelude::*,
    solana_program::{ clock::Clock, entrypoint::ProgramResult },
    system_program,
    { AnchorDeserialize, AnchorSerialize },
};

use anchor_spl::token::Transfer;

use context::*;
use events::*;
use constants::*;
use errors::*;
use utils::{
    BoyncTokenTransfer,
    token_transfer,
    assert_auction_active,
    assert_auction_over,
    process_time_extension,
};


declare_id!("DykznMHLnGMNLhDPPPu8BPCSeFb9sWkdiB1731SqPQCN");

#[program]
pub mod boync_anchor_program {
    use super::*;

    pub fn initialize(
        ctx: Context<InitializeAuction2>,
        app_idx: i64,
        state_bump: u8,
        fp: u64,
        start_at: i64,
        end_at: i64
    ) -> Result<()> {
        msg!("[BoyncProgram] Initializing new Boync Auction State");

        // let clock = Clock::get()?;
        let auction_state = &mut ctx.accounts.state;

        auction_state.id = app_idx; // App index is UnixTimestamp
        auction_state.start_auction_at = start_at;
        auction_state.end_auction_at = end_at;
        auction_state.starting_price = (0.05 * (fp as f64)) as u64;
        auction_state.next_bid = auction_state.starting_price.clone();
        auction_state.authority = ctx.accounts.signer.key().clone();
        auction_state.treasury_mint = ctx.accounts.treasury_mint.key().clone();
        auction_state.treasury = ctx.accounts.treasury.key().clone();
        auction_state.bidders_chest = ctx.accounts.bidders_chest.key().clone();
        auction_state.bump = state_bump;

        msg!("Initialized new Boync Auction State with treasury: {}", auction_state.treasury.key());

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
            &[auction_state.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        msg!("[BoyncDebug] Created Seeds");

        // Token program instruction to send SPL token.
        let transfer_instruction = Transfer {
            from: ctx.accounts.signer_token_account.to_account_info(),
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

        auction_state.claimed = 0;
        // if auction_state.ended(clock.unix_timestamp)? {
        //     auction_state.state = AuctionState::Ended;
        // } else {
        //     auction_state.state = AuctionState::Created;
        // }

        msg!("[BoyncDebug] Done!");

        emit!(BoyncInitializeEvent {
            auction_pubkey: auction_state.key(),
            label: "initialize".to_string(),
        });

        Ok(())
    }

    pub fn initialize_auction_2(
        ctx: Context<InitializeAuction2>,
        app_idx: i64,
        state_bump: u8,
        fp: u64,
        start_at: i64,
        end_at: i64
    ) -> ProgramResult {
        msg!("[BoyncProgram] Initializing new Boync Auction State");

        // let clock = Clock::get()?;
        let auction_state = &mut ctx.accounts.state;

        auction_state.id = app_idx; // App index is UnixTimestamp
        auction_state.start_auction_at = start_at;
        auction_state.end_auction_at = end_at;
        auction_state.starting_price = (0.05 * (fp as f64)) as u64;
        auction_state.next_bid = auction_state.starting_price.clone();
        auction_state.authority = ctx.accounts.signer.key().clone();
        auction_state.treasury_mint = ctx.accounts.treasury_mint.key().clone();
        auction_state.treasury = ctx.accounts.treasury.key().clone();
        auction_state.bidders_chest = ctx.accounts.bidders_chest.key().clone();
        auction_state.bump = state_bump;

        msg!("[BoyncDebug] Initialized with treasury: {}", auction_state.treasury.key());

        let auction_state_clone = auction_state.to_account_info();

        let transfer_accounts = BoyncTokenTransfer {
            auction_state: auction_state_clone.to_account_info(),
            token: ctx.accounts.signer_token_account.to_account_info(),
            token_owner: ctx.accounts.signer.to_account_info(),
            destination: ctx.accounts.treasury.to_account_info(),
            destination_owner: auction_state_clone,
            mint: ctx.accounts.treasury_mint.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            owner_token_record: ctx.accounts.owner_token_record.to_account_info(),
            destination_token_record: ctx.accounts.destination_token_record.to_account_info(),
            authority: ctx.accounts.signer.to_account_info(),
            payer: ctx.accounts.signer.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            sysvar_instructions: ctx.accounts.sysvar_instructions.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_ata_program: ctx.accounts.associated_token_program.to_account_info(),
            auth_rules_program: ctx.accounts.auth_rules_token_program.to_account_info(),
            auth_rules: ctx.accounts.auth_rules.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            transfer_accounts
        );

        token_transfer(cpi_ctx, &auction_state.id, 1)?;

        msg!("[BoyncDebug] Token transfered to treasury: {}", auction_state.treasury.key());

        emit!(BoyncInitializeEvent {
            auction_pubkey: auction_state.key(),
            label: "initialize".to_string(),
        });

        msg!("[BoyncDebug] Initialize event sent");

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
            from:       ctx.accounts.signer_ata.to_account_info(),
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

    // pub fn start(ctx: Context<UpdateAuctionState2>) -> Result<()> {
    //     let auction = &mut ctx.accounts.auction;

    //     // // Can't start an Auction that has has already yet started.
    //     require!(auction.state == AuctionState::Created,
    //         AuctionError::InvalidState);

    //     auction.state = auction.state.start()?;

    //     emit!(BoyncStartEvent {
    //         auction_pubkey: auction.key(),
    //         updated_auction_state: auction.state,
    //         label:          "start".to_string()
    //     });

    //     Ok(())
    // }

    pub fn end(ctx: Context<EndAuction>, bidder_chest_bump: u8) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        let bidders_chest = &mut ctx.accounts.bidders_chest;
        let treasury = &mut ctx.accounts.treasury;
        let clock = Clock::get()?;

        // Can't end an Auction that is already ended.
        // require!(auction_state.state == AuctionState::Started,
        //     AuctionError::InvalidState);
        assert_auction_active(&auction_state)?;

        // auction_state.state = auction_state.state.end()?;
        auction_state.end_auction_at = clock.unix_timestamp * MS_IN_SEC;

        /* Build bidders_chest PDA to sign transaction */
        // let bump = *ctx.bumps.get("wallet").unwrap();
        let auction_auth = auction_state.authority.clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            WALLET_SEED,
            auction_auth.as_ref(),
            app_idx_bytes.as_ref(),
            &[bidder_chest_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let total_lamports: u64 = bidders_chest.lamports();

        /* transfer 75% of bidders_chest to authority account */
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: bidders_chest.to_account_info(),
                    to: ctx.accounts.authority.to_account_info(),
                },
                signer_seeds
            ),
            ((total_lamports as f64) * 0.75) as u64
        )?;

        /* transfer rest (25%) of bidders_chest to treasury account */
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: bidders_chest.to_account_info(),
                    to: treasury.to_account_info(),
                },
                signer_seeds
            ),
            bidders_chest.lamports().clone()
        )?;

        emit!(BoyncEndEvent {
            auction_pubkey: auction_state.key(),
            updated_end_timestamp: auction_state.end_auction_at,
            label: "end".to_string(),
        });

        Ok(())
    }

    pub fn update_auction_2(ctx: Context<UpdateAuction2>, ts: i64) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        // let clock = Clock::get()?;

        // Can't bid on an Auction that is expired.
        // require!(!auction_state.ended(clock.unix_timestamp)?,
        //     AuctionError::AuctionExpired);
        assert_auction_active(&auction_state)?;

        // Can't bid on an Auction that is not started.
        // require!(auction_state.state == AuctionState::Started,
        //     AuctionError::InvalidState);

        // Can't bid on an Auction that was already claimed.
        require!(auction_state.claimed == 0, AuctionError::AuctionClaimed);

        // Can't bid on an Auction you're the authority of.
        require!(
            auction_state.authority.key() != ctx.accounts.bidder.key(),
            AuctionError::AuctionAuthorityBid
        );

        // Can't bid on an Auction if you're already Last Bidder
        require!(
            auction_state.last_bidder.key() != ctx.accounts.bidder.key(),
            AuctionError::AuctionAlreadyLastBidder
        );

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
            &[auction_state.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let transfer_instruction = anchor_lang::system_program::Transfer {
            from: ctx.accounts.bidder.to_account_info(),
            to: ctx.accounts.bidders_chest.to_account_info(),
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
        // auction_state.end_auction_at += 60 * MS_IN_SEC; // Add 60 seconds to countdown
        process_time_extension(auction_state)?;
        auction_state.next_bid = (1.05 * (auction_state.next_bid as f64)) as u64;

        emit!(BoyncBidEvent {
            auction_pubkey: auction_state.key(),
            bidder_pubkey: auction_state.last_bidder.clone(),
            updated_bid_value: auction_state.next_bid.clone(),
            updated_end_timestamp: auction_state.end_auction_at,
            label: "bid".to_string(),
            ts: ts,
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
            from:       ctx.accounts.signer_ata.to_account_info(),
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
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let auction_state = &mut ctx.accounts.state;
        // let clock = Clock::get()?;

        // Can't withdraw on an Auction that is ongoing.
        // require!(auction_state.ended(clock.unix_timestamp)?,
        //     AuctionError::AuctionOngoing);
        assert_auction_over(&auction_state)?;

        // Can't claim on an Auction that was already claimed.
        require!(auction_state.claimed == 0, AuctionError::AuctionClaimed);

        // Can't claim an Auction that is not in ended state.
        // require!(auction_state.state == AuctionState::Ended,
        //     AuctionError::InvalidState);

        // Only Winner can claim rewards.
        // FIX: *MAYBE REDUNDAND because of
        // #[account(constraint=state.last_bidder == winner.key())

        // If last_bidder is system program Id => no bids has been placed => claimable only by authority
        if auction_state.last_bidder.key() == system_program::ID.key() {
            require!(
                auction_state.authority.key() == ctx.accounts.winner.key(),
                AuctionError::YouAreNotTheAuthority
            );
        } else {
            require!(
                auction_state.last_bidder.key() == ctx.accounts.winner.key(),
                AuctionError::YouAreNotTheWinner
            );
        }

        let treasury_mint = ctx.accounts.treasury_mint.key().clone();
        let auction_auth = auction_state.authority.clone();
        let app_idx_bytes = auction_state.id.to_le_bytes();
        let seeds = &[
            AUCTION_SEED,
            auction_auth.as_ref(),
            treasury_mint.as_ref(),
            app_idx_bytes.as_ref(),
            &[auction_state.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let auction_state_clone = auction_state.to_account_info();

        let transfer_accounts = BoyncTokenTransfer {
            auction_state: auction_state_clone.to_account_info(),
            token: ctx.accounts.treasury.to_account_info(),
            token_owner: auction_state_clone.to_account_info(),
            destination: ctx.accounts.winner_token_account.to_account_info(),
            destination_owner: ctx.accounts.winner.to_account_info(),
            mint: ctx.accounts.treasury_mint.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            edition: ctx.accounts.edition.to_account_info(),
            owner_token_record: ctx.accounts.owner_token_record.to_account_info(),
            destination_token_record: ctx.accounts.destination_token_record.to_account_info(),
            authority: auction_state_clone.to_account_info(),
            payer: ctx.accounts.winner.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            sysvar_instructions: ctx.accounts.sysvar_instructions.to_account_info(),
            spl_token_program: ctx.accounts.token_program.to_account_info(),
            spl_ata_program: ctx.accounts.associated_token_program.to_account_info(),
            auth_rules_program: ctx.accounts.auth_rules_token_program.to_account_info(),
            auth_rules: ctx.accounts.auth_rules.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_metadata_program.to_account_info(),
            transfer_accounts,
            signer_seeds
        );

        token_transfer(cpi_ctx, &auction_state.id, 1)?;

        msg!("[BoyncDebug][claim_rewards] treasury transfered token.");

        // Use the `reload()` function on an account to reload it's state. Since we performed the
        // transfer, we are expecting the `amount` field to have changed.
        // TODO: *PROPERLY CLOSE TREASURY ACCOUNT*

        auction_state.claimed = 1;

        emit!(BoyncClaimEvent {
            auction_pubkey: auction_state.key(),
            claimed: auction_state.claimed,
            label: "claim".to_string(),
        });

        msg!("[BoyncDebug][claim_rewards] BoyncClaimEvent sent.");

        Ok(())
    }
}
