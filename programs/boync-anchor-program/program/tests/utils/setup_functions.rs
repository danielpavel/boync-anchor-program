use super::digital_asset::*;
use super::constants::*;
use anchor_lang::*;
use solana_program_test::*;

use anchor_client::solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
    transaction::Transaction,
};

use boync_anchor_program::{
    accounts::{
        InitializeAuction2 as InitializeAuction2Accounts, UpdateAuction2 as UpdateAuction2Accounts,
        ClaimRewards as ClaimRewardsAccounts,
    },
    instruction::{
        InitializeAuction2 as InitializeAuction2Data, UpdateAuction2 as UpdateAuction2Data,
        ClaimRewards as ClaimRewardsData
    },
    pda::{
        find_boync_auction_address, find_boync_bidder_state_address,
        find_boync_bidders_chest_address, find_boync_treasury_address,
    },
    account::{BoyncAuction2, BoyncUserBid},
};
use mpl_token_metadata::pda::{find_master_edition_account, find_token_record_account};


pub async fn boync_get_auction_data(
    context: &mut ProgramTestContext,
    auction: &Pubkey,
) -> BoyncAuction2 {
    let auction_house_acc = context
        .banks_client
        .get_account(*auction)
        .await
        .expect("account not found")
        .expect("account empty");

    BoyncAuction2::try_deserialize(&mut auction_house_acc.data.as_ref()).unwrap()
}

pub async fn boync_get_bidder_state_data(
    context: &mut ProgramTestContext,
    bidder: &Pubkey,
) -> BoyncUserBid {
    let bidder_acc = context
        .banks_client
        .get_account(*bidder)
        .await
        .expect("account not found")
        .expect("account empty");

    BoyncUserBid::try_deserialize(&mut bidder_acc.data.as_ref()).unwrap()
}

pub fn boync_update_auction_claim(
    context: &mut ProgramTestContext,
    digital_asset: &DigitalAsset,
    auction: &Pubkey,
    treasury_token: &Pubkey,
    winner_token: &Pubkey,
    winner: &Keypair,
) -> (ClaimRewardsAccounts, Transaction) {

    let mint = &digital_asset.mint.pubkey();

    let edition = if let Some(edition) = digital_asset.master_edition {
        edition
    } else {
        let (edition, _) = find_master_edition_account(mint);
        edition
    };

    let (owner_token_record, _) = find_token_record_account(mint, &treasury_token);
    let (destination_token_record, _bump) = find_token_record_account(mint, &winner_token);

    let accounts = ClaimRewardsAccounts {
        state: *auction,
        treasury: *treasury_token,
        treasury_mint: *mint,
        winner: winner.pubkey(),
        winner_token_account: *winner_token,
        metadata: digital_asset.metadata,
        edition,
        owner_token_record,
        destination_token_record,
        auth_rules: mpl_token_auth_rules::id(),
        system_program: system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        auth_rules_token_program: mpl_token_auth_rules::id(),
        token_metadata_program: mpl_token_metadata::id(),
        rent: sysvar::rent::id(),
        sysvar_instructions: sysvar::instructions::id(),
    };
    let accounts_meta = accounts.to_account_metas(None);

    let data = ClaimRewardsData{}.data();

    let instruction = Instruction {
        program_id: boync_anchor_program::id(),
        data,
        accounts: accounts_meta,
    };

    (
        accounts,
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&winner.pubkey()),
            &[winner],
            context.last_blockhash,
        ),
    )
}

pub fn boync_update_auction_bid(
    context: &mut ProgramTestContext,
    auction: &Pubkey,
    bidders_chest: &Pubkey,
    bidder: &Keypair,
    ts: &i64,
) -> (UpdateAuction2Accounts, Transaction) {
    let (bidder_state, _) = find_boync_bidder_state_address(auction, &bidder.pubkey(), ts);
    let accounts = UpdateAuction2Accounts {
        state: *auction,
        bidders_chest: *bidders_chest,
        bidder_state,
        bidder: bidder.pubkey(),
        system_program: system_program::id(),
        rent: sysvar::rent::id(),
    };
    let accounts_meta = accounts.to_account_metas(None);

    let data = UpdateAuction2Data { ts: *ts }.data();

    let instruction = Instruction {
        program_id: boync_anchor_program::id(),
        data,
        accounts: accounts_meta,
    };

    (
        accounts,
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&bidder.pubkey()),
            &[bidder],
            context.last_blockhash,
        ),
    )
}

pub fn boync_initialize_2(
    context: &mut ProgramTestContext,
    creator: &Keypair,
    digital_asset: &DigitalAsset,
    auction: &Pubkey,
    auction_bump: u8,
    treasury_token: &Pubkey,
    bidders_chest: &Pubkey,
    timestamp: &i64,
    creator_token: &Pubkey,
    auction_duration: Option<&i64>
) -> (InitializeAuction2Accounts, Transaction) {
    // let token = &digital_asset.token.pubkey();
    let mint = &digital_asset.mint.pubkey();

    let edition = if let Some(edition) = digital_asset.master_edition {
        edition
    } else {
        let (edition, _) = find_master_edition_account(mint);
        edition
    };

    let (owner_token_record, _) = find_token_record_account(mint, &creator_token);
    let (destination_token_record, _bump) = find_token_record_account(mint, &treasury_token);

    let accounts = InitializeAuction2Accounts {
        state: *auction,
        treasury: *treasury_token,
        bidders_chest: *bidders_chest,
        signer: creator.pubkey(),
        treasury_mint: *mint,
        metadata: digital_asset.metadata,
        edition,
        owner_token_record,
        destination_token_record,
        auth_rules: mpl_token_auth_rules::id(), // !!!NOT USED
        signer_token_account: *creator_token,
        system_program: system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        auth_rules_token_program: mpl_token_auth_rules::id(),
        token_metadata_program: mpl_token_metadata::id(),
        rent: sysvar::rent::id(),
        sysvar_instructions: sysvar::instructions::id(),
    };
    let accounts_meta = accounts.to_account_metas(None);

    let duration = if let Some(duration) = auction_duration {
        duration
    } else {
        &THIRTY_MINUTES_IN_MSEC
    };

    let data = InitializeAuction2Data {
        app_idx: *timestamp,
        state_bump: auction_bump,
        fp: 3 * ONE_SOL,
        start_at: *timestamp,
        end_at: *(timestamp) + duration,
    }
    .data();

    let instruction = Instruction {
        program_id: boync_anchor_program::id(),
        data,
        accounts: accounts_meta,
    };

    (
        accounts,
        Transaction::new_signed_with_payer(
            &[instruction],
            Some(&creator.pubkey()),
            &[creator],
            context.last_blockhash,
        ),
    )
}

pub fn find_boync_auction_pdas(
    authority: &Pubkey,
    mint: &Pubkey,
    current_timestamp: &i64,
) -> ((Pubkey, u8), Pubkey, Pubkey) {
    let (auction, auction_bump) = find_boync_auction_address(authority, mint, current_timestamp);
    let (treasury, _) = find_boync_treasury_address(authority, mint, current_timestamp);
    let (bidders_chest, _) = find_boync_bidders_chest_address(authority, current_timestamp);

    ((auction, auction_bump), treasury, bidders_chest)
}
