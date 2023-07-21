use spl_associated_token_account::get_associated_token_address;
use solana_program_test::*;
use super::digital_asset::*;

use anchor_client::solana_sdk::{
    instruction::Instruction,
    transaction::Transaction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
};

use anchor_lang::*;

const ONE_SOL: u64 = 1_000_000_000;
const ONE_MINUTE_IN_MSEC: i64 = 60 * 60 * 1000;

pub fn boync_initialize_2(
    context: &mut ProgramTestContext,
    creator: &Keypair,
    digital_asset: &DigitalAsset,
    auction: &Pubkey,
    auction_bump: u8,
    treasury: &Pubkey,
    bidders_chest: &Pubkey,
    timestamp: &i64,
) -> (
    boync_anchor_program::accounts::InitializeAuction2,
    Transaction,
) {
    // We have to find:
    // 1. Auction State Account Address ("auction", creator, mint, now_timestamp)
    // 2. Treasury Account Address  ("treasury", creator, mint, now_timestamp)
    // 3. Bidders Chest Account Address ("wallet", creator, now_timestamp)
    let creator_token_account = get_associated_token_address(&creator.pubkey(), &digital_asset.mint.pubkey());

    let accounts = boync_anchor_program::accounts::InitializeAuction2 {
        state: *auction,
        treasury: *treasury,
        bidders_chest: *bidders_chest,
        signer: creator.pubkey(),
        treasury_mint: digital_asset.mint.pubkey(),
        metadata: digital_asset.metadata,
        edition: digital_asset.master_edition,
        owner_token_record: None,
        destination_token_record: None,
        signer_withdraw_wallet: creator_token_account,
        system_program: system_program::id(),
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        rent: sysvar::rent::id(),
    };
    let accounts_meta = accounts.to_account_metas(None);

    let data = boync_anchor_program::instruction::InitializeAuction2 {
        app_idx: timestamp,
        state_bump: auction_bump,
        fp: 3 * ONE_SOL,
        start_at: timestamp,
        end_at: timestamp + ((30 as i64) * ONE_MINUTE_IN_MSEC),
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
